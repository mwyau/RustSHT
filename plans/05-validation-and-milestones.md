# Active numerical implementation and validation roadmap

**Status:** Active plan for `opt/sht-performance-parity`, revised 2026-10-18. The current objective is a **faithful native Rust port of DUCC's numerical execution algorithms**, followed by evidence-based optimization and integration across the supported SHT operations. The old M0–M6 history, long benchmark records and prior validation evidence have been preserved verbatim in [the historical milestone ledger](05-validation-and-milestones-history.md). They are **not** an active task queue. The historical “M6 — spharmgrid” label is unrelated to the present CPU-parity campaign.

**Read order:** [AGENTS.md](../AGENTS.md) → [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md) → this brief active roadmap. Open [numerical specification](02-numerics-and-grids.md), [DUCC crosswalk](06-ducc-contract-crosswalk.md), [decisions](00-scope-and-decisions.md), and archived evidence only for the task at hand. Do not reread the historical ledger each session.

## Goal and execution principles

- Reproduce DUCC's **whole computational algorithm** and efficient execution architecture, not only its Ishioka recurrence formulas. Work in native stable Rust with `pulp` SIMD, bounded reusable scratch, portable fallback and existing RustFFT. Preserve GPL source attribution.
- Aim for comparable numerical accuracy and CPU performance across **arbitrary supported bandlimits, shapes, precisions and operations**. No fixed trio of T511/T1023/T2047 performance targets. Those remain optional established regression cases, not mandatory daily screens.
- **Implement first.** Make working, tested source increments; use focused correctness tests and small performance screens, and defer exhaustive statistical comparisons until major integration milestones. Avoid a proliferation of benchmark-protocol and report commits.
- Preserve all supported numerical behavior, but **not every superseded implementation**. Keep one minimal, proven robust safety fallback where required and retire genuinely dead/dominated kernels from `src/` into [kernel backups](08-kernel-naming-and-retirement.md). Preserve historical reports/artifacts and archived patches. Do not repeat rejected hemisphere or order-owned prototypes unchanged. Detailed status is indexed in [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md).

## Milestones (in order)

### D1 — DUCC scalar synthesis execution port

**Existing implementation:** The order-major active/reflected-ring scalar kernel in `src/legendre/scalar_ordered.rs` has algorithm-based internal names, SIMD initialization/scaling and parallel harmonic-order scheduling. Continue refining it under [the active cleanup brief](08-kernel-naming-and-retirement.md); do not restart or reintroduce superseded prefix experiments.

Complete and harden the f64 scalar GL Legendre synthesis kernel based on DUCC's pinned `sht_inner_loop.h`, `sht.cc`, associated SIMD and `Ylmgen` structures. Reproduce the **complete pathway**: per-order active-ring gathering; north/south reflection and even/odd parity; `fill_a2m`; `init_lambda`; scaled rescaling and `iter_to_ieee` insignificant-prefix advancement; `calc_alm2map`; unrolled vector `alm2map_kernel`; `extract_a2m`; coefficient transformation and output scatter. Avoid unnecessary allocations, duplicate per-order preparation and callbacks in the hot recurrence loop.

Retain only the smallest validated generic safety fallback needed for exceptional or still-unsupported cases; move provably inferior redundant kernels outside production source after coverage is secured. DUCC-style insignificant-prefix skipping must meet a predeclared accuracy contract; route extreme finite, subnormal and unsafe inputs to a robust implementation. Do not use FFI. Test scalar and SIMD code, reflection, polar lanes, all tails, phase, odd/even longitude and reduced `mmax`. **Extend the working Rust implementation before extensive profiling.**

### D2 — Integrate, identify and close the gap

Integrate the new kernel into private `ShtPlan` dispatch without changing public APIs. Compare **equivalent Legendre-stage work** against the pinned DUCC kernel and the existing Rust implementation, then end-to-end transforms. Attribute remaining differences to degree work, ring packing, SIMD/unrolling/assembly, cache and scheduling before tuning. Implement a true general-purpose single-thread path first, then order/latitude parallelism. Do not retry the previously rejected order-owned or north/south prototypes without eliminating their recorded overhead.

### D3 — Port the other DUCC operator paths

Use the faithful scalar architecture where mathematically appropriate, without forcing unsuitable reuse: scalar **analysis**, spin-1 **synthesis and analysis**, derivative and appropriate adjoint paths; then efficient **native f32** and eligible spin-2 pathways. Preserve spin-3/4 behavior. Validate GL first; share the kernels with CC/F1 through their existing prepared/resampling operators and preserve distinct exact-analysis rules. Maintain numerically robust fallbacks.

### D4 — Acceptance and performance parity

Run matched native benchmark panels for spin 0/1, S/A, f64/f32, q1/q8 and applicable grids after integrated milestones. Include low, middle, high and irregular truncations, differing longitude factorization, `mmax<lmax`, oversampled rings and field reuse as applicable. Include **DUCC reference version, full-operation timings, accuracy, memory and regression checks**. Compare machine code if arithmetic work is equivalent yet time differs. Publish achieved ranges and outstanding gaps; do not declare parity from isolated microbenchmarks.

## Validation contract (applies at every milestone)

- Numerical comparison must include a pinned DUCC source/package **and** an independent mathematical check. Verify analytic scalar harmonics, spin E/B signs, grid/weight geometry, derivative identities and distinct S/S*/A/A* adjoint identities using correct real harmonic inner products.
- Test zero/sectoral/near-sectoral coefficients, `m=0`, `mmax<lmax`, `l<spin`, odd/even `nphi`, nonzero `phi0`, polar and equatorial rings, SIMD tails and exceptional/very large finite values. Guard output integrity if a fast route declines.
- Preserve GL, CC, F1 sampling constraints, mode contracts, f32/f64 precision, serial/parallel behavior and existing spin 0–4 features. Find exact formulas and exceptions in [02](02-numerics-and-grids.md) and [06](06-ducc-contract-crosswalk.md); do not weaken tolerances retroactively.
- Small/medium regression tests run regularly; opt-in high-bandlimit tests stay available. Historical T511/T1023/T2047 cases remain available, but are **not** universal milestones, release promises or preferred optimization points. Test a range of odd and intermediate degrees.
- Format, Clippy and focused tests on each substantive increment; full suites periodically. Python tests for binding/API changes. Pin the DUCC revision when comparing and record actual grid/dtype/spin/operator/worker coverage.

## Handoff and documentation

Update [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md) **briefly** after meaningful code milestones, with the new architecture, verified checks, representative timing evidence, outstanding gap and next implementation task. Archive detailed reproducibility data under `reports/` without deleting prior work. Historical review [ASTRA_INTEGRATED_REVIEW.md](../ASTRA_INTEGRATED_REVIEW.md) and the [old ledger](05-validation-and-milestones-history.md) are evidence, not instructions.

Current development authorizations: branch `opt/sht-performance-parity`, commit/push validated changes there, fast-forward-only synchronization with remote before push, **at most one Luna benchmarking subagent, no Sol reviewer**, and no changes to `main` without explicit owner permission.
