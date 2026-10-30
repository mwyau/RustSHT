import hashlib
import json
import os
import random
import statistics
import subprocess
import sys
import time
from pathlib import Path

CANDIDATE = Path(os.environ.get('RUSTSHT_REPO', Path(__file__).resolve().parents[4]))
ROOT = Path(os.environ.get('B2_SCREEN_ROOT', Path.home() / 'RustSHT-m6-b2-current-source-15pair'))
F32_BIN = ROOT / 'target/release/examples/ducc_f32_full_transform_parity'
F64_BIN = ROOT / 'f64-target/release/m6_b2_f64_worker'
DUCC_BIN = Path(os.environ['DUCC_BIN'])
OUT = ROOT / 'b2-results'
SCRATCH = ROOT / 'b2-scratch'
TARGET_MS = 25.0
PAIRS = 15

sys.path.insert(0, str(CANDIDATE / 'scripts'))
import benchmark_ducc_f32_native as bench

def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()

def git(*args):
    return subprocess.check_output(['git', '-C', str(CANDIDATE), *args], text=True).strip()

def f64_worker(case, fixture, run_dir):
    q = case['threads']
    cpus = '0' if q == 1 else '0-7'
    cmd = [
        str(F64_BIN), '--worker', case['operation'], str(case['spin']), str(q),
        str(case['ntheta']), str(case['nphi']), str(case['lmax']), str(case['mmax']),
        '0.23', str(fixture['map0']), str(fixture['map1']),
        str(fixture['alm0']), str(fixture['alm1']), str(TARGET_MS), '-',
    ]
    env = os.environ.copy()
    env['RAYON_NUM_THREADS'] = str(q)
    started = time.monotonic()
    proc = subprocess.run(['taskset', '-c', cpus, *cmd], text=True, capture_output=True, env=env, check=True)
    data = json.loads(proc.stdout.strip().splitlines()[-1])
    data['median_ms'] = statistics.median(data['batch_ms'])
    data['process_elapsed_s'] = time.monotonic() - started
    data['binary_sha256'] = sha(F64_BIN)
    run_dir.mkdir(parents=True, exist_ok=True)
    (run_dir / 'result.json').write_text(json.dumps(data, indent=2) + '\n')
    return data

def bootstrap_ci(values, seed):
    rng = random.Random(seed)
    samples = sorted(statistics.median(rng.choices(values, k=len(values))) for _ in range(10000))
    return [samples[250], samples[9749]]

original_cases = bench.default_cases
cases = [case for case in original_cases(False)
         if case['grid'] == 'GL' and case['lmax'] == 720 and case['ntheta'] == 721
         and case['spin'] == 1 and case['operation'] == 'synthesis']
