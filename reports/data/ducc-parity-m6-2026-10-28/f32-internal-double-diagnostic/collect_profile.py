import importlib.util, json, os, subprocess, hashlib
from pathlib import Path
repo_root=Path(__file__).resolve().parents[4]
root=Path(os.environ['M6_SCRATCH_ROOT']).resolve()
spec=importlib.util.spec_from_file_location('bench',repo_root/'scripts/benchmark_ducc_f32_native.py')
bench=importlib.util.module_from_spec(spec);spec.loader.exec_module(bench)
bins={
 'rust_f32':root/'target-profile/release/examples/ducc_f32_full_transform_parity',
 'rust_f64_default':root/'target-profile/release/examples/ducc_f32_io_f64_internal',
 'rust_f64_paired':root/'target-profile/release/examples/ducc_f32_io_f64_internal',
 'ducc':root/'build-ducc-profile/ducc_f32_full_transform_parity',
}
rows=[]
for mmax in (720,180):
 nphi=bench.smooth_even_at_least(2*mmax+2)
 case={'grid':'GL','ntheta':721,'nphi':nphi,'lmax':720,'mmax':mmax,'longitude':'even','spin':0,'operation':'synthesis','threads':1}
 fixture=bench.make_fixture(case,root/f'fixtures/T720-M{mmax}')
 hashes={k:hashlib.sha256(v.read_bytes()).hexdigest() for k,v in fixture.items()}
 for threads in (1,8):
  cpus='15' if threads==1 else '8-15'
  case['threads']=threads
  args=['--worker','synthesis','0','GL',str(threads),'721',str(nphi),'720',str(mmax),'0.23',str(fixture['map0']),str(fixture['map1']),str(fixture['alm0']),str(fixture['alm1']),str(root/'unused0.f32'),str(root/'unused1.f32'),'20']
  for name,binary in bins.items():
   env=os.environ.copy();env['M6_PROFILE_LEGENDRE']='1';env['M6_PROFILE_EXIT_AFTER_FIRST']='1';env['RAYON_NUM_THREADS']=str(threads)
   env.pop('M6_FORCE_PAIRED_F64',None);env.pop('M6_COUNT_DUCC_UPDATES',None)
   if name=='rust_f64_paired':env['M6_FORCE_PAIRED_F64']='1'
   if name=='ducc':env['M6_COUNT_DUCC_UPDATES']='1'
   cp=subprocess.run(['taskset','-c',cpus,str(binary),*args],env=env,text=True,capture_output=True,timeout=240)
   if cp.returncode:raise RuntimeError(f'{name} failed {case}: {cp.stderr}\n{cp.stdout}')
   row={'case':dict(case),'backend':name,'affinity':cpus,'stdout':cp.stdout,'stderr':cp.stderr}
   rows.append(row)
   print(json.dumps({'case':row['case'],'backend':name,'affinity':cpus,'stderr':cp.stderr.strip()}),flush=True)
  out=root/'profile-counts.json';out.write_text(json.dumps({'source_sha':'102675a171f59faeab93fece11817ab589bbacc8','ducc_sha':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b','rustflags':'-C target-cpu=x86-64-v3','fixtures':hashes,'rows':rows},indent=2)+'\n')
