#!/usr/bin/env python3
"""Matched GL T720 whole-call Rust/DUCC f32/f64 benchmark."""
from __future__ import annotations
import hashlib, importlib.util, json, os, statistics, struct, subprocess, sys, time
from pathlib import Path
import numpy as np

HERE=Path(__file__).resolve().parent
ROOT=Path(os.environ.get('RUSTSHT_CANDIDATE_ROOT', HERE.parents[4])).resolve()
PARENT=Path(os.environ.get('RUSTSHT_PARENT_ROOT', ROOT)).resolve()
BASELINE=Path(os.environ.get('RUSTSHT_BASELINE_ROOT', ROOT.parent/'m6-parity-baseline-20261030')).resolve()
OUT=Path(os.environ.get('RUSTSHT_RESULTS_DIR', ROOT/'target'/'b1-full-q8-control-results')).resolve()
DUCC=Path(os.environ.get('RUSTSHT_DUCC_ROOT', ROOT.parent/'RustSHT-m6-ducc-20261030')).resolve()
F32_RUST=ROOT/'target/release/examples/ducc_f32_full_transform_parity'
PARENT_F32=PARENT/'target/release/examples/ducc_f32_full_transform_parity'
F64_SCALAR_RUST=ROOT/'target/release/examples/ducc_full_transform_parity'
F64_SPIN_RUST=ROOT/'target/release/examples/ducc_spin1_full_transform_parity'
PARENT_F64_SCALAR=PARENT/'target/release/examples/ducc_full_transform_parity'
PARENT_F64_SPIN=PARENT/'target/release/examples/ducc_spin1_full_transform_parity'
DUCC_F32=BASELINE/'ducc-f32'/'ducc_f32_full_transform_parity'
DUCC_F64_SCALAR=BASELINE/'ducc-f64-scalar'/'ducc_full_transform_parity'
DUCC_F64_SPIN=BASELINE/'ducc-f64-spin'/'ducc_spin1_full_transform_parity'
PATCH=Path(os.environ.get('RUSTSHT_CANDIDATE_PATCH', HERE/'source.patch.gz')).resolve()
TARGET_MS=25.0
PAIRS=15
PHI0='0.23'
sys.path.insert(0,str(ROOT/'scripts'))
spec=importlib.util.spec_from_file_location('f32bench',ROOT/'scripts/benchmark_ducc_f32_native.py')
bench=importlib.util.module_from_spec(spec); spec.loader.exec_module(bench)

def sha(path:Path)->str:
    data=path.read_bytes()
    if path.suffix == '.gz':
        import gzip
        data=gzip.decompress(data)
    return hashlib.sha256(data).hexdigest()
    data=path.read_bytes()
    if path.suffix == '.gz':
        import gzip
        data=gzip.decompress(data)
    return hashlib.sha256(data).hexdigest()
def f64_fixture(src:Path,dst:Path)->None:
    np.fromfile(src,dtype='<f4').astype('<f8').tofile(dst)
def family(case): return ('spin1_' if case['spin']==1 else 'scalar_')+case['operation']
def case_id(c): return f"GL-T720-M{c['mmax']}-even-s{c['spin']}-{c['operation']}-q{c['threads']}"
def output_paths(directory:Path, precision:str, spin:int, operation:str):
    directory.mkdir(parents=True,exist_ok=True)
    if spin==0: return [directory/('out.f32' if precision=='f32' else 'out.f64')]
    suffix='f32' if precision=='f32' else 'f64'
    return [directory/f'out0.{suffix}',directory/f'out1.{suffix}',directory/f'oute.{suffix}',directory/f'outb.{suffix}']
def run_f64(binary:Path, case:dict, fixture:dict[str,Path], run_dir:Path, backend:str, cpus:str):
    paths=output_paths(run_dir,'f64',case['spin'],case['operation'])
    q=case['threads']; base=[str(binary),'--worker',case['operation']]
    if case['spin']==0:
        command=base+[str(q),str(case['ntheta']),str(case['nphi']),str(case['lmax']),str(case['mmax']),PHI0,
                      str(fixture['map0']),str(fixture['alm0']),str(paths[0]),str(TARGET_MS),'0','GL']
    else:
        command=base+['GL',str(q),str(case['ntheta']),str(case['nphi']),str(case['lmax']),str(case['mmax']),PHI0,
                      str(fixture['map0']),str(fixture['map1']),str(fixture['alm0']),str(fixture['alm1']),
                      *(str(p) for p in paths),str(TARGET_MS)]
    env=os.environ.copy(); env['RAYON_NUM_THREADS']=str(q)
    start=time.monotonic()
    proc=subprocess.run(['taskset','-c',cpus,*command],text=True,capture_output=True,env=env)
    if proc.returncode: raise RuntimeError(f'{backend} failed {case}:\n{proc.stderr}\n{proc.stdout}')
    data=json.loads(proc.stdout.strip().splitlines()[-1]); data['process_elapsed_s']=time.monotonic()-start
    data['backend']=backend; data['binary_sha256']=sha(binary); relevant=paths if case['spin']==0 else (paths[:2] if case['operation']=='synthesis' else paths[2:]); data['outputs']=[str(p) for p in relevant]
    run_dir.mkdir(parents=True,exist_ok=True); (run_dir/'result.json').write_text(json.dumps(data,indent=2)+'\n')
    return data

