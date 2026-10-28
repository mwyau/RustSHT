#!/usr/bin/env python3
"""Three interleaved parent/candidate/DUCC adjacent and f64 control screens."""
import hashlib
import importlib.util
import json
import math
import os
import pathlib
import random
import statistics
import subprocess
import sys

import numpy as np

ROOT = pathlib.Path(os.environ['M6_EXPERIMENT_ROOT'])
REPO = pathlib.Path(os.environ.get('M6_REPO_ROOT', pathlib.Path(__file__).resolve().parents[4]))
OUT = ROOT / 'control-screens'
FIXTURES = ROOT / 'control-fixtures'
PARENT = ROOT / 'target-parent/release/examples'
CANDIDATE = ROOT / 'target-candidate/release/examples'
DUCC_PY = ROOT / 'ducc-venv/bin/python'
DUCC_WORKER = ROOT / 'ducc-worker-general.py'
sys.path.insert(0, str(REPO / 'scripts'))
spec = importlib.util.spec_from_file_location('f32bench', REPO / 'scripts/benchmark_ducc_f32_native.py')
f32bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f32bench)


def file_sha(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def nalm(lmax, mmax):
    return (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) // 2


def write_f64_fixture(case, directory):
    directory.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(0xF64D00D + case['lmax'] * 31 + case['nphi'])
    map_count = case['ntheta'] * case['nphi']
    count = nalm(case['lmax'], case['mmax'])
    maps = [rng.uniform(-1.0, 1.0, map_count).astype('<f8') for _ in range(2)]
    alms = []
    scale = 1.0 / (case['lmax'] + 1)
    for _ in range(2):
        real = rng.uniform(-scale, scale, count)
        imag = rng.uniform(-scale, scale, count)
        offset = 0
        for m in range(case['mmax'] + 1):
            size = case['lmax'] - m + 1
            if m == 0:
                imag[offset:offset + size] = 0.0
            offset += size
        values = (real + 1j * imag).astype('<c16')
        values[:1] = 0.0
        alms.append(values)
    paths = {}
    for name, values in (('map0', maps[0]), ('map1', maps[1]), ('alm0', alms[0]), ('alm1', alms[1])):
        path = directory / f'{name}.bin'
        values.tofile(path)
        paths[name] = path
    return paths


def rust_command(backend, case, fixture, outdir, target_ms):
    if backend.startswith('rust32'):
        binary = (PARENT if backend == 'rust32-parent' else CANDIDATE) / 'ducc_f32_full_transform_parity'
        outputs = [outdir / 'out0.f32', outdir / 'out1.f32']
        args = [str(binary), '--worker', case['operation'], str(case['spin']), case['grid'],
                str(case['threads']), str(case['ntheta']), str(case['nphi']), str(case['lmax']),
                str(case['mmax']), '0.23', str(fixture['map0']), str(fixture['map1']),
                str(fixture['alm0']), str(fixture['alm1']), str(outputs[0]), str(outputs[1]), str(target_ms)]
        return args, outputs
    binary = (PARENT if backend == 'rust64-parent' else CANDIDATE) / 'ducc_spin1_full_transform_parity'
    outputs = [outdir / 'out0.f64', outdir / 'out1.f64', outdir / 'oute.f64', outdir / 'outb.f64']
    args = [str(binary), '--worker', case['operation'], case['grid'], str(case['threads']),
            str(case['ntheta']), str(case['nphi']), str(case['lmax']), str(case['mmax']), '0.23',
            str(fixture['map0']), str(fixture['map1']), str(fixture['alm0']), str(fixture['alm1']),
            str(outputs[0]), str(outputs[1]), str(outputs[2]), str(outputs[3]), str(target_ms)]
    return args, outputs


def run_backend(backend, case, fixture, directory, target_ms=25.0):
    directory.mkdir(parents=True, exist_ok=True)
    if backend == 'ducc':
        precision = case['precision']
        outputs = [directory / f'out0.{precision}', directory / f'out1.{precision}']
        args = [str(DUCC_PY), str(DUCC_WORKER), '--worker', precision, case['operation'],
                str(case['spin']), case['grid'], str(case['threads']), str(case['ntheta']),
                str(case['nphi']), str(case['lmax']), str(case['mmax']), '0.23',
                str(fixture['map0']), str(fixture['map1']), str(fixture['alm0']), str(fixture['alm1']),
                str(outputs[0]), str(outputs[1]), str(target_ms)]
        env = os.environ.copy()
        env['PYTHONPATH'] = str(ROOT / 'ducc-install')
        env.pop('M6_PROFILE_LEGENDRE', None)
    else:
        args, outputs = rust_command(backend, case, fixture, directory, target_ms)
        env = os.environ.copy()
        env['RAYON_NUM_THREADS'] = str(case['threads'])
    cpus = '0' if case['threads'] == 1 else '0,2,4,6,8,10,12,14'
    started = __import__('time').monotonic()
    done = subprocess.run(['taskset', '-c', cpus, *args], env=env, text=True,
                          capture_output=True, timeout=600)
    if done.returncode:
        raise RuntimeError(f'{backend} failed {case}:\n{done.stderr}\n{done.stdout}')
    result = json.loads(done.stdout.strip().splitlines()[-1])
    result['process_elapsed_s'] = __import__('time').monotonic() - started
    result['outputs'] = [str(path) for path in outputs if path.exists() and path.stat().st_size]
    if case['operation'] == 'analysis':
        output_dtype = np.dtype('<c8' if case['precision'] == 'f32' else '<c16')
    else:
        output_dtype = np.dtype('<f4' if case['precision'] == 'f32' else '<f8')
    result['finite_outputs'] = all(
        np.isfinite(np.fromfile(path, dtype=output_dtype).real).all()
        and np.isfinite(np.fromfile(path, dtype=output_dtype).imag).all()
        for path in result['outputs']
    )
    return result


def error_metrics(ref_paths, actual_paths, dtype):
    ref = np.concatenate([np.fromfile(path, dtype=dtype) for path in ref_paths])
    actual = np.concatenate([np.fromfile(path, dtype=dtype) for path in actual_paths])
    diff = actual - ref
    refnorm = np.linalg.norm(ref)
    return {
        'count': int(ref.size), 'relative_l2': float(np.linalg.norm(diff) / max(refnorm, np.finfo(float).tiny)),
        'max_absolute': float(np.max(np.abs(diff), initial=0.0)),
        'bitwise_equal': bool(np.array_equal(ref.view(np.uint8), actual.view(np.uint8))),
    }


def run_screen(cases, kind, pair_count=3):
    (OUT / kind).mkdir(parents=True, exist_ok=True)
    rows = []
    rng = random.Random(0x6C0A7)
    orders = [('parent', 'candidate', 'ducc'), ('ducc', 'candidate', 'parent'), ('candidate', 'parent', 'ducc')]
    for case in cases:
        case_id = f"{case['precision']}-{case['grid']}-T{case['lmax']}-M{case['mmax']}-s{case['spin']}-{case['operation']}-q{case['threads']}"
        fixture_dir = FIXTURES / case_id
        if case['precision'] == 'f32':
            fixture = f32bench.make_fixture(case, fixture_dir)
        else:
            fixture = write_f64_fixture(case, fixture_dir)
        pair_rows = []
        for pair_idx in range(pair_count):
            backends = list(orders[pair_idx])
            if rng.random() < 0.5:
                backends.reverse()
            samples = {}
            for backend in backends:
                key = {'parent': f"rust{case['precision'][-2:]}-parent", 'candidate': f"rust{case['precision'][-2:]}-candidate", 'ducc': 'ducc'}[backend]
                # Current control matrix uses f32 worker for f32 and the f64 spin worker for f64.
                if case['precision'] == 'f32':
                    key = 'rust32-parent' if backend == 'parent' else ('rust32-candidate' if backend == 'candidate' else 'ducc')
                else:
                    key = 'rust64-parent' if backend == 'parent' else ('rust64-candidate' if backend == 'candidate' else 'ducc')
                run_dir = OUT / kind / case_id / f'pair-{pair_idx+1}' / backend
                samples[backend] = run_backend(key, case, fixture, run_dir)
            medians = {name: statistics.median(value['batch_ms']) for name, value in samples.items()}
            dtype = np.dtype('<f4' if case['precision'] == 'f32' else '<f8')
            if case['operation'] == 'analysis':
                dtype = np.dtype('<c8' if case['precision'] == 'f32' else '<c16')
            output_count = 2 if case['spin'] == 1 else 1
            ref = samples['parent']['outputs'][:output_count]
            cand = samples['candidate']['outputs'][:output_count]
            ducc = samples['ducc']['outputs'][:output_count]
            pair = {
                'pair': pair_idx + 1, 'order': backends, 'times_ms': medians,
                'candidate_parent': medians['candidate'] / medians['parent'],
                'candidate_ducc': medians['candidate'] / medians['ducc'],
                'errors_candidate_parent': error_metrics(ref, cand, dtype),
                'errors_candidate_ducc': error_metrics(ducc, cand, dtype),
                'finite_repeat_reset': {name: {k: samples[name][k] for k in ('finite_outputs', 'repeat_consistent', 'reset_consistent')} for name in samples},
            }
            pair_rows.append(pair)
            (OUT / kind / case_id / f'pair-{pair_idx+1}.json').write_text(json.dumps(pair, indent=2) + '\n')
        row = {
            'case': case, 'case_id': case_id,
            'parent_ms': statistics.median(p['times_ms']['parent'] for p in pair_rows),
            'candidate_ms': statistics.median(p['times_ms']['candidate'] for p in pair_rows),
            'ducc_ms': statistics.median(p['times_ms']['ducc'] for p in pair_rows),
            'candidate_parent_median': statistics.median(p['candidate_parent'] for p in pair_rows),
            'candidate_ducc': statistics.median(p['candidate_ducc'] for p in pair_rows),
            'pairs': pair_rows,
        }
        rows.append(row)
        print(json.dumps({k: row[k] for k in ('case_id', 'parent_ms', 'candidate_ms', 'ducc_ms', 'candidate_parent_median', 'candidate_ducc')}), flush=True)
    (OUT / kind / 'results.json').write_text(json.dumps({
        'parent_sha': '49295a89f2d13d8b02c2cbfa8566a8c5c21c6a80',
        'candidate_source': 'isolated reflected-ring shared-kernel patch',
        'ducc_sha': '42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
        'rust_flags': '-C target-cpu=x86-64-v3', 'ducc_flags': '-O3 -ffast-math -march=x86-64-v3 -ffp-contract=fast',
        'pairs_per_case': 3, 'target_ms': 25.0, 'rows': rows,
    }, indent=2) + '\n')


def main():
    global OUT
    args = set(sys.argv[1:])
    pair_count = next((int(arg.split('=', 1)[1]) for arg in args if arg.startswith('--pairs=')), 3)
    if '--v2' in args:
        OUT = ROOT / 'control-screens-v2'
    f32_cases = []
    for mmax, nphi in ((720, 1458), (180, 378)):
        for threads in (1, 8):
            f32_cases.append({'precision': 'f32', 'grid': 'GL', 'ntheta': 721, 'nphi': nphi,
                              'lmax': 720, 'mmax': mmax, 'spin': 0, 'operation': 'analysis', 'threads': threads})
    for operation in ('analysis', 'synthesis'):
        for threads in (1, 8):
            f32_cases.append({'precision': 'f32', 'grid': 'GL', 'ntheta': 721, 'nphi': 1458,
                              'lmax': 720, 'mmax': 720, 'spin': 1, 'operation': operation, 'threads': threads})
    f64_cases = []
    for threads in (1, 8):
        f64_cases.append({'precision': 'f64', 'grid': 'GL', 'ntheta': 1024, 'nphi': 2048,
                          'lmax': 1023, 'mmax': 1023, 'spin': 1, 'operation': 'synthesis', 'threads': threads})
    if '--f64-only' in args:
        run_screen(f64_cases, 'f64-protected', pair_count)
    elif '--f32-only' in args:
        run_screen(f32_cases, 'f32-adjacent', pair_count)
    else:
        run_screen(f32_cases, 'f32-adjacent', pair_count)
        run_screen(f64_cases, 'f64-protected', pair_count)


if __name__ == '__main__':
    main()
