import argparse, hashlib, importlib.util, json, math, os, random, statistics, tempfile
from pathlib import Path
repo=Path(__file__).resolve().parents[4]
parser=argparse.ArgumentParser()
parser.add_argument('--scratch', type=Path, default=Path(tempfile.gettempdir())/'rustsht-m6-f32-packet-suffix')
parser.add_argument('--q1-cpu', type=int)
parser.add_argument('--q8-cpus', help='comma-separated CPU IDs for eight physical cores')
args=parser.parse_args()
root=args.scratch
available=sorted(os.sched_getaffinity(0))
q1_cpu=args.q1_cpu if args.q1_cpu is not None else available[0]
q8_cpus=args.q8_cpus if args.q8_cpus else ','.join(map(str,available[:8]))
if len(q8_cpus.split(',')) != 8:
 raise SystemExit('--q8-cpus must name eight CPUs')
spec=importlib.util.spec_from_file_location('bench',repo/'scripts/benchmark_ducc_f32_native.py')
bench=importlib.util.module_from_spec(spec); spec.loader.exec_module(bench)
bins={
 'parent':root/'parent-v3-target/release/examples/ducc_f32_full_transform_parity',
 'candidate':root/'candidate-v3-target/release/examples/ducc_f32_full_transform_parity',
 'ducc':root/'ducc-unprofiled'}
orders=[('parent','candidate','ducc'),('ducc','parent','candidate'),('candidate','ducc','parent')]
out=root/'v3-matrix'; out.mkdir(parents=True,exist_ok=True)
def sha(path):
 h=hashlib.sha256();
 with open(path,'rb') as f:
  for b in iter(lambda:f.read(1<<20),b''): h.update(b)
 return h.hexdigest()
def bootstrap_ci(values, n=30000, seed=43107):
 rng=random.Random(seed); means=[]; count=len(values)
 for _ in range(n): means.append(statistics.median(rng.choices(values,k=count)))
 means.sort(); return [means[int(.025*n)],means[int(.975*n)]]
def hash_fixture(fixture): return {k:sha(v) for k,v in fixture.items()}
def run_case(case, count, label):
 case_id=f"GL-T720-M{case['mmax']}-s{case['spin']}-{case['operation']}-q{case['threads']}"
 fixture=bench.make_fixture(case,root/'fixtures-v3'/case_id)
 cpus=str(q1_cpu) if case['threads']==1 else q8_cpus
 rows=[]
 for pi in range(1,count+1):
  order=orders[(pi-1)%3]; workers={}
  for backend in order:
   run_dir=out/case_id/f'pair-{pi:02d}'/backend
   workers[backend]=bench.run_worker(bins[backend],backend,case,fixture,run_dir,25.0,cpus)
  meds={k:statistics.median(workers[k]['batch_ms']) for k in bins}
  e_cp=bench.error_metrics([Path(x) for x in workers['parent']['outputs']],[Path(x) for x in workers['candidate']['outputs']])
  e_cd=bench.error_metrics([Path(x) for x in workers['ducc']['outputs']],[Path(x) for x in workers['candidate']['outputs']])
  ratio=meds['candidate']/meds['parent']
  row={'pair':pi,'order':order,'batch_medians_ms':meds,'candidate_parent_ratio':ratio,
       'parent_ducc_ratio':meds['parent']/meds['ducc'],'candidate_ducc_ratio':meds['candidate']/meds['ducc'],
       'checks':{k:{x:workers[k].get(x) for x in ('finite_outputs','repeat_consistent','reset_consistent','simd_arch','f32_lanes')} for k in bins},
       'errors':{'candidate_parent':e_cp,'candidate_ducc':e_cd},
       'telemetry':{k:workers[k].get('telemetry') for k in bins}}
  rows.append(row)
  for k in bins:
   for p in workers[k].get('outputs',[]):
    try: Path(p).unlink()
    except FileNotFoundError: pass
  print(json.dumps({'case':case_id,'pair':pi,'order':order,'ms':meds,'cp':ratio,'cd':row['candidate_ducc_ratio'],'error':e_cp['relative_l2']}),flush=True)
 ratios=[r['candidate_parent_ratio'] for r in rows]
 cell={'case_id':case_id,'case':case,'pairs':rows,
       'medians_ms':{k:statistics.median([r['batch_medians_ms'][k] for r in rows]) for k in bins},
       'paired_ratio_medians':{k:statistics.median([r[k] for r in rows]) for k in ('candidate_parent_ratio','parent_ducc_ratio','candidate_ducc_ratio')},
       'candidate_parent_bootstrap_95_ci':bootstrap_ci(ratios) if count>=15 else None,
       'candidate_parent_wins':sum(x<1 for x in ratios),
       'max_candidate_parent_relative_l2':max(r['errors']['candidate_parent']['relative_l2'] for r in rows),
       'max_candidate_parent_absolute':max(r['errors']['candidate_parent']['max_absolute'] for r in rows),
       'max_candidate_ducc_relative_l2':max(r['errors']['candidate_ducc']['relative_l2'] for r in rows),
       'max_candidate_ducc_absolute':max(r['errors']['candidate_ducc']['max_absolute'] for r in rows),
       'fixtures_sha256':hash_fixture(fixture)}
 return cell
cases=[]
for mmax in (720,180):
 for q in (1,8): cases.append(({'grid':'GL','ntheta':721,'nphi':bench.smooth_even_at_least(2*mmax+2),'lmax':720,'mmax':mmax,'longitude':'even','spin':0,'operation':'synthesis','threads':q},3,'primary'))
for spin,op in ((0,'analysis'),(1,'synthesis'),(1,'analysis')):
 for q in (1,8): cases.append(({'grid':'GL','ntheta':721,'nphi':1458,'lmax':720,'mmax':720,'longitude':'even','spin':spin,'operation':op,'threads':q},3,'control'))
results={'parent_source':'54b166f2c0698f1c70e88f8679d12fa4376704fe',
 'candidate_patch_sha256':'67bcd6dfd59da3077636aec1bf1df09284e4663ea5c0043148e6900b977db242',
 'ducc_source':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
 'binaries_sha256':{k:sha(v) for k,v in bins.items()},
 'rust_target_cpu':'x86-64-v3','rust_flags':'-C target-cpu=x86-64-v3','ducc_flags':'-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3',
 'host':'AMD Ryzen 9 5950X','compiler':'rustc 1.99.1 / LLVM 23.1.1; DUCC GCC 15.2.0',
 'warm_protocol':'same fixture per backend; reusable plans/workspaces; fresh worker processes interleaved P/C/D; warm batches target 25 ms; fixed q1/q8 CPU affinity',
 'cells':[]}
for case,n,label in cases:
 cell=run_case(case,n,label); cell['group']=label; results['cells'].append(cell)
 (out/'results.partial.json').write_text(json.dumps(results,indent=2)+'\n')
(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
