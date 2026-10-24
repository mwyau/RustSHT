# Plan 17 f32 spin recurrence range-skip experiment

This report closes one bounded Plan 17 Milestone B experiment. It tested a
SIMD normal-range scan that skips per-lane recurrence normalization while both
spin recurrences remain in the ordinary exponent range. The scan retained the
existing lane normalization whenever a lane left that range.

## Matched comparison

- Rust baseline: `83070d527c56d57732f1339ea1f1aa6933a31490`.
- Candidate source: uncommitted experiment against that baseline. The 32k-work
candidate binary SHA-256 is
  `60942ce3fccaed00e98f4c98ca4f05d1c4e1640ad06dbd722521a082823ccee2`;
  the 33k-work binary is
  `698c08569c64040f68ffbf0d931cdf8a343b5448c01a3ee65e8fb36e94ad2002`.
- Rust 1.99.1 / LLVM 23.1.1, release profile with the repository's default
  flags; runtime Pulp V3 (AVX2/FMA), eight f32 lanes.
- DUCC 0.42.0,
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, built with GCC 15.2 and
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`.
- Ryzen 9 5950X; q1 pinned to CPU 0, q8 to CPUs 0–7 in one L3 domain. Runs
  were interleaved across three fresh process pairs. Calls below 100 ms used
  ten calibrated ~25 ms batches per process; longer calls used five
  individually timed invocations per process. Setup, first call and warm
  latency are separate in the CSV. Sample counts are 30 batches for short
  calls and 15 individual calls for long calls.

## Warm results

The table's candidate column uses the 32k-work binary. The follow-up 33k-work
binary disabled the SIMD scan at T511 while retaining it at T720 full and
quarter order. It still regressed at T511, so that raised threshold was not a
production fix.

| Grid / lmax / mmax | Operation | Threads | Rust baseline ms | Candidate ms | DUCC ms | Candidate / baseline | Candidate / DUCC | Candidate vs Rust relative L2 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| GL / 255 / 255 | Synthesis | 8 | 21.563 | 22.547 | 0.572 | 1.046 | 39.44 | 0 |
| GL / 511 / 511 | Analysis | 8 | 109.236 | 112.785 | 1.710 | 1.032 | 65.96 | 0 |
| GL / 511 / 511 | Synthesis | 8 | 144.326 | 147.672 | 2.333 | 1.023 | 63.31 | 0 |
| GL / 720 / 720 | Analysis | 8 | 296.356 | 288.618 | 4.113 | 0.974 | 70.17 | 0 |
| GL / 720 / 720 | Synthesis | 8 | 704.632 | 681.456 | 5.600 | 0.967 | 121.68 | 0 |
| GL / 720 / 180 | Analysis | 8 | 124.443 | 113.529 | 1.726 | 0.912 | 65.79 | 0 |
| GL / 720 / 180 | Synthesis | 8 | 335.751 | 320.340 | 2.504 | 0.954 | 127.92 | 0 |
| GL / 720 / 720 | Analysis | 1 | 2245.299 | 2183.120 | 28.532 | 0.972 | 76.52 | 0 |
| GL / 720 / 720 | Synthesis | 1 | 2735.449 | 2612.749 | 40.013 | 0.955 | 65.30 | 0 |

At T720 the candidate reduced Rust latency by 2.6–8.8%. At T255 and T511 it
was 2.3–4.6% slower. In the threshold-33k follow-up, T511 q8 analysis and
synthesis remained 3.2% and 3.7% slower even though the scan was disabled.
Thus the observed low-degree cost is not explained by executing the scan
itself. The V3 analysis specialization with range checking disabled has the
same 448 decoded instructions and `0x8a6` byte size as the baseline symbol;
the experiment did not isolate whether the remaining difference is dispatch,
code placement or process timing. No claim is made that the normal-range scan
alone caused the T720 gains.

All candidate outputs were bit-for-bit identical to Rust baseline, finite,
repeat-consistent and reset-consistent. DUCC-relative errors were unchanged
from baseline and ranged from `1.16e-5` to `1.43e-4` on these fixtures. The
CSV contains p10/p90 distributions, per-process paired ratios, first-call and
setup timings, sample counts, errors and reset checks.

## Validation and disposition

With the candidate active, release focused tests passed:

- f32 library tests: 16 passed, 1 ignored.
- f32 integration tests: 11 passed, 1 ignored, with and without default
  features.
- Extreme scaling tests: 4 passed, with and without default features.

The candidate is not retained in production: it helps at T720 but regresses
T255/T511, including the route where the scan is disabled. A future attempt
needs to preserve the baseline low-work dispatch/code path and demonstrate the
high-work benefit again before promotion.

Machine-readable results: [results.csv](data/plan17-f32-spin-normal-range-2026-10-22/results.csv).
