# RustSHT performance evidence — reading guide

**Current implementation plan:** [Plan 14](../plans/14-small-serial-parallel-performance.md).
**Current factual status and priorities:** [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md).
**Recent measured results:** [Plan 14 sparse spectrum-tail validation and current synthesis profile](plan14-sparse-spectrum-tail-2026-10-21.md), [Plan 14 reflected-geometry preflight](plan14-reflected-geometry-preflight-2026-10-21.md), [Plan 14 T16 fused analysis route](plan14-t16-analysis-cutoff-2026-10-21.md), [Plan 14 T64 fused analysis route](plan14-t64-fused-analysis-cutoff-2026-10-21.md), [Plan 14 T127 compact-table SIMD analysis](plan14-t127-table-analysis-simd-2026-10-21.md), [Plan 14 small-q1 baseline and T127 analysis attribution](plan14-small-q1-baseline-2026-10-21.md), [Plan 14 direct synthesis output](plan14-direct-spectrum-output-2026-10-21.md) and [Plan 11 scalar-analysis closeout](scalar-analysis-cumulative-2026-10-20.md).
Do not follow forward-looking instructions embedded in historical reports.

## Current and accepted evidence

- [Current-source dense RustSHT/DUCC scalar GL sweep (2026-10-21)](ducc-current-full-sweep-2026-10-21.md): all `lmax=0…128` plus 28 sampled degrees through T3072, full/reduced order, analysis/synthesis, q1/q8, even-smooth coverage throughout and selected odd-minimum cases. Pinned Rust source `e487b85b`, DUCC 0.42.0 `42ce8ab5`; 1,504 complete measurement groups, 752 numerical validation configurations and no failed cases. The broad result is mixed by degree/order/thread count: large-degree full-order ratios cluster near parity, while some T64–T256 full-order q8 analysis cases are substantially slower. Raw samples, coverage audit and figures are linked from the report. This is a current-source baseline, not a Plan 14 optimization A/B.
- [Plan 14 scalar GL closeout (2026-10-21)](plan14-closeout-2026-10-21.md): the full sweep remeasured the T720 reduced odd q8 synthesis residual at +0.228 ms, matching the prior stage profile's +0.231 ms within variation. That residual splits between recurrence/spectrum (+0.112 ms) and inverse/writeback (+0.121 ms); the 1.62 µs sparse-tail initialization is not material. The checked real-C2R alternative was slower, so no further production change is supported. The q1 reduced odd T720 excess remains documented and un-attributed.
- [T2047/T2048 cutoff and previous T720 A/B findings](scalar-analysis-cumulative-2026-10-20.md):
  q1 nearly at DUCC parity at T2047/2048 latitudes with 256-pair
  tiles. The extra latitude-count prefix gate was removed after matched
  T2047/T2048/T3072 validation; the 3.1–3.3x discontinuity is closed.
  Fixed SIMD scratch remains opt-in; packed reduction, indexed traversal
  and direct coefficient accumulation are retired.
- [Plan 14 direct q8 synthesis output](plan14-direct-spectrum-output-2026-10-21.md):
  a new order-owned ring-major output path removes the intermediate
  order-major spectrum and parallel scatter. This is distinct from the
  retired analysis direct-coefficient-accumulation experiment.
- [Plan 14 reflected-geometry preflight](plan14-reflected-geometry-preflight-2026-10-21.md):
  hoisting the unchanged GL reflection/polar-order validation out of each
  T720 q8 synthesis order job reduced matched Rust median latency by 4.1%;
  generated-code inlining also changed, so the full gain is not assigned to
  the scan alone.
- [Plan 14 sparse spectrum-tail validation and current synthesis profile](plan14-sparse-spectrum-tail-2026-10-21.md):
  the current T720 synthesis matrix confirms the reduced odd q8 gap is
  +0.210 ms; matched stage timers rank recurrence/spectrum and odd inverse
  rows at roughly +0.112/+0.121 ms. No new recurrence or FFT implementation
  is justified; Plan 14 is in closeout.
- [Plan 14 T127 table analysis SIMD](plan14-t127-table-analysis-simd-2026-10-21.md):
  a V3 two-complex-lane table accumulator improved matched q1 whole-call
  latency by 5.5%; q8 was inconclusive and did not show a gain.
- [Plan 14 T64 fused analysis route](plan14-t64-fused-analysis-cutoff-2026-10-21.md):
  lowering the checked f64 fused-route floor from T127 to T64 improved
  matched q1 T64 m16 analysis by 35–36%; the T63 controls were stable.
- [T64 Legendre table/recurrence dispatch decision](scalar-t64-recurrence-transition-2026-10-20.md):
  retained synthesis-only recurrence cutoff; analysis and spin-1
  keep their own policies. This is a **completed** dispatch study,
  not an instruction to repeat it.
- [Raw benchmark datasets and reproduction evidence](data/):
  remain in their original paths. Dataset source revisions and
  compiler/ISA flags vary. Preserve exact attribution.

## Historical evidence — archived, not live directions

- [2026-10-20 full cumulative scalar-analysis experiments](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md):
  complete rejected/opt-in candidate history before consolidation.
- [DUCC 0.42.0 GCC/Clang post-upgrade baseline](archive/experiments/ducc-upgrade-baseline-2026-10-20.md):
  compiler/source contract and kernel evidence; high-contention
  differences cannot be treated as clean current conclusions.
- [Older full-transform 214-configuration sweep](archive/experiments/ducc-full-transform-parity-2026-10-20.md):
  Rust 1.99.0 and its original pinned DUCC source; T3072 historical
  ratios are not current-source benchmarks.
- [2026-10-19 consolidated review of 80 older reports](archive/performance-findings-2026-10-19-consolidated.md):
  source catalog and rejected prototypes, not a new task queue.
- [Prior Plan 11 and status snapshots](archive/README.md),
  [earlier dated experiments](archive/experiments/) and
  [past reviews/plans](archive/README.md).

**Method:** Do not combine different code revisions, compiler flags,
ISA dispatch, timing boundaries, truncation/order/longitude work,
q1/q8 thread placement or cold/warm states into a trend.
Matched whole-transform and reproducible numerical checks govern
production decisions. **Keep 256 reflected-latitude-pair tiles as
the current default; do not repeat the retired 64/128 comparison.**

**Deferred work only:** [cache-informed Legendre choice](../plans/12-cache-informed-legendre-dispatch.md)
and [fused batches](../plans/13-fused-batched-sht.md).
