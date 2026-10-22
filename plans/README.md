# RustSHT plans — current status and reference index

**Updated 2026-10-22 UTC.** This file is an execution queue, not a second source of implementation rules. Follow current explicit owner instructions, [AGENTS.md](../AGENTS.md), the live scalar-ablation record and then the relevant queued plan. Work on `opt/sht-performance-parity`, with safe integration of concurrent remote updates.

## Execution queue

1. **CURRENT:** Finish the [owner-directed scalar-analysis cumulative-ablation](../reports/scalar-analysis-cumulative-ablation-2026-10-21.md), including the remaining fixed SIMD layout and combined/ablation evaluation. This follow-on is distinct from completed Plan 11.
2. **QUEUED:** [16 — DUCC spin-1 recurrence and dataflow](16-ducc-spin1-recurrence-parity.md). Optimize verified f64 spin-1 CC/F1/GL recurrence and spectrum overhead with a DUCC algorithm crosswalk, parity-separated SIMD test, downstream prepared-coefficient reuse, and conditional order-major work.
3. **QUEUED AFTER PLAN 16:** [17 — f32 and high-resolution atmospheric SHT performance](17-f32-atmospheric-performance.md). Establish current matched native DUCC f32 scalar/spin-1 parity and address only measured f32 or T1023–T2047 scaling gaps; preserve odd-FFT hold.
4. **DEFERRED, NOT AUTOMATIC:** [12 — Cache-informed Legendre dispatch](12-cache-informed-legendre-dispatch.md) and [13 — True fused batched SHT](13-fused-batched-sht.md). These are separate future projects and require explicit owner authorization.

## Completed and retained

- **[11 — Scalar f64 GL analysis parity](11-ducc-equivalent-scalar-analysis.md):** completed at `a37f74d68631ab5fd373a4097640895e9f0c0c23`.
- **[14 — Small serial and parallel SHT parity](14-small-serial-parallel-performance.md):** completed at `f2d9ce47fdf04946ef5cbe478a30b0fc38956f8e`.
- **[15 — Spin-1 vector GL/CC/F1 performance](15-spin1-vector-gl-cc-f1-performance.md):** completed. Its synthesis dataflow closeout retains accepted scheduling/coefficient-reuse changes and negative SIMD experiments.

**Owner hold:** defer odd-length FFT optimization, including paired-real-row FFTs, `realfft` substitution and standalone/custom FFT backends. Keep odd/even sampling as correctness and performance controls only. Do not extend Plan 17 into batch APIs or Plan 12's cache-detection machinery.

## Retained specifications — references, not task queues

- [01 — Provenance, bibliography and licensing](01-ducc-provenance-and-license.md)
- [02 — Numerical/grid and operator contracts](02-numerics-and-grids.md)
- [06 — DUCC operation crosswalk](06-ducc-contract-crosswalk.md)

Historical references may use past/future implementation language
from initial porting. Such language is not an active assignment or a
new acceptance criterion; use these files for mathematical, API
and licensing invariants only.

## Archived historical plans — direct links

The retired plan documents are stored **only in the archive**, not as
redirect stubs in `plans/`. These links are for historical context, **not**
active implementation instructions:

- [00 — Scope and decisions](../reports/archive/plans/00-scope-and-decisions.md)
- [03 — API and Python](../reports/archive/plans/03-api-and-python.md)
- [04 — Architecture and performance](../reports/archive/plans/04-architecture-and-performance.md)
- [05 — Validation and milestones](../reports/archive/plans/05-validation-and-milestones.md)
- [05 — Validation and milestones history](../reports/archive/plans/05-validation-and-milestones-history.md)
- [07 — DUCC performance gap](../reports/archive/plans/07-ducc-performance-gap.md)
- [08 — Kernel naming and retirement](../reports/archive/plans/08-kernel-naming-and-retirement.md)
- [09 — Matched DUCC–Rust profiling](../reports/archive/plans/09-matched-ducc-rust-profiling.md)
- [10 — Three-paper method audit](../reports/archive/plans/10-three-paper-method-audit.md)

The [older full Plan 11 before the 2026-10-21 cleanup](../reports/archive/plans/11-ducc-equivalent-scalar-analysis-pre-refresh-2026-10-21.md)
and [old performance status](../reports/archive/performance-status-2026-10-21-pre-refresh.md)
are also historical. Do not revive rejected profiling, tile-size,
packed-reduction or SIMD experiments without new measured evidence.

## Evidence and data

Use the [recent experiment index](../reports/scalar-analysis-cumulative-2026-10-20.md)
and [reports guide](../reports/README.md) for the best available
current/archived benchmark distinctions. Raw data remain under
[reports/data](../reports/data/).
