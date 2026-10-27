"""Matched parent/candidate/pinned-DUCC CC T720 reduced spin-1 synthesis screen."""
from __future__ import annotations

import csv
import hashlib
import json
import os
import pathlib
import statistics
import subprocess
import sys

import numpy as np

REPO = pathlib.Path(os.environ.get('RUSTSHT_REPO', pathlib.Path(__file__).resolve().parents[3]))
SCRATCH = pathlib.Path(os.environ['RUSTSHT_M6_SCRATCH'])
sys.path.insert(0, str(REPO / 'scripts'))
import benchmark_ducc_full_transform_parity as common  # noqa: E402
import benchmark_ducc_spin1_native as bench  # noqa: E402

PARENT = SCRATCH / 'parent'
CANDIDATE = SCRATCH / 'candidate'
DUCC_SOURCE = SCRATCH / 'ducc-source'
RESULTS = SCRATCH / 'results'
DUCC_PIN = '42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b'


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            digest.update(chunk)
    return digest.hexdigest()


def map_pair(paths: dict[str, pathlib.Path]) -> np.ndarray:
    return np.concatenate((bench.output_values(paths['map0'], 'real'),
                           bench.output_values(paths['map1'], 'real')))


def bitwise_equal(left: np.ndarray, right: np.ndarray) -> bool:
    return left.shape == right.shape and np.array_equal(
        left.view(np.uint64), right.view(np.uint64)
    )


def compact_diag(diagnostics: dict) -> dict:
    frequencies = diagnostics['frequency_ghz']
    return {
        'wait_fraction': diagnostics['wait_fraction'],
        'involuntary_switches': diagnostics['involuntary_switches'],
        'frequency_samples': len(frequencies),
        'frequency_ghz_median': float(np.median(frequencies)) if frequencies else None,
        'frequency_ghz_p10': float(np.quantile(frequencies, 0.1)) if frequencies else None,
        'frequency_ghz_p90': float(np.quantile(frequencies, 0.9)) if frequencies else None,
    }


