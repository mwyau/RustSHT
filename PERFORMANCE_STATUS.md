# RustSHT performance checkpoint — current findings

**Updated:** 2026-10-21 UTC. **Branch:** `opt/sht-performance-parity`.
**Only active objective:** [Plan 11 — scalar f64 GL analysis parity](plans/11-ducc-equivalent-scalar-analysis.md).
This is a **measured checkpoint, not a standalone instruction queue**.
The previous long status, including intermediate experiments and historical
next-step text, is preserved
[here](reports/archive/performance-status-2026-10-21-pre-refresh.md).
For reproducible raw benchmark tables see [reports/README](reports/README.md).

## Highest-confidence current finding — T2048 prefix discontinuity

Matched **T2047, f64 GL STANDARD analysis**, `mmax=2047`,
`nphi=4096`, 256 reflected-latitude-pair tiles, Ryzen 5950X;
pinned DUCC 0.42.0 `42ce8ab5`.
Numbers below are *whole-transform medians*, not extrapolated kernel-only
performance:

| Geometry | q | RustSHT | DUCC | Interpretation |
| --- | --- | ---: | ---: | --- |
| T2047, 2048 latitudes | 1 | 251.5 ms | 248.6 ms | essentially at q1 parity |
| T2047, 2049 latitudes | 1 | 828.4 ms | 249.4 ms | abrupt ~3.3× Rust/DUCC excess |
| T2047, 2048 latitudes | 8 | 62.2 ms | 45.8 ms | remaining ~1.36× q8 gap |
| T2047, 2049 latitudes | 8 | 143.7 ms | 45.9 ms | abrupt ~3.1× ratio of rounded medians |
| T2048, 2049 latitudes | 1 | 832.4 ms | 250.1 ms | adding degree is not the principal cause |
| T2048, 2049 latitudes | 8 | 134.0 ms | 42.4 ms | same pathological route |

Controls at unchanged `mmax`/`nphi` isolate **one additional latitude**:
2,432 prefix-enabled blocks at `ntheta=2048` become 2,434
checked-scaled blocks at `ntheta=2049`. The extra Rust-only
`latitude_count * 2^-60 <= 8*EPSILON` global insignificance guard
flips at 2048/2049. DUCC uses `sht_ftol=2^-60` in
`iter_to_ieee` but **does not have this global gate**. The latest
analysis strongly points at the route switch; the controlled removal
experiment still must prove actual safety, numerical fidelity and causality.
Neither removal nor a replacement guard is yet a validated production change.

Matched method: Rust 1.99.1/LLVM 23.1.1 strict V3 + DUCC GCC 15.2
native V3 with `-O3 -ffast-math -ffp-contract=fast`.
Three interleaved rounds; 15 whole-call observations per long
backend/tile case, 30 calibrated batches for shorter q8 cases.
Maximum reported relative L2 error `1.18e-13`; all worker resets passed.
Runqueue interference affects q8 precision more than q1.
[Current cutoff report and measured table](reports/scalar-analysis-cumulative-2026-10-20.md)
· [Data and reproduction](reports/data/scalar-analysis-cumulative-2026-10-20/t2048-cutoff-current/).

**Next:** compare DUCC's scale transition and Rust's guard, experiment
with removing **only** the extra gate while preserving DUCC's fixed
`2^-60` threshold and Rust's range/finite/fallback invariants;
validate polar, cancellation, subnormal, extremes and adjoints,
and rerun exactly matched T2047/2048/T3072 q1/q8.
No safety or correctness compromise is authorized.

## Tile decision — not an open recurring benchmark

**Retain 256 reflected-latitude pairs per scalar-analysis tile.**
At fast-prefix T2047/2048 latitudes, tile-256 vs tile-128 vs tile-64:

| q | 256 | 128 | 64 |
| --- | ---: | ---: | ---: |
| 1 | **251.5 ms** | 261.9 ms | 282.1 ms |
| 8 | **62.2 ms** | 63.1 ms | 67.7 ms |

Tile-64 partially improves the **different, pathological
prefix-disabled** T2047/2049-latitude route by ~14–17%, but
still loses dramatically to DUCC. This is not a reason
to revert production to 64 or run more 64/128 matrices.
A tile contains up to 256 reflected north/south **latitude pairs**
(512 physical latitudes), not 256 maps, degrees or CPU threads.

A single **optional** 512-pair experiment is deferred until
the prefix issue is resolved and a reasoned cache/setup hypothesis
exists; current fixed SIMD-array scratch capacity is only 256,
so this requires a coordinated and validated capacity change.
See [Plan 11](plans/11-ducc-equivalent-scalar-analysis.md).

