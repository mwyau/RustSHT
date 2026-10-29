import hashlib, importlib.util, json, os, pathlib, random, statistics, subprocess
ROOT = pathlib.Path(os.environ['M6_EXPERIMENT_ROOT'])
REPO = pathlib.Path(os.environ['M6_REPO_ROOT'])
spec = importlib.util.spec_from_file_location('bench', REPO / 'scripts/benchmark_ducc_f32_native.py')
bench = importlib.util.module_from_spec(spec); spec.loader.exec_module(bench)
BINS = {
    'parent': ROOT / 'target-parent/release/examples/ducc_f32_full_transform_parity',
    'candidate': ROOT / 'target-candidate/release/examples/ducc_f32_full_transform_parity',
    'ducc': ROOT / 'ducc-build/ducc_f32_full_transform_parity',
}
OUT = ROOT / 'controls-f32-results'; FIX = ROOT / 'controls-f32-fixtures'; RUN = ROOT / 'controls-f32-runs'
OUT.mkdir(exist_ok=True); FIX.mkdir(exist_ok=True); RUN.mkdir(exist_ok=True)
def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda:f.read(1<<20),b''): h.update(b)
    return h.hexdigest()
def make_case(lmax,mmax,nphi,spin,operation,threads):
    return {'grid':'GL','ntheta':lmax+1,'nphi':nphi,'lmax':lmax,'mmax':mmax,
            'longitude':'even','spin':spin,'operation':operation,'threads':threads}
cases=[]
for mmax,nphi in ((720,1458),(180,378)):
    for threads in (1,8):
        pairs=15 if threads==8 else 3
        cases.append((make_case(720,mmax,nphi,0,'analysis',threads),pairs))
for operation in ('analysis','synthesis'):
    for threads in (1,8):
        cases.append((make_case(720,720,1458,1,operation,threads),3))
# Dispatch is intentionally off outside T720. Verify a smaller and a larger bandlimit.
for lmax,nphi in ((511,1026),(1023,2050)):
    cases.append((make_case(lmax,lmax,nphi,0,'synthesis',8),3))
orders=[('parent','candidate','ducc'),('candidate','ducc','parent'),('ducc','parent','candidate')]
rng=random.Random(0xC07A01)
summary=[]
for idx,(case,pair_count) in enumerate(cases):
    ident=f"GL-T{case['lmax']}-M{case['mmax']}-s{case['spin']}-{case['operation']}-q{case['threads']}"
    fixture=bench.make_fixture(case,FIX/ident)
    cpus='0' if case['threads']==1 else '0,2,4,6,8,10,12,14'
    cell=[]
    for pair in range(1,pair_count+1):
        order=list(orders[(pair-1)%3])
        if rng.random()<0.5: order.reverse()
        workers={}
        for name in order:
            workers[name]=bench.run_worker(BINS[name],name,case,fixture,RUN/ident/f'pair-{pair:02d}'/name,25.0,cpus)
        times={name:statistics.median(workers[name]['batch_ms']) for name in BINS}
        paths={name:[pathlib.Path(p) for p in workers[name]['outputs']] for name in BINS}
        candidate_parent=bench.error_metrics(paths['parent'],paths['candidate'])
        candidate_ducc=bench.error_metrics(paths['ducc'],paths['candidate'])
        row={'pair':pair,'order':order,'times_ms':times,
             'candidate_parent_ratio':times['candidate']/times['parent'],
             'candidate_ducc_ratio':times['candidate']/times['ducc'],
             'candidate_vs_parent':candidate_parent,'candidate_vs_ducc':candidate_ducc,
             'checks':{name:{k:workers[name].get(k) for k in ('finite_outputs','repeat_consistent','reset_consistent','simd_arch','f32_lanes')} for name in BINS},
             'telemetry':{name:workers[name].get('telemetry') for name in BINS}}
        for group in paths.values():
            for path in group: path.unlink(missing_ok=True)
        cell.append(row)
        (OUT/f'{ident}-pair-{pair:02d}.json').write_text(json.dumps(row,indent=2)+'\n')
        print('completed',ident,'pair',pair,flush=True)
    ratios=[r['candidate_parent_ratio'] for r in cell]
    median=lambda key:statistics.median(row['times_ms'][key] for row in cell)
    stats={'case_id':ident,'case':case,'pairs':pair_count,'parent_median_ms':median('parent'),
           'candidate_median_ms':median('candidate'),'ducc_median_ms':median('ducc'),
           'candidate_parent_paired_median':statistics.median(ratios),
           'candidate_parent_pairs_faster':sum(x<1 for x in ratios),
           'max_candidate_parent_relative_l2':max(r['candidate_vs_parent']['relative_l2'] for r in cell),
           'max_candidate_parent_absolute':max(r['candidate_vs_parent']['max_absolute'] for r in cell),
           'max_candidate_ducc_relative_l2':max(r['candidate_vs_ducc']['relative_l2'] for r in cell),
           'all_worker_checks':all(all(v.get('finite_outputs') and v.get('repeat_consistent') and v.get('reset_consistent') for v in r['checks'].values()) for r in cell),
           'fixture_sha256':{n:sha(p) for n,p in fixture.items()},'pairs_data':cell}
    summary.append(stats)
    (OUT/f'{ident}-summary.json').write_text(json.dumps(stats,indent=2)+'\n')
    print(json.dumps({k:stats[k] for k in ('case_id','parent_median_ms','candidate_median_ms','ducc_median_ms','candidate_parent_paired_median','candidate_parent_pairs_faster','all_worker_checks')}),flush=True)
manifest={'parent_sha':subprocess.check_output(['git','-C',str(ROOT/'parent'),'rev-parse','HEAD'],text=True).strip(),
          'candidate_patch_sha256':sha(ROOT/'candidate.patch'),'ducc_source_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
          'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs','ducc_flags':'-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math',
          'q1_affinity':'0','q8_affinity':'0,2,4,6,8,10,12,14','warm_target_ms':25,'case_summaries':summary}
(OUT/'results.json').write_text(json.dumps(manifest,indent=2)+'\n')