def main() -> int:
    binaries = {
        'parent': SCRATCH / 'build/parent-target/release/examples/ducc_spin1_full_transform_parity',
        'candidate': SCRATCH / 'build/candidate-target/release/examples/ducc_spin1_full_transform_parity',
        'ducc': SCRATCH / 'build/ducc-build/ducc_spin1_full_transform_parity',
    }
    for name, binary in binaries.items():
        if not binary.is_file():
            raise FileNotFoundError(f'missing {name} binary: {binary}')

    case = {
        'case_id': 'CC-T720-m180-n722-exact-p378-even-smooth',
        'grid': 'CC', 'lmax': 720, 'mmax': 180, 'ntheta': 722, 'nphi': 378,
        'latitude_mode': 'exact', 'longitude_mode': 'even-smooth',
    }
    fixture_dir = SCRATCH / 'fixtures'
    fixture_dir.mkdir(parents=True, exist_ok=True)
    fixture = bench.make_fixture(case, fixture_dir)
    physical = common.physical_cpus()
    monitor_cpu = physical[8]
    orders = [
        ['parent', 'candidate', 'ducc'],
        ['candidate', 'ducc', 'parent'],
        ['ducc', 'parent', 'candidate'],
    ]
    workers: list[dict] = []
    triplets: list[dict] = []
    RESULTS.mkdir(parents=True, exist_ok=True)

    for threads in (1, 8):
        cpus = physical[:threads]
        for pair_index, order in enumerate(orders):
            records: dict[str, dict] = {}
            output_paths: dict[str, dict[str, pathlib.Path]] = {}
            for position, backend in enumerate(order):
                paths = {
                    name: RESULTS / f'{backend}-q{threads}-pair{pair_index}-{name}.bin'
                    for name in ('map0', 'map1', 'e', 'b')
                }
                command = bench.worker_command(
                    binaries[backend], case, fixture, 'synthesis', threads, paths, 25.0
                )
                record, diagnostics = common.run_pinned(command, cpus, monitor_cpu)
                record.update({
                    'backend': backend, 'threads': threads, 'process_pair': pair_index,
                    'position': position, 'fixture_sha256': fixture['sha256'],
                    'output_sha256': {name: sha256(paths[name]) for name in ('map0', 'map1')},
                    'host_diagnostics': compact_diag(diagnostics),
                })
                records[backend] = record
                output_paths[backend] = paths
                workers.append(record)

            outputs = {backend: map_pair(output_paths[backend])
                       for backend in ('parent', 'candidate', 'ducc')}
            medians = {key: float(statistics.median(records[key]['batch_ms']))
                       for key in ('parent', 'candidate', 'ducc')}
            triplet = {
                'threads': threads, 'process_pair': pair_index, 'process_order': order,
                'candidate_parent_bitwise_equal': bitwise_equal(outputs['parent'], outputs['candidate']),
                'finite_outputs': all(bool(np.isfinite(value).all()) for value in outputs.values()),
                'repeat_consistent': {key: bool(records[key]['repeat_consistent']) for key in records},
                'reset_consistent': {key: bool(records[key]['reset_consistent']) for key in records},
                'timing_median_ms': medians,
                'candidate_parent_ratio': medians['candidate'] / medians['parent'],
                'parent_ducc_ratio': medians['parent'] / medians['ducc'],
                'candidate_ducc_ratio': medians['candidate'] / medians['ducc'],
                'candidate_parent_error': common.error_metrics(outputs['candidate'], outputs['parent']),
                'parent_vs_ducc_error': common.error_metrics(outputs['parent'], outputs['ducc']),
                'candidate_vs_ducc_error': common.error_metrics(outputs['candidate'], outputs['ducc']),
                'batch_samples_ms': {key: records[key]['batch_ms'] for key in records},
                'host_diagnostics': {key: records[key]['host_diagnostics'] for key in records},
                'output_sha256': {key: records[key]['output_sha256'] for key in records},
            }
            triplets.append(triplet)
            print(
                f"DONE q={threads} pair={pair_index} order={','.join(order)} "
                f"C/P={triplet['candidate_parent_ratio']:.6f} "
                f"P/D={triplet['parent_ducc_ratio']:.6f} "
                f"bitwise={triplet['candidate_parent_bitwise_equal']}", flush=True,
            )

    metadata = {
        'parent_source_sha': subprocess.check_output(
            ['git', '-C', str(PARENT), 'rev-parse', 'HEAD'], text=True
        ).strip(),
        'candidate_base_source_sha': subprocess.check_output(
            ['git', '-C', str(CANDIDATE), 'rev-parse', 'HEAD'], text=True
        ).strip(),
        'candidate_patch_sha256': sha256(SCRATCH / 'candidate.patch'),
        'candidate_source_file_sha256': {
            relative: sha256(CANDIDATE / relative) for relative in (
                'src/legendre/scalar_paired/synthesis.rs',
                'src/transform/spin/synthesis.rs',
            )
        },
        'binary_sha256': {key: sha256(value) for key, value in binaries.items()},
        'ducc_revision': subprocess.check_output(
            ['git', '-C', str(DUCC_SOURCE), 'rev-parse', 'HEAD'], text=True
        ).strip(),
        'rustc': subprocess.check_output(['rustc', '-Vv'], text=True),
        'gxx': subprocess.check_output(['g++', '--version'], text=True).splitlines()[0],
        'rust_flags': '-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
        'ducc_flags': 'SHT -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver overrides with -fno-fast-math -ffp-contract=fast',
        'cpu_model': common.cpu_model(),
        'physical_cpus_q1': physical[:1], 'physical_cpus_q8': physical[:8],
        'monitor_cpu': monitor_cpu,
        'case': case, 'operation': 'spin-1 synthesis', 'phi0': 0.23,
        'target_batch_ms': 25.0, 'triplets_per_thread_count': 3,
        'fixture_seed': fixture['seed'], 'fixture_sha256': fixture['sha256'],
    }
    result = {'metadata': metadata, 'workers': workers, 'triplets': triplets}
    json_path = RESULTS / 'triplets.json'
    json_path.write_text(json.dumps(result, indent=2, allow_nan=False) + '\n')
    csv_path = RESULTS / 'triplets.csv'
    fields = (
        'threads', 'process_pair', 'candidate_parent_ratio', 'parent_ducc_ratio',
        'candidate_ducc_ratio', 'candidate_parent_bitwise_equal', 'finite_outputs',
        'candidate_parent_relative_l2', 'candidate_parent_absolute_max',
        'candidate_ducc_relative_l2', 'candidate_ducc_absolute_max',
        'parent_wait_fraction', 'candidate_wait_fraction', 'ducc_wait_fraction',
    )
    with csv_path.open('w', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        for item in triplets:
            writer.writerow({
                'threads': item['threads'], 'process_pair': item['process_pair'],
                'candidate_parent_ratio': item['candidate_parent_ratio'],
                'parent_ducc_ratio': item['parent_ducc_ratio'],
                'candidate_ducc_ratio': item['candidate_ducc_ratio'],
                'candidate_parent_bitwise_equal': item['candidate_parent_bitwise_equal'],
                'finite_outputs': item['finite_outputs'],
                'candidate_parent_relative_l2': item['candidate_parent_error']['relative_l2'],
                'candidate_parent_absolute_max': item['candidate_parent_error']['absolute_max'],
                'candidate_ducc_relative_l2': item['candidate_vs_ducc_error']['relative_l2'],
                'candidate_ducc_absolute_max': item['candidate_vs_ducc_error']['absolute_max'],
                'parent_wait_fraction': item['host_diagnostics']['parent']['wait_fraction'],
                'candidate_wait_fraction': item['host_diagnostics']['candidate']['wait_fraction'],
                'ducc_wait_fraction': item['host_diagnostics']['ducc']['wait_fraction'],
            })
    print(f'WROTE {json_path}', flush=True)
    return 0 if all(
        item['finite_outputs']
        and item['candidate_parent_bitwise_equal']
        and item['candidate_parent_error']['relative_l2'] is not None
        and item['candidate_parent_error']['relative_l2'] <= 2e-12
        and item['candidate_parent_error']['absolute_max'] is not None
        and item['candidate_parent_error']['absolute_max'] <= 2e-12
        and item['candidate_vs_ducc_error']['relative_l2'] is not None
        and item['candidate_vs_ducc_error']['relative_l2'] <= 2e-12
        and all(item['repeat_consistent'].values())
        and all(item['reset_consistent'].values())
        for item in triplets
    ) else 2


if __name__ == '__main__':
    raise SystemExit(main())
