#!/usr/bin/env python3
import hashlib,json,os,re,subprocess
from pathlib import Path
root=Path(os.environ['M6_OUTPUT_ROOT'])
parent=Path(os.environ['M6_PARENT_SOURCE'])
candidate=Path(os.environ['M6_CANDIDATE_SOURCE'])
fixtures=Path(os.environ['M6_FIXTURES'])
ducc=Path(os.environ['M6_DUCC_BINARY'])
outdir=root/'results'/'extension-q8-reduced';outdir.mkdir(parents=True,exist_ok=True)
case={'tag':'reduced_q8','fixture':'reduced','threads':8,'nphi':378,'mmax':180}
test='transform::tests::m6_profile_scalar_analysis_f32_fft_f64_recurrence'
cargo=['cargo','test','--release','--features','parallel','--lib',test,'--','--exact','--nocapture']
def rust(pair,backend):
 src,target=(parent,Path(os.environ['M6_PARENT_TARGET'])) if backend=='parent' else (candidate,Path(os.environ['M6_CANDIDATE_TARGET']))
 output=outdir/f'p{pair}-{backend}.f32c';env=os.environ.copy()
 env.update({'CARGO_TARGET_DIR':str(target),'M6_SCALAR_NPHI':'378','M6_SCALAR_MMAX':'180','M6_SCALAR_THREADS':'8','M6_SCALAR_ROUTE':'f32_fft_f64_recurrence','M6_SCALAR_RUNS':'7','M6_SCALAR_MAP':str(fixtures/'reduced-map0.f32'),'M6_SCALAR_OUTPUT':str(output)})
 proc=subprocess.run(cargo,cwd=src,env=env,text=True,capture_output=True,check=True)
 line=next(x for x in proc.stdout.splitlines() if '"route"' in x)
 line=re.sub(r'"peak_rss_kib":Some\((\d+)\)',r'"peak_rss_kib":\1',line)
 summary=json.loads(line[line.index('{'):])
 return {'case':case['tag'],'pair':pair,'backend':backend,'elapsed_ms':summary['median_ms'],'summary':summary,'output':str(output),'sha256':hashlib.sha256(output.read_bytes()).hexdigest(),'finite':summary['finite'],'reset':summary['poison_reset']}
def ducc_run(pair):
 output=outdir/f'p{pair}-ducc.f32c';fx='reduced'
 cmd=[str(ducc),'--worker','analysis','0','GL','8','721','378','720','180','0.23',str(fixtures/f'{fx}-map0.f32'),str(fixtures/f'{fx}-map1.f32'),str(fixtures/f'{fx}-alm0.f32c'),str(fixtures/f'{fx}-alm1.f32c'),str(output),str(outdir/f'p{pair}-unused.f32c'),'25']
 proc=subprocess.run(cmd,cwd=root,text=True,capture_output=True,check=True)
 summary=json.loads(next(x for x in proc.stdout.splitlines() if x.startswith('{"backend"')))
 times=sorted(summary['batch_ms'])
 return {'case':case['tag'],'pair':pair,'backend':'ducc','elapsed_ms':times[len(times)//2],'summary':summary,'output':str(output),'sha256':hashlib.sha256(output.read_bytes()).hexdigest(),'finite':summary['finite_outputs'],'reset':summary['reset_consistent']}
os.sched_setaffinity(0,set(range(8)))
orders=[['parent','candidate','ducc'],['ducc','parent','candidate'],['candidate','ducc','parent']]
rows=[]
for pair in range(4,16):
 current={}
 for backend in orders[(pair-1)%3]:
  r=ducc_run(pair) if backend=='ducc' else rust(pair,backend)
  current[backend]=r;rows.append(r)
 print(json.dumps({'case':case['tag'],'pair':pair,'order':orders[(pair-1)%3],'candidate_parent':current['candidate']['elapsed_ms']/current['parent']['elapsed_ms'],'candidate_ducc':current['candidate']['elapsed_ms']/current['ducc']['elapsed_ms']},sort_keys=True),flush=True)
(outdir/'raw.json').write_text(json.dumps(rows,indent=2)+'\n')
