# Paired longitude measurements

Each log is one fresh `m1b_bench` process. The process records five samples for
q1 and q8 synthesis and analysis on the same plan and workspace. The first
pair uses odd-then-even process order; the second reverses the order. T720 also
compares FFT-friendly nphi=1458 with minimum-even nphi=1442. The inputs,
`phi0`, geometry, `ntheta`, `mmax`, and benchmark binary match the native
screen. Raw medians and setup/RSS fields are expanded into `summary.csv` by
`summarize_paired_longitudes.py`.

Pairs 1 and 2 use CPUs 8–15. During pair 1 at T2047, even nphi=4096, spin 1,
an unrelated RustFFT benchmark was observed pinned to CPU30, the SMT sibling
of CPU14. That one process is flagged `possible_smt_overlap_cpu30_vs_cpu14`.
Pairs 3 and 4 repeat the T2047 spin-1 odd/even comparison with CPU affinity
`7,8-13,15`, which excludes core 14. The host remains shared, and the raw
within-process sample lists retain visible outliers; those runs are not treated
as isolated machine measurements.

The run used `m1b_bench` SHA-256
`34274c791ae73ef1c909639aebfa74747141173bb31390a27b490c289601861b`. The
binary was built from source revision
`2756ba4a1d9bcf6b7b1a1293a185e98e8e6c74b2`; the paired batch started at
repository revision `e5d9171`.
