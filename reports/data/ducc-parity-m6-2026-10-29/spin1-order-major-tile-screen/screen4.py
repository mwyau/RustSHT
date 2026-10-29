#!/usr/bin/env python3
import hashlib, importlib.util, itertools, json, os, pathlib, random, statistics, sys
root = pathlib.Path(os.environ['M6_EXPERIMENT_ROOT'])
repo = pathlib.Path(os.environ['M6_REPO_ROOT'])
binaries = {
    'native_f32': pathlib.Path(os.environ['M6_NATIVE_BINARY']),
    'internal_parent': pathlib.Path(os.environ['M6_INTERNAL_PARENT_BINARY']),
    'internal_candidate': pathlib.Path(os.environ['M6_INTERNAL_CANDIDATE_BINARY']),
    'ducc_f32': pathlib.Path(os.environ['M6_DUCC_BINARY']),
}
out = root / os.environ.get('M6_RESULTS_DIR', 'results-screen')
scratch = root / os.environ.get('M6_RUNS_DIR', 'runs-screen')
sys.path.insert(0, str(repo / 'scripts'))
spec = importlib.util.spec_from_file_location('bench', repo / 'scripts/benchmark_ducc_f32_native.py')
bench = importlib.util.module_from_spec(spec); spec.loader.exec_module(bench)
def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda:stream.read(1<<20), b''): h.update(block)
    return h.hexdigest()
def main():
    out.mkdir(parents=True,exist_ok=True); scratch.mkdir(parents=True,exist_ok=True)
    pair_count=int(os.environ.get('M6_PAIRS','3'))
    cases=[]
    for q in (1,8):
        for mmax,nphi in ((720,1458),(180,378)):
            cases.append({'grid':'GL','ntheta':721,'nphi':nphi,'lmax':720,'mmax':mmax,'longitude':'even','spin':1,'operation':'synthesis','threads':q})
    selected=set(os.environ.get('M6_CASES','').split(','))
    if selected != {''}:
        cases=[case for case in cases if f"GL-T720-M{case['mmax']}-s1-synthesis-q{case['threads']}" in selected]
    orders=list(itertools.permutations(binaries))
    rows=[]
    for case_index,case in enumerate(cases):
        cid=f"GL-T720-M{case['mmax']}-s1-synthesis-q{case['threads']}"
        fixture=bench.make_fixture(case,scratch/'fixtures'/cid)
        pairs=[]
        cpus='0' if case['threads']==1 else '0,2,4,6,8,10,12,14'
        for pair in range(1,pair_count+1):
            order=orders[(case_index*7+pair-1)%len(orders)]
            if random.Random(0xB33F+case_index*31+pair).random()<0.5: order=tuple(reversed(order))
            workers={}
            for name in order:
                workers[name]=bench.run_worker(binaries[name],name,case,fixture,scratch/cid/f'pair-{pair:02}'/name,25.0,cpus)
            times={name:statistics.median(workers[name]['batch_ms']) for name in binaries}
            outputs={name:[pathlib.Path(x) for x in workers[name]['outputs']] for name in binaries}
            pairrow={
                'pair':pair,'order':order,'times_ms':times,
                'candidate_parent_ratio':times['internal_candidate']/times['internal_parent'],
                'candidate_ducc_ratio':times['internal_candidate']/times['ducc_f32'],
                'parent_ducc_ratio':times['internal_parent']/times['ducc_f32'],
                'native_ducc_ratio':times['native_f32']/times['ducc_f32'],
                'errors':{
                    'candidate_vs_parent':bench.error_metrics(outputs['internal_parent'],outputs['internal_candidate']),
                    'candidate_vs_ducc':bench.error_metrics(outputs['ducc_f32'],outputs['internal_candidate']),
                    'native_vs_ducc':bench.error_metrics(outputs['ducc_f32'],outputs['native_f32'])},
                'workers':{name:{key:workers[name][key] for key in ('backend','first_ms','warmup_ms','setup_ms','pool_setup_ms','fixture_load_ms','repetitions_per_batch','batch_ms','batch_total_ms','process_elapsed_s','output_checksum','finite_outputs','repeat_consistent','reset_consistent','simd_arch','f32_lanes','f64_lanes','internal_precision','reusable_conversion_buffer_bytes','telemetry') if key in workers[name]} for name in binaries}}
            pairs.append(pairrow)
            for paths in outputs.values():
                for path in paths: path.unlink(missing_ok=True)
            (out/f'{cid}-pair-{pair:02}.json').write_text(json.dumps(pairrow,indent=2)+'\n')
            print(f'completed {cid} pair {pair}',flush=True)
        row={'cell_id':cid,'case':case,'pairs':pair_count,
             'median_times_ms':{name:statistics.median(p['times_ms'][name] for p in pairs) for name in binaries},
             'candidate_parent_ratios':[p['candidate_parent_ratio'] for p in pairs],
             'candidate_ducc_ratios':[p['candidate_ducc_ratio'] for p in pairs],
             'candidate_parent_median':statistics.median(p['candidate_parent_ratio'] for p in pairs),
             'candidate_ducc_median':statistics.median(p['candidate_ducc_ratio'] for p in pairs),
             'candidate_parent_equal_bits':all(p['errors']['candidate_vs_parent']['relative_l2']==0 for p in pairs),
             'max_candidate_ducc_relative_l2':max(p['errors']['candidate_vs_ducc']['relative_l2'] for p in pairs),
             'all_workers_finite_repeat_reset':all(all(p['workers'][name].get(k) is True for k in ('finite_outputs','repeat_consistent','reset_consistent')) for p in pairs for name in binaries),
             'fixture_sha256':{name:sha(path) for name,path in fixture.items()},'pairs_data':pairs}
        rows.append(row)
        (out/f'{cid}-summary.json').write_text(json.dumps(row,indent=2)+'\n')
        print(json.dumps({k:row[k] for k in ('cell_id','median_times_ms','candidate_parent_median','candidate_ducc_median','candidate_parent_ratios','candidate_ducc_ratios','candidate_parent_equal_bits','all_workers_finite_repeat_reset')}),flush=True)
    manifest={
        'source_parent':'b3b93f17fe4d404fe4c033ffc7420eb190dc00b0',
        'candidate_patch_sha256':os.environ['M6_CANDIDATE_PATCH_SHA256'],
        'candidate_change':'SPIN1_ORDER_MAJOR_TILE 1 -> 4',
        'ducc_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
        'rust_flags':'--locked --release --features parallel -C target-cpu=x86-64-v3 -Aunexpected_cfgs',
        'ducc_flags':'-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math',
        'affinity':{'q1':'0','q8':'0,2,4,6,8,10,12,14'},'warm_target_ms':25.0,'pairs':pair_count,
        'binary_sha256':{name:sha(path) for name,path in binaries.items()},'cells':rows}
    (out/'results.json').write_text(json.dumps(manifest,indent=2)+'\n')
if __name__=='__main__': main()
