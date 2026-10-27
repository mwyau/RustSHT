from __future__ import annotations
import hashlib, itertools, json, os, statistics, sys, shutil
from pathlib import Path
import numpy as np
root=Path(os.environ['RUSTSHT_M6_SCRATCH']).resolve()
repo=Path(os.environ['RUSTSHT_REPO']).resolve()
sys.path.insert(0,str(repo/'scripts'))
import benchmark_ducc_full_transform_parity as common
import benchmark_ducc_spin1_native as spin
from benchmark_ducc_full_transform_parity import smooth_even_at_least
bins={
 'parent_scalar':root/'targets/parent/release/examples/ducc_full_transform_parity',
 'candidate_scalar':root/'targets/candidate/release/examples/ducc_full_transform_parity',
 'ducc_scalar':root/'ducc-build-f64scalar/ducc_full_transform_parity',
 'parent_spin':root/'targets/parent/release/examples/ducc_spin1_full_transform_parity',
 'candidate_spin':root/'targets/candidate/release/examples/ducc_spin1_full_transform_parity',
 'ducc_spin':root/'ducc-build-f64spin/ducc_spin1_full_transform_parity'}
for k,p in bins.items():
 if not p.is_file(): raise SystemExit(f'missing {k} {p}')
physical=common.physical_cpus(); monitor=physical[8]
orders=[('parent','candidate','ducc'),('candidate','ducc','parent'),('ducc','parent','candidate')]
provenance={'parent_sha':'0677b5bfe98117c1d1a5d5204b0a710828fcb9c8',
 'candidate_base_sha':'0677b5bfe98117c1d1a5d5204b0a710828fcb9c8',
 'ducc_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
 'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
 'ducc_flags':'DUCC numerical source -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver overrides fast math',
 'pairs':3,'binary_sha256':{k:hashlib.sha256(v.read_bytes()).hexdigest() for k,v in bins.items()},
 'q1_cpus':physical[:1],'q8_cpus':physical[:8]}
