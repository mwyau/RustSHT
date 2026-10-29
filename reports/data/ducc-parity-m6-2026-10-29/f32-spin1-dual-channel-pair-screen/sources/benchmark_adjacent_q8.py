import hashlib
import importlib.util
import json
import pathlib
import statistics
import sys

ROOT = pathlib.Path(sys.argv[1])
REPO = pathlib.Path(sys.argv[2])
OUT = ROOT / 'results-adjacent-q8'
RUNS = ROOT / 'runs-adjacent-q8'
sys.path.insert(0, str(REPO / 'scripts'))
spec = importlib.util.spec_from_file_location('bench', REPO / 'scripts/benchmark_ducc_f32_native.py')
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)
bins = {'parent': ROOT / 'bin/parent', 'candidate': ROOT / 'bin/candidate', 'ducc': ROOT / 'bin/ducc'}
orderings = (('parent', 'candidate', 'ducc'), ('ducc', 'parent', 'candidate'), ('candidate', 'ducc', 'parent'))
cases = []
for spin in (0, 1):
    for mmax, nphi in ((720, 1458), (180, 378)):
        cases.append({'grid': 'GL', 'ntheta': 721, 'nphi': nphi, 'lmax': 720, 'mmax': mmax,
                      'longitude': 'even', 'spin': spin, 'operation': 'analysis', 'threads': 8})
OUT.mkdir(exist_ok=True)
RUNS.mkdir(exist_ok=True)
for case in cases:
    cid = f"GL-T720-M{case['mmax']}-s{case['spin']}-analysis-q8"
    fixture = bench.make_fixture(case, RUNS / 'fixtures' / cid)
    fixture_hashes = {key: hashlib.sha256(path.read_bytes()).hexdigest() for key, path in fixture.items()}
    pairs = []
    for pair in range(1, 4):
        row = {'pair': pair, 'order': orderings[pair - 1], 'workers': {}}
        outputs = {}
        for backend in row['order']:
            result = bench.run_worker(bins[backend], backend, case, fixture, RUNS / cid / f'pair-{pair:02}' / backend,
                                      25.0, '0,2,4,6,8,10,12,14')
            row['workers'][backend] = {key: value for key, value in result.items() if key != 'outputs'}
            outputs[backend] = [pathlib.Path(value) for value in result['outputs']]
        times = {key: statistics.median(row['workers'][key]['batch_ms']) for key in bins}
        row['times_ms'] = times
        row['candidate_parent_ratio'] = times['candidate'] / times['parent']
        row['candidate_ducc_ratio'] = times['candidate'] / times['ducc']
        row['errors'] = {'candidate_vs_parent': bench.error_metrics(outputs['parent'], outputs['candidate']),
                         'candidate_vs_ducc': bench.error_metrics(outputs['ducc'], outputs['candidate'])}
        row['output_sha256'] = {key: [hashlib.sha256(path.read_bytes()).hexdigest() for path in values]
                                for key, values in outputs.items()}
        row['fixture_sha256'] = fixture_hashes
        pairs.append(row)
        (OUT / f'{cid}-pair-{pair:02}.json').write_text(json.dumps(row, indent=2) + '\n')
        for values in outputs.values():
            for path in values:
                path.unlink(missing_ok=True)
    ratios = [row['candidate_parent_ratio'] for row in pairs]
    summary = {'case_id': cid, 'case': case, 'pairs': 3,
               'parent_median_ms': statistics.median(row['times_ms']['parent'] for row in pairs),
               'candidate_median_ms': statistics.median(row['times_ms']['candidate'] for row in pairs),
               'ducc_median_ms': statistics.median(row['times_ms']['ducc'] for row in pairs),
               'candidate_parent_paired_median': statistics.median(ratios),
               'candidate_parent_pair_range': [min(ratios), max(ratios)],
               'all_worker_checks': all(all(row['workers'][name].get(key) is True for key in
                   ('finite_outputs', 'repeat_consistent', 'reset_consistent')) for row in pairs for name in bins),
               'fixture_sha256': fixture_hashes, 'pairs_data': pairs}
    (OUT / f'{cid}-summary.json').write_text(json.dumps(summary, indent=2) + '\n')
