# Paired recurrence range-check fusion experiment

## Result

The prototype fused the normal-range predicate into the two-degree recurrence update in `ForEachPairedYlm`. It checked the old current value and the newly computed value while those vectors were live, then called the existing lane normalizer only when the predicate failed. The check used the same `BIG` and `SMALL` boundaries and retained the existing scaled fallback, including NaN, infinity, subnormal and scalar-tail behavior. The candidate did not produce a reproducible whole-transform gain and was removed. Avoiding the later array scan did not offset the range comparisons in the recurrence loop. No production code, workspace, API or numerical convention changed.

## Native benchmark

The baseline was commit `3b8e6159d7ffcd405377037ddfe12645f8e930d6`; the candidate changed only the paired recurrence check in `src/legendre/scalar_paired.rs`. Both builds used Rust `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch, and no `target-cpu=native`. The host was an x86-64 CPU running Linux. Processes were 8 pinned CPU cores. `m1b_bench` ran f64 GL spin-1 STANDARD analysis at T511 (`ntheta=512`, `nphi=1025`, `mmax=511`, `phi0=0.37`). Each process reused its plan and workspace, warmed each operation, and recorded five timed transforms. Five process pairs ran baseline then candidate on the same affinity. Times below are medians of the five per-process medians; paired deltas use each adjacent baseline/candidate pair.

| Thread pool | Baseline | Candidate | Change from medians | Median paired change (range) |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 39.707 ms | 39.423 ms | -0.72% | -0.35% (-1.41% to +0.54%) |
| 8 | 12.888 ms | 12.633 ms | -1.98% | -1.25% (-3.00% to +1.67%) |

The paired ranges cross zero, so neither apparent improvement meets the retention threshold. Raw process medians and exact commands are in [`reports/data/paired-recurrence-range-check-2026-10-14/paired-process-samples.txt`](../../data/paired-recurrence-range-check-2026-10-14/paired-process-samples.txt).


## Numerical validation

With the candidate active, `cargo test -p rustsht --all-features --lib paired_` passed all 15 selected tests, including the mixed normal/scaled-lane comparison, extreme finite intermediate-product fallback, SIMD range guard with vector tails, and the paired recurrence's scaled-to-normal reference check. The candidate Python extension was also compared with installed `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) at T511 GL spin-1 STANDARD analysis, with one and eight threads. Both runs reported max absolute, RMS and relative-L2 coefficient errors of `1.401e-14`, `1.910e-16` and `3.740e-14`. One Python process per thread count was used, so those Python timings are contextual only and were not used for acceptance.

## Decision

The check duplicated range comparisons next to the recurrence arithmetic and showed no benefit beyond run variability. Keep the simpler post-update range helper. Future analysis work should target the larger recurrence and accumulation costs rather than this predicate unless a profile isolates a different cost.
