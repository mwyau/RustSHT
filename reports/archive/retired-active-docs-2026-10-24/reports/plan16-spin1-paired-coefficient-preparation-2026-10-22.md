# Plan 16: paired coefficient preparation reuse

**Disposition:** do not retain the within-group reuse prototype or add a
transform-wide prepared-coefficient cache. The q1 profile showed repeated
preparation, but the matched whole-transform comparison did not show a
repeatable gain. The q8 measurements leave little wall-time opportunity for a
cache large enough to span ring groups.

## Hypothesis and profile

`synthesize_spin1_scalar_block` prepared the same four even/odd coefficient
streams again for each 64-latitude block in a ring group. The streams depend
on the Fourier order, converted E/B rows and spin factors, all fixed through
the group's latitude blocks. A temporary candidate shared their checked
preparation within the current order and ring group. It added no workspace or
cross-call cache; polar/overflow declines continued through the existing
fallback.

On T720 exact CC (`ntheta=722`), q1 processes each ring group in twelve
64-latitude blocks. The baseline paired route prepared once per eligible
block: 5,293 eligible blocks at full order and 1,784 at `mmax=180`. The
candidate made 721 and 181 preparation calls, respectively. Its test-only
preparation timer included both paired and separate fallback preparations;
the baseline timer covered only the paired route, so those internal timings
are not an exact phase-to-phase comparison. q8 groups contain one such block,
so this reuse does not remove the repeated preparation across groups.

A transform-wide q8 cache would hold four `Complex64` streams for every
order/degree pair: about 7.98 MiB at full T720 and 3.49 MiB at `mmax=180`.
The current profile's busiest-worker preparation time was 1.09–1.24 ms full
and 0.46–0.49 ms reduced, against 23–25 ms and 17–19 ms total calls. These
values bound a potential saving; they do not establish that cache building,
sharing and synchronization would be faster.

## Matched full-transform comparison

Baseline source was `dd39f82714e7b27b9521671048aab2a84c80f14e`. The candidate
was that source plus the archived [`candidate.patch`](data/plan16-spin1-paired-coefficient-block-reuse-2026-10-22/candidate.patch).
Two candidate runs bracketed one baseline run. Each run covered T720 exact
CC, full and quarter order, even-smooth and odd-minimum longitude counts,
analysis and synthesis, q1 and q8. Each source run used three interleaved
Rust/DUCC process pairs. Short calls used 30 calibrated batches; long q1
operations used individually timed calls. The table reports warmed synthesis
medians in milliseconds. Candidate is the median of the two separate run
medians; see the CSV for each pass and p10/p90 distributions.

| `mmax` | Longitude | Threads | Rust baseline | Candidate pass 1 | Candidate pass 2 | Candidate / baseline | DUCC baseline | Max relative L2 |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 720 | even-smooth | q1 | 94.157 | 93.933 | 94.437 | +0.03% | 43.945 | `2.29e-13` |
| 720 | odd-minimum | q1 | 110.529 | 110.919 | 109.527 | −0.28% | 61.613 | `2.34e-13` |
| 720 | even-smooth | q8 | 23.613 | 23.454 | 23.862 | +0.19% | 6.111 | `2.29e-13` |
| 720 | odd-minimum | q8 | 25.334 | 25.284 | 25.300 | −0.16% | 8.468 | `2.34e-13` |
| 180 | even-smooth | q1 | 44.111 | 43.523 | 43.885 | −0.92% | 18.577 | `3.63e-13` |
| 180 | odd-minimum | q1 | 47.163 | 47.060 | 46.494 | −0.82% | 19.252 | `3.47e-13` |
| 180 | even-smooth | q8 | 17.441 | 17.261 | 17.304 | −0.91% | 2.644 | `3.63e-13` |
| 180 | odd-minimum | q8 | 17.760 | 17.640 | 17.682 | −0.56% | 2.721 | `3.47e-13` |

No synthesis change exceeded 0.92%; p10–p90 ranges overlapped in nearly all
cases. Reduced odd q1 was lower in the second candidate run but not in the
first. Analysis controls ranged from −0.46% to +1.18%, with wider tails in
some q8 distributions. The measurement therefore does not support attributing
a whole-transform gain to this change.

Across the three source runs, all 144 Rust/DUCC comparisons produced finite
outputs and passed repeat and destination-reset checks. Maximum relative L2
error was `3.63e-13`. Host measurements were similar: median runqueue wait
0.27–0.38%, p90 6.49–6.64%, and median frequency 4.604–4.619 GHz. The host was
an AMD Ryzen 9 5950X; q1 used CPU 0 and q8 CPUs 0–7 in one L3 domain. Rust was
1.99.1/LLVM 23.1.1, runtime Pulp V3, with `-C target-cpu=x86-64-v3`. DUCC was
0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, with
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for SHT and strict
floating point for the driver.

## Artifacts and next step

The [data directory](data/plan16-spin1-paired-coefficient-block-reuse-2026-10-22/README.md)
contains per-case distributions, build metadata and the exact rejected patch.
The 2,655 per-batch timing and host samples remain local and untracked. The
experiment changes were removed from production. Plan 16 Milestone C is closed without a cache;
continue to Milestone D's order-major dataflow investigation, retaining the
current group-scheduled implementation as the control.
