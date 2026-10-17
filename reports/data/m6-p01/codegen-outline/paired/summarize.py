#!/usr/bin/env python3
"""Summarize paired baseline/outlined-P01 benchmark outputs."""
import csv, random, re, statistics
from pathlib import Path
HERE=Path(__file__).resolve().parent
NAME=re.compile(r"t(?P<lmax>\d+)-s(?P<spin>\d+)-q(?P<threads>\d+)-pair(?P<pair>\d+)-(?P<variant>baseline|outlined)\.stdout$")
SAMPLES=re.compile(r"(?P<op>synthesis|analysis)_rayon_samples_ms=\[([^]]+)\]")
RSS=re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
EXIT=re.compile(r"Exit status:\s*(\d+)")

def read(path):
    m=NAME.fullmatch(path.name); assert m, path
    meta={k:int(v) if k!='variant' else v for k,v in m.groupdict().items()}
    text=path.read_text(); config=text.splitlines()[0]
    ntheta=meta['lmax']+1; nphi=2*ntheta
    assert f"GL {ntheta}x{nphi}" in config and 'requested_only=true' in config and 'phi0=0.37' in config
    vals={m.group('op'):[float(x) for x in m.group(2).split(',')] for m in SAMPLES.finditer(text)}
    assert set(vals)=={'synthesis','analysis'} and all(len(v)==5 for v in vals.values())
    time=path.with_suffix('.time').read_text(); rss=RSS.search(time); status=EXIT.search(time)
    assert rss and status and status.group(1)=='0'
    return {**meta,'values':vals,'rss':int(rss.group(1))}

procs={}
for path in HERE.glob('t*-s*-q*-pair*-*.stdout'):
    p=read(path); key=(p['lmax'],p['spin'],p['threads'],p['pair'],p['variant'])
    assert key not in procs; procs[key]=p
cells=sorted({k[:3] for k in procs})
expected={(511,1,1),(1023,0,1),(511,0,1)}
assert set(cells)==expected
pair_rows=[]; summary_rows=[]; rng=random.Random(20261017)
def quantile(a,p):
    a=sorted(a); x=(len(a)-1)*p; lo=int(x); hi=min(lo+1,len(a)-1); f=x-lo
    return a[lo]*(1-f)+a[hi]*f
for cell in cells:
    pair_ids=sorted({k[3] for k in procs if k[:3]==cell}); assert pair_ids==list(range(1,7))
    for op in ('synthesis','analysis'):
        ratios=[]; base_meds=[]; out_meds=[]; rss_delta=[]; base_values=[]; out_values=[]
        for pair in pair_ids:
            b=procs[(*cell,pair,'baseline')]; o=procs[(*cell,pair,'outlined')]
            bv=b['values'][op]; ov=o['values'][op]; bm=statistics.median(bv); om=statistics.median(ov); ratio=om/bm
            ratios.append(ratio); base_meds.append(bm); out_meds.append(om); rss_delta.append(o['rss']-b['rss'])
            base_values.extend(bv); out_values.extend(ov)
            pair_rows.append({'lmax':cell[0],'spin':cell[1],'threads':cell[2],'operation':op,'pair':pair,'baseline_first':pair%2==1,
              'baseline_samples_ms':';'.join(f'{x:.3f}' for x in bv),'outlined_samples_ms':';'.join(f'{x:.3f}' for x in ov),
              'baseline_median_ms':f'{bm:.6f}','outlined_median_ms':f'{om:.6f}','outlined_over_baseline':f'{ratio:.8f}',
              'baseline_rss_kib':b['rss'],'outlined_rss_kib':o['rss'],'rss_delta_kib':o['rss']-b['rss']})
        boot=[statistics.median(rng.choice(ratios) for _ in ratios) for _ in range(50000)]
        summary_rows.append({'lmax':cell[0],'spin':cell[1],'threads':cell[2],'operation':op,'n_pairs':6,
          'baseline_median_ms':f'{statistics.median(base_meds):.6f}','outlined_median_ms':f'{statistics.median(out_meds):.6f}',
          'median_paired_ratio':f'{statistics.median(ratios):.8f}','bootstrap95_low':f'{quantile(boot,.025):.8f}',
          'bootstrap95_high':f'{quantile(boot,.975):.8f}','paired_ratio_min':f'{min(ratios):.8f}','paired_ratio_max':f'{max(ratios):.8f}',
          'baseline_sample_min_ms':f'{min(base_values):.3f}','baseline_sample_max_ms':f'{max(base_values):.3f}',
          'outlined_sample_min_ms':f'{min(out_values):.3f}','outlined_sample_max_ms':f'{max(out_values):.3f}',
          'median_rss_delta_kib':int(statistics.median(rss_delta))})
for name,rows in [('process-pairs.csv',pair_rows),('summary.csv',summary_rows)]:
    with (HERE/name).open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
print(f'wrote {len(pair_rows)} pair rows and {len(summary_rows)} summary rows')