for case in cases:
    case_id = f"GL-T720-M{case['mmax']}-even-s1-synthesis-q{case['threads']}"
    case_dir = OUT / case_id
    case_dir.mkdir(parents=True, exist_ok=True)
    fixture = bench.make_fixture(case, SCRATCH / case_id)
    pair_rows = []
    for pair in range(1, PAIRS + 1):
        if pair % 3 == 1:
            order = ('rust', 'f64', 'ducc')
        elif pair % 3 == 2:
            order = ('ducc', 'rust', 'f64')
        else:
            order = ('f64', 'ducc', 'rust')
        results = {}
        for backend in order:
            run_dir = case_dir / f'pair-{pair}' / backend
            if backend in ('rust', 'ducc'):
                binary = F32_BIN if backend == 'rust' else DUCC_BIN
                cpus = '0' if case['threads'] == 1 else '0,1,2,3,4,5,6,7'
                data = bench.run_worker(binary, backend, case, fixture, run_dir, TARGET_MS, cpus)
                data['median_ms'] = statistics.median(data['batch_ms'])
                data['binary_sha256'] = sha(binary)
                (run_dir / 'result.json').write_text(json.dumps(data, indent=2) + '\n')
            else:
                data = f64_worker(case, fixture, run_dir)
            results[backend] = data
        row = {
            'pair': pair,
            'rust_f32_ms': results['rust']['median_ms'],
            'rust_f64_ms': results['f64']['median_ms'],
            'ducc_f32_ms': results['ducc']['median_ms'],
            'paired_ratio_f32_f64': results['rust']['median_ms'] / results['f64']['median_ms'],
            'paired_ratio_f32_ducc': results['rust']['median_ms'] / results['ducc']['median_ms'],
            'rust_f32': results['rust'],
            'rust_f64': results['f64'],
            'ducc_f32': results['ducc'],
        }
        pair_rows.append(row)
        for backend in ('rust', 'ducc'):
            for output_path in results[backend].get('outputs', []):
                Path(output_path).unlink(missing_ok=True)
        print(json.dumps({k: row[k] for k in ('pair', 'rust_f32_ms', 'rust_f64_ms', 'ducc_f32_ms', 'paired_ratio_f32_f64', 'paired_ratio_f32_ducc')}), flush=True)
    ratios64 = [row['paired_ratio_f32_f64'] for row in pair_rows]
    ratios_ducc = [row['paired_ratio_f32_ducc'] for row in pair_rows]
    summary = {
        **case,
        'case_id': case_id,
        'f32_median_ms': statistics.median(row['rust_f32_ms'] for row in pair_rows),
        'f64_median_ms': statistics.median(row['rust_f64_ms'] for row in pair_rows),
        'ducc_median_ms': statistics.median(row['ducc_f32_ms'] for row in pair_rows),
        'median_paired_ratio_f32_f64': statistics.median(ratios64),
        'paired_ratio_f32_f64_bootstrap_95_ci': bootstrap_ci(ratios64, 0xB2F64 + case['mmax'] + case['threads']),
        'median_paired_ratio_f32_ducc': statistics.median(ratios_ducc),
        'paired_ratio_f32_ducc_bootstrap_95_ci': bootstrap_ci(ratios_ducc, 0xB2D00C + case['mmax'] + case['threads']),
        'pairs': pair_rows,
    }
    (case_dir / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps({k: summary[k] for k in ('case_id', 'f32_median_ms', 'f64_median_ms', 'ducc_median_ms', 'median_paired_ratio_f32_f64', 'paired_ratio_f32_f64_bootstrap_95_ci', 'median_paired_ratio_f32_ducc', 'paired_ratio_f32_ducc_bootstrap_95_ci')}), flush=True)

candidate_diff = subprocess.check_output(['git', '-C', str(CANDIDATE), 'diff', '--binary', 'HEAD'])
metadata = {
    'source_base': git('rev-parse', 'HEAD'),
    'candidate_head': git('rev-parse', 'HEAD'),
    'candidate_patch_sha256': hashlib.sha256(candidate_diff).hexdigest(),
    'ducc_revision': bench.DUCC_SHA,
    'rust_flags': '-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
    'rustc_vv': subprocess.check_output(['rustc', '-vV'], text=True),
    'cpu_model': subprocess.check_output(['lscpu'], text=True).split('Model name:')[1].splitlines()[0].strip(),
    'affinity': {'q1': '0', 'q8': '0-7'},
    'process_pairs': PAIRS,
    'target_ms': TARGET_MS,
    'input_precision': 'f32 fixture widened component-wise to f64 only for the matched f64 diagnostic worker',
    'binary_sha256': {'rust_f32': sha(F32_BIN), 'rust_f64': sha(F64_BIN), 'ducc_f32': sha(DUCC_BIN)},
    'fixture_sha256': {p.name: {k: sha(v) for k, v in bench.make_fixture(cases[0], SCRATCH / p.name).items()} for p in []},
}
# Record fixture hashes from each case after all cases have been generated.
metadata['fixture_sha256'] = {}
for case in cases:
    cid = f"GL-T720-M{case['mmax']}-even-s1-synthesis-q{case['threads']}"
    fixture_dir = SCRATCH / cid
    metadata['fixture_sha256'][cid] = {name: sha(fixture_dir / f'{name}.f32') for name in ('map0', 'map1', 'alm0', 'alm1')}
metadata['results_sha256'] = {p.name: sha(p) for p in OUT.glob('GL-T720-*/summary.json')}
(OUT / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
print(f'SAVED {OUT / "metadata.json"}', flush=True)
