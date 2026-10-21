# RustSHT performance checkpoint — current findings

**Updated:** 2026-10-21 UTC. **Branch:** `opt/sht-performance-parity`.
**Active objective:** [Plan 14 — small serial and large parallel SHT parity](plans/14-small-serial-parallel-performance.md).
This is a **measured checkpoint, not a standalone instruction queue**.
The previous long status, including intermediate experiments and historical
next-step text, is preserved
[here](reports/archive/performance-status-2026-10-21-pre-refresh.md).
For reproducible raw benchmark tables see [reports/README](reports/README.md).

**Plan transition (2026-10-21):** Plan 11's reduced-`mmax` frontend study
is closed at `a37f74d68631ab5fd373a4097640895e9f0c0c23`. The input scan,
row FFT, spectrum pruning, order preparation and recurrence stage evidence
is recorded in the cumulative report and CSV. Plan 14 is now active on the
same branch/session. Its final-source rebaseline is the next task; Plans
12/13 remain deferred.


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

**Current q8 result:** commit `dfed3ff7` parallelizes independent order-row
preparation for recurrence plans; `3b32f2de` bundles its shared read-only
state. The compact-table safety pass remains serial, and recurrence,
fallbacks, prefix threshold and 256-pair tile are unchanged. Order-prep phase
time fell from 23.5 to about 6.4 ms at T2047 and 40.5 to about 15.4 ms at
T3072. Matched whole-transform Rust q8 latency fell 25.6% at T2047 full
`mmax` (57.310 to 42.655 ms) and 14.9% at T3072 (144.337 to 122.835 ms);
T720 fell 14.4% (3.597 to 3.081 ms). Current median Rust/DUCC ratios are
1.003x, 1.042x and 1.067x, respectively, with distribution overlap at
T2047. Commit `41c7223f` then pruned unused real-FFT output bins for scalar
GL analysis without changing the FFT algorithm, finite-input checks,
workspace stride or other grid/spin paths. Against the preceding source,
reduced-order q8 latency improved 12.4% at T2047 `mmax=511` (21.399 to
18.739 ms) and 7.8% at T3072 `mmax=768` (64.306 to 59.266 ms); current
median ratios are 1.098x and 1.017x. Full-order changes stayed within the
measured distributions. The q8 Rust outputs were bitwise identical
before/after, and DUCC errors remained at most `2.08e-13` relative L2. The
pruning source is current commit `1fddce4a` (the source-equivalent pre-rebase
hash was `41c7223f`). The follow-up substage screen found no remaining
forward-FFT excess against DUCC `map2leg`; the safe input check remains fused
with full-row FFT packing, and no further frontend rewrite is justified by
the overlapping whole-call distributions. See the [candidate comparison and
frontend closeout](reports/scalar-analysis-cumulative-2026-10-20.md#plan-11-reduced-mmax-frontend-closeout-2026-10-21),
[q1/q8 summary](reports/data/scalar-analysis-cumulative-2026-10-20/truncated-output-candidate/summary.csv)
and [order-preparation results](reports/data/scalar-analysis-cumulative-2026-10-20/order-prep-q8-candidate/summary.csv).

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
- **Final Plan 11 current-source analysis checkpoint:** at T720 full
  `mmax`, q8 Rust/DUCC was 3.063/2.937 ms (1.043×); reduced `mmax=180`
  was 1.728/2.002 ms (0.863×). At T2047 full `mmax`, q8 was
  42.907/42.563 ms (1.008×), and reduced `mmax=511` was
  18.739/17.064 ms (1.098×). At T3072 full `mmax`, q8 was
  124.428/117.119 ms (1.062×), while reduced `mmax=768` was
  59.266/58.253 ms (1.017×). The final reduced frontend attribution
  found no material Rust forward-row excess over DUCC `map2leg`; do not
  repeat it. Plan 14 should remeasure selected cases with its interleaved
  protocol and investigate the remaining absolute excess, especially full
  T3072, separately from small-q1 fixed costs.
- **Compiler and inner kernel:** GCC/Clang DUCC results differ even
  with verified source flags; host contention limits attribution.
  The corrected warmed T720 `m=1` recurrence/accumulation kernel
  is competitive with DUCC in one GCC comparison. Do not
  optimize checked recurrence as if it were the warmed hot path;
  profile the actual residual at equivalent boundaries.
  [Historical toolchain report](reports/archive/experiments/ducc-upgrade-baseline-2026-10-20.md).

## Current next steps and deferred work

1. **Plan 14 rebaseline:** freeze Plan 11 closeout `a37f74d6`; run the
   selected small-q1 A/S sweep and T720/T2047 q8 A/S subset with matched
   Rust/DUCC inputs, process interleaving and calibrated batches. Report
   cold/setup separately from warm calls, then attribute the largest
   measured absolute excess before changing production code.
2. **Future only:** [Plan 12](plans/12-cache-informed-legendre-dispatch.md)
   cache-topology queries plus optional offline pointer-chase diagnostic
   (no startup autotuning); [Plan 13](plans/13-fused-batched-sht.md)
   true basis-shared batches with measured internal blocking.

All prior measurements, full machine-readable CSV/JSON, rejected
experiments, historical plans and source versions are intentionally
**retained in the archive or reports/data**. Historical sections and
old forward-looking language do not override active Plan 14.
