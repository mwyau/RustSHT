#!/usr/bin/env python3
import hashlib
import json
import os
import pathlib
import random
import re
import statistics
import struct
import subprocess

PARENT = pathlib.Path(os.environ['M6_PARENT_SOURCE'])
PARENT_TARGET = pathlib.Path(os.environ['M6_PARENT_TARGET'])
CANDIDATE = pathlib.Path(os.environ['M6_CANDIDATE_SOURCE'])
CANDIDATE_TARGET = pathlib.Path(os.environ['M6_CANDIDATE_TARGET'])
DUCC = pathlib.Path(os.environ['M6_DUCC_BINARY'])
FIXTURES = pathlib.Path(os.environ['M6_FIXTURES'])
OUTPUT = pathlib.Path(os.environ['M6_OUTPUT_ROOT'])
PAIRS = int(os.environ.get('M6_PAIRS', '3'))
START_PAIR = int(os.environ.get('M6_START_PAIR', '1'))
RUSTFLAGS = '-C target-cpu=x86-64-v3 -Aunexpected_cfgs'
CASES = [
    {'tag':'reduced_q1','fixture':'reduced','threads':1,'nphi':378,'mmax':180},
    {'tag':'reduced_q8','fixture':'reduced','threads':8,'nphi':378,'mmax':180},
    {'tag':'full_q1','fixture':'full','threads':1,'nphi':1458,'mmax':720},
    {'tag':'full_q8','fixture':'full','threads':8,'nphi':1458,'mmax':720},
]
OUTPUT.mkdir(parents=True, exist_ok=True)
previous_path=OUTPUT/'paired-results.json'
previous=json.loads(previous_path.read_text()) if START_PAIR>1 and previous_path.exists() else []
selected={c['tag'] for c in CASES}
rows=[r for r in previous if r['case']['tag'] not in selected]

def rust(case,pair,backend):
    source,target=(PARENT,PARENT_TARGET) if backend=='parent' else (CANDIDATE,CANDIDATE_TARGET)
    out=OUTPUT/case['tag']/f'pair-{pair:02d}-{backend}.f32c'; out.parent.mkdir(parents=True,exist_ok=True)
    env=os.environ.copy(); env.update({'RUSTFLAGS':RUSTFLAGS,'CARGO_TARGET_DIR':str(target),'M6_SCALAR_NPHI':str(case['nphi']),'M6_SCALAR_MMAX':str(case['mmax']),'M6_SCALAR_THREADS':str(case['threads']),'M6_SCALAR_ROUTE':os.environ.get('M6_RUST_ROUTE','f32_fft_f64_recurrence_direct_spectra'),'M6_SCALAR_RUNS':'7','M6_SCALAR_MAP':str(FIXTURES/f"{case['fixture']}-map0.f32"),'M6_SCALAR_OUTPUT':str(out)})
    cmd=['cargo','test','--release','--features','parallel','--lib','transform::tests::m6_profile_scalar_analysis_f32_fft_f64_recurrence','--','--exact','--nocapture']
    proc=subprocess.run(cmd,cwd=source,env=env,text=True,capture_output=True,check=True)
    combined=proc.stdout+'\n'+proc.stderr
    line=next(line for line in combined.splitlines() if '"route"' in line)
    result=json.loads(re.sub(r'"peak_rss_kib":Some\((\d+)\)',r'"peak_rss_kib":\1',line[line.index('{'):]))
    exe_line=next(line for line in combined.splitlines() if 'Running unittests src/lib.rs' in line)
    exe=pathlib.Path(exe_line.split('(')[-1].rstrip(')'))
    if not exe.is_absolute(): exe=source/exe
    return {'elapsed_ms':result['median_ms'],'path':str(out),'sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'binary_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'finite':result['finite'],'reset':result['poison_reset'],'samples_ms':result['samples_ms']}

def ducc(case,pair):
    f=case['fixture']; out0=OUTPUT/case['tag']/f'pair-{pair:02d}-ducc-0.f32c'; out1=OUTPUT/case['tag']/f'pair-{pair:02d}-ducc-1.f32c'; out0.parent.mkdir(parents=True,exist_ok=True)
    cmd=[str(DUCC),'--worker','analysis','0','GL',str(case['threads']),'721',str(case['nphi']),'720',str(case['mmax']),'0.23',str(FIXTURES/f'{f}-map0.f32'),str(FIXTURES/f'{f}-map1.f32'),str(FIXTURES/f'{f}-alm0.f32c'),str(FIXTURES/f'{f}-alm1.f32c'),str(out0),str(out1),'25']
    proc=subprocess.run(cmd,text=True,capture_output=True,check=True); summary=json.loads(proc.stdout.strip().splitlines()[-1])
    return {'elapsed_ms':statistics.median(summary['batch_ms']),'path':str(out0),'sha256':hashlib.sha256(out0.read_bytes()).hexdigest(),'finite':summary['finite_outputs'],'reset':summary['reset_consistent'],'samples_ms':summary['batch_ms']}

def read(path):
    raw=pathlib.Path(path).read_bytes(); return struct.unpack(f'<{len(raw)//4}f',raw)
def error(ref,got):
    a=read(ref); b=read(got); d=[x-y for x,y in zip(b,a)]
    return {'relative_l2':(sum(x*x for x in d)/sum(x*x for x in a))**0.5,'max_absolute':max(map(abs,d),default=0.0)}

orders=[['parent','candidate','ducc'],['ducc','parent','candidate'],['candidate','ducc','parent']]
for case in CASES:
    pair_rows=[r for r in previous if r['case']['tag']==case['tag'] and r['pair']<START_PAIR]
    rows.extend(pair_rows)
    for pair in range(START_PAIR,PAIRS+1):
        order=orders[(pair-1)%3]; os.sched_setaffinity(0,set(range(case['threads']))); results={}
        for backend in order: results[backend]=ducc(case,pair) if backend=='ducc' else rust(case,pair,backend)
        row={'case':case,'pair':pair,'order':order,'times_ms':{k:results[k]['elapsed_ms'] for k in results},'candidate_parent_ratio':results['candidate']['elapsed_ms']/results['parent']['elapsed_ms'],'candidate_ducc_ratio':results['candidate']['elapsed_ms']/results['ducc']['elapsed_ms'],'outputs':{k:{'sha256':results[k]['sha256'],'finite':results[k]['finite'],'reset':results[k]['reset']} for k in results},'candidate_vs_parent':error(results['parent']['path'],results['candidate']['path']),'candidate_vs_ducc':error(results['ducc']['path'],results['candidate']['path']),'binary_sha256':{k:results[k]['binary_sha256'] for k in ('parent','candidate')}}
        for result in results.values(): pathlib.Path(result['path']).unlink(missing_ok=True)
        pair_rows.append(row); rows.append(row); print(json.dumps(row,sort_keys=True),flush=True)
    summary={'case':case,'pairs':pair_rows,'candidate_parent_paired_median':statistics.median(r['candidate_parent_ratio'] for r in pair_rows),'candidate_ducc_paired_median':statistics.median(r['candidate_ducc_ratio'] for r in pair_rows),'candidate_parent_faster':sum(r['candidate_parent_ratio']<1 for r in pair_rows)}
    (OUTPUT/f"{case['tag']}-summary.json").write_text(json.dumps(summary,indent=2)+'\n')
(OUTPUT/'paired-results.json').write_text(json.dumps(rows,indent=2)+'\n')
