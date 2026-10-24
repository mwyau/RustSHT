# Plans — minimal execution index

**Startup context:** [AGENTS.md](../AGENTS.md), [one active f64 spin-1 plan](f64-spin1-ducc-reflected-fused-parity-2026-10-23.md), [short status](../PERFORMANCE_STATUS.md), and relevant source. Do not auto-read archives.

- **ACTIVE COMPLETION PLAN:** [f64 DUCC parity, ALL supported spin-0/1 representative cases](f64-ducc-parity-all-cases-2026-10-24.md) — GL/CC/F1, synthesis/analysis, full/reduced mmax, q1/q2/q4/q8. No >2× or near-parity early exit.
- **IMPLEMENTATION HISTORY:** [prior spin-1 adapter plan](f64-spin1-ducc-reflected-fused-parity-2026-10-23.md); already-accepted scalar-ordered recurrence, SIMD scaling and queue improvements remain.
- **IMMEDIATE PRIORITY:** GL T1023 q1 spin-1 synthesis, GL T720 q1/q8 spin-1 analysis, then every remaining slower/unmeasured cell. Maintain per-case pass/gap/unmeasured/inconclusive status.
- **DEFER NONPARITY WORK ONLY:** f32, spin>1 tuning, unrelated FFT/API/batching and figures. S*/A* correctness preserved. Comprehensive parity-matrix benchmark after source-level convergence.
- **REFERENCE CONTRACTS:** [licensing](01-ducc-provenance-and-license.md), [numerics/grids](02-numerics-and-grids.md), [DUCC contract](06-ducc-contract-crosswalk.md).

Historical plans and full pre-adapter instructions/status are available in the [dated archive](../reports/archive/context-2026-10-23/prepared-scalar-adapter/README.md); raw data and past reports remain untouched. Historical documents are **not** execution assignments.
