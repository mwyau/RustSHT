#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Summarize local diagnostic logs; launch no transforms or builds."""
import argparse
import csv
import json
import pathlib
import re
import statistics
from collections import defaultdict


def write_csv(path, rows):
    with path.open('w') as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--logs', type=pathlib.Path, default=pathlib.Path('/tmp'))
    parser.add_argument('--ducc', type=pathlib.Path, required=True)
    parser.add_argument('--adjoint', type=pathlib.Path,
                        help='optional directory of T214 diagnostic input/output arrays')
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(exist_ok=True, parents=True)
    scalar = []
    routes = []
    pattern = re.compile(
        r'PLAN14_(ANALYSIS|SYNTHESIS)_PHASE_BATCH T(\d+) m(\d+) '
        r'grid=(\d+)x(\d+) q(\d+) batch=(\d+) repeats=(\d+) wall_us=([\d.]+) (.*)')
    for path in sorted(args.logs.glob('rustsht-cc-independent-720-*-q*-run*.log')):
        for line in path.read_text().splitlines():
            if 'INDEPENDENT_ROUTE_PROFILE' in line:
                routes.append({'log': path.name, 'counts': line[line.index('ScalarAnalysisRouteCounts'): ]})
            match = pattern.search(line)
            if not match:
                continue
            op, l, m, nt, np, q, batch, repetitions, wall, tail = match.groups()
            values = {'wall': float(wall)}
            values.update({key.strip(): float(value) for key, value in
                           re.findall(r'([^=]+)=([\d.]+)(?: |$)', tail)})
            for phase, value in values.items():
                scalar.append({'backend': 'rustsht', 'operation': op.lower(), 'lmax': int(l),
                    'mmax': int(m), 'ntheta': int(nt), 'nphi': int(np), 'threads': int(q),
                    'run': int(re.search(r'run(\d+)', path.name)[1]), 'batch': int(batch),
                    'repetitions': int(repetitions), 'phase': phase, 'ms': value/1000})
    for path in sorted(args.ducc.glob('ducc-*-q*-run*.stdout')):
        op, q, run = re.search(r'ducc-(analysis|synthesis)-q(\d+)-run(\d+)', path.name).groups()
        record = json.loads(path.read_text())
        n, batches = record['repetitions'], len(record['batch_ms'])
        # Diagnostics print one line per call; the final n*batches calls are timed.
        lines = path.with_suffix('.stderr').read_text().splitlines()[-n*batches:]
        for batch in range(batches):
            calls = [dict((key, float(value)) for key, value in
                          re.findall(r'(\w+)_us=([\d.]+)', line))
                     for line in lines[batch*n:(batch+1)*n]]
            values = {key: statistics.mean(call[key] for call in calls)/1000 for key in calls[0]}
            values['wall'] = record['batch_ms'][batch]
            for phase, value in values.items():
                scalar.append({'backend': 'ducc', 'operation': op, 'lmax': 720, 'mmax': 720,
                    'ntheta': 722, 'nphi': 1458, 'threads': int(q), 'run': int(run),
                    'batch': batch+1, 'repetitions': n, 'phase': phase, 'ms': value})
    write_csv(args.output/'scalar-stage-batches.csv', scalar)
    grouped = defaultdict(list)
    for row in scalar:
        grouped[(row['backend'], row['operation'], row['mmax'], row['threads'], row['phase'])].append(row['ms'])
    summaries = []
    for (backend, op, m, q, phase), values in sorted(grouped.items()):
        ordered = sorted(values)
        summaries.append({'backend': backend, 'operation': op, 'lmax': 720, 'mmax': m,
            'threads': q, 'phase': phase, 'observations': len(values),
            'median_ms': statistics.median(values), 'min_ms': ordered[0], 'max_ms': ordered[-1]})
    write_csv(args.output/'scalar-stage-summary.csv', summaries)
    spin = []
    workers = []
    for path in sorted(args.logs.glob('rustsht-cc-independent-spin1-T*.log')):
        for line in path.read_text().splitlines():
            fields = line.split(',')
            if line.startswith('PROFILE_INNER,'):
                spin.append({'lmax': int(fields[1]), 'mmax': int(fields[2]),
                    'nphi': int(fields[3]), 'threads': int(fields[5]), 'sample': int(fields[6]),
                    'kind': 'worker sum', 'phase': fields[7], 'ms': float(fields[8])})
            if line.startswith('PROFILE_PHASE,'):
                spin.append({'lmax': int(fields[1]), 'mmax': int(fields[2]),
                    'nphi': int(fields[3]), 'threads': int(fields[5]), 'sample': int(fields[6]),
                    'kind': 'wall phase', 'phase': ','.join(fields[7:-1]), 'ms': float(fields[-1])})
            if line.startswith('PROFILE_ROUTES,'):
                routes.append({'log': path.name, 'counts': ','.join(fields[7:])})
            if line.startswith('PROFILE_WORKER,'):
                workers.append({'lmax': int(fields[1]), 'mmax': int(fields[2]),
                    'nphi': int(fields[3]), 'threads': int(fields[4]), 'sample': int(fields[5]),
                    'scratch_worker': int(fields[6]), 'rayon_worker': int(fields[7]),
                    'task_ms': float(fields[8])/1000, 'pair_success': int(fields[12]),
                    'fallback_blocks': int(fields[13])})
    write_csv(args.output/'spin1-profile.csv', spin)
    write_csv(args.output/'spin1-workers.csv', workers)
    (args.output/'route-counts.json').write_text(json.dumps(routes, indent=2)+'\n')
    # Local counter timings are diagnostics, not a new performance acceptance run.
    scalar_synthesis = []
    path = args.logs/'rustsht-cc-independent-synthesis-routes.log'
    for line in path.read_text().splitlines():
        match = re.search(r'REVIEW_SYNTH_ROUTES T(\d+) m(\d+) q(\d+) '
                          r'fused_declined_transition_fallback=\[(.*?)\] groups=\[(.*)\]', line)
        if not match:
            continue
        l, m, q, counts, raw = match.groups()
        counts = [int(v) for v in counts.split(',')]
        jobs = [(int(a), int(b), int(c)) for a,b,c in re.findall(r'\((\d+), (\d+), (\d+)\)', raw)]
        per_worker = defaultdict(int)
        for worker, ring, ns in jobs:
            per_worker[worker] += ns
        scalar_synthesis.append({'lmax': int(l), 'mmax': int(m), 'threads': int(q),
            'fused_blocks': counts[0], 'declined_blocks': counts[1],
            'transition_blocks': counts[2], 'visitor_fallback_blocks': counts[3],
            'groups': len(jobs), 'observed_workers': sorted(per_worker),
            'diagnostic_worker_busy_ns': dict(per_worker)})
    (args.output/'scalar-synthesis-routes.json').write_text(json.dumps(scalar_synthesis, indent=2)+'\n')
    if args.adjoint:
        import numpy as np
        def real(name):
            return np.fromfile(args.adjoint/name,dtype='<f8').astype(np.longdouble)
        def comp(name):
            return np.fromfile(args.adjoint/name,dtype='<f8').reshape(-1,2).astype(np.longdouble)
        map_input, alm = real('map.bin'), comp('alm.bin')
        weights = np.full(len(alm),2,dtype=np.longdouble)
        weights[:215] = 1
        def harmonic_dot(left,right):
            return np.sum(weights*(left*right).sum(axis=1),dtype=np.longdouble)
        def harmonic_norm(values):
            return np.sqrt(harmonic_dot(values,values))
        adjoint = []
        for q in (1,8):
            synthesized, adjoint_analysis = real(f'synthesized-q{q}.bin'), real(f'adjoint-analysis-q{q}.bin')
            analyzed, adjoint_synthesis = comp(f'analyzed-q{q}.bin'), comp(f'adjoint-synthesis-q{q}.bin')
            cases = [('synthesis',np.sum(synthesized*map_input,dtype=np.longdouble),
                harmonic_dot(alm,adjoint_synthesis),np.linalg.norm(synthesized)*np.linalg.norm(map_input)
                +harmonic_norm(alm)*harmonic_norm(adjoint_synthesis)),
                ('analysis',harmonic_dot(analyzed,alm),np.sum(map_input*adjoint_analysis,dtype=np.longdouble),
                harmonic_norm(analyzed)*harmonic_norm(alm)+np.linalg.norm(map_input)*np.linalg.norm(adjoint_analysis))]
            for op,left,right,scale in cases:
                adjoint.append({'q':q,'op':op,'lhs':float(left),'rhs':float(right),
                    'relative_to_dot':float(abs(left-right)/max(abs(left),abs(right))),
                    'normwise_residual':float(abs(left-right)/scale),
                    'norm_product_over_dot':float(scale/max(abs(left),abs(right)))})
        (args.output/'adjoint-cancellation.json').write_text(json.dumps(adjoint,indent=2)+'\n')
    print(f'{len(scalar)} scalar phase observations; {len(spin)} spin phase observations')


if __name__ == '__main__':
    main()
