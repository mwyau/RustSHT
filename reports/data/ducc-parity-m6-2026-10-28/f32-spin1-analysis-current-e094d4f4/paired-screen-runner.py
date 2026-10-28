import importlib.util
import json
import statistics
import hashlib
from pathlib import Path

root = Path(__file__).parent
repo = Path(__file__).resolve().parents[4]
spec = importlib.util.spec_from_file_location('bench', repo / 'scripts/benchmark_ducc_f32_native.py')
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)
bins = {
    'rust': root / 'cargo-current/release/examples/ducc_f32_full_transform_parity',
    'ducc': root / 'ducc-build/ducc_f32_full_transform_parity',
}
fixture_root = root / 'fixtures'
results_root = root / 'results'
results_root.mkdir(exist_ok=True)
cases = []
for threads in (1, 8):
    cases.append({
        'grid': 'GL', 'ntheta': 721, 'nphi': 1458, 'lmax': 720,
        'mmax': 720, 'longitude': 'even', 'spin': 1,
        'operation': 'analysis', 'threads': threads,
    })
rows = []
for case in cases:
    case_id = f"GL-T720-M720-s1-analysis-q{case['threads']}"
    fixture = bench.make_fixture(case, fixture_root / case_id)
    cpus = '3' if case['threads'] == 1 else '8-15'
    pairs = []
    for pair in range(1, 4):
        order = ('rust', 'ducc') if pair % 2 else ('ducc', 'rust')
        outputs = {}
        for backend in order:
            run_dir = results_root / case_id / f'pair-{pair:02d}' / backend
            outputs[backend] = bench.run_worker(bins[backend], backend, case, fixture, run_dir, 35.0, cpus)
        medians = {name: statistics.median(outputs[name]['batch_ms']) for name in ('rust', 'ducc')}
        error = bench.error_metrics(
            [Path(path) for path in outputs['ducc']['outputs']],
            [Path(path) for path in outputs['rust']['outputs']],
        )
        row = {
            'pair': pair, 'order': order, 'times_ms': medians,
            'rust_ducc_ratio': medians['rust'] / medians['ducc'],
            'error': error,
            'checks': {name: {key: outputs[name].get(key) for key in (
                'finite_outputs', 'repeat_consistent', 'reset_consistent', 'simd_arch', 'f32_lanes'
            )} for name in outputs},
        }
        pairs.append(row)
        for output in outputs.values():
            for path in output.get('outputs', []):
                try:
                    Path(path).unlink()
                except FileNotFoundError:
                    pass
        print(json.dumps({'case': case_id, **row}), flush=True)
    summary = {
        'case_id': case_id,
        'case': case,
        'pairs': pairs,
        'median_ms': {name: statistics.median(row['times_ms'][name] for row in pairs) for name in ('rust', 'ducc')},
        'paired_ratio_median': statistics.median(row['rust_ducc_ratio'] for row in pairs),
        'max_relative_l2': max(row['error']['relative_l2'] for row in pairs),
        'max_absolute': max(row['error']['max_absolute'] for row in pairs),
        'all_checks': all(
            all(output.get('finite_outputs') and output.get('repeat_consistent') and output.get('reset_consistent')
                for output in row['checks'].values())
            for row in pairs
        ),
        'fixture_sha256': {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in fixture.items()},
    }
    rows.append(summary)
    (results_root / f'{case_id}-summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps({'summary': summary}, indent=2), flush=True)
(results_root / 'summary.json').write_text(json.dumps({
    'source': 'e094d4f4b3a2d42f9ef7e94779ffc4afc078b198',
    'ducc_source': '42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
    'rust_flags': '-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
    'ducc_flags': '-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math',
    'process_pairs': 3,
    'cases': rows,
}, indent=2) + '\n')
