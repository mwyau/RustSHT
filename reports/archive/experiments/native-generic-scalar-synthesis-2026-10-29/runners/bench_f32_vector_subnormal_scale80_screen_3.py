from __future__ import annotations
import filecmp, hashlib, json, os, pathlib, statistics, sys, tempfile

root = pathlib.Path(os.environ['RUSTSHT_NATIVE_SOURCE'])
sys.path.insert(0, str(root / 'scripts'))
import benchmark_ducc_f32_native as bench

parent = pathlib.Path(os.environ['RUSTSHT_PARENT_BIN'])
candidate = pathlib.Path(os.environ['RUSTSHT_CANDIDATE_BIN'])
ducc = pathlib.Path(os.environ['RUSTSHT_DUCC_BIN'])
binaries = {'parent': parent, 'candidate': candidate, 'ducc': ducc}
for name, path in binaries.items():
    if not path.is_file(): raise FileNotFoundError(f'{name}: {path}')
    print(f'{name} {path} {hashlib.sha256(path.read_bytes()).hexdigest()}', flush=True)

cases = []
for mmax, nphi in ((720, 1458), (180, 378)):
    for threads in (1, 8):
        cases.append({'grid':'GL','ntheta':721,'nphi':nphi,'lmax':720,'mmax':mmax,
                      'longitude':'even','spin':0,'operation':'synthesis','threads':threads})
records=[]
root_tmp = pathlib.Path(os.environ['RUSTSHT_BENCH_SCRATCH'])
root_tmp.mkdir(parents=True,exist_ok=True)
fixture_hashes={}
for case in cases:
    case_id=f"GL-T{case['lmax']}-M{case['mmax']}-even-s0-synthesis-q{case['threads']}"
    fixture=bench.make_fixture(case,root_tmp/'fixtures'/case_id)
    fixture_hashes[case_id]={name:hashlib.sha256(path.read_bytes()).hexdigest() for name,path in fixture.items()}
    cpus='0' if case['threads']==1 else '0,2,4,6,8,10,12,14'
    for pair in range(3):
        order=[['parent','candidate','ducc'],['candidate','ducc','parent'],['ducc','parent','candidate']][pair%3]
        run={}
        with tempfile.TemporaryDirectory(prefix=f'{case_id}-p{pair+1}-',dir=root_tmp) as temp_name:
            temp=pathlib.Path(temp_name)
            for name in order:
                result=bench.run_worker(binaries[name],name,case,fixture,temp/name,25.0,cpus)
                result.update({'backend':name,'pair':pair+1,**case})
                run[name]=result
            parent_out=[pathlib.Path(p) for p in run['parent']['outputs']]
            candidate_out=[pathlib.Path(p) for p in run['candidate']['outputs']]
            ducc_out=[pathlib.Path(p) for p in run['ducc']['outputs']]
            row={
                'case_id':case_id,'threads':case['threads'],'pair':pair+1,'order':order,
                'parent_ms':statistics.median(run['parent']['batch_ms']),
                'candidate_ms':statistics.median(run['candidate']['batch_ms']),
                'ducc_ms':statistics.median(run['ducc']['batch_ms']),
                'candidate_parent_ratio':statistics.median(run['candidate']['batch_ms'])/statistics.median(run['parent']['batch_ms']),
                'candidate_ducc_ratio':statistics.median(run['candidate']['batch_ms'])/statistics.median(run['ducc']['batch_ms']),
                'parent_candidate_exact':all(filecmp.cmp(a,b,shallow=False) for a,b in zip(parent_out,candidate_out)),
                'candidate_vs_parent':bench.error_metrics(parent_out,candidate_out),
                'candidate_vs_ducc':bench.error_metrics(ducc_out,candidate_out),
                'all_outputs_finite':all(r['finite_outputs'] for r in run.values()),
                'checks':{name:{'repeat':r['repeat_consistent'],'reset':r['reset_consistent'],
                                'simd_arch':r.get('simd_arch'),'f32_lanes':r.get('f32_lanes')}
                          for name,r in run.items()},
                'fixture_hashes':fixture_hashes[case_id],
            }
            records.append(row)
            print(f"{case_id} pair {pair+1}: {row['parent_ms']:.3f}/{row['candidate_ms']:.3f}/{row['ducc_ms']:.3f} ms",flush=True)

summary={}
for case in cases:
    case_id=f"GL-T{case['lmax']}-M{case['mmax']}-even-s0-synthesis-q{case['threads']}"
    group=[r for r in records if r['case_id']==case_id]
    summary[case_id]={
        'parent_median_ms':statistics.median(r['parent_ms'] for r in group),
        'candidate_median_ms':statistics.median(r['candidate_ms'] for r in group),
        'ducc_median_ms':statistics.median(r['ducc_ms'] for r in group),
        'candidate_parent_paired_ratio_median':statistics.median(r['candidate_parent_ratio'] for r in group),
        'candidate_ducc_paired_ratio_median':statistics.median(r['candidate_ducc_ratio'] for r in group),
    }
meta={
 'parent_sha':'39dd8a883d21a269654e1b74de862ee2e97d22ee',
 'candidate_patch_sha256':'ec6468793f641109f1f136a979d5c7f7ef932df6c59517e4082d99379e1de7ed',
 'candidate_binary_sha256':hashlib.sha256(candidate.read_bytes()).hexdigest(),
 'parent_binary_sha256':hashlib.sha256(parent.read_bytes()).hexdigest(),
 'ducc_binary_sha256':hashlib.sha256(ducc.read_bytes()).hexdigest(),
 'ducc_sha':bench.DUCC_SHA,'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
 'process_pairs_per_cell':3,'target_ms':25,'q1_affinity':'0','q8_affinity':'0,2,4,6,8,10,12,14',
 'cpu_model':'AMD Ryzen 9 5950X','fixture_hashes':fixture_hashes,
 'method':'same native-f32 fixtures; fresh parent/candidate/DUCC processes interleaved in balanced order; reusable plan/workspace per worker; output files deleted after each matched triple',
}
out={'metadata':meta,'summary':summary,'pairs':records}
pathlib.Path(os.environ['RUSTSHT_RESULT_JSON']).write_text(json.dumps(out,indent=2)+'\n')
