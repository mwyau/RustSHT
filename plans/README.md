# RustSHT plans — current status and reference index

**Updated 2026-10-21 UTC.** Plan 11 is complete at closeout commit
`a37f74d68631ab5fd373a4097640895e9f0c0c23`; Plan 14 is complete at
`f2d9ce47fdf04946ef5cbe478a30b0fc38956f8e`; **Plan 15 is active**. This
index distinguishes deferred proposals,
numerical contracts and archived historical instructions. In any conflict,
current explicit owner instructions, [AGENTS.md](../AGENTS.md) and
[Plan 15](15-spin1-vector-gl-cc-f1-performance.md) control.

## Active plan

**[15 — Spin-1 vector GL/CC/F1 performance](15-spin1-vector-gl-cc-f1-performance.md)**
is active on `opt/sht-performance-parity` after Plan 14's full scalar
sweep and closeout. Its native-DUCC baseline proceeds through GL, CC and F1.
The owner has **deferred all odd-length FFT optimization**, including paired-real
rows, `realfft` substitution and native/custom FFT integration; odd/even
benchmark and numerical validation controls remain in scope. The completed
scalar T64–T256 dispatch diagnostic is reference evidence, not a new scalar
task. See [current benchmark decisions](../PERFORMANCE_STATUS.md).

**[11 — Scalar f64 GL analysis parity](11-ducc-equivalent-scalar-analysis.md)**
is **complete** at `a37f74d68631ab5fd373a4097640895e9f0c0c23`; retain its
findings and data as the validated baseline rather than reopening its work.

## Plan 15 scope — spin-1 across GL, CC and F1

Plan 15 specifically covers **spin-1 vector E/B analysis and synthesis on
all three grids in order: GL, CC, then F1**, with the correct grid-specific
CC/F1 meridional-resampling and adjoint contracts. It is **not** a general
scalar CC/F1 optimization project. The staged native-DUCC baseline, focused
code changes, numerical tests and performance report are in the plan.

## Deferred TODOs — Plans 12 and 13 remain inactive

- **[12 — Cache-informed Legendre dispatch](12-cache-informed-legendre-dispatch.md):**
  offline multi-host calibration of a cheap cross-platform cache-metadata
  heuristic, with an optional **diagnostic** pointer-chase benchmark,
  but **no runtime transform benchmarking at import or plan creation**.
- **[13 — True fused batched SHT](13-fused-batched-sht.md):**
  arbitrary practical public batch size with bounded internal field
  blocks, reusing the Legendre basis; test 1/2/4/8/16/32/64+
  against shared-plan independent calls. Batch-aware Plan 12
  calibration follows after actual fused kernels exist.

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
