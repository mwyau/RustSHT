import hashlib, importlib.util, json, os, pathlib, random, statistics, subprocess, sys
import numpy as np
ROOT=pathlib.Path(os.environ['M6_EXPERIMENT_ROOT']); REPO=pathlib.Path(os.environ['M6_REPO_ROOT']); sys.path.insert(0,str(REPO/'scripts'))
spec=importlib.util.spec_from_file_location('spinbench',REPO/'scripts/benchmark_ducc_spin1_native.py'); bench=importlib.util.module_from_spec(spec); spec.loader.exec_module(bench)
B={'parent':ROOT/'target-parent/release/examples/ducc_spin1_full_transform_parity','candidate':ROOT/'target-candidate/release/examples/ducc_spin1_full_transform_parity','ducc':ROOT/'ducc-f64-build/ducc_spin1_full_transform_parity'}
OUT=ROOT/'protected-f64-results'; FIX=ROOT/'protected-f64-fixtures'; RUN=ROOT/'protected-f64-runs'
for p in (OUT,FIX,RUN):p.mkdir(exist_ok=True)
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for x in iter(lambda:f.read(1<<20),b''):h.update(x)
 return h.hexdigest()
def boot(values,seed):
 rng=random.Random(seed); x=[]
 for _ in range(20000):x.append(statistics.median([values[rng.randrange(len(values))] for _ in values]))
 x.sort();return [x[int(.025*(len(x)-1))],x[int(.975*(len(x)-1))]]
case={'case_id':'GL-T1023-M1023-exact-p2048','grid':'GL','lmax':1023,'mmax':1023,'ntheta':1024,'nphi':2048,'latitude_mode':'exact','longitude_mode':'even-smooth'}
(FIX/'T1023').mkdir(parents=True,exist_ok=True); fixture=bench.make_fixture(case,FIX/'T1023')
orders=[('parent','candidate','ducc'),('candidate','ducc','parent'),('ducc','parent','candidate')]
rng=random.Random(0xF641023); all_summaries=[]
for q in (1,8):
 rows=[];cpus=[0] if q==1 else [0,2,4,6,8,10,12,14];monitor=8
 for i in range(1,4):
  order=list(orders[(i-1)%3]);
  if rng.random()<.5:order.reverse()
  w={};outmap={}
  for name in order:
   out={n:RUN/f'q{q}-pair{i:02d}-{name}-{n}.bin' for n in ('map0','map1','e','b')}
   cmd=bench.worker_command(B[name],case,fixture,'synthesis',q,out,25.0)
   result,diag=bench.common.run_pinned(cmd,cpus,monitor)
   result.update({'backend':name,'pair':i,'case':case,'threads':q,'host_diagnostics':diag})
   w[name]=result;outmap[name]=out
  t={name:statistics.median(w[name]['batch_ms']) for name in B}
  vals={name:np.concatenate([bench.output_values(outmap[name][x],'real') for x in ('map0','map1')]) for name in B}
  row={'pair':i,'order':order,'times_ms':t,'candidate_parent_ratio':t['candidate']/t['parent'],'candidate_ducc_ratio':t['candidate']/t['ducc'],
       'candidate_vs_parent':bench.common.error_metrics(vals['parent'],vals['candidate']),'candidate_vs_ducc':bench.common.error_metrics(vals['ducc'],vals['candidate']),
       'checks':{name:{k:w[name].get(k) for k in ('finite_outputs','repeat_consistent','reset_consistent')} for name in B},
       'host':{name:w[name].get('host_diagnostics') for name in B}}
  for outputs in outmap.values():
   for path in outputs.values():path.unlink(missing_ok=True)
  rows.append(row);(OUT/f'q{q}-pair-{i:02d}.json').write_text(json.dumps(row,indent=2)+'\n');print(f'completed f64 q{q} pair {i}',flush=True)
 ratios=[x['candidate_parent_ratio'] for x in rows]
 summary={'case':case,'threads':q,'pairs':15,'parent_median_ms':statistics.median(x['times_ms']['parent'] for x in rows),'candidate_median_ms':statistics.median(x['times_ms']['candidate'] for x in rows),'ducc_median_ms':statistics.median(x['times_ms']['ducc'] for x in rows),
          'candidate_parent_paired_median':statistics.median(ratios),'candidate_parent_bootstrap_95':boot(ratios,0xF640+q),'candidate_parent_faster':sum(x<1 for x in ratios),
          'candidate_ducc_paired_median':statistics.median(x['candidate_ducc_ratio'] for x in rows),'max_candidate_parent_relative_l2':max(x['candidate_vs_parent']['relative_l2'] for x in rows),'max_candidate_parent_absolute':max(x['candidate_vs_parent']['absolute_max'] for x in rows),
          'all_worker_checks':all(all(v.get('finite_outputs') and v.get('repeat_consistent') and v.get('reset_consistent') for v in r['checks'].values()) for r in rows),
          'fixture_sha256':{k:sha(v) for k,v in fixture['paths'].items()},'rows':rows}
 all_summaries.append(summary);(OUT/f'q{q}-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({k:summary[k] for k in ('threads','parent_median_ms','candidate_median_ms','ducc_median_ms','candidate_parent_paired_median','candidate_parent_bootstrap_95','candidate_parent_faster','all_worker_checks')}),flush=True)
metadata={'parent_sha':subprocess.check_output(['git','-C',str(ROOT/'parent'),'rev-parse','HEAD'],text=True).strip(),'candidate_patch_sha256':sha(ROOT/'candidate.patch'),'ducc_source_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b','rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs','ducc_flags':'-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math','q1_affinity':'0','q8_affinity':'0,2,4,6,8,10,12,14','warm_target_ms':25,'summaries':all_summaries}
(OUT/'results.json').write_text(json.dumps(metadata,indent=2)+'\n')