def run_f32(binary:Path, case:dict, fixture:dict[str,Path], run_dir:Path, backend:str, cpus:str):
    data=bench.run_worker(binary,backend,case,fixture,run_dir,TARGET_MS,cpus)
    data['binary_sha256']=sha(binary); (run_dir/'result.json').write_text(json.dumps(data,indent=2)+'\n')
    return data

def median_ms(data): return statistics.median(data['batch_ms'])
def read_outputs(data,precision,case,backend):
    dtype='<f4' if precision=='f32' else '<f8'
    arrays=[np.fromfile(p,dtype=dtype).astype(np.float64) for p in data['outputs']]
    # DUCC's scalar f64 analysis adapter retains the mmax-length leading pad.
    if backend=='ducc64' and case['spin']==0 and case['operation']=='analysis':
        arrays[0]=arrays[0][2*case['mmax']:]
    return arrays
def rel_l2(a,b):
    aa=np.concatenate(a); bb=np.concatenate(b)
    den=float(np.linalg.norm(bb))
    return float(np.linalg.norm(aa-bb)/den) if den else (0.0 if np.array_equal(aa,bb) else float('inf'))
def unpin_outputs(*results):
    for result in results:
        for path in result.get('outputs',[]): Path(path).unlink(missing_ok=True)

