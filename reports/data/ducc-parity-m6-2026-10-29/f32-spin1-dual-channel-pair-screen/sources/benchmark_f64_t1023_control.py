import argparse
import hashlib
import json
import os
import pathlib
import random
import statistics
import subprocess
import sys

def run(binary, case, fixture, outputs, target_ms, cpus):
    args = [str(binary), '--worker', 'synthesis', 'GL', str(case['threads']), '1024', '2048', '1023', '1023', '0.23',
            *(str(fixture / name) for name in ('map0.f64', 'map1.f64', 'e.f64', 'b.f64')),
            *(str(path) for path in outputs), str(target_ms)]
    env = os.environ.copy()
    env['RAYON_NUM_THREADS'] = str(case['threads'])
    result = subprocess.run(['taskset', '-c', cpus, *args], text=True, capture_output=True, env=env, check=True)
    return json.loads(result.stdout.strip().splitlines()[-1])

def digest(path):
    h = hashlib.sha256()
    with pathlib.Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixture-dir', type=pathlib.Path, required=True)
    parser.add_argument('--output-dir', type=pathlib.Path, required=True)
    parser.add_argument('--parent', type=pathlib.Path, required=True)
    parser.add_argument('--candidate', type=pathlib.Path, required=True)
    parser.add_argument('--ducc', type=pathlib.Path, required=True)
    parser.add_argument('--q8-pairs', type=int, default=15)
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    bins = {'parent': args.parent, 'candidate': args.candidate, 'ducc': args.ducc}
    orders = (('parent', 'candidate', 'ducc'), ('ducc', 'parent', 'candidate'), ('candidate', 'ducc', 'parent'))
    inputs = [args.fixture_dir / name for name in ('map0.f64', 'map1.f64', 'e.f64', 'b.f64')]
    fixture_hashes = {path.name: digest(path) for path in inputs}
    for threads, pair_count in ((1, 3), (8, args.q8_pairs)):
        case = {'threads': threads}
        cpus = '0' if threads == 1 else '0,2,4,6,8,10,12,14'
        pairs = []
        for pair in range(1, pair_count + 1):
            row = {'pair': pair, 'order': orders[(pair - 1) % 3], 'workers': {}}
            outputs = {}
            for backend in row['order']:
                out = [args.output_dir / f'q{threads}-pair{pair}-{backend}-map{i}.f64' for i in range(2)]
                dummy = [args.output_dir / f'q{threads}-pair{pair}-{backend}-alm{i}.f64' for i in range(2)]
                worker = run(bins[backend], case, args.fixture_dir, out + dummy, 25.0, cpus)
                worker['binary_sha256'] = digest(bins[backend])
                worker['outputs'] = [str(path) for path in out]
                row['workers'][backend] = worker
                outputs[backend] = out
            times = {key: statistics.median(row['workers'][key]['batch_ms']) for key in bins}
            row['times_ms'] = times
            row['candidate_parent_ratio'] = times['candidate'] / times['parent']
            row['candidate_ducc_ratio'] = times['candidate'] / times['ducc']
            row['fixture_sha256'] = fixture_hashes
            row['output_sha256'] = {key: [digest(path) for path in values] for key, values in outputs.items()}
            row['checks'] = {key: {'repeat_consistent': value['repeat_consistent'],
                                  'reset_consistent': value['reset_consistent']} for key, value in row['workers'].items()}
            pairs.append(row)
            (args.output_dir / f'q{threads}-pair{pair}.json').write_text(json.dumps(row, indent=2) + '\n')
            for values in outputs.values():
                for path in values:
                    path.unlink(missing_ok=True)
        ratios = [row['candidate_parent_ratio'] for row in pairs]
        summary = {'pairs': pair_count, 'case': {'grid': 'GL', 'ntheta': 1024, 'nphi': 2048, 'lmax': 1023,
                   'mmax': 1023, 'spin': 1, 'operation': 'synthesis', 'threads': threads},
                   'fixture_sha256': fixture_hashes, 'pairs_data': pairs,
                   'parent_median_ms': statistics.median(row['times_ms']['parent'] for row in pairs),
                   'candidate_median_ms': statistics.median(row['times_ms']['candidate'] for row in pairs),
                   'ducc_median_ms': statistics.median(row['times_ms']['ducc'] for row in pairs),
                   'candidate_parent_paired_median': statistics.median(ratios),
                   'candidate_parent_pair_range': [min(ratios), max(ratios)],
                   'candidate_parent_output_hashes_equal': all(row['output_sha256']['parent'] == row['output_sha256']['candidate'] for row in pairs),
                   'all_checks': all(all(value for value in row['checks'][key].values()) for row in pairs for key in bins)}
        (args.output_dir / f'q{threads}-summary.json').write_text(json.dumps(summary, indent=2) + '\n')

if __name__ == '__main__':
    main()
