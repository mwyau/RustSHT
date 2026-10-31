from __future__ import annotations
import json, os, pathlib, sys, tempfile, statistics, hashlib
import numpy as np

root = pathlib.Path(os.environ['RUSTSHT_NATIVE_SOURCE'])
sys.path.insert(0, str(root / 'scripts'))
import benchmark_ducc_spin1_native as bench
import benchmark_ducc_full_transform_parity as common

candidate = pathlib.Path(os.environ['RUSTSHT_CANDIDATE_BIN'])
parent = pathlib.Path(os.environ['RUSTSHT_PARENT_BIN'])
ducc = pathlib.Path(os.environ['RUSTSHT_DUCC_BIN'])
binaries = {'parent': parent, 'candidate': candidate, 'ducc': ducc}
for name, path in binaries.items():
    if not path.is_file(): raise FileNotFoundError(f'{name}: {path}')
    print(f'{name} {path} {hashlib.sha256(path.read_bytes()).hexdigest()}', flush=True)

case = next(c for c in bench.make_cases([1023], ['GL'], False)
            if c['mmax'] == 1023 and c['ntheta'] == 1024 and c['longitude_mode'] == 'even-smooth')
records=[]
with tempfile.TemporaryDirectory(prefix='f64-zero-restore-', dir=os.environ['RUSTSHT_BENCH_SCRATCH']) as tmp:
    temp=pathlib.Path(tmp)
    fixture=bench.make_fixture(case,temp)
    for threads in (1,8):
        cpus=common.physical_cpus()[:threads]
        monitor=common.physical_cpus()[8]
        for pair in range(3):
            order = [['parent','candidate','ducc'], ['candidate','ducc','parent'], ['ducc','parent','candidate']][pair % 3]
            run={}
            for name in order:
                outputs={k:temp/f'{name}-q{threads}-p{pair}-{k}.bin' for k in ('map0','map1','e','b')}
                command=bench.worker_command(binaries[name],case,fixture,'synthesis',threads,outputs,25.0)
                result,diagnostics=common.run_pinned(command,cpus,monitor)
                result.update({'backend':name,'threads':threads,'pair':pair,'fixture_sha256':fixture['sha256'],'host_diagnostics':diagnostics})
                run[name]={'result':result,'outputs':outputs}
            parent_values=np.concatenate([bench.output_values(run['parent']['outputs'][name],'real') for name in ('map0','map1')])
            candidate_values=np.concatenate([bench.output_values(run['candidate']['outputs'][name],'real') for name in ('map0','map1')])
            ducc_values=np.concatenate([bench.output_values(run['ducc']['outputs'][name],'real') for name in ('map0','map1')])
            records.append({
                'threads':threads,'pair':pair,'order':order,
                'rust_parent_ms':statistics.median(run['parent']['result']['batch_ms']),
                'rust_candidate_ms':statistics.median(run['candidate']['result']['batch_ms']),
                'ducc_ms':statistics.median(run['ducc']['result']['batch_ms']),
                'candidate_parent_ratio':statistics.median(run['candidate']['result']['batch_ms'])/statistics.median(run['parent']['result']['batch_ms']),
                'candidate_ducc_ratio':statistics.median(run['candidate']['result']['batch_ms'])/statistics.median(run['ducc']['result']['batch_ms']),
                'parent_candidate_exact':bool(np.array_equal(parent_values,candidate_values)),
                'candidate_vs_parent':common.error_metrics(candidate_values,parent_values),
                'candidate_vs_ducc':common.error_metrics(candidate_values,ducc_values),
                'all_outputs_finite':bool(np.isfinite(parent_values).all() and np.isfinite(candidate_values).all() and np.isfinite(ducc_values).all()),
                'worker_checks':{name:{'repeat':r['result']['repeat_consistent'],'reset':r['result']['reset_consistent']} for name,r in run.items()},
                'fixture_sha256':fixture['sha256'],
            })
            print(f"completed q{threads} pair {pair+1}: {records[-1]['rust_parent_ms']:.3f}/{records[-1]['rust_candidate_ms']:.3f}/{records[-1]['ducc_ms']:.3f} ms",flush=True)

summary={}
for threads in (1,8):
    group=[r for r in records if r['threads']==threads]
    summary[f'q{threads}']={
        'parent_median_ms':statistics.median(r['rust_parent_ms'] for r in group),
        'candidate_median_ms':statistics.median(r['rust_candidate_ms'] for r in group),
        'ducc_median_ms':statistics.median(r['ducc_ms'] for r in group),
        'candidate_parent_paired_ratio_median':statistics.median(r['candidate_parent_ratio'] for r in group),
        'candidate_ducc_paired_ratio_median':statistics.median(r['candidate_ducc_ratio'] for r in group),
        'candidate_parent_ratios':[r['candidate_parent_ratio'] for r in group],
        'candidate_ducc_ratios':[r['candidate_ducc_ratio'] for r in group],
    }
out={
    'metadata':{'case':case,'parent_sha':'b6a96aa7161a64f678b614d73478496f50f0551e','candidate_source_diff_sha256':os.environ['RUSTSHT_CANDIDATE_PATCH_SHA'],'candidate_binary_sha256':hashlib.sha256(candidate.read_bytes()).hexdigest(),'parent_binary_sha256':hashlib.sha256(parent.read_bytes()).hexdigest(),'ducc_binary_sha256':hashlib.sha256(ducc.read_bytes()).hexdigest(),'ducc_sha':bench.PINNED_DUCC,'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs','process_pairs':3,'target_ms':25,'candidate_source':'isolated generic scalar synthesis; precision-specific zero-restore shortcut'},
    'summary':summary,'pairs':records,
}
pathlib.Path(os.environ['RUSTSHT_RESULT_JSON']).write_text(json.dumps(out,indent=2)+'\n')