def main():
    binaries=[F32_RUST,PARENT_F32,F64_SCALAR_RUST,F64_SPIN_RUST,PARENT_F64_SCALAR,PARENT_F64_SPIN,DUCC_F32,DUCC_F64_SCALAR,DUCC_F64_SPIN]
    for b in binaries:
        if not b.is_file(): raise SystemExit(f'missing worker: {b}')
    allowed=sorted(os.sched_getaffinity(0)); q1=str(allowed[0]); q8=','.join(map(str,allowed[:8]))
    cases=[c for c in bench.default_cases(False) if c['grid']=='GL' and c['lmax']==720 and c['ntheta']==721
           and c['mmax']==720 and c['threads']==8 and c['longitude']=='even'
           and c['operation']=='synthesis' and c['spin']==1]
    OUT.mkdir(parents=True,exist_ok=True)
    meta={'rust_source_sha':subprocess.check_output(['git','-C',str(ROOT),'rev-parse','HEAD'],text=True).strip(),
          'rust_source_tree':subprocess.check_output(['git','-C',str(ROOT),'rev-parse','HEAD^{tree}'],text=True).strip(),
          'analysis_source_sha256':sha(ROOT/'src/legendre/scalar_paired/analysis.rs'),'candidate_patch_sha256':sha(PATCH),
          'ducc_revision':subprocess.check_output(['git','-C',str(DUCC),'rev-parse','HEAD'],text=True).strip(),
          'ducc_expected_revision':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
          'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
          'cpu_affinity':{'q1':q1,'q8':q8},'cpu_model':subprocess.check_output(['lscpu'],text=True).split('Model name:')[1].splitlines()[0].strip(),
          'rustc':subprocess.check_output(['rustc','-Vv'],text=True), 'pairs':PAIRS,'target_batch_ms':TARGET_MS,
          'binary_sha256':{str(b):sha(b) for b in binaries},'cases':[]}
    if meta['ducc_revision']!=meta['ducc_expected_revision']: raise SystemExit(f"DUCC SHA mismatch: {meta['ducc_revision']}")
    for case in cases:
        cid=case_id(case); case_dir=OUT/cid; fixture32=bench.make_fixture(case,OUT/'fixtures32'/cid)
        fixture64={}
        for name,p in fixture32.items():
            dst=OUT/'fixtures64'/cid/f'{name}.f64'; dst.parent.mkdir(parents=True,exist_ok=True); f64_fixture(p,dst); fixture64[name]=dst
        for pair in range(1,PAIRS+1):
            rot=[('rust32',F32_RUST,'f32'),('parent32',PARENT_F32,'f32'),
                 ('rust64',F64_SPIN_RUST if case['spin'] else F64_SCALAR_RUST,'f64'),
                 ('parent64',PARENT_F64_SPIN if case['spin'] else PARENT_F64_SCALAR,'f64'),
                 ('ducc32',DUCC_F32,'f32'),('ducc64',DUCC_F64_SPIN if case['spin'] else DUCC_F64_SCALAR,'f64')]
            if pair%2==0: rot=list(reversed(rot))
            elif pair%3==0: rot=rot[2:]+rot[:2]
            res={}
            for name,binary,prec in rot:
                run_dir=case_dir/f'pair-{pair}'/name
                cpus=q1 if case['threads']==1 else q8
                data=run_f32(binary,case,fixture32,run_dir,name,cpus) if prec=='f32' else run_f64(binary,case,fixture64,run_dir,name,cpus)
                res[name]=data
            ms={k:median_ms(v) for k,v in res.items()}
            values={k:read_outputs(res[k],'f32' if k.endswith('32') else 'f64',case,k) for k in res}
            pairrow={'pair':pair,'ms':ms,
                     'ratio_f32_over_f64':ms['rust32']/ms['rust64'],
                     'ratio_rust_f32_over_ducc_f32':ms['rust32']/ms['ducc32'],
                     'ratio_rust_f64_over_ducc_f64':ms['rust64']/ms['ducc64'],
                     'ratio_candidate_over_parent':ms['rust32']/ms['parent32'],
                     'ratio_candidate_f64_over_parent_f64':ms['rust64']/ms['parent64'],
                     'output_rel_l2':{'rust_f32_vs_ducc_f32':rel_l2(values['rust32'],values['ducc32']),
                                      'rust_f64_vs_ducc_f64':rel_l2(values['rust64'],values['ducc64']),
                                      'rust_f32_vs_widened_rust_f64':rel_l2(values['rust32'],values['rust64']),
                                      'candidate_vs_parent_f32':rel_l2(values['rust32'],values['parent32']),
                                      'candidate_vs_parent_f64':rel_l2(values['rust64'],values['parent64'])},
                     'dispatch':{k:{x:v.get(x) for x in ('simd_arch','f32_lanes','backend','output_checksum','repeat_consistent','reset_consistent') if x in v} for k,v in res.items()},
                     'fixture_sha256':{k:sha(v) for k,v in fixture32.items()},
                     'workers':res}
            (case_dir/f'pair-{pair}.json').write_text(json.dumps(pairrow,indent=2)+'\n')
            summary_line={k:pairrow[k] for k in ('pair','ms','ratio_candidate_over_parent','ratio_candidate_f64_over_parent_f64','ratio_f32_over_f64','ratio_rust_f32_over_ducc_f32','ratio_rust_f64_over_ducc_f64')}
            print(json.dumps({'case':cid,**summary_line},sort_keys=True),flush=True)
            unpin_outputs(*res.values())
        pairrows=[json.loads(p.read_text()) for p in sorted(case_dir.glob('pair-*.json'))]
        metrics=('ratio_f32_over_f64','ratio_rust_f32_over_ducc_f32','ratio_rust_f64_over_ducc_f64','ratio_candidate_over_parent','ratio_candidate_f64_over_parent_f64')
        ratios={k:[row[k] for row in pairrows] for k in metrics}
        med={k:statistics.median(v) for k,v in ratios.items()}
        statuses={k:('GAP' if all(x>1.0 for x in v) else 'INCONCLUSIVE' if min(v)<=1.0<=max(v) else 'SCREEN_PASS_NEEDS_15_PAIR') for k,v in ratios.items()}
        entry={'case':case,'case_id':cid,'route':('current f32 production route; see source dispatch; f64 workers are matched diagnostic controls'),
               'paired_ratios':ratios,'median_ratios':med,'screen_status':statuses,'pair_workers':pairrows}
        (case_dir/'summary.json').write_text(json.dumps(entry,indent=2)+'\n')
        meta['cases'].append({'case_id':cid,'fixture_sha256':{k:sha(v) for k,v in fixture32.items()},'summary':entry})
    (OUT/'metadata.json').write_text(json.dumps(meta,indent=2)+'\n')
    print('SAVED '+str(OUT/'metadata.json'),flush=True)
if __name__=='__main__': main()
