# RustSHT plans — current status and reference index

**Updated 2026-10-21 UTC.** Plan 11 is complete at closeout commit
`a37f74d68631ab5fd373a4097640895e9f0c0c23`; **Plan 14 is the active
implementation plan**. This index also distinguishes deferred proposals,
numerical contracts and archived historical instructions. In any conflict,
current explicit owner instructions, [AGENTS.md](../AGENTS.md) and
[Plan 14](14-small-serial-parallel-performance.md) control.

## Active plan

**[14 — Small serial and large parallel SHT parity](14-small-serial-parallel-performance.md)**
is active on `opt/sht-performance-parity`, using Plan 11's final source
baseline. It targets small q1 fixed costs and remaining large q8 excess;
the reduced-`mmax` front end, T2048 cutoff and q8 order preparation are
closed. See [current benchmark decisions](../PERFORMANCE_STATUS.md).

**[11 — Scalar f64 GL analysis parity](11-ducc-equivalent-scalar-analysis.md)**
is **complete** at `a37f74d68631ab5fd373a4097640895e9f0c0c23`; retain its
findings and data as the validated baseline rather than reopening its work.

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

## Already retired historical plans

Files 00, 03, 04, 05 (including history), 07, 08, 09 and 10 are
**short links** to preserved prior plans under
[reports/archive/plans](../reports/archive/plans/).
The [older full Plan 11 before the 2026-10-21 cleanup](../reports/archive/plans/11-ducc-equivalent-scalar-analysis-pre-refresh-2026-10-21.md)
and [old status log](../reports/archive/performance-status-2026-10-21-pre-refresh.md)
are also historical. Do not resume their rejected profiling,
tile-size, packed-reduction or SIMD plans without new measured evidence.

## Evidence and data

Use the [recent experiment index](../reports/scalar-analysis-cumulative-2026-10-20.md)
and [reports guide](../reports/README.md) for the best available
current/archived benchmark distinctions. Raw data remain under
[reports/data](../reports/data/).
