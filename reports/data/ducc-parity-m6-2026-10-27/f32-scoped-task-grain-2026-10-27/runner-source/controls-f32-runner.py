from __future__ import annotations
import hashlib, itertools, json, os, statistics, sys, shutil
from pathlib import Path
root=Path(os.environ['RUSTSHT_M6_SCRATCH']).resolve()
repo=Path(os.environ['RUSTSHT_REPO']).resolve()
sys.path.insert(0,str(repo/'scripts'))
from benchmark_ducc_f32_native import error_metrics, make_fixture, run_worker, smooth_even_at_least
bins={'parent':root/'targets/parent/release/examples/ducc_f32_full_transform_parity',
      'candidate':root/'targets/candidate/release/examples/ducc_f32_full_transform_parity',
      'ducc':root/'ducc-build/f32-native/ducc_f32_full_transform_parity'}
orders=[('parent','candidate','ducc'),('candidate','ducc','parent'),('ducc','parent','candidate')]
provenance={'parent_sha':'0677b5bfe98117c1d1a5d5204b0a710828fcb9c8',
 'candidate_base_sha':'0677b5bfe98117c1d1a5d5204b0a710828fcb9c8',
 'ducc_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
 'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs','q1_cpu':'0','q8_cpus':'0-7',
 'pairs':3,'target_ms':10.0,'binary_sha256':{k:hashlib.sha256(v.read_bytes()).hexdigest() for k,v in bins.items()}}
allrows=[]
for spin,op,threads in [(0,'analysis',1),(0,'analysis',8),(1,'analysis',1),(1,'analysis',8),(1,'synthesis',1),(1,'synthesis',8)]:
 case={'grid':'GL','ntheta':721,'nphi':smooth_even_at_least(1442),'lmax':720,'mmax':720,
       'longitude':'even','spin':spin,'operation':op,'threads':threads}
 cid=f"GL-T720-M720-even-s{spin}-{op}-q{threads}"
 fix=make_fixture(case,root/'results/controls-f32/fixtures'/cid)
 fixture_hashes={k:hashlib.sha256(v.read_bytes()).hexdigest() for k,v in fix.items()}
 rows=[]
 for pair,order in enumerate(orders,1):
  workers={}; pairroot=root/'results/controls-f32'/cid/f'pair-{pair}'
  cpus='0' if threads==1 else '0-7'
  for backend in order:
   out=run_worker(bins[backend],backend,case,fix,pairroot/backend,10.0,cpus)
   for key in ('finite_outputs','repeat_consistent','reset_consistent'):
    if not out.get(key,False): raise RuntimeError(f'{key} failure in {cid} pair {pair} {backend}')
   paths=[Path(p) for p in out['outputs']]
   workers[backend]=(out,paths)
  if not (len(workers['parent'][1])==len(workers['candidate'][1])==len(workers['ducc'][1])): raise RuntimeError('output count differs')
  p,c,d=(workers[k][0] for k in ('parent','candidate','ducc'))
  pp,cc,dd=(workers[k][1] for k in ('parent','candidate','ducc'))
  row={'pair':pair,'order':list(order),'case':case,'fixture_sha256':fixture_hashes,
   'workers':{k:{'batch_ms':workers[k][0]['batch_ms'],'median_ms':statistics.median(workers[k][0]['batch_ms']),
      'finite_outputs':workers[k][0]['finite_outputs'],'repeat_consistent':workers[k][0]['repeat_consistent'],
      'reset_consistent':workers[k][0]['reset_consistent'],'output_sha256':[hashlib.sha256(x.read_bytes()).hexdigest() for x in workers[k][1]]} for k in workers},
   'candidate_parent_bitwise_equal':all(a.read_bytes()==b.read_bytes() for a,b in zip(pp,cc)),
   'candidate_over_parent':statistics.median(c['batch_ms'])/statistics.median(p['batch_ms']),
   'candidate_over_ducc':statistics.median(c['batch_ms'])/statistics.median(d['batch_ms']),
   'error_candidate_vs_parent':error_metrics(pp,cc),'error_candidate_vs_ducc':error_metrics(dd,cc)}
  rows.append(row); allrows.append({'case_id':cid,**row})
  file=root/'results/controls-f32'/f'{cid}.json'; file.parent.mkdir(parents=True,exist_ok=True)
  file.write_text(json.dumps({'provenance':provenance,'case_id':cid,'fixture_sha256':fixture_hashes,'pairs':rows},indent=2)+'\n')
  print(json.dumps({'case':cid,'pair':pair,'c_over_p':row['candidate_over_parent'],'c_over_ducc':row['candidate_over_ducc'],'bitwise':row['candidate_parent_bitwise_equal'],'c_ducc_l2':row['error_candidate_vs_ducc']['relative_l2']}),flush=True)
  shutil.rmtree(pairroot)
 summary={'case_id':cid,'parent_ms':statistics.median(r['workers']['parent']['median_ms'] for r in rows),
  'candidate_ms':statistics.median(r['workers']['candidate']['median_ms'] for r in rows),
  'ducc_ms':statistics.median(r['workers']['ducc']['median_ms'] for r in rows),
  'median_candidate_over_parent':statistics.median(r['candidate_over_parent'] for r in rows),
  'candidate_faster_pairs':sum(r['candidate_over_parent']<1 for r in rows),
  'all_bitwise':all(r['candidate_parent_bitwise_equal'] for r in rows),
  'max_candidate_ducc_l2':max(r['error_candidate_vs_ducc']['relative_l2'] for r in rows),
  'max_candidate_ducc_abs':max(r['error_candidate_vs_ducc']['max_absolute'] for r in rows)}
 file=root/'results/controls-f32'/f'{cid}.json'
 file.write_text(json.dumps({'provenance':provenance,'summary':summary,'pairs':rows},indent=2)+'\n')
 print(json.dumps({'summary':summary}),flush=True)
print(f'completed {len(allrows)} adjacent f32 parent/candidate/DUCC triplets')
