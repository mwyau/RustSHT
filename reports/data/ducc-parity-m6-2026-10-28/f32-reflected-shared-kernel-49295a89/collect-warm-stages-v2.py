#!/usr/bin/env python3
"""Collect warmed, instrumented Legendre-stage diagnostics for M6."""
import json
import os
import pathlib
import statistics
import subprocess

ROOT = pathlib.Path(os.environ['M6_EXPERIMENT_ROOT'])
OUT = ROOT / 'warm-stage-profiles-v2'
OUT.mkdir(parents=True, exist_ok=True)
RUST = {
    'parent': ROOT / 'target-profile-parent/release/examples/ducc_f32_full_transform_parity',
    'candidate': ROOT / 'target-profile-candidate/release/examples/ducc_f32_full_transform_parity',
}
DUCC_PYTHON = ROOT / 'ducc-venv/bin/python'
DUCC_WORKER = ROOT / 'ducc-worker.py'


def fields(line):
    parts = line.split('\t')
    return {part.split('=', 1)[0]: part.split('=', 1)[1] for part in parts[1:] if '=' in part}


def main():
    rows = []
    for threads in (1, 8):
        cpus = '0' if threads == 1 else '0,2,4,6,8,10,12,14'
        for mmax, nphi in ((720, 1458), (180, 378)):
            case = f'GL-T720-M{mmax}-synthesis-q{threads}'
            fixture = ROOT / 'runs-15/fixtures' / case
            for backend in ('parent', 'candidate', 'ducc'):
                output_path = OUT / f'{case}-{backend}.f32'
                args = [
                    '--worker', 'synthesis', '0', 'GL', str(threads), '721', str(nphi),
                    '720', str(mmax), '0.23', str(fixture / 'map0.f32'),
                    str(fixture / 'map1.f32'), str(fixture / 'alm0.f32'),
                    str(fixture / 'alm1.f32'), str(output_path), str(OUT / 'unused1.f32'), '25',
                ]
                env = os.environ.copy()
                env['RAYON_NUM_THREADS'] = str(threads)
                env['M6_PROFILE_LEGENDRE'] = '1'
                env.pop('M6_PROFILE_EXIT_AFTER_FIRST', None)
                env.pop('M6_PROFILE_ORDER_JOBS', None)
                env.pop('M6_PROFILE_ORDER_LINES', None)
                if backend == 'ducc':
                    env['PYTHONPATH'] = str(ROOT / 'ducc-profile-install')
                    env['M6_COUNT_DUCC_UPDATES'] = '1'
                    command = [str(DUCC_PYTHON), str(DUCC_WORKER), *args]
                else:
                    env.pop('M6_COUNT_DUCC_UPDATES', None)
                    env['M6_PROFILE_WARM_STAGE'] = '1'
                    command = [str(RUST[backend]), *args]
                p = subprocess.run(['taskset', '-c', cpus, *command], env=env,
                                   text=True, capture_output=True, timeout=240)
                if p.returncode:
                    raise RuntimeError(f'{backend} failed {case}:\n{p.stderr}\n{p.stdout}')
                lines = p.stderr.splitlines()
                rust_stage = [fields(x) for x in lines if x.startswith('M6_RUST_WARM_STAGE\t')]
                rust_workers = [fields(x) for x in lines if x.startswith('M6_RUST_WARM_WORKER\t')]
                rust_reflected = [fields(x) for x in lines if x.startswith('M6_RUST_WARM_REFLECTED_ROUTE\t')]
                ducc_stage = [fields(x) for x in lines if x.startswith('M6_DUCC_LEGENDRE_PROFILE\t')]
                ducc_stage_ms = [float(x['stage_ms']) for x in ducc_stage]
                row = {
                    'case': case, 'backend': backend, 'affinity': cpus,
                    'returncode': p.returncode, 'stdout': p.stdout, 'stderr': p.stderr,
                    'rust_stage': rust_stage, 'rust_workers': rust_workers,
                    'rust_reflected_route': rust_reflected,
                    'ducc_stage': ducc_stage,
                    'ducc_stage_ms_all': ducc_stage_ms,
                    'ducc_stage_ms_last10': ducc_stage_ms[-10:],
                    'ducc_stage_ms_last10_median': statistics.median(ducc_stage_ms[-10:]) if len(ducc_stage_ms) >= 10 else None,
                    'rust_stage_ms_samples': [float(x['nanos']) / 1e6 for x in rust_stage],
                    'rust_stage_ms_median': statistics.median(float(x['nanos']) / 1e6 for x in rust_stage) if rust_stage else None,
                }
                rows.append(row)
                (OUT / f'{case}-{backend}.json').write_text(json.dumps(row, indent=2) + '\n')
                print(json.dumps({
                    'case': case, 'backend': backend,
                    'rust_stage_ms': row['rust_stage_ms_samples'],
                    'ducc_stage_ms_count': len(ducc_stage_ms),
                    'ducc_last10_median_ms': row['ducc_stage_ms_last10_median'],
                    'rust_warm_worker_rows': len(rust_workers),
                    'ducc_last_profile': ducc_stage[-1] if ducc_stage else None,
                }), flush=True)
    (OUT / 'results.json').write_text(json.dumps({
        'rust_parent_sha': '49295a89f2d13d8b02c2cbfa8566a8c5c21c6a80',
        'candidate_patch_sha256': '5b60814d0f1e5adc3274ef53ca1fea904be2d5478ac720b9046a2e30657b9ca4',
        'ducc_sha': '42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
        'rust_flags': '-C target-cpu=x86-64-v3',
        'ducc_flags': '-O3 -ffast-math -march=x86-64-v3 -ffp-contract=fast',
        'method': 'separate instrumented build; Rust first call followed by 6 warmed stage samples; DUCC warmed worker emits per-call stage times and update counts; instrumented values are diagnostic, not production latency',
        'rows': rows,
    }, indent=2) + '\n')

if __name__ == '__main__':
    main()
