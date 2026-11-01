import hashlib, importlib.util, json, os, pathlib, statistics, sys, tempfile
root=pathlib.Path(os.environ['RUSTSHT_CANDIDATE_TREE'])
sys.path.insert(0,str(root/'scripts'))
import benchmark_ducc_spin1_native as bench
import benchmark_ducc_full_transform_parity as common
bins={'parent':pathlib.Path(os.environ['RUSTSHT_PARENT_SPIN_WORKER']),
      'candidate':root/'target/release/examples/ducc_spin1_full_transform_parity'}
case=next(c for c in bench.make_cases([1023],['GL'],False) if c['mmax']==1023 and c['ntheta']==1024 and c['longitude_mode']=='even-smooth')
physical=common.physical_cpus(); monitor=physical[8]; rows=[]
out=pathlib.Path(os.environ.get('RUSTSHT_OUTPUT_JSON', root/'t1023-protected-15.json'))
with tempfile.TemporaryDirectory(prefix='m6-b1-t1023-15-',dir=os.environ.get('RUSTSHT_TMPDIR')) as tmp:
 temp=pathlib.Path(tmp); fixture=bench.make_fixture(case,temp)
 for threads in (1,8):
  cpus=physical[:threads]
  orders=(('parent','candidate'),('candidate','parent'))
  for pair in range(15):
   order=orders[pair%2]; run={}
   for backend in order:
    outputs={name:temp/f'{backend}-q{threads}-p{pair}-{name}.bin' for name in ('map0','map1','e','b')}
    command=bench.worker_command(bins[backend],case,fixture,'synthesis',threads,outputs,25.0)
    result,_=common.run_pinned(command,cpus,monitor)
    run[backend]={'result':result,'outputs':outputs}
   pmed=statistics.median(run['parent']['result']['batch_ms']); cmed=statistics.median(run['candidate']['result']['batch_ms'])
   pvals=[bench.output_values(run['parent']['outputs'][name],'real') for name in ('map0','map1')]
   cvals=[bench.output_values(run['candidate']['outputs'][name],'real') for name in ('map0','map1')]
   row={'threads':threads,'pair':pair+1,'order':order,'parent_ms':pmed,'candidate_ms':cmed,'ratio':cmed/pmed,
        'outputs_exact':all((a==b).all() for a,b in zip(pvals,cvals)),
        'parent_repeat':run['parent']['result']['repeat_consistent'],'candidate_repeat':run['candidate']['result']['repeat_consistent'],
        'parent_reset':run['parent']['result']['reset_consistent'],'candidate_reset':run['candidate']['result']['reset_consistent'],
        'parent_batches_ms':run['parent']['result']['batch_ms'],'candidate_batches_ms':run['candidate']['result']['batch_ms']}
   rows.append(row); print(json.dumps({k:row[k] for k in ('threads','pair','parent_ms','candidate_ms','ratio','outputs_exact')}),flush=True)
meta={'source_parent':'56561116fb9b78afadd20132b8f90f98b7e30d0c (spin source identical to ecb66 parent binary)',
      'candidate_source_base':'56561116fb9b78afadd20132b8f90f98b7e30d0c plus B1 range predicate patch',
      'parent_binary_sha256':hashlib.sha256(bins['parent'].read_bytes()).hexdigest(),
      'candidate_binary_sha256':hashlib.sha256(bins['candidate'].read_bytes()).hexdigest(),
      'fixture_sha256':fixture['sha256'],'cpu_model':common.cpu_model(),'q1_cpus':physical[:1],'q8_cpus':physical[:8],
      'ducc_reference':bench.PINNED_DUCC,'pairs':rows}
out.write_text(json.dumps(meta,indent=2)+'\n'); print('SAVED',out)
