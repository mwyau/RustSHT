from __future__ import annotations
import hashlib, itertools, json, os, random, statistics, sys, shutil
from pathlib import Path
root=Path(os.environ['RUSTSHT_M6_SCRATCH']).resolve()
repo=Path(os.environ['RUSTSHT_REPO']).resolve()
sys.path.insert(0,str(repo/'scripts'))
from benchmark_ducc_f32_native import error_metrics, make_fixture, run_worker, smooth_even_at_least
bins={
 'parent':root/'targets/parent/release/examples/ducc_f32_full_transform_parity',
 'candidate':root/'targets/candidate/release/examples/ducc_f32_full_transform_parity',
 'ducc':root/'ducc-build/f32-native/ducc_f32_full_transform_parity'}
for name,p in bins.items():
 if not p.is_file(): raise SystemExit(f'missing {name} binary {p}')
threads=int(sys.argv[1]); target_pairs=int(sys.argv[2])
provenance={
 'parent_sha':'0677b5bfe98117c1d1a5d5204b0a710828fcb9c8',
 'candidate_base_sha':'0677b5bfe98117c1d1a5d5204b0a710828fcb9c8',
 'candidate_patch_sha256':hashlib.sha256((root/'scoped-schedule.patch').read_bytes()).hexdigest(),
 'ducc_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
 'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
 'ducc_flags':'DUCC numerical sources -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math -ffp-contract=fast',
 'threads':threads,'cpus':'0' if threads==1 else '0-7','target_ms':10.0,
 'binary_sha256':{k:hashlib.sha256(v.read_bytes()).hexdigest() for k,v in bins.items()}}
orders=[('parent','candidate','ducc'),('candidate','ducc','parent'),('ducc','parent','candidate')]
orders=(orders*5)[:target_pairs]
random.Random(0x6D362026 + threads).shuffle(orders)
for mmax in (720,180):
 case={'grid':'GL','ntheta':721,'nphi':smooth_even_at_least(2*mmax+2),'lmax':720,'mmax':mmax,
       'longitude':'even','spin':0,'operation':'synthesis','threads':threads}
 cid=f'GL-T720-M{mmax}-even-s0-synthesis-q{threads}'
 fix=make_fixture(case,root/'results/synthesis/fixtures'/cid)
 fixture_hashes={k:hashlib.sha256(v.read_bytes()).hexdigest() for k,v in fix.items()}
 outfile=root/'results/synthesis'/f'{cid}.json'
 rows=[]
 if outfile.exists():
  old=json.loads(outfile.read_text())
  if old.get('provenance')!=provenance or old.get('fixture_sha256')!=fixture_hashes:
   raise RuntimeError(f'checkpoint mismatch {cid}')
  rows=old.get('pairs',[])
  if len(rows)>target_pairs: raise RuntimeError(f'checkpoint has too many pairs for {cid}')
 for pair,order in enumerate(orders[len(rows):],len(rows)+1):
  worker={}; paths={}; pairroot=root/'results/synthesis'/cid/f'pair-{pair:02d}'
  for backend in order:
   r=run_worker(bins[backend],backend,case,fix,pairroot/backend,10.0,provenance['cpus'])
   for key in ('finite_outputs','repeat_consistent','reset_consistent'):
    if not r.get(key,False): raise RuntimeError(f'{key} failed: {cid} pair {pair} {backend}')
   ps=[Path(x) for x in r['outputs']]
   worker[backend]={'batch_ms':r['batch_ms'],'median_ms':statistics.median(r['batch_ms']),
     'finite_outputs':r['finite_outputs'],'repeat_consistent':r['repeat_consistent'],
     'reset_consistent':r['reset_consistent'],'simd_arch':r.get('simd_arch'),'f32_lanes':r.get('f32_lanes'),
     'output_sha256':[hashlib.sha256(x.read_bytes()).hexdigest() for x in ps],
     'process_elapsed_s':r['process_elapsed_s'],'telemetry':r.get('telemetry')}
   paths[backend]=ps
  p,c,d=(worker[k] for k in ('parent','candidate','ducc'))
  pp,cp,dp=(paths[k] for k in ('parent','candidate','ducc'))
  row={'pair':pair,'order':list(order),'case':case,'fixture_sha256':fixture_hashes,'workers':worker,
    'candidate_parent_bitwise_equal':all(a.read_bytes()==b.read_bytes() for a,b in zip(pp,cp)),
    'candidate_over_parent':c['median_ms']/p['median_ms'],'candidate_over_ducc':c['median_ms']/d['median_ms'],
    'parent_over_ducc':p['median_ms']/d['median_ms'],
    'error_candidate_vs_parent':error_metrics(pp,cp),'error_candidate_vs_ducc':error_metrics(dp,cp),
    'error_parent_vs_ducc':error_metrics(dp,pp)}
  rows.append(row)
  outfile.parent.mkdir(parents=True,exist_ok=True)
  outfile.write_text(json.dumps({'provenance':provenance,'case_id':cid,'fixture_sha256':fixture_hashes,'pairs':rows},indent=2)+'\n')
  print(json.dumps({'case':cid,'pair':pair,'order':list(order),'parent_ms':p['median_ms'],
    'candidate_ms':c['median_ms'],'ducc_ms':d['median_ms'],'candidate_over_parent':row['candidate_over_parent'],
    'candidate_over_ducc':row['candidate_over_ducc'],'bitwise':row['candidate_parent_bitwise_equal'],
    'candidate_ducc_l2':row['error_candidate_vs_ducc']['relative_l2'],
    'candidate_ducc_max_abs':row['error_candidate_vs_ducc']['max_absolute']}),flush=True)
  shutil.rmtree(pairroot)
 ratios=[r['candidate_over_parent'] for r in rows]
 summary={'count':len(rows),'median_candidate_over_parent':statistics.median(ratios),
  'min':min(ratios),'max':max(ratios),'candidate_faster_pairs':sum(x<1 for x in ratios),
  'parent_ms_median':statistics.median(r['workers']['parent']['median_ms'] for r in rows),
  'candidate_ms_median':statistics.median(r['workers']['candidate']['median_ms'] for r in rows),
  'ducc_ms_median':statistics.median(r['workers']['ducc']['median_ms'] for r in rows),
  'median_candidate_over_ducc':statistics.median(r['candidate_over_ducc'] for r in rows),
  'bitwise_all':all(r['candidate_parent_bitwise_equal'] for r in rows),
  'max_candidate_parent_relative_l2':max(r['error_candidate_vs_parent']['relative_l2'] for r in rows),
  'max_candidate_parent_absolute':max(r['error_candidate_vs_parent']['max_absolute'] for r in rows),
  'max_candidate_ducc_relative_l2':max(r['error_candidate_vs_ducc']['relative_l2'] for r in rows),
  'max_candidate_ducc_absolute':max(r['error_candidate_vs_ducc']['max_absolute'] for r in rows)}
 result={'provenance':provenance,'case_id':cid,'fixture_sha256':fixture_hashes,'summary':summary,'pairs':rows}
 outfile.write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({'summary':summary}),flush=True)
