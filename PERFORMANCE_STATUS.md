# RustSHT performance checkpoint — current findings

**Updated:** 2026-10-21 UTC. **Branch:** `opt/sht-performance-parity`.
**Only active objective:** [Plan 11 — scalar f64 GL analysis parity](plans/11-ducc-equivalent-scalar-analysis.md).
This is a **measured checkpoint, not a standalone instruction queue**.
The previous long status, including intermediate experiments and historical
next-step text, is preserved
[here](reports/archive/performance-status-2026-10-21-pre-refresh.md).
For reproducible raw benchmark tables see [reports/README](reports/README.md).

## Highest-confidence current finding — prefix discontinuity removed

The controlled change at `0bc5f153` removes only RustSHT's global
`latitude_count * 2^-60 <= 8*EPSILON` gate. The fixed DUCC threshold and
all finite-value, exponent/range and safe-fallback checks remain. Matched
f64 GL STANDARD analysis uses `mmax=2047`, `nphi=4096`, tile 256, Rust
1.99.1/Pulp V3 and DUCC 0.42.0 `42ce8ab5`, GCC 15.2 V3 with
`-O3 -ffast-math -ffp-contract=fast` for the SHT kernel.

| Geometry | q | Rust before → after | DUCC after | Rust/DUCC before → after |
| --- | ---: | ---: | ---: | ---: |
| T2047, 2048 latitudes | 1 | 251.5 → 254.6 ms | 246.5 ms | 1.011× → 1.033× |
| T2047, 2048 latitudes | 8 | 62.2 → 58.6 ms | 42.3 ms | 1.358× → 1.384× |
| T2047, 2049 latitudes | 1 | 828.4 → 254.6 ms | 245.7 ms | 3.321× → 1.036× |
| T2047, 2049 latitudes | 8 | 143.7 → 57.3 ms | 42.1 ms | 3.129× → 1.361× |
| T2048, 2049 latitudes | 1 | 832.4 → 255.4 ms | 248.3 ms | 3.329× → 1.029× |
| T2048, 2049 latitudes | 8 | 134.0 → 58.3 ms | 42.2 ms | 3.159× → 1.383× |

Three interleaved process rounds collected 15 long q1 calls per backend/tile
and 30 calibrated q8 batches. The pathological route is gone: the 2049-latitude
cases now use 2,434 prefix-scaled blocks and zero checked-scaled blocks.
The extra gate bounded a worst-case coherent sum of omitted terms; it was an
additional accuracy budget, not a finite/exponent-safety invariant. Numerical
and exceptional-value results and full timing distributions are in the
[follow-up report](reports/scalar-analysis-cumulative-2026-10-20.md)
and [data](reports/data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/).

T3072 full-order validation (`ntheta=3073`, `nphi=6174`, q1) also selected
the prefix route: 5,780 prefix-scaled blocks, zero checked-scaled blocks and
zero declines. Rust/DUCC analysis was 775.8/735.1 ms (1.056×); synthesis was
1121.4/1152.6 ms (0.973×). These are one-process five-call directional
measurements, not a large-resolution timing campaign. Random output relative
L2 errors were `1.96e-13` for analysis and `2.11e-13` for synthesis; extreme,
subnormal, polar/high-order, cancellation and adjoint checks passed.

**Current q8 finding:** the warmed recurrence/accumulation phase is close to
DUCC at T2047 and T3072, while Rust's order-spectrum preparation stays nearly
serial at q8 (23.5 ms at T2047; 40.5 ms at T3072). Matched whole-analysis
ratios are 1.36x at T2047 q8 and 1.22x at T3072 q8. The next focused test is
parallel preparation of independent order rows, preserving table-path safety
checks and the 256-pair recurrence tile. See the current q8 stage section in
the [cumulative report](reports/scalar-analysis-cumulative-2026-10-20.md).

## Tile decision — not an open recurring benchmark

**Retain 256 reflected-latitude pairs per scalar-analysis tile.**
After restoring the prefix route at 2049 latitudes, tile-256 vs tile-128 vs
tile-64:

| Geometry / q | 256 | 128 | 64 |
| --- | ---: | ---: | ---: |
| T2047 q1 | **254.6 ms** | 263.5 ms | 280.7 ms |
| T2047 q8 | **57.3 ms** | 59.2 ms | 61.9 ms |
| T2048 q1 | **255.4 ms** | 263.6 ms | 282.1 ms |
| T2048 q8 | **58.3 ms** | 58.6 ms | 60.9 ms |

Tiles 64/128 are diagnostic only. They lose to 256 after the prefix fix;
there is no reason to revert production or run more 64/128 matrices.
A tile contains up to 256 reflected north/south **latitude pairs**
(512 physical latitudes), not 256 maps, degrees or CPU threads.

A single **optional** 512-pair experiment remains deferred until a
reasoned cache/setup hypothesis exists; current fixed SIMD-array
scratch capacity is only 256, so it requires a coordinated and
validated capacity change.
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
- **T3072 current-source check:** after the prefix fix, full-order q1 GL
  measured 775.8 ms Rust vs 735.1 ms DUCC analysis (1.056×) and
  1121.4 vs 1152.6 ms synthesis (0.973×), with five whole-call samples per
  backend in one process. These timings are directional; errors, edge cases
  and adjoint checks passed. The older Rust 1.99.0 / DUCC `80388904` 214-case
  full sweep observed up to 8.39×/7.44× q1/q8 T3072 analysis and remains
  historical, not a current-source claim.
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

1. **Active:** attribute the remaining q8 whole-transform costs at
   representative geometries, starting with the largest measured stage.
   Reuse the matched T2047/T2048 and T3072 results instead of restarting
   their matrices. Keep 256 as default.
2. **Then:** investigate opt-in fixed scratch only when a source-grounded
   hypothesis and matched whole-transform measurements justify it.
3. **Future only:** [Plan 12](plans/12-cache-informed-legendre-dispatch.md)
   cache-topology queries plus optional offline pointer-chase diagnostic
   (no startup autotuning); [Plan 13](plans/13-fused-batched-sht.md)
   true basis-shared batches with measured internal blocking.

All prior measurements, full machine-readable CSV/JSON, rejected
experiments, historical plans and source versions are intentionally
**retained in the archive or reports/data**. Historical sections and
old forward-looking language do not override Plan 11.
