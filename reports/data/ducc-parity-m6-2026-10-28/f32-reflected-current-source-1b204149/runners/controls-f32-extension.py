import hashlib, importlib.util, json, os, pathlib, random, statistics, subprocess
ROOT=pathlib.Path(os.environ['M6_EXPERIMENT_ROOT']); REPO=pathlib.Path(os.environ['M6_REPO_ROOT'])
spec=importlib.util.spec_from_file_location('bench',REPO/'scripts/benchmark_ducc_f32_native.py'); bench=importlib.util.module_from_spec(spec); spec.loader.exec_module(bench)
B={'parent':ROOT/'target-parent/release/examples/ducc_f32_full_transform_parity','candidate':ROOT/'target-candidate/release/examples/ducc_f32_full_transform_parity','ducc':ROOT/'ducc-build/ducc_f32_full_transform_parity'}
OUT=ROOT/'controls-f32-extension-results'; FIX=ROOT/'controls-f32-extension-fixtures'; RUN=ROOT/'controls-f32-extension-runs'
for p in (OUT,FIX,RUN): p.mkdir(exist_ok=True)
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for x in iter(lambda:f.read(1<<20),b''): h.update(x)
 return h.hexdigest()
def c(l,m,nphi,spin,op,q): return {'grid':'GL','ntheta':l+1,'nphi':nphi,'lmax':l,'mmax':m,'longitude':'even','spin':spin,'operation':op,'threads':q}
cases=[(c(720,720,1458,0,'analysis',1),15),(c(720,180,378,0,'analysis',1),15),(c(720,720,1458,1,'synthesis',8),15),(c(1023,1023,bench.smooth_even_at_least(2*1023+2),0,'synthesis',8),15)]
orders=[('parent','candidate','ducc'),('candidate','ducc','parent'),('ducc','parent','candidate')]
rng=random.Random(0xE8715); all_rows=[]
for case,n in cases:
 ident=f"GL-T{case['lmax']}-M{case['mmax']}-s{case['spin']}-{case['operation']}-q{case['threads']}"
 fixture=bench.make_fixture(case,FIX/ident); cpus='0' if case['threads']==1 else '0,2,4,6,8,10,12,14'; rows=[]
 for i in range(1,n+1):
  order=list(orders[(i-1)%3]);
  if rng.random()<.5: order.reverse()
  w={}
  for name in order: w[name]=bench.run_worker(B[name],name,case,fixture,RUN/ident/f'pair-{i:02d}'/name,25.0,cpus)
  t={name:statistics.median(w[name]['batch_ms']) for name in B}
  paths={name:[pathlib.Path(x) for x in w[name]['outputs']] for name in B}
  row={'pair':i,'order':order,'times_ms':t,'candidate_parent_ratio':t['candidate']/t['parent'],'candidate_ducc_ratio':t['candidate']/t['ducc'],
       'candidate_vs_parent':bench.error_metrics(paths['parent'],paths['candidate']),'candidate_vs_ducc':bench.error_metrics(paths['ducc'],paths['candidate']),
       'checks':{name:{k:w[name].get(k) for k in ('finite_outputs','repeat_consistent','reset_consistent','simd_arch','f32_lanes')} for name in B},
       'telemetry':{name:w[name].get('telemetry') for name in B}}
  for group in paths.values():
   for p in group:p.unlink(missing_ok=True)
  rows.append(row); (OUT/f'{ident}-pair-{i:02d}.json').write_text(json.dumps(row,indent=2)+'\n'); print('completed',ident,'pair',i,flush=True)
 ratios=[x['candidate_parent_ratio'] for x in rows]
 stats={'case_id':ident,'case':case,'pairs':n,'parent_median_ms':statistics.median(x['times_ms']['parent'] for x in rows),'candidate_median_ms':statistics.median(x['times_ms']['candidate'] for x in rows),'ducc_median_ms':statistics.median(x['times_ms']['ducc'] for x in rows),
        'candidate_parent_paired_median':statistics.median(ratios),'candidate_parent_faster':sum(x<1 for x in ratios),'max_parent_rel_l2':max(x['candidate_vs_parent']['relative_l2'] for x in rows),'max_parent_abs':max(x['candidate_vs_parent']['max_absolute'] for x in rows),
        'all_worker_checks':all(all(v.get('finite_outputs') and v.get('repeat_consistent') and v.get('reset_consistent') for v in x['checks'].values()) for x in rows),
        'fixture_sha256':{k:sha(v) for k,v in fixture.items()},'rows':rows}
 all_rows.append(stats); (OUT/f'{ident}-summary.json').write_text(json.dumps(stats,indent=2)+'\n'); print(json.dumps({k:stats[k] for k in ('case_id','parent_median_ms','candidate_median_ms','ducc_median_ms','candidate_parent_paired_median','candidate_parent_faster','all_worker_checks')}),flush=True)
manifest={'parent_sha':subprocess.check_output(['git','-C',str(ROOT/'parent'),'rev-parse','HEAD'],text=True).strip(),'candidate_patch_sha256':sha(ROOT/'candidate.patch'),'ducc_source_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b','rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs','ducc_flags':'-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math','q1_affinity':'0','q8_affinity':'0,2,4,6,8,10,12,14','warm_target_ms':25,'case_summaries':all_rows}
(OUT/'results.json').write_text(json.dumps(manifest,indent=2)+'\n')
