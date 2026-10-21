# RustSHT performance evidence — reading guide

**Current implementation plan:** [Plan 14](../plans/14-small-serial-parallel-performance.md).
**Current factual status and priorities:** [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md).
**Recent measured results:** [Plan 14 direct synthesis output](plan14-direct-spectrum-output-2026-10-21.md) and [Plan 11 scalar-analysis closeout](scalar-analysis-cumulative-2026-10-20.md).
Do not follow forward-looking instructions embedded in historical reports.

## Current and accepted evidence

- [T2047/T2048 cutoff and previous T720 A/B findings](scalar-analysis-cumulative-2026-10-20.md):
  q1 nearly at DUCC parity at T2047/2048 latitudes with 256-pair
  tiles; one-latitude increase triggers the Rust-only prefix gate
  and a severe checked-scaled route regression. Current investigations
  concern the *necessity* of that extra gate, not small tile sweeps.
  Fixed SIMD scratch remains opt-in; packed reduction, indexed
  traversal and direct coefficient accumulation are retired.
- [Plan 14 direct q8 synthesis output](plan14-direct-spectrum-output-2026-10-21.md):
  a new order-owned ring-major output path removes the intermediate
  order-major spectrum and parallel scatter. This is distinct from the
  retired analysis direct-coefficient-accumulation experiment.
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