cases=[('spin1-T1023-S',1,'synthesis',1023,1023,1024)]
all_summaries=[]
for name,spin_value,op,lmax,mmax,ntheta in cases:
 nphi=smooth_even_at_least(2*mmax+2)
 case={'case_id':name,'grid':'GL','lmax':lmax,'mmax':mmax,'ntheta':ntheta,'nphi':nphi,
       'longitude_mode':'even_smooth'}
 if spin_value==0:
  fixture_dir=root/'results/controls-f64/fixtures'/name; fixture_dir.mkdir(parents=True,exist_ok=True)
  common.make_fixture(case,fixture_dir)
  fixture={'map_path':case['map_path'],'alm_path':case['alm_path']}
  input_hashes=case['input_sha256']
  backends={'parent':bins['parent_scalar'],'candidate':bins['candidate_scalar'],'ducc':bins['ducc_scalar']}
 else:
  fixture_dir=root/'results/controls-f64/fixtures'/name
  fixture_dir.mkdir(parents=True,exist_ok=True)
  fixture=spin.make_fixture(case,fixture_dir)
  input_hashes=fixture['sha256']
  backends={'parent':bins['parent_spin'],'candidate':bins['candidate_spin'],'ducc':bins['ducc_spin']}
 case_record={k:v for k,v in case.items() if k not in ('map_path','alm_path')}
 case_record['input_sha256']=input_hashes
 for threads in (1,8):
  rows=[]
  for pair,order in enumerate(orders,1):
   workers={}; outputs={}; pairroot=root/'results/controls-f64'/name/f'q{threads}'/f'pair-{pair}'
   pairroot.mkdir(parents=True,exist_ok=True)
   cpus=physical[:threads]
   for backend in order:
    outpath=pairroot/f'{backend}.bin'
    if spin_value==0:
     cmd=common.worker_command(backends,backend,case,op,threads,outpath,25.0,False)
     record,diag=common.run_pinned(cmd,cpus,monitor)
     arr=np.fromfile(outpath,dtype='<f8')
     outputs[backend]=[outpath]
    else:
     outpaths={k:pairroot/f'{backend}-{k}.bin' for k in ('map0','map1','e','b')}
     cmd=spin.worker_command(backends[backend],case,fixture,op,threads,outpaths,25.0)
     record,diag=common.run_pinned(cmd,cpus,monitor)
     names=('map0','map1') if op=='synthesis' else ('e','b')
     outputs[backend]=[outpaths[k] for k in names]
     dtype='<f8' if op=='synthesis' else '<c16'
     arr=np.concatenate([np.fromfile(outpaths[k],dtype=dtype) for k in names])
    if not record.get('repeat_consistent',record.get('reset_consistent',False)) or not record.get('reset_consistent',False):
     raise RuntimeError(f'repeat/reset failure {name} q{threads} {pair} {backend}')
    if not np.isfinite(arr).all(): raise RuntimeError(f'nonfinite output {name} q{threads} {pair} {backend}')
    workers[backend]={'batch_ms':record.get('batch_ms',[]),
      'median_ms':statistics.median(record['batch_ms']),
      'repeat_consistent':record.get('repeat_consistent',record.get('reset_consistent')),'reset_consistent':record['reset_consistent'],
      'output_sha256':[hashlib.sha256(p.read_bytes()).hexdigest() for p in outputs[backend]],
      'scheduler_wait_fraction':diag.get('wait_fraction')}
   ppaths,cpaths,dpaths=(outputs[k] for k in ('parent','candidate','ducc'))
   if len(ppaths)!=len(cpaths) or len(cpaths)!=len(dpaths): raise RuntimeError('output path count mismatch')
   pbytes=b''.join(p.read_bytes() for p in ppaths); cbytes=b''.join(p.read_bytes() for p in cpaths)
   pval=np.concatenate([np.fromfile(p,dtype='<f8' if spin_value==0 or op=='synthesis' else '<c16') for p in ppaths])
   cval=np.concatenate([np.fromfile(p,dtype='<f8' if spin_value==0 or op=='synthesis' else '<c16') for p in cpaths])
   dval=np.concatenate([np.fromfile(p,dtype='<f8' if spin_value==0 or op=='synthesis' else '<c16') for p in dpaths])
   row={'pair':pair,'order':list(order),'threads':threads,'case':case,
     'workers':workers,'case':case_record,'candidate_parent_bitwise_equal':pbytes==cbytes,
     'candidate_over_parent':workers['candidate']['median_ms']/workers['parent']['median_ms'],
     'candidate_over_ducc':workers['candidate']['median_ms']/workers['ducc']['median_ms'],
     'error_candidate_vs_parent':common.error_metrics(cval,pval),
     'error_candidate_vs_ducc':common.error_metrics(cval,dval)}
   rows.append(row)
   print(json.dumps({'case':name,'q':threads,'pair':pair,'c_over_p':row['candidate_over_parent'],
    'c_over_ducc':row['candidate_over_ducc'],'bitwise':row['candidate_parent_bitwise_equal'],
    'c_ducc_l2':row['error_candidate_vs_ducc']['relative_l2'],
    'wait_p':workers['parent']['scheduler_wait_fraction'],'wait_c':workers['candidate']['scheduler_wait_fraction']}),flush=True)
   shutil.rmtree(pairroot)
  summary={'case_id':name,'threads':threads,
    'parent_ms':statistics.median(r['workers']['parent']['median_ms'] for r in rows),
    'candidate_ms':statistics.median(r['workers']['candidate']['median_ms'] for r in rows),
    'ducc_ms':statistics.median(r['workers']['ducc']['median_ms'] for r in rows),
    'median_candidate_over_parent':statistics.median(r['candidate_over_parent'] for r in rows),
    'candidate_faster_pairs':sum(r['candidate_over_parent']<1 for r in rows),
    'all_bitwise':all(r['candidate_parent_bitwise_equal'] for r in rows),
    'max_candidate_ducc_l2':max(r['error_candidate_vs_ducc']['relative_l2'] for r in rows),
    'max_candidate_ducc_abs':max(r['error_candidate_vs_ducc']['absolute_max'] for r in rows)}
  all_summaries.append(summary)
  outfile=root/'results/controls-f64'/f'{name}-q{threads}.json'; outfile.parent.mkdir(parents=True,exist_ok=True)
  outfile.write_text(json.dumps({'provenance':provenance,'summary':summary,'pairs':rows},indent=2)+'\n')
  print(json.dumps({'summary':summary}),flush=True)
print(f'completed {len(all_summaries)} f64 control cells')
