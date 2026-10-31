import argparse, hashlib, importlib.util, json, os, statistics, subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--baseline-dir', type=Path, required=True, help='matched fixtures, DUCC binaries and baseline runner')
parser.add_argument('--parent32-bin', type=Path, required=True)
parser.add_argument('--parent64-bin', type=Path, required=True)
parser.add_argument('--candidate-root', type=Path, default=Path.cwd())
parser.add_argument('--output-dir', type=Path, required=True)
args = parser.parse_args()
BASELINE, ROOT, OUT = args.baseline_dir, args.candidate_root, args.output_dir
spec = importlib.util.spec_from_file_location('baseline', BASELINE / 'run_baseline.py')
b = importlib.util.module_from_spec(spec)
spec.loader.exec_module(b)
b.TARGET_MS = 25.0

bins = {
    'parent32': args.parent32_bin,
    'parent64': args.parent64_bin,
    'candidate32': ROOT / 'target/release/examples/ducc_f32_full_transform_parity',
    'candidate64': ROOT / 'target/release/examples/ducc_full_transform_parity',
    'ducc32': BASELINE / 'ducc-f32/ducc_f32_full_transform_parity',
    'ducc64': BASELINE / 'ducc-f64-scalar/ducc_full_transform_parity',
}
for path in bins.values():
    if not path.is_file():
        raise SystemExit(f'missing binary: {path}')

allowed = sorted(os.sched_getaffinity(0))
cpus = {1: str(allowed[0]), 8: ','.join(map(str, allowed[:8]))}
cases = [c for c in b.bench.default_cases(False)
         if c['grid'] == 'GL' and c['lmax'] == 720 and c['spin'] == 0
         and c['operation'] == 'synthesis' and c['ntheta'] == 721
         and c['mmax'] in (720, 180) and c['longitude'] == 'even']
rows = []
for case in cases:
    cid = b.case_id(case)
    fixture32dir = BASELINE / 'results/fixtures32' / cid
    fixture64dir = BASELINE / 'results/fixtures64' / cid
    fixture32 = {k: fixture32dir / f'{k}.f32' for k in ('map0', 'map1', 'alm0', 'alm1')}
    fixture64 = {k: fixture64dir / f'{k}.f64' for k in ('map0', 'map1', 'alm0', 'alm1')}
    for path in (*fixture32.values(), *fixture64.values()):
        if not path.is_file():
            raise SystemExit(f'missing fixture: {path}')
    case_rows = []
    order = ['parent32', 'parent64', 'candidate32', 'candidate64', 'ducc32', 'ducc64']
    for pair in range(1, 2):
        run_order = order if pair == 1 else (list(reversed(order)) if pair == 2 else order[2:] + order[:2])
        result = {}
        for name in run_order:
            run_dir = OUT / cid / f'pair-{pair}' / name
            affinity = cpus[case['threads']]
            if name.endswith('32'):
                data = b.run_f32(bins[name], case, fixture32, run_dir, name, affinity)
            else:
                data = b.run_f64(bins[name], case, fixture64, run_dir, name, affinity)
            result[name] = data
        ms = {name: b.median_ms(result[name]) for name in order}
        values = {name: b.read_outputs(result[name], 'f32' if name.endswith('32') else 'f64', case, name)
                  for name in order}
        row = {
            'pair': pair,
            'ms': ms,
            'parent32_over_parent64': ms['parent32'] / ms['parent64'],
            'candidate32_over_candidate64': ms['candidate32'] / ms['candidate64'],
            'parent32_over_ducc32': ms['parent32'] / ms['ducc32'],
            'candidate32_over_ducc32': ms['candidate32'] / ms['ducc32'],
            'candidate64_over_ducc64': ms['candidate64'] / ms['ducc64'],
            'candidate32_over_parent32': ms['candidate32'] / ms['parent32'],
            'output_rel_l2': {
                'candidate32_vs_parent32': b.rel_l2(values['candidate32'], values['parent32']),
                'candidate32_vs_ducc32': b.rel_l2(values['candidate32'], values['ducc32']),
                'parent32_vs_ducc32': b.rel_l2(values['parent32'], values['ducc32']),
                'candidate32_vs_candidate64': b.rel_l2(values['candidate32'], values['candidate64']),
                'parent32_vs_parent64': b.rel_l2(values['parent32'], values['parent64']),
                'ducc32_vs_ducc64': b.rel_l2(values['ducc32'], values['ducc64']),
                'candidate64_vs_parent64': b.rel_l2(values['candidate64'], values['parent64']),
                'candidate64_vs_ducc64': b.rel_l2(values['candidate64'], values['ducc64']),
            },
            'dispatch': {name: {key: value.get(key) for key in ('simd_arch', 'f32_lanes', 'backend', 'output_checksum', 'repeat_consistent', 'reset_consistent') if key in value}
                         for name, value in result.items()},
            'fixture_sha256': {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in fixture32.items()},
        }
        case_rows.append(row)
        print(json.dumps({'case': cid, **{k: row[k] for k in ('pair', 'ms', 'candidate32_over_parent32', 'candidate32_over_candidate64', 'candidate32_over_ducc32', 'candidate64_over_ducc64', 'output_rel_l2')}}, sort_keys=True), flush=True)
        b.unpin_outputs(*result.values())
    medians = {key: statistics.median(row[key] for row in case_rows)
               for key in ('candidate32_over_parent32', 'candidate32_over_candidate64', 'candidate32_over_ducc32', 'candidate64_over_ducc64')}
    summary = {'case': cid, 'medians': medians, 'pairs': case_rows}
    (OUT / cid / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    rows.append(summary)

patch = subprocess.check_output(['git', '-C', str(ROOT), 'diff', '--binary', 'HEAD'], text=False)
metadata = {
    'source_parent_sha': subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip(),
    'candidate_patch_sha256': hashlib.sha256(patch).hexdigest(),
    'ducc_sha': '42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
    'rust_flags': '-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
    'cpu_model': subprocess.check_output(['lscpu'], text=True).split('Model name:')[1].splitlines()[0].strip(),
    'affinity_policy': {1: 'first allowed CPU', 8: 'first eight allowed CPUs'},
    'binary_sha256': {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in bins.items()},
    'fixture_sha256': {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in fixture32.items()},
    'pairs': 3,
    'target_batch_ms': b.TARGET_MS,
    'cells': rows,
}
(OUT / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
