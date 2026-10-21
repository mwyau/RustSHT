# Plan 15: CC q8 synthesis coefficient reuse

**Sources:** baseline RustSHT `9b93cf32d7399fbbd4ca8f8231ec605238c49076`;
candidate `be3c48eb2e430fa3ca7e91fe33e6cc503f2a1b9d`; DUCC 0.42.0 pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Change

For exact CC synthesis, the spin-1 E/B-to-scalar coefficient conversion is
identical for each ring group. The previous q8 implementation repeated that
conversion and finite scan for each of 12 groups at T720. The candidate uses
the existing bounded workspace cache only when the CC grid is exact, ring
groups outnumber execution workers, and estimated `order_count × ntheta`
work exceeds `12,000 × worker_count`. Existing q8, lmax, memory, fast-route,
finite-order and exceptional-value guards remain in force.

CC keeps its previous static group scheduling in this comparison. This
separates coefficient reuse from the dynamic group queue used by the F1 and
exact-GL routes. The cache itself is the tested implementation change.

At T720 there are 12 groups. The source-derived conversion work drops from
`12 × 260,281` to `260,281` degree positions full-order, and from
`12 × 114,211` to `114,211` for `mmax=180`. The finiteness scan drops from
`12 × 1,041,124` to `1,041,124` converted scalar values full-order and from
`12 × 261,364` to `261,364` reduced-order. These counts describe the loop
work, not measured hardware counters. Cache storage is 15.9 MiB full-order
and 4.0 MiB reduced-order, within the existing 17 MiB workspace cap.

## Matched whole-transform results

The native RustSHT/DUCC drivers use matching two-channel E/B synthesis,
reused plans/workspaces, fixture data, output resets and even-smooth exact
CC geometry (`ntheta=722`, `phi0=0.23`). Three interleaved process pairs used
ten calibrated batches per short run targeting 25 ms. q1 was pinned to CPU 0;
q8 used CPUs 0–7. The timings below are pooled warm-batch medians with p10–p90;
paired candidate/baseline ratios preserve each process pair. q1 cases whose
single calls exceeded 100 ms were individually timed rather than forced into
the short-call batch protocol.

| Case | Rust baseline ms (p10–p90) | Candidate ms (p10–p90) | DUCC ms (p10–p90) | Candidate/baseline paired ratios | Rust/DUCC paired median before → after | Pooled-median excess before → after |
|---|---:|---:|---:|---:|---:|---:|
| CC T720 full, q8 | 48.063 (47.507–48.847) | 40.668 (40.074–42.027) | 6.129 (6.096–6.256) | 0.858, 0.845, 0.842; median **0.845** | 7.855× → 6.668× | 41.933 → 34.538 ms |
| CC T720 `mmax=180`, q8 | 26.082 (25.302–27.457) | 21.344 (20.830–21.791) | 2.659 (2.639–2.797) | 0.832, 0.813, 0.819; median **0.819** | 9.823× → 7.984× | 23.423 → 18.686 ms |

Each paired process improved in both synthesis workloads: 15.5% median for
full order and 18.1% for reduced order. Whole-transform excess remains large,
so coefficient reuse closes only part of the CC gap.

The change did not affect q8 analysis: pooled full-order medians were 32.392
ms before and 32.387 ms after; reduced-order values were 12.314 and 12.247 ms.
q1 synthesis, where the q8 cache gate is disabled, was effectively unchanged:
paired candidate/baseline ratios were 1.005–1.031 reduced and 0.989–1.040
full. This is a useful route control.

Host wait was low to moderate for q8 synthesis, but not identical between
process sets. Runqueue wait medians were 1.90% baseline, 0.62% candidate and
0.34% DUCC (candidate p90 0.94%). The candidate was also observed at slightly
higher median CPU frequency than baseline in these captures. Therefore, the
consistent direction supports the cache mechanism, while the exact 15–18%
gain should be treated as a host-specific measurement rather than a universal
speedup. The full-order q8 first-call medians were 48.42 ms baseline and
48.47 ms candidate, with setup medians 14.68 and 14.30 ms; no cold-start gain
is claimed.

## Correctness and fallback

All measured candidate outputs were bitwise equal to baseline. DUCC relative
L2 error was at most `3.64e-13` reduced-order and `2.29e-13` full-order;
outputs remained finite and repeat/reset checks passed. The new release test
compares q8 output with q1 after ordinary coefficients, poisoned output
buffers and changed coefficients. It also supplies extreme finite E/B values
that make the cached scalar conversion nonfinite, verifies that the order is
marked unsafe, and compares the general fallback output with q1. The targeted
test passes. The full all-feature release suite passed (249 tests; the updated
fallback test was rerun separately), the package no-default-features release
suite passed (201 tests), and strict Clippy passed.

## Data and next work

Sanitized per-backend distributions, pair ratios, source/toolchain/build
metadata, fixture and binary hashes, setup/first-call measurements, and host
summaries are in
[`reports/data/plan15-spin1-cc-coefficient-reuse-2026-10-21/`](data/plan15-spin1-cc-coefficient-reuse-2026-10-21/README.md).
Raw process captures remain local and untracked.

**Next Plan 15 task:** attribute the remaining T720 q8 CC/F1 synthesis excess
at matched recurrence/spectrum and inverse-output boundaries. Do not keep
extending coefficient-cache eligibility without a further measured source
benefit; keep odd-FFT optimization deferred.
