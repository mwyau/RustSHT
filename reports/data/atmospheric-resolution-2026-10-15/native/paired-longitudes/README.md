# Paired longitude measurements

Each log is one fresh `m1b_bench` process. The process records five samples for
q1 and q8 synthesis and analysis on the same plan and workspace. The first
pair uses odd-then-even process order; the second reverses the order. T720 also
compares FFT-friendly nphi=1458 with minimum-even nphi=1442. The inputs,
`phi0`, geometry, `ntheta`, `mmax`, and benchmark binary match the native
screen. Raw medians and setup/RSS fields are expanded into `summary.csv` by
`summarize_paired_longitudes.py`. Pairs 1 and 2 use 8 pinned CPU cores. Concurrent process-placement details were retained only in local diagnostics. That one process is flagged `possible_smt_overlap_cpu30_vs_cpu14`. Pairs 3 and 4 repeat the T2047 spin-1 odd/even comparison with worker-sized CPU affinity, which excludes CPU id. The host remains shared, and the raw
within-process sample lists retain visible outliers; those runs are not treated
as isolated machine measurements. The run used `m1b_bench` SHA-256
`34274c791ae73ef1c909639aebfa74747141173bb31390a27b490c289601861b`. The
binary was built from source revision
`2756ba4a1d9bcf6b7b1a1293a185e98e8e6c74b2`; the paired batch started at
repository revision `e5d9171`.
