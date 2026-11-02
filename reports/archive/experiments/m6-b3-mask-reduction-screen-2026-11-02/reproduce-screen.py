#!/usr/bin/env python3
"""Interleaved three-way native spin-1 analysis screen for GL T720."""
import argparse
import hashlib
import json
import math
import pathlib
import statistics
import sys
import tempfile

import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument('--repo', type=pathlib.Path, required=True)
parser.add_argument('--parent', type=pathlib.Path, required=True)
parser.add_argument('--candidate', type=pathlib.Path, required=True)
parser.add_argument('--ducc', type=pathlib.Path, required=True)
parser.add_argument('--results', type=pathlib.Path, required=True)
args = parser.parse_args()
repo, parent, candidate, ducc = (p.resolve() for p in
                                 (args.repo, args.parent, args.candidate, args.ducc))
sys.path.insert(0, str(repo / 'scripts'))
import benchmark_ducc_spin1_native as bench

physical = bench.common.physical_cpus()
monitor_cpu = physical[8] if len(physical) > 8 else physical[0]
cases = [case for case in bench.make_cases([720], ['GL'], False)
         if case['longitude_mode'] == 'even-smooth' and case['ntheta'] == 721]
assert [(case['mmax'], case['nphi']) for case in cases] == [(180, 378), (720, 1458)]
results = []
for case in cases:
    with tempfile.TemporaryDirectory(prefix='b3-mask-fixture-') as fixture_dir:
        fixture = bench.make_fixture(case, pathlib.Path(fixture_dir))
        for threads in (1, 8):
            cpus = physical[:threads]
            for pair in range(3):
                order = (['parent', 'candidate', 'ducc'] if pair == 0 else
                         ['ducc', 'candidate', 'parent'] if pair == 1 else
                         ['candidate', 'parent', 'ducc'])
                records, outputs = {}, {}
                for backend in order:
                    out = {name: pathlib.Path(fixture_dir) / f'{backend}-{pair}-{name}.bin'
                           for name in ('map0', 'map1', 'e', 'b')}
                    executable = ducc if backend == 'ducc' else (parent if backend == 'parent' else candidate)
                    command = bench.worker_command(executable, case, fixture, 'analysis',
                                                   threads, out, 25.0)
                    record, _host = bench.common.run_pinned(command, cpus, monitor_cpu)
                    records[backend] = record
                    outputs[backend] = out
                arrays = {
                    backend: np.concatenate([
                        bench.output_values(outputs[backend]['e'], 'complex'),
                        bench.output_values(outputs[backend]['b'], 'complex'),
                    ])
                    for backend in ('parent', 'candidate', 'ducc')
                }
                exact_parent = np.array_equal(arrays['parent'].view(np.uint64),
                                              arrays['candidate'].view(np.uint64))
                assert exact_parent, (case['case_id'], threads, pair, 'parent/candidate output mismatch')
                for backend in records:
                    assert records[backend]['repeat_consistent']
                    assert records[backend]['reset_consistent']
                parent_error = bench.common.error_metrics(arrays['parent'], arrays['ducc'])
                candidate_error = bench.common.error_metrics(arrays['candidate'], arrays['ducc'])
                times = {key: float(np.median(records[key]['batch_ms'])) for key in records}
                results.append({
                    'case_id': case['case_id'], 'mmax': case['mmax'], 'nphi': case['nphi'],
                    'threads': threads, 'pair': pair, 'order': order,
                    'times_ms': times,
                    'candidate_parent_ratio': times['candidate'] / times['parent'],
                    'candidate_ducc_ratio': times['candidate'] / times['ducc'],
                    'parent_ducc_ratio': times['parent'] / times['ducc'],
                    'parent_ducc_error': parent_error,
                    'candidate_ducc_error': candidate_error,
                    'fixture_sha256': fixture['sha256'],
                    'worker_measurements': {
                        key: {name: records[key].get(name) for name in
                              ('first_ms', 'warmup_ms', 'setup_ms', 'fixture_load_ms',
                               'pool_ms', 'repetitions_per_batch', 'batch_ms',
                               'batch_total_ms', 'repeat_consistent', 'reset_consistent')}
                        for key in records
                    },
                })
                print(f"PAIR {case['case_id']} q{threads} p{pair} "
                      f"parent={times['parent']:.6f} candidate={times['candidate']:.6f} "
                      f"ducc={times['ducc']:.6f} c/p={times['candidate']/times['parent']:.4f} "
                      f"c/d={times['candidate']/times['ducc']:.4f} bitwise={exact_parent}",
                      flush=True)

summary = []
for case in cases:
    for threads in (1, 8):
        group = [row for row in results if row['case_id'] == case['case_id']
                 and row['threads'] == threads]
        for ratio_key in ('candidate_parent_ratio', 'candidate_ducc_ratio', 'parent_ducc_ratio'):
            ratios = [row[ratio_key] for row in group]
            logs = [math.log(value) for value in ratios]
            mean = statistics.mean(logs)
            half = 4.303 * statistics.stdev(logs) / math.sqrt(len(logs))
            summary.append({'case_id': case['case_id'], 'threads': threads,
                            'ratio': ratio_key, 'geomean': math.exp(mean),
                            'ci95_t_log': [math.exp(mean-half), math.exp(mean+half)],
                            'paired': ratios})

payload = {
    'source_parent': 'be8f72d7fc55c420da5cf12b843b9353d2316eec',
    'source_candidate_base': 'be8f72d7fc55c420da5cf12b843b9353d2316eec',
    'candidate_change': 'combine finite and in-range SIMD lane predicates before the common-path horizontal reduction; retain finite fallback and scaling logic',
    'ducc_revision': '42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
    'parent_binary_sha256': hashlib.sha256(parent.read_bytes()).hexdigest(),
    'candidate_binary_sha256': hashlib.sha256(candidate.read_bytes()).hexdigest(),
    'ducc_binary_sha256': hashlib.sha256(ducc.read_bytes()).hexdigest(),
    'cpu_model': bench.common.cpu_model(), 'cpu_features': sorted(bench.common.cpu_flags()),
    'allowed_physical_cpus': physical, 'q1_cpus': physical[:1], 'q8_cpus': physical[:8],
    'rust_toolchain': bench.common.command_output(['rustc', '-Vv'], repo),
    'rust_flags': '-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
    'ducc_flags': 'SHT -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math',
    'target_ms': 25.0, 'process_pairs': 3,
    'geometry': [{'case_id': c['case_id'], 'mmax': c['mmax'], 'nphi': c['nphi'],
                  'ntheta': c['ntheta'], 'lmax': c['lmax']} for c in cases],
    'results': results, 'summary': summary,
}
args.results.parent.mkdir(parents=True, exist_ok=True)
args.results.write_text(json.dumps(payload, indent=2, allow_nan=False) + '\n')
print('SUMMARY')
for row in summary:
    print(json.dumps(row, sort_keys=True))
print('JSON', args.results)
