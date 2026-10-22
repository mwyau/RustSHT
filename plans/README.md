# RustSHT plans — current status and reference index

**Updated 2026-10-22 UTC.** This file is an execution queue, not a second source of implementation rules. Follow current explicit owner instructions, [AGENTS.md](../AGENTS.md), and the active plan. Work on `opt/sht-performance-parity`, with safe integration of concurrent remote updates.

## Execution queue

1. **COMPLETE:** [Owner-directed scalar-analysis cumulative ablation](../reports/scalar-analysis-cumulative-ablation-2026-10-21.md). Retain the staged, checked, 256-pair production path.
2. **COMPLETE:** [16 — DUCC spin-1 recurrence and dataflow](16-ducc-spin1-recurrence-parity.md). Retain accepted q8 order-major/cache improvements and negative experiments; later spin-1 work is explicitly reauthorized under the new owner directive.
3. **ACTIVE — latest owner direction (Luna Max):** [f64 CC benchmark closeout, GL algorithm port to CC, then f64 spin-1 parity](f64-cc-gl-kernel-port-and-spin1-parity-2026-10-22.md). Milestone 0 completes CC coverage/acceptance/figures and safe repository cleanup. Then scalar CC synthesis scheduling/ordered-engine reuse; scalar CC parallel analysis; f64 GL table policy; f64 spin-1 DUCC reflected-pair/scale investigation.
4. **RETAINED SPECIFICATION, SUPERSEDED AS QUEUE:** [Earlier f64 scalar CC full-band and GL table directive](scalar-f64-cc-full-band-and-gl-table-crossover-2026-10-22.md). Its coverage/numerical instructions remain valid; execution order is controlled by the new active plan.
5. **PAUSED — STRICT f64 GATE:** [17 — f32 and atmospheric SHT performance](17-f32-atmospheric-performance.md). Do not resume simply because the CC sweep or GL cutoff study ends; require representative f64 scalar and spin-1 GL/CC/F1 near native DUCC parity or better, without important systematic gaps.
6. **DEFERRED, NOT AUTOMATIC:** [12 — Cache-informed Legendre dispatch](12-cache-informed-legendre-dispatch.md) and [13 — True fused batched SHT](13-fused-batched-sht.md). Need explicit future authorization.

## Completed and retained

- **[11 — Scalar f64 GL analysis parity](11-ducc-equivalent-scalar-analysis.md):** completed at `a37f74d68631ab5fd373a4097640895e9f0c0c23`.
- **[14 — Small serial and parallel SHT parity](14-small-serial-parallel-performance.md):** completed at `f2d9ce47fdf04946ef5cbe478a30b0fc38956f8e`.
- **[15 — Spin-1 vector GL/CC/F1 performance](15-spin1-vector-gl-cc-f1-performance.md):** completed. Its synthesis dataflow closeout retains accepted scheduling/coefficient-reuse changes and negative SIMD experiments.

**Completed owner-directed checkpoint:** the guarded paired odd-longitude FFT is already in production (commits `c70b19a3`/`a2c5535d`). Its current-source T0–T256 correctness validation and selector-refresh A/B are in [the production validation report](../reports/paired-odd-longitude-fft-production-validation-2026-10-22.md). The full scalar sweep and table-policy evidence are linked from [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md). Plan 14 remains complete; the checkpoint did not authorize an FFT rewrite.

The owner-directed [current-source native scalar sweep](../reports/ducc-full-transform-parity-current-2026-10-22.md) covers exhaustive T0–T256 plus selected sizes through T3072 against pinned DUCC. Reaudited at HEAD `09460dd7`: `src/legendre/scalar_table.rs`, `src/plan.rs`, `src/transform.rs`, and `src/fft.rs` have no changes relative to measured scalar baseline `819d1f44`; later Plan 17 edits add only f32 code and test-only profiling. The forced-route table/recurrence sweep and paired-FFT correctness/A-B reports also cover the requested T0–T256 scope, so these validated matrices were reused rather than rerun. Plan 16's original milestones are complete; the latest owner-directed f64 CC/GL port and spin-1 sequence supersedes the prior execution queue and maintains a strict f32 hold.

**Owner hold:** do not expand or further optimize the existing paired-real-row odd-length FFT route, substitute `realfft`, or add standalone/custom FFT backends without explicit further owner direction. The current validation and full-transform measurement checkpoint do not change Plan 16/17 scope. Keep odd/even sampling as correctness and performance controls. Do not extend Plan 17 into batch APIs or Plan 12's cache-detection machinery.

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
