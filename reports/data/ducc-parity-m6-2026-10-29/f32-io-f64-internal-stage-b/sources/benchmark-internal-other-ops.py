#!/usr/bin/env python3
"""Matched native-f32 versus reusable f32-I/O/f64-internal remaining operations."""
import hashlib, importlib.util, json, os, pathlib, random, statistics, sys
ROOT = pathlib.Path(os.environ['M6_EXPERIMENT_ROOT'])
REPO = pathlib.Path(os.environ['M6_REPO_ROOT'])
PARENT = ROOT / 'bin/ducc_f32_native'
CANDIDATE = pathlib.Path(os.environ.get('M6_INTERNAL_BINARY', str(ROOT / 'bin/ducc_f32_internal_f64_parallel_conv')))
DUCC = ROOT / 'bin/ducc_f32_pinned'
OUT = ROOT / os.environ.get('M6_RESULTS_DIR', 'results-other-ops-screen-3')
SCRATCH = ROOT / os.environ.get('M6_RUNS_DIR', 'runs-other-ops-screen-3')
sys.path.insert(0, str(REPO / 'scripts'))
spec = importlib.util.spec_from_file_location('bench', REPO / 'scripts/benchmark_ducc_f32_native.py')
bench = importlib.util.module_from_spec(spec); spec.loader.exec_module(bench)

def sha256(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda:f.read(1<<20),b''): h.update(b)
    return h.hexdigest()

def boot_ci(values, seed):
    rng=random.Random(seed); sims=[]
    for _ in range(30000): sims.append(statistics.median(values[rng.randrange(len(values))] for _ in values))
    return [bench.percentile(sims,.025),bench.percentile(sims,.975)]

