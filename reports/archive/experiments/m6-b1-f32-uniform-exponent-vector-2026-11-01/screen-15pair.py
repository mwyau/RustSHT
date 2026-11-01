import hashlib, importlib.util, json, os, pathlib, statistics, sys
root = pathlib.Path(os.environ['RUSTSHT_CANDIDATE_TREE'])
spec = importlib.util.spec_from_file_location('harness', root / 'scripts/benchmark_ducc_f32_native.py')
h = importlib.util.module_from_spec(spec); sys.modules['harness'] = h; spec.loader.exec_module(h)
bins = {
    'parent': pathlib.Path(os.environ['RUSTSHT_PARENT_F32_WORKER']),
    'candidate': pathlib.Path(os.environ['RUSTSHT_CANDIDATE_F32_WORKER']),
    'ducc': pathlib.Path(os.environ['RUSTSHT_DUCC_F32_WORKER']),
}
assert all(p.is_file() for p in bins.values())
fixture_root = pathlib.Path(os.environ['RUSTSHT_FIXTURE_ROOT'])
out = pathlib.Path(os.environ.get('RUSTSHT_OUTPUT_DIR', root / 'screen-uniform-factor3'))
out.mkdir(parents=True, exist_ok=True)
physical=[]; seen=set()
for cpu in sorted(os.sched_getaffinity(0)):
    top=pathlib.Path(f'/sys/devices/system/cpu/cpu{cpu}/topology')
    key=(top.joinpath('physical_package_id').read_text().strip(),top.joinpath('core_id').read_text().strip())
    if key not in seen: seen.add(key); physical.append(cpu)
q1=str(physical[0]); q8=','.join(map(str,physical[:8]))
rows=[]
for mmax in (720,180):
    for threads in (1,8):
        nphi=h.smooth_even_at_least(2*mmax+2)
        case={'grid':'GL','ntheta':721,'nphi':nphi,'lmax':720,'mmax':mmax,'longitude':'even','spin':0,'operation':'analysis','threads':threads}
        cid=f"GL-T720-M{mmax}-even-s0-analysis-q{threads}"
        fixture={name:fixture_root/cid/f'{name}.f32' for name in ('map0','map1','alm0','alm1')}
        assert all(p.is_file() for p in fixture.values()), fixture
        cell=[]
        orders=[(('parent','candidate','ducc'),('candidate','ducc','parent'),('ducc','parent','candidate'))[pair % 3] for pair in range(15)]
        for pair,order in enumerate(orders,1):
            obs={}
            for backend in order:
                run_dir=out/cid/f'pair-{pair}'/backend
                result=h.run_worker(bins[backend],backend,case,fixture,run_dir,40.0,q1 if threads==1 else q8)
                obs[backend]=result
            ms={k:statistics.median(obs[k]['batch_ms']) for k in obs}
            def paths(key): return [pathlib.Path(p) for p in obs[key]['outputs']]
            row={'pair':pair,'order':order,'parent_ms':ms['parent'],'candidate_ms':ms['candidate'],'ducc_ms':ms['ducc'],
                 'candidate_parent':ms['candidate']/ms['parent'],'parent_ducc':ms['parent']/ms['ducc'],'candidate_ducc':ms['candidate']/ms['ducc'],
                 'candidate_vs_parent':h.error_metrics(paths('parent'),paths('candidate')),
                 'candidate_vs_ducc':h.error_metrics(paths('ducc'),paths('candidate')),
                 'dispatch':{k:{x:obs[k].get(x) for x in ('simd_arch','f32_lanes','backend','repeat_consistent','reset_consistent')} for k in obs},
                 'batch_ms':{k:obs[k]['batch_ms'] for k in obs},
                 'fixture_sha256':{k:hashlib.sha256(p.read_bytes()).hexdigest() for k,p in fixture.items()},
                 'binary_sha256':{k:hashlib.sha256(p.read_bytes()).hexdigest() for k,p in bins.items()}}
            cell.append(row)
            print(cid, 'pair',pair, json.dumps({k:row[k] for k in ('parent_ms','candidate_ms','ducc_ms','candidate_parent','candidate_ducc','candidate_vs_parent')}), flush=True)
            for result in obs.values():
                for output in result.get('outputs',[]): pathlib.Path(output).unlink(missing_ok=True)
        rows.append({'case_id':cid,'case':case,'pairs':cell,'median_candidate_parent':statistics.median(r['candidate_parent'] for r in cell),
                     'median_candidate_ducc':statistics.median(r['candidate_ducc'] for r in cell),
                     'median_parent_ducc':statistics.median(r['parent_ducc'] for r in cell)})
        print('SUMMARY',cid,'candidate/parent',rows[-1]['median_candidate_parent'],'candidate/DUCC',rows[-1]['median_candidate_ducc'],flush=True)
patch=__import__('subprocess').check_output(['git','diff','--binary'],cwd=root)
meta={'parent_source_sha':__import__('subprocess').check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),
      'candidate_diff_sha256':hashlib.sha256(patch).hexdigest(),'ducc_source_sha':h.DUCC_SHA,
      'cpu_model':open('/proc/cpuinfo').read().split('model name')[1].splitlines()[0].strip(),
      'q1_affinity':q1,'q8_affinity':q8,'rust_flags':'-C target-cpu=x86-64-v3 -Aunexpected_cfgs',
      'binaries':{k:{'path':str(v),'sha256':hashlib.sha256(v.read_bytes()).hexdigest()} for k,v in bins.items()},'cells':rows}
(out/'results.json').write_text(json.dumps(meta,indent=2)+'\n')
print('RESULTS',out/'results.json')
