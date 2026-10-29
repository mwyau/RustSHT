#!/usr/bin/env python3
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

CANDIDATE_SRC = Path(os.environ['M6_CANDIDATE_SOURCE'])
CANDIDATE_TARGET = Path(os.environ['M6_CANDIDATE_TARGET'])
PARENT_SRC = Path(os.environ['M6_PARENT_SOURCE'])
PARENT_TARGET = Path(os.environ['M6_PARENT_TARGET'])
EXPERIMENT_ROOT = Path(os.environ['M6_OUTPUT_ROOT'])
FIXTURES = Path(os.environ['M6_FIXTURES'])
DUCC = Path(os.environ['M6_DUCC_BINARY'])
RESULTS = EXPERIMENT_ROOT / 'results'
RESULTS.mkdir(exist_ok=True)
TEST = 'transform::tests::m6_profile_scalar_analysis_f32_fft_f64_recurrence'
CARGO = ['cargo', 'test', '--release', '--features', 'parallel', '--lib', TEST, '--', '--exact', '--nocapture']

CASES = [
    {'tag':'reduced_q1','fixture':'reduced','threads':1,'nphi':378,'mmax':180},
    {'tag':'reduced_q8','fixture':'reduced','threads':8,'nphi':378,'mmax':180},
    {'tag':'full_q1','fixture':'full','threads':1,'nphi':1458,'mmax':720},
    {'tag':'full_q8','fixture':'full','threads':8,'nphi':1458,'mmax':720},
]
ORDERS = [
    ['parent','candidate','native_parent','native_candidate','ducc'],
    ['ducc','native_candidate','candidate','parent','native_parent'],
    ['native_parent','parent','ducc','native_candidate','candidate'],
]

def median(values):
    values=sorted(values)
    return values[len(values)//2]

def rust(case, pair, backend):
    source, target = (PARENT_SRC, PARENT_TARGET) if backend.endswith('parent') or backend == 'parent' else (CANDIDATE_SRC, CANDIDATE_TARGET)
    route = 'native' if backend.startswith('native_') else 'f32_fft_f64_recurrence'
    output = RESULTS / f"{case['tag']}-p{pair}-{backend}.f32c"
    env=os.environ.copy()
    env.update({
        'CARGO_TARGET_DIR':str(target),
        'M6_SCALAR_NPHI':str(case['nphi']),
        'M6_SCALAR_MMAX':str(case['mmax']),
        'M6_SCALAR_THREADS':str(case['threads']),
        'M6_SCALAR_ROUTE':route,
        'M6_SCALAR_RUNS':'7',
        'M6_SCALAR_MAP':str(FIXTURES / f"{case['fixture']}-map0.f32"),
        'M6_SCALAR_OUTPUT':str(output),
    })
    proc=subprocess.run(CARGO,cwd=source,env=env,text=True,capture_output=True,check=True)
    line=next(line for line in proc.stdout.splitlines() if '"route"' in line)
    line=re.sub(r'"peak_rss_kib":Some\((\d+)\)',r'"peak_rss_kib":\1',line)
    summary=json.loads(line[line.index('{'):])
    return {'case':case['tag'],'pair':pair,'backend':backend,'summary':summary,
            'elapsed_ms':summary['median_ms'],'output':str(output),
            'sha256':hashlib.sha256(output.read_bytes()).hexdigest(),
            'finite':summary['finite'],'reset':summary['poison_reset']}

def ducc(case,pair):
    output=RESULTS / f"{case['tag']}-p{pair}-ducc.f32c"
    fixture=case['fixture']
    cmd=[str(DUCC),'--worker','analysis','0','GL',str(case['threads']),'721',
         str(case['nphi']),'720',str(case['mmax']),'0.23',
         str(FIXTURES/f'{fixture}-map0.f32'),str(FIXTURES/f'{fixture}-map1.f32'),
         str(FIXTURES/f'{fixture}-alm0.f32c'),str(FIXTURES/f'{fixture}-alm1.f32c'),
         str(output),str(RESULTS/f'{case["tag"]}-unused.f32c'),'25']
    proc=subprocess.run(cmd,cwd=EXPERIMENT_ROOT,text=True,capture_output=True,check=True)
    summary=json.loads(next(line for line in proc.stdout.splitlines() if line.startswith('{"backend"')))
    return {'case':case['tag'],'pair':pair,'backend':'ducc','summary':summary,
            'elapsed_ms':median(summary['batch_ms']),'output':str(output),
            'sha256':hashlib.sha256(output.read_bytes()).hexdigest(),
            'finite':summary['finite_outputs'],'reset':summary['reset_consistent']}

rows=[]
for case in CASES:
    os.sched_setaffinity(0,set(range(case['threads'])))
    for pair,order in enumerate(ORDERS,1):
        current={}
        for backend in order:
            result=ducc(case,pair) if backend=='ducc' else rust(case,pair,backend)
            current[backend]=result
            rows.append(result)
        print(json.dumps({'case':case['tag'],'pair':pair,'order':order,
             'parent_candidate':current['candidate']['elapsed_ms']/current['parent']['elapsed_ms'],
             'candidate_ducc':current['candidate']['elapsed_ms']/current['ducc']['elapsed_ms'],
             'native_parent_candidate':current['native_candidate']['elapsed_ms']/current['native_parent']['elapsed_ms']},sort_keys=True),flush=True)
    (RESULTS/f"{case['tag']}-pair-summary.json").write_text(json.dumps(rows[-15:],indent=2)+'\n')
(RESULTS/'raw-screen.json').write_text(json.dumps(rows,indent=2)+'\n')