def main():
    OUT.mkdir(parents=True,exist_ok=True); SCRATCH.mkdir(parents=True,exist_ok=True)
    pair_count=int(os.environ.get('M6_PAIRS','3'))
    cases=[]
    for operation,spin in [('analysis',0),('synthesis',1),('analysis',1)]:
      for threads in (1,8):
       for mmax,nphi in ((720,1458),(180,378)):
        cases.append({'grid':'GL','ntheta':721,'nphi':nphi,'lmax':720,'mmax':mmax,
          'longitude':'even','spin':spin,'operation':operation,'threads':threads})
    selected=set(os.environ.get('M6_CASES','').split(','))
    if selected and selected != {''}:
      cases=[c for c in cases if f"GL-T720-M{c['mmax']}-s{c['spin']}-{c['operation']}-q{c['threads']}" in selected]
    orderings=(('parent','internal_f64','ducc'),('ducc','parent','internal_f64'),('internal_f64','ducc','parent'))
    binaries={'parent':PARENT,'internal_f64':CANDIDATE,'ducc':DUCC}; all_rows=[]
    for index,case in enumerate(cases):
      cid=f"GL-T720-M{case['mmax']}-s{case['spin']}-{case['operation']}-q{case['threads']}"
      fixture=bench.make_fixture(case,SCRATCH/'fixtures'/cid)
      pairs=[]
      cpus='0' if case['threads']==1 else '0,2,4,6,8,10,12,14'
      for pair in range(1,pair_count+1):
        order=list(orderings[(pair-1)%3]);
        if random.Random(0x5a17+index*19+pair).random()<.5: order.reverse()
        workers={}
        for backend in order:
          workers[backend]=bench.run_worker(binaries[backend],backend,case,fixture,
            SCRATCH/cid/f'pair-{pair:02}'/backend,25.0,cpus)
        times={k:statistics.median(workers[k]['batch_ms']) for k in binaries}
        paths={k:[pathlib.Path(p) for p in workers[k]['outputs']] for k in binaries}
        errors={
          'internal_vs_ducc':bench.error_metrics(paths['ducc'],paths['internal_f64']),
          'internal_vs_parent':bench.error_metrics(paths['parent'],paths['internal_f64']),
          'parent_vs_ducc':bench.error_metrics(paths['ducc'],paths['parent'])}
        pairs.append({'pair':pair,'order':order,'times_ms':times,
          'internal_parent_ratio':times['internal_f64']/times['parent'],
          'internal_ducc_ratio':times['internal_f64']/times['ducc'],
          'parent_ducc_ratio':times['parent']/times['ducc'],'errors':errors,
          'workers':{k:{x:workers[k][x] for x in ('backend','first_ms','warmup_ms','setup_ms','pool_setup_ms','fixture_load_ms','repetitions_per_batch','batch_ms','batch_total_ms','process_elapsed_s','output_checksum','finite_outputs','repeat_consistent','reset_consistent','simd_arch','f32_lanes','f64_lanes','internal_precision','reusable_conversion_buffer_bytes','telemetry') if x in workers[k]} for k in binaries}})
        for ps in paths.values():
          for p in ps:p.unlink(missing_ok=True)
        (OUT/f'{cid}-pair-{pair:02}.json').write_text(json.dumps(pairs[-1],indent=2)+'\n')
        print(f'completed {cid} pair {pair}',flush=True)
      ip=[r['internal_parent_ratio'] for r in pairs]; idu=[r['internal_ducc_ratio'] for r in pairs]
      row={'case_id':cid,'case':case,'pairs':pair_count,
        'parent_median_ms':statistics.median(r['times_ms']['parent'] for r in pairs),
        'internal_f64_median_ms':statistics.median(r['times_ms']['internal_f64'] for r in pairs),
        'ducc_median_ms':statistics.median(r['times_ms']['ducc'] for r in pairs),
        'internal_parent_paired_median':statistics.median(ip),'internal_parent_bootstrap_95':boot_ci(ip,0xF640+index),
        'internal_parent_faster':sum(v<1 for v in ip),
        'internal_ducc_paired_median':statistics.median(idu),'internal_ducc_bootstrap_95':boot_ci(idu,0xD0CC+index),
        'internal_ducc_faster':sum(v<1 for v in idu),
        'max_internal_ducc_relative_l2':max(r['errors']['internal_vs_ducc']['relative_l2'] for r in pairs),
        'max_internal_ducc_absolute':max(r['errors']['internal_vs_ducc']['max_absolute'] for r in pairs),
        'all_worker_checks':all(all(r['workers'][k].get(key) is True for key in ('finite_outputs','repeat_consistent','reset_consistent')) for r in pairs for k in binaries),
        'fixture_sha256':{k:sha256(p) for k,p in fixture.items()},'pairs_data':pairs}
      all_rows.append(row)
      (OUT/f'{cid}-summary.json').write_text(json.dumps(row,indent=2)+'\n')
      print(json.dumps({k:row[k] for k in ('case_id','parent_median_ms','internal_f64_median_ms','ducc_median_ms','internal_parent_paired_median','internal_ducc_paired_median','internal_ducc_bootstrap_95','all_worker_checks')}),flush=True)
    manifest={'parent_source_sha':'fdae803a01273ac7e4e941936f022fa11e770df5',
      'candidate_source_parent_sha':'fdae803a01273ac7e4e941936f022fa11e770df5',
      'candidate_example_sha256':sha256(pathlib.Path(os.environ['M6_INTERNAL_SOURCE'])),
      'ducc_source_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
      'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
      'ducc_flags':'-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math',
      'q1_affinity':'0','q8_affinity':'0,2,4,6,8,10,12,14','warm_target_ms':25.0,
      'process_order_balanced':True,'pairs_per_cell':pair_count,
      'fixture':'scripts/benchmark_ducc_f32_native.py make_fixture, phi0=0.23',
      'parent_binary_sha256':sha256(PARENT),'internal_f64_binary_sha256':sha256(CANDIDATE),
      'ducc_f32_binary_sha256':sha256(DUCC),'results':all_rows}
    (OUT/'results.json').write_text(json.dumps(manifest,indent=2)+'\n')
if __name__=='__main__':main()
