# General-grid synthesis performance investigation

**Status:** active, evidence gathering. This focused work complements the all-cases f64 spin-0/1 DUCC parity plan; it does not replace that goal.

## Scope and order

Prioritize whole-transform f64 synthesis on general GL and CC geometries. Start from the published `sht_bench` source-matrix results, verify their source/build/workload/timing provenance, then reproduce a bounded subset on the current `dev` source and the exact benchmarked RustSHT revision where available. Keep source-matrix DUCC `31d9fe99fefa5ca29df17b7e6d2362b8c5c4a05f` distinct from pinned parity DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Phases

1. **Audit and reproduce.** Inspect benchmark adapters, output semantics, plan/workspace lifetime, allocation, warmup, timing calibration, thread controls, affinity, validation, source hashes and native build flags. Compare allocating and preallocated transforms fairly. Reproduce f64 GL T128/T320 and CC T180/T360, spin-0/1, q1/q8, for both minimum quadrature (`nlat=lmax+1` GL, `lmax+2` CC) and the published oversampled geometry (`2*lmax` GL, `2*lmax+1` CC). Keep `nlon`, `mmax`, precision, source SHA and thread count explicit. Add only targeted larger controls needed to protect accepted paths.

2. **Locate the work.** Trace current and benchmark-revision dispatch through plan selection, coefficient preparation, scalar ordered/general recurrence, reflected spin-1 routes, spectrum initialization/accumulation, FFT, output materialization and scheduling. Record route eligibility/decline and useful work with temporary diagnostics outside production hot loops. Profile complete transforms with matched Callgrind PCs and optimized disassembly; normalize by equivalent active latitude, degree, channel and output work.

3. **Separate causes.** Attribute recurrence/table work, spectrum clearing and layout, FFT, allocations/materialization, setup, API differences and thread scheduling. Compare native allocating and preallocated calls, then Python adapter time where meaningful. Use q1 and q8 to distinguish single-thread kernel cost from parallel scaling. Report unmeasured components as unresolved.

4. **Choose one bounded change.** Select a source-grounded general mechanism based on plausible whole-transform time and instruction savings. State a falsifiable work-count/PC prediction. Run focused numerical, polar/extreme, reset/fallback and adjoint checks before exact-parent/candidate/pinned-DUCC comparisons. Do not introduce grid-size-specific dispatch, large duplicated kernels, or substantial scratch state for marginal gains.

5. **Validate and extend.** Accept only changes with preserved correctness, repeatable whole-transform improvement on intended cells, no material regression in established f64 spin-0/1 paths, explainable work/instruction effects and acceptable complexity. Use at least 12–15 interleaved comparisons on quiet physical cores for finalists; otherwise label the result preliminary or inconclusive. Run workspace/all-target/all-feature check, strict Clippy, Rust tests and Python tests for accepted changes. Address f32 only after the primary f64 synthesis causes are understood.

## Deliverables and evidence

- Maintain `reports/SYNTHESIS_SOURCE_MATRIX_ROOT_CAUSE.md` with verified benchmark provenance, exact source/build settings, route coverage, disjoint phase attribution, instruction evidence, timing uncertainty and selected/rejected mechanisms.
- Keep `PERFORMANCE_STATUS.md` limited to accepted matched measurements, `reports/PERFORMANCE_CANDIDATES.md` as the recoverable experiment ledger, and the five newest meaningful UTC checkpoints in `reports/PROGRESS.md`.
- Keep raw profiles and temporary instrumentation outside Git unless a small curated artifact is needed to reproduce a finding.
- Do not alter the benchmark harness merely to improve ratios. Document a verified methodology defect and isolate any required harness correction.

## Initial bounded matrix

| Grid | `lmax` | Minimum-quadrature `nlat` | Oversampled `nlat` | `nlon` | Spins | Threads |
|---|---:|---:|---:|---:|---|---|
| GL | 128, 320 | 129, 321 | 256, 640 | explicit from source profile | 0, 1 | 1, 8 |
| CC | 180, 360 | 182, 362 | 361, 721 | explicit from source profile | 0, 1 | 1, 8 |

The source-matrix `nlon` convention and actual values must be read from recorded cases before running. First screen a small subset; do not launch this entire matrix concurrently or on a contended host.
