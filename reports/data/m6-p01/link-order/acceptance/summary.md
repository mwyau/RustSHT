# Ordered P01 acceptance matrix results

## Workload and provenance

Six alternating fresh-process pairs completed for each of six whole-transform
cells: T511 scalar q1, T511 spin-1 q1, T1023 scalar q1/q8, and T2047 scalar
q1/q8. Each process ran both STANDARD GL synthesis and analysis in f64 with
`ntheta=L+1`, `nphi=2*(L+1)`, full `mmax=L`, `phi0=0.37`, and
`requested-only`. q1 was pinned to one CPU core; q8 was 8 pinned CPU cores. Five raw
samples per operation and `/usr/bin/time -v` peak RSS are retained for each
process. The baseline binary is SHA-256
`4fd894d46b7195d8a7a4c24cc52f3f04a85ccd1c213d5a2c896f7c828156ccd0` and the
ordered P01 binary is SHA-256
`22cb00437aa6cbe33222fc1a8140694c69ef8ff63694fbaa7310f50561b968f2`. The P01-only source patch SHA-256 is
`fff74a3059aec58741072b3eb1a96421c006ddcc4ba6847949098d17266faecf`; P02 is
excluded. The build-specific LLD order file and symbol movement evidence are
in the parent `link-order/` directory. Host and toolchain details and the
predeclared method are in [`method.md`](method.md).

## Per-operation paired ratios

Ratios are ordered-process median divided by baseline-process median within
each pair, summarized by the median across six paired ratios. Values below
1.0 favor the ordered binary. Brackets show the deterministic 95% bootstrap
interval over the six pair ratios (50,000 resamples, seed 20261017).

| Cell | Synthesis ratio [95% interval] | Analysis ratio [95% interval] |
|---|---:|---:|
| T511 scalar q1 | 0.99804 [0.99612, 1.00221] | 0.98649 [0.97520, 1.00149] |
| T511 spin-1 q1 | 0.99161 [0.98088, 0.99829] | 0.98115 [0.97413, 0.98592] |
| T1023 scalar q1 | 0.65845 [0.65754, 0.68422] | 0.98509 [0.97867, 0.99604] |
| T1023 scalar q8 | 0.62971 [0.62663, 0.65412] | 0.95658 [0.94948, 0.98438] |
| T2047 scalar q1 | 0.73957 [0.73325, 0.74639] | 1.00197 [0.99637, 1.01039] |
| T2047 scalar q8 | 0.68709 [0.68489, 0.68813] | 1.00993 [1.00439, 1.01875] |

All measured operation medians and bootstrap upper bounds are below the
predeclared 1.02 regression threshold. The closest upper interval is T2047
scalar q8 analysis at 1.01875; its largest paired ratio is 1.01985. Scalar
synthesis medians improve by 34.2% (T1023 q1), 37.0% (T1023 q8), 26.0%
(T2047 q1), and 31.3% (T2047 q8). These results describe the ordered binary;
they do not establish that a normal production link without the build-specific
symbol order preserves the same spin-1 control result.

## Contention and raw-sample review

CPU-pressure checks refused four attempted processes before measurement. Exact transient readings and refusal captures remain local.
the T511 spin-1 pair 3 baseline, a DUCC wheel build during T1023 pair 2, a
DUCC LTO build before T2047 q1 pair 1's ordered side, and an xarray batching
benchmark before T2047 q1 pair 2's ordered side. Each missing process was
started only after two clean checks. A separate `uv build` and wheel install
started after per-process preflight and overlapped both T2047 q1 pair 3
processes. Those suspect processes are documented by their validity outcomes; raw host and process captures remain local
and are excluded; a clean pair 3 replacement was run after two clean checks. T2047 q1 pair 1's original baseline had a 34.34s elapsed time, a 4012.783ms
synthesis sample, and analysis samples at 2101–2238ms while other process
analysis medians were about 1.08–1.15s. Although no compiler was visible at its
19:35:54Z preflight, the DUCC LTO build was active by 19:36:28Z. Both original
sides are retained in [`excluded-suspect/`](excluded-suspect/) and a clean
alternating replacement pair was collected after two clean checks. The remaining raw scan found isolated within-process sample variation without
a competing workload in the corresponding preflight snapshot: T1023 q1
ordered synthesis had one high sample in pairs 1 and 2 (pair 2 max 228.711ms),
T1023 q1 ordered analysis pair 1 had one 126.905ms sample, and T2047 q8
ordered analysis pair 6 had one 256.095ms sample. The five-sample process
medians were retained; no concrete overlapping workload was found for these
samples, so they were not excluded or rerun. All five samples, timing/RSS
Structured timing samples and validity outcomes remain in the report; raw process, refusal and host-state captures remain local and untracked. `summary.csv` and `process-pairs.csv` contain the complete
operation and pair-level calculations. This matrix is performance evidence only. P01 remains pending review of the
production link strategy and full regression-suite results on the exact
candidate source; no acceptance or commit is recorded here.