## T720 production/candidate screening — resolved dispositions

Matched 30-batch, three-process-pair Rust/DUCC full-transform A/S,
q1/q8, T720 `mmax=180/720`, odd-minimum/even-smooth
longitudes at source `b25c011d`. Maximum relative L2
error `5.20e-14`.

| Candidate | Measured result | Disposition |
| --- | --- | --- |
| Fixed aligned SIMD scratch | Analysis nearly neutral: -2.8% to +0.2% alone; independently interleaved q8 **synthesis 11.4–18.0% faster** across four geometry cases, q1 near-neutral | **Opt-in investigation only**; mechanism and memory/workspace trade-off unresolved |
| Packed complex reduction | Small isolated kernel gains, whole-analysis -2.0% to +0.4% alone, inconsistent across shapes | **Retired from production** |
| Direct coefficient accumulation | Earlier sizable T720 analysis regression; guarded variant not shown to beat production | **Retired** |
| Safe indexed SIMD traversal | 0.2–0.3% directional isolated changes within uncertainty; no full-call proof | **Retired** |
| Sectoral seed and scaled-prefix epoch/preflight reuse | Correctness passes; included in measured production, **not separately ablated** in current-source A/B | **Retained; independent win unquantified** |

The earlier cumulative build with all flags produced very large
T720 analysis regressions and is **not** the production recommendation.
Do not revive retired alternatives based solely on instruction counts,
isolated `m=1` results or a desire to assemble another candidate matrix.

[Current A/B data](reports/data/scalar-analysis-cumulative-2026-10-20/t720-ab-current/)
· [Historical full experiment narrative](reports/archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md).

## Established policy and remaining gaps

- **Precomputed table policy:** f64 scalar **synthesis** switches to
  recurrence at T64; scalar **analysis** and spin-1 use distinct
  selectors. Global T64 recurrence regressed q1 analysis at T64/T127/T128.
  [Retained dispatch study](reports/scalar-t64-recurrence-transition-2026-10-20.md).
  Multi-host cache-aware selection is **future Plan 12**, not current work.
- **Earlier full-sweep gap:** the pinned Rust 1.99.0 / DUCC `80388904`
  historical 214-configuration GL study observed up to **8.39×**
  T3072 q1 and **7.44×** T3072 q8 analysis ratio, with q1
  T1536 near parity. Those measurements are **not** a
  current-source regression claim; test T3072 anew after the prefix fix.
  [Historical baseline](reports/archive/experiments/ducc-full-transform-parity-2026-10-20.md).
- **T720 current-source residual:** `mmax=720`, odd-minimum
  q1 analysis 23.60 ms Rust vs 22.16 ms DUCC; even-smooth
  q1 analysis 16.38 vs 14.60 ms. q8 analysis
  3.90 vs 3.19 and 3.04 vs 2.13 ms, respectively.
  This shows **geometry-/thread-dependent** gaps, not one universal 2×.
  Validate further under quiet-host conditions.
- **Compiler and inner kernel:** GCC/Clang DUCC results differ even
  with verified source flags; host contention limits attribution.
  The corrected warmed T720 `m=1` recurrence/accumulation kernel
  is competitive with DUCC in one GCC comparison. Do not
  optimize checked recurrence as if it were the warmed hot path;
  profile the actual residual at equivalent boundaries.
  [Historical toolchain report](reports/archive/experiments/ducc-upgrade-baseline-2026-10-20.md).

## Current next steps and deferred work

1. **Active:** determine whether the Rust-only insignificant-prefix
   global guard is required, safely restore efficient prefix handling
   across 2048/2049 latitudes if validated, and obtain matched
   whole-call results at T2047/T2048 and representative larger/smaller grids.
2. **Then:** attribute remaining q8 whole-transform costs;
   investigate opt-in fixed scratch only when separate, justified
   by actual measurements. Keep 256 as default.
3. **Future only:** [Plan 12](plans/12-cache-informed-legendre-dispatch.md)
   cache-topology queries plus optional offline pointer-chase diagnostic
   (no startup autotuning); [Plan 13](plans/13-fused-batched-sht.md)
   true basis-shared batches with measured internal blocking.

All prior measurements, full machine-readable CSV/JSON, rejected
experiments, historical plans and source versions are intentionally
**retained in the archive or reports/data**. Historical sections and
old forward-looking language do not override Plan 11.
