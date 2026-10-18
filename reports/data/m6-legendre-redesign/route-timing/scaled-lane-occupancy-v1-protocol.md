# Scaled-lane occupancy profile v1

## Purpose

The scaled-prefix event profile recorded 1,034,964 normalization calls across
109,490,856 lane visits, but it did not count how many lanes carried nonzero
recurrence exponents after each normalization. This diagnostic measures that
occupancy before considering a lane-class partition. It is not a performance
acceptance benchmark.

## Frozen source and workload

Use the event-counter source from profile v1, SHA-256
`5eb7276d2c460baf962c78ff887f42a59aff1f081463db7cf63ba4afb031a99c`, and
apply only a test-gated occupancy-counter patch. Record the patch, resulting
source, and release test executable SHA-256 values in the result report.
Keep the candidate isolated; do not change production routing or recurrence
arithmetic.

Run the ignored `m6_fast_route_counts` release test at T2047, q1, GL f64
scalar, full `mmax`, `phi0=0.37`, and `2048x4096`. Use one unprofiled warm-up
synthesis and one instrumented synthesis. Require bitwise equality between the
output buffers. Retain the existing route counts and count the number of
nonzero recurrence exponents after each prefix normalization in the same
counter pass. Record the total nonzero-exponent lane visits, maximum such lanes
in any one normalization, and a histogram of calls with 0, 1, 2–4, 5–8, 9–16,
17–32, 33–64, or 65+ nonzero-exponent lanes. Do not add another per-lane scan.

Use Rust 1.99.0, Cargo release settings, all features, one Rayon worker, and
one allowed logical CPU. Compile and run in the disposable source tree with a
separate target directory. Do not run another benchmark or build concurrently.

## Validity

Use the current CPU-pressure protocol: reject if Linux CPU PSI `some avg10`
exceeds 2.00% at preflight or postflight. PSI from 0.50% through 2.00% is a
warning. Keep the existing timing stability limits unchanged, although timing
stability is not an acceptance criterion for this count-only profile. Review
CPU placement, frequency behavior, involuntary context switches, cgroup
throttling, and concurrent CPU-intensive work locally. Stop and mark the
profile inconclusive if clear interference occurs. Keep raw host and process
captures untracked.

The diagnostic can establish the distribution of scaled exponent lanes for
this synthetic T2047 coefficient workload. It cannot establish a general lane
distribution, candidate speedup, numerical acceptance, or a production
optimization. Use its histogram to decide whether to test a lane-class split;
do not alter the acceptance criteria or expand the benchmark matrix.
