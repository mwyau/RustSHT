import hashlib,importlib.util,json,os,pathlib,random,statistics,subprocess
ROOT=pathlib.Path(os.environ['M6_EXPERIMENT_ROOT']);REPO=pathlib.Path(os.environ['M6_REPO_ROOT'])
spec=importlib.util.spec_from_file_location('bench',REPO/'scripts/benchmark_ducc_f32_native.py');bench=importlib.util.module_from_spec(spec);spec.loader.exec_module(bench)
B={'parent':ROOT/'target-parent/release/examples/ducc_f32_full_transform_parity','candidate':ROOT/'target-candidate-v2/release/examples/ducc_f32_full_transform_parity','ducc':ROOT/'ducc-build/ducc_f32_full_transform_parity'}
OUT=ROOT/'controls-v2-triage-results';FIX=ROOT/'controls-v2-triage-fixtures';RUN=ROOT/'controls-v2-triage-runs'
for p in (OUT,FIX,RUN):p.mkdir(exist_ok=True)
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for x in iter(lambda:f.read(1<<20),b''):h.update(x)
 return h.hexdigest()
def case(l,m,nphi,spin,op,q):return {'grid':'GL','ntheta':l+1,'nphi':nphi,'lmax':l,'mmax':m,'longitude':'even','spin':spin,'operation':op,'threads':q}
cases=[]
for m,n in ((720,1458),(180,378)):
 for q in (1,8):cases.append(case(720,m,n,0,'synthesis',q))
for m,n in ((720,1458),(180,378)):
 for q in (1,8):cases.append(case(720,m,n,0,'analysis',q))
cases.extend([case(720,720,1458,1,'synthesis',8),case(720,720,1458,1,'analysis',8),case(1023,1023,bench.smooth_even_at_least(2048),0,'synthesis',8)])
orders=[('parent','candidate','ducc'),('candidate','ducc','parent'),('ducc','parent','candidate')];rng=random.Random(0xB2C0DE);summaries=[]
for c in cases:
 ident=f"GL-T{c['lmax']}-M{c['mmax']}-s{c['spin']}-{c['operation']}-q{c['threads']}";fixture=bench.make_fixture(c,FIX/ident);cpus='0' if c['threads']==1 else '0,2,4,6,8,10,12,14';rows=[]
 for i in range(1,4):
  order=list(orders[(i-1)%3]);
  if rng.random()<.5:order.reverse()
  w={}
  for name in order:w[name]=bench.run_worker(B[name],name,c,fixture,RUN/ident/f'pair-{i:02d}'/name,25.,cpus)
  t={name:statistics.median(w[name]['batch_ms']) for name in B};paths={name:[pathlib.Path(x) for x in w[name]['outputs']] for name in B}
  row={'pair':i,'order':order,'times_ms':t,'candidate_parent_ratio':t['candidate']/t['parent'],'candidate_ducc_ratio':t['candidate']/t['ducc'],
       'candidate_vs_parent':bench.error_metrics(paths['parent'],paths['candidate']),'candidate_vs_ducc':bench.error_metrics(paths['ducc'],paths['candidate']),
       'checks':{name:{k:w[name].get(k) for k in ('finite_outputs','repeat_consistent','reset_consistent')} for name in B}}
  for grp in paths.values():
   for p in grp:p.unlink(missing_ok=True)
  rows.append(row);(OUT/f'{ident}-pair-{i}.json').write_text(json.dumps(row,indent=2)+'\n');print('completed',ident,'pair',i,flush=True)
 summary={'case_id':ident,'pairs':len(rows),'candidate_parent_paired_median':statistics.median(x['candidate_parent_ratio'] for x in rows),'candidate_parent_faster':sum(x['candidate_parent_ratio']<1 for x in rows),
          'parent_median_ms':statistics.median(x['times_ms']['parent'] for x in rows),'candidate_median_ms':statistics.median(x['times_ms']['candidate'] for x in rows),'ducc_median_ms':statistics.median(x['times_ms']['ducc'] for x in rows),
          'candidate_ducc_paired_median':statistics.median(x['candidate_ducc_ratio'] for x in rows),'max_parent_relative_l2':max(x['candidate_vs_parent']['relative_l2'] for x in rows),'all_checks':all(all(v.get('repeat_consistent') and v.get('reset_consistent') for v in x['checks'].values()) and x['candidate_vs_parent']['finite'] for x in rows),'rows':rows}
 summaries.append(summary);(OUT/f'{ident}-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({k:summary[k] for k in ('case_id','candidate_parent_paired_median','candidate_parent_faster','all_checks')}),flush=True)
(OUT/'results.json').write_text(json.dumps({'parent_sha':subprocess.check_output(['git','-C',str(ROOT/'parent'),'rev-parse','HEAD'],text=True).strip(),'candidate_v2_source':'method-order isolation variant','ducc_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b','rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs','ducc_flags':'-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math','cases':summaries},indent=2)+'\n')
