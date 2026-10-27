from __future__ import annotations
import hashlib, itertools, json, os, random, shutil, statistics, sys
from pathlib import Path
root = Path(os.environ['RUSTSHT_M6_SCRATCH']).resolve()
repo = Path(os.environ['RUSTSHT_REPO']).resolve()
sys.path.insert(0, str(repo / 'scripts'))
from benchmark_ducc_f32_native import error_metrics, make_fixture, run_worker, smooth_even_at_least
binaries = {
    'parent': root / 'targets/parent/release/examples/ducc_f32_full_transform_parity',
    'candidate': root / 'targets/candidate/release/examples/ducc_f32_full_transform_parity',
    'ducc': root / 'ducc-build/ducc_f32_full_transform_parity',
}
for name, path in binaries.items():
    if not path.is_file(): raise SystemExit(f'missing {name} binary: {path}')
provenance = {
    'parent_sha': '0677b5bfe98117c1d1a5d5204b0a710828fcb9c8',
    'candidate_base_sha': '0677b5bfe98117c1d1a5d5204b0a710828fcb9c8',
    'ducc_sha': '42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
    'rust_flags': '-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
    'ducc_flags': 'Release x86-64-v3; DUCC 0.42.0 CMake driver',
    'process_pairs': 15,
    'threads': 8,
    'q8_cpus': '0-7',
    'warm_target_ms': 10.0,
    'binary_sha256': {k: hashlib.sha256(v.read_bytes()).hexdigest() for k,v in binaries.items()},
    'schedule_patch_sha256': hashlib.sha256((root/'schedule.patch').read_bytes()).hexdigest(),
}
orders = list(itertools.permutations(('parent','candidate','ducc')))
orders = orders * 2 + orders[:3]
random.Random(0x6D362026).shuffle(orders)
all_rows = []
for mmax in (720,180):
    case = {'grid':'GL','ntheta':721,'nphi':smooth_even_at_least(2*mmax+2),
            'lmax':720,'mmax':mmax,'longitude':'even','spin':0,
            'operation':'synthesis','threads':8}
    case_id = f'GL-T720-M{mmax}-even-s0-synthesis-q8'
    fixture = make_fixture(case, root/'results/repeat-q8/fixtures'/case_id)
    fixture_hashes = {k: hashlib.sha256(v.read_bytes()).hexdigest() for k,v in fixture.items()}
    outpath = root/'results/repeat-q8'/f'{case_id}.json'
    rows = []
    if outpath.exists():
        previous = json.loads(outpath.read_text())
        if previous.get('provenance') != provenance or previous.get('fixture_sha256') != fixture_hashes:
            raise RuntimeError(f'checkpoint identity mismatch: {case_id}')
        rows = previous.get('pairs', [])
        if any(row['pair'] != index for index, row in enumerate(rows, 1)):
            raise RuntimeError(f'checkpoint pair sequence is incomplete: {case_id}')
    all_rows.extend({'case_id':case_id, **row} for row in rows)
    if len(rows) > len(orders): raise RuntimeError(f'too many checkpoint pairs: {case_id}')
    for pair, order in enumerate(orders[len(rows):], len(rows)+1):
        workers = {}
        pair_root = root/'results/repeat-q8'/case_id/f'pair-{pair:02d}'
        for backend in order:
            out = run_worker(binaries[backend], backend, case, fixture,
                             pair_root/backend, 10.0, '0-7')
            for key in ('finite_outputs','repeat_consistent','reset_consistent'):
                if not out.get(key, False): raise RuntimeError(f'{key} failed: {case_id} pair {pair} {backend}')
            paths = [Path(p) for p in out['outputs']]
            out_summary = {
                'batch_ms': out['batch_ms'],
                'median_ms': statistics.median(out['batch_ms']),
                'finite_outputs': out['finite_outputs'],
                'repeat_consistent': out['repeat_consistent'],
                'reset_consistent': out['reset_consistent'],
                'simd_arch': out.get('simd_arch'),
                'f32_lanes': out.get('f32_lanes'),
                'output_sha256': [hashlib.sha256(p.read_bytes()).hexdigest() for p in paths],
                'process_elapsed_s': out['process_elapsed_s'],
            }
            workers[backend] = (out_summary, paths)
        p, c, d = (workers[k][0] for k in ('parent','candidate','ducc'))
        p_paths, c_paths, d_paths = (workers[k][1] for k in ('parent','candidate','ducc'))
        if len(p_paths) != len(c_paths) or len(c_paths) != len(d_paths):
            raise RuntimeError(f'output count mismatch: {case_id} pair {pair}')
        row = {
            'pair': pair,
            'order': list(order),
            'case': case,
            'fixture_sha256': fixture_hashes,
            'workers': {k: workers[k][0] for k in workers},
            'candidate_parent_bitwise_equal': all(a.read_bytes()==b.read_bytes() for a,b in zip(p_paths,c_paths)),
            'candidate_over_parent': c['median_ms']/p['median_ms'],
            'candidate_over_ducc': c['median_ms']/d['median_ms'],
            'parent_over_ducc': p['median_ms']/d['median_ms'],
            'error_candidate_vs_parent': error_metrics(p_paths,c_paths),
            'error_candidate_vs_ducc': error_metrics(d_paths,c_paths),
            'error_parent_vs_ducc': error_metrics(d_paths,p_paths),
        }
        rows.append(row)
        all_rows.append({'case_id':case_id, **row})
        result = {'provenance':provenance,'case_id':case_id,'fixture_sha256':fixture_hashes,'pairs':rows}
        outpath.parent.mkdir(parents=True,exist_ok=True)
        outpath.write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps({'case':case_id,'pair':pair,'order':list(order),
                          'parent_ms':p['median_ms'],'candidate_ms':c['median_ms'],
                          'ducc_ms':d['median_ms'],'candidate_over_parent':row['candidate_over_parent'],
                          'bitwise':row['candidate_parent_bitwise_equal'],
                          'candidate_ducc_l2':row['error_candidate_vs_ducc']['relative_l2']}),flush=True)
        shutil.rmtree(pair_root)
    ratios=[r['candidate_over_parent'] for r in rows]
    summary={'count':len(rows),'median_candidate_over_parent':statistics.median(ratios),
             'min':min(ratios),'max':max(ratios),'candidate_faster_pairs':sum(x<1 for x in ratios),
             'parent_ms_median':statistics.median(r['workers']['parent']['median_ms'] for r in rows),
             'candidate_ms_median':statistics.median(r['workers']['candidate']['median_ms'] for r in rows),
             'ducc_ms_median':statistics.median(r['workers']['ducc']['median_ms'] for r in rows),
             'bitwise_all':all(r['candidate_parent_bitwise_equal'] for r in rows),
             'max_candidate_parent_relative_l2':max(r['error_candidate_vs_parent']['relative_l2'] for r in rows),
             'max_candidate_parent_absolute':max(r['error_candidate_vs_parent']['max_absolute'] for r in rows),
             'max_candidate_ducc_relative_l2':max(r['error_candidate_vs_ducc']['relative_l2'] for r in rows),
             'max_candidate_ducc_absolute':max(r['error_candidate_vs_ducc']['max_absolute'] for r in rows)}
    result={'provenance':provenance,'case_id':case_id,'fixture_sha256':fixture_hashes,
            'summary':summary,'pairs':rows}
    (root/'results/repeat-q8'/f'{case_id}.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'summary':case_id,**summary}),flush=True)
print(f'completed {len(all_rows)} fresh-process q8 triplets')
