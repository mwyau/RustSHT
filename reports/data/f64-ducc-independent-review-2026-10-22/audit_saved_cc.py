#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Read saved CC shards and GL route data; never launch transform workers."""
import argparse
import csv
import hashlib
import json
import math
import pathlib
import statistics
from collections import Counter

HIGH = [257,258,300,320,384,511,512,513,640,720,721,768,896,1023,1024,1025,1280,1536,1792,2047,2048,2049,2560,3072]
ODD = {0,1,2,3,4,8,16,32,48,62,63,64,65,127,128,159,191,223,248,254,255,256,257,511,512,513,720,721,1023,1024,1025,2047,2048,2049,2560,3072}

def smooth_even(m):
    n = 2*m+2
    while True:
        r = n
        for f in (2,3,5,7):
            while r % f == 0:
                r //= f
        if r == 1:
            return n
        n += 2

def geometry(row):
    return tuple(row[k] for k in ('lmax','mmax','ntheta','nphi'))

def key(row):
    return (*geometry(row),row['operation'],row['threads'])

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--shards', type=pathlib.Path, required=True)
    ap.add_argument('--repo', type=pathlib.Path, required=True)
    ap.add_argument('--output', type=pathlib.Path, required=True)
    a = ap.parse_args()
    files = sorted(a.shards.glob('*/results.json'))
    docs = [json.loads(p.read_text()) for p in files]
    rows = [r for d in docs for r in d['rows']]
    samples = [r for d in docs for r in d['samples']]
    numerics = [r for d in docs for r in d['numerics']]
    validations = [r for d in docs for r in d['validation']]
    expected = set()
    for l in list(range(257))+HIGH:
        for m in {l,l//4}:
            for n in {smooth_even(m)} | ({2*m+1} if l in ODD else set()):
                for op in ('analysis','synthesis'):
                    for q in (1,8):
                        expected.add((l,m,l+2,n,op,q))
    actual = {key(r) for r in rows}
    def max_metric(items, field):
        r = max(items, key=lambda r:r[field])
        return {'value':r[field], 'case_id':r['case_id'],
                **{k:r[k] for k in ('operation','threads') if k in r}}
    validation_maxima = {}
    for backend in ('rustsht','ducc'):
        for op in ('analysis','synthesis'):
            field = op+'_relative_residual'
            v = max(validations, key=lambda v:v['adjoints'][backend][field])
            validation_maxima[backend+'_'+op+'_adjoint'] = {
                'value':v['adjoints'][backend][field], 'case_id':v['case_id'], 'threads':v['threads']}
        for metric in ('roundtrip','basis_roundtrip','basis_cross_map'):
            vals = []
            for v in validations:
                if metric == 'roundtrip':
                    vals.append((v['roundtrip'][backend]['relative_l2'],v['case_id'],v['threads']))
                else:
                    for b in v['basis']:
                        field = 'analysis_of_synthesis_'+('rust' if backend=='rustsht' else 'ducc') if metric=='basis_roundtrip' else 'map_rust_ducc'
                        vals.append((b[field]['relative_l2'],v['case_id'],v['threads'],b['l'],b['m']))
            validation_maxima[backend+'_'+metric] = max(vals)
    selected = [r for r in rows if r['longitude_mode']=='even_smooth' and r['lmax'] in (63,127,255,511,720,1023,2048)]
    fields = ['case_id','lmax','mmax','ntheta','nphi','operation','threads','rust_warm_ms','rust_p10_ms','rust_p90_ms','ducc_warm_ms','ducc_p10_ms','ducc_p90_ms','rust_ducc_ratio','excess_ms','batch_count','process_invocations_per_backend']
    a.output.mkdir(parents=True,exist_ok=True)
    with (a.output/'selected-cc-latencies.csv').open('w') as f:
        w=csv.DictWriter(f,fieldnames=fields,lineterminator="\n");w.writeheader()
        w.writerows({k:r[k] for k in fields} for r in sorted(selected,key=key))
    batches=Counter()
    short_total=short_valid=0
    for s in samples:
        batches[(s['backend'],len(s['batch_ms']))]+=1
        if s['warmup_ms']<100:
            short_total+=len(s['batch_total_ms'])
            short_valid+=sum(20<=v<=100 for v in s['batch_total_ms'])
    sample_counts=Counter()
    for sample in samples:
        sample_counts[(sample['backend'], key(sample))] += len(sample['batch_ms'])
    gl=[]
    data=a.repo/'reports/data/legendre-policy-2026-10-22'
    for file in ('repeat-2-q1.csv','repeat-3-q1.csv','repeat-4-q1-t120-128.csv'):
        for r in csv.DictReader((data/file).open()):
            if r['operation']=='Analysis' and r['mmax']==r['lmax']:
                t=[float(x) for x in r['table_samples_us'].split(';')]
                c=[float(x) for x in r['recurrence_samples_us'].split(';')]
                gl.append({'file':file,'lmax':int(r['lmax']),'nphi':int(r['nphi']),
                    'ratio':statistics.median(t)/statistics.median(c),
                    'table_us':statistics.median(t),'recurrence_us':statistics.median(c),
                    'table_build_us':float(r['table_build_us']),
                    'payload_mib':int(r['table_payload_estimate_bytes'])/(1024**2)})
    with (a.output/'gl-q1-focused-analysis.csv').open('w') as f:
        w=csv.DictWriter(f,fieldnames=list(gl[0]),lineterminator="\n");w.writeheader();w.writerows(gl)
    screen=list(csv.DictReader((data/'table-recurrence-screen.csv').open()))
    classes=[]
    for lo,hi in ((63,96),(97,127),(128,191),(192,217),(218,255)):
        for vector in (True,False):
            class_rows=[r for r in screen if r['operation']=='Analysis' and r['threads']=='1'
                and r['longitude']=='even_smooth' and r['lmax']==r['mmax']
                and lo<=int(r['lmax'])<=hi and ((int(r['lmax'])+1)%4==0)==vector]
            values=[float(r['table_over_recurrence']) for r in class_rows]
            classes.append({'lmax_lo':lo,'lmax_hi':hi,
                'table_dispatch':'AVX2 cached' if vector else 'scalar','cases':len(values),
                'geomean_table_over_recurrence':statistics.geometric_mean(values),
                'minimum_ratio':min(values),'maximum_ratio':max(values),
                'protocol':'exploratory 5 batches, 1 ms target'})
    with (a.output/'gl-dispatch-classes.csv').open('w') as f:
        w=csv.DictWriter(f,fieldnames=list(classes[0]),lineterminator="\n");w.writeheader();w.writerows(classes)
    matched=[]
    glfile=a.repo/'reports/data/ducc-full-transform-parity-current-2026-10-22/results.csv'
    glrows={tuple(int(r[k]) if k not in ('operation',) else r[k] for k in ('lmax','mmax','nphi','operation','threads')):r for r in csv.DictReader(glfile.open())}
    for r in selected:
        k=tuple(r[k] for k in ('lmax','mmax','nphi','operation','threads'))
        if k in glrows:
            g=glrows[k]
            matched.append({'case_id':r['case_id'],'operation':r['operation'],'threads':r['threads'],
                'cc_rust_ms':r['rust_warm_ms'],'cc_ducc_ms':r['ducc_warm_ms'],
                'gl_rust_ms':float(g['rust_warm_ms']),'gl_ducc_ms':float(g['ducc_warm_ms']),
                'cc_over_gl_ratio':r['rust_ducc_ratio']/float(g['rust_ducc_ratio'])})
    with (a.output/'cc-gl-selected.csv').open('w') as f:
        w=csv.DictWriter(f,fieldnames=list(matched[0]),lineterminator="\n");w.writeheader();w.writerows(matched)
    out = {'inputs':[{'shard':str(p.parent.name),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in files],
        'shards':len(files),'rows':len(rows),'distinct_geometries':len({geometry(r) for r in rows}),
        'numerics':len(numerics),'validation':len(validations),'failures':sum(len(d['failures']) for d in docs),
        'duplicate_rows':len(rows)-len(actual),'degrees':sorted({r['lmax'] for r in rows}),
        'owner_expected_geometries':len(expected)//4,'owner_expected_groups':len(expected),
        'group_key_fields':['lmax','mmax','ntheta','nphi','operation','threads'],
        'saved_requested_groups':len(actual & expected),
        'missing_groups':sorted(expected-actual),'extra_groups':sorted(actual-expected),
        'numeric_max_relative_l2':max_metric(numerics,'relative_l2'),
        'numeric_max_absolute':max_metric(numerics,'absolute_max'),
        'validation_maxima':validation_maxima,
        'all_reset_consistent':all(s['reset_consistent'] for s in samples),
        'all_finite':all(r['finite_outputs'] for r in numerics+validations),
        'all_exact_zero':all(v['edge_cases']['zero']['exact_zero'] for v in validations),
        'sample_batch_count_histogram':{str(k):v for k,v in batches.items()},
        'short_batch_in_window':short_valid,'short_batch_total':short_total,
        'sample_count_histogram_by_backend':{b:dict(Counter(sample_counts[(b,key(r))] for r in rows)) for b in ('rustsht','ducc')},
        'binary_hashes':{k:sorted({d['metadata'][k] for d in docs}) for k in ('rust_binary_sha256','ducc_binary_sha256')},
        'harness_revisions':sorted({d['metadata']['rust_source_revision'] for d in docs}),
        'wait_fraction_gt_0_1_processes':sum((s['host_diagnostics'].get('wait_fraction') or 0)>0.1 for s in samples),
    }
    (a.output/'audit.json').write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps({k:v for k,v in out.items() if k not in ('inputs','degrees','missing_groups','extra_groups')},indent=2))
    print('missing groups:',len(expected-actual),'extra groups:',len(actual-expected))

if __name__=='__main__':
    main()
