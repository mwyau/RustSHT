# Active plan — scalar-analysis SIMD execution and code generation

**Branch:** `opt/sht-performance-parity`  
**Updated:** 2026-10-19. **Last reviewed production SHA:** `96ffd14` (fetch current HEAD before working).  
**Numerical reference:** pinned DUCC `80388904b8cfc32400ba84af8a6a45aabfe60f86`, especially `sht_inner_loop.h` `fill_m2a`, `iter_to_ieee`, `calc_map2alm` and `map2alm_kernel`.

## Objective

Make RustSHT's **native f64 scalar GL analysis (A)** computationally competitive with DUCC by identifying and removing **measured differences in equivalent Legendre execution and generated SIMD instructions**. Numerical correctness, robust scaled arithmetic, bounded memory and the public API are requirements, not performance trade-offs.

**Do not use a fixed 10–20% parity threshold, a particular harmonic truncation, a prescribed speedup or a benchmark matrix as a completion criterion.** Report the actual improvement and remaining attributable gap. The goal is a validated explanation and implementation, not a promise of parity.

## Current baseline and established findings

- Production scalar analysis uses active mirrored-ring compaction, order-level scheduling, ordinary SIMD recurrence/accumulation, an integrated mixed/scaled-to-ordinary path, and guarded insignificant-prefix advancement. Keep these rather than starting another port.
- A prior bounded Rust/DUCC comparison showed a **roughly 2× stage deficit**, but measurement boundaries differed. **This is not a current equivalent-work parity result.** Follow-up timings and the `a707780` FMA optimization are Rust-only baseline/candidate screens.
- Optimized assembly at `a707780` showed Rust ordinary coefficient accumulation was using separate `vmulpd`/`vaddpd`; the accepted `mul_add_f64s` change generates FMA like DUCC. This improved representative Rust-only q1 and q8 medians, with overlap/noise and one inconclusive small q8 case.
- Follow-up `96ffd14` moved cosine-square and scaled-correction scratch to reusable per-worker workspace, reduced one inspected Rust frame from 1,088 to 896 bytes, and lowered YMM stack operands from 31 to 16. It also folded approximately 689,643 per-pair lane-validity reductions into approximately 3,384 per-block reductions for T720. These assembly-level improvements **did not establish a whole-call speedup**: the short T720 screen regressed while a smaller q1 case improved. Inspect equivalent function boundaries and actual hot-path costs; a frame-size comparison alone cannot prove spilling.
- Pinned DUCC `map2alm_kernel` in the codegen comparison has no YMM/XMM stack operands, but that narrower function is **not equivalent** to Rust's enclosing initialization/recurrence wrapper. Remaining Rust stack accesses mostly concern checked scaled-transition branches.
- Rust still advances/checks one degree pair at a time, while DUCC's ordinary kernel processes two. A previous naive four-degree unroll, wide packing and several layout experiments regressed. **Do not repeat them unchanged.**
- Earlier profiling found many initially scaled blocks but **no late normal-kernel recurrence failures**. This is real scaled work, not replay of discarded completed degree loops.

Evidence: [latest AVX2 codegen follow-up](../reports/scalar-analysis-codegen-followup-2026-10-19.md), [earlier AVX2 report](../reports/scalar-analysis-avx2-codegen-2026-10-19.md), [scalar-analysis stage profile](../reports/scalar-analysis-stage-profile-2026-10-18.md) and [mixed-path screen](../reports/data/scalar-analysis-mixed-fusion/README.md). The long former handoff is [archived](../reports/archive/performance-status-2026-10-19-pre-cleanup.md).

## Next implementation sequence

1. **Equivalent-work comparison.** Identify the actual Pulp AVX2/FMA monomorphization and pinned DUCC specialization used by their production dispatch. Define paired native kernel timings with identical recurrence inputs, useful latitude/degree work, coefficient conventions and source/ISA settings. Time preparation, normal recurrence, mixed/scaled recurrence and commit separately; include full-call timing as a separate metric. Test representative large and irregular/reduced-order inputs, odd/even longitudes, q1/q8, tails and polar cases. Record timing variance and phase-boundary differences rather than combining incompatible numbers.
2. **Assembly-led diagnosis.** Inspect optimized symbolized disassembly for redundant memory traversals, register spills/reloads, stack accesses, FMA use, horizontal reductions, masks/range checks, branches, inlining and call overhead. Confirm whether remaining scaled-branch memory accesses, lane range checks, staging, or recurrence dependency chains **actually** cost time. The earlier cosine-square scratch move and per-block mask aggregation already landed and did not produce a consistent whole-call win. Optional `llvm-mca` is a model, not a measured result. Do not block on inaccessible `perf` counters or change host permissions.
3. **One bounded implementation at a time.** Prefer a measured structural cost: preparing stable `cos²(theta)` outside repeated hot blocks, lowering unnecessary scratch traffic, reducing expensive safety-check frequency **without weakening safeguards**, or improving accumulator instruction-level parallelism. Inspect the emitted assembly after each change. Retain only repeatable, validated improvements at whole-call and equivalent-kernel boundaries; archive rejected experiments.
4. **Numerical acceptance.** Check pinned DUCC outputs, analytic harmonics, distinct `A` versus `S*` and `A*`, adjoint identities, GL weights, `m=0`, reflection parity, longitude phase, irregular/reduced `mmax`, extremes, subnormals and SIMD tails. Other grids, spins, operators and precisions must not regress; do not generalize this kernel beyond validated regimes. Use stable Rust/Pulp and existing tests.
5. **Decisive closeout.** Report source hashes, compiler/ISA settings, actual code path, before/after q1/q8, changes in assembly, numerical error, accepted/rejected ideas, remaining component times and limitations. If progress plateaus, state the residual bottleneck rather than expanding into speculative unrolling or another broad performance campaign.

## Workflow and constraints

- Work only on `opt/sht-performance-parity`; fetch/reconcile concurrent work before each push. No force push, reset-away, or `main` change. Commit validated milestones; keep raw machine/host/process artifacts untracked.
- Preserve GPL provenance, neutral production names, public APIs, the robust scaled/fallback paths, and one minimal implementation per necessary regime. Stable toolchain and portable SIMD with scalar fallback; no FFT backend rewrite or unrelated dependency.
- At most one Luna benchmark-only subagent and no Sol reviewer unless explicitly authorized. Validate formatting, Clippy, relevant release/all-features/no-default-feature tests; Python tests only if bindings change.
- After this **scalar-analysis** investigation produces either an accepted improvement or a grounded residual, propose a separate goal for spin-1, f32 and other grids. They are **not active scope here**.

## Reference specifications (not separate task lists)

- [Numerics and grids](02-numerics-and-grids.md)
- [DUCC operation crosswalk](06-ducc-contract-crosswalk.md)
- [Source licensing](01-ducc-provenance-and-license.md)

All other prior milestones, ASTRA reviews and superseded performance plans are retained under [historical reports](../reports/archive/README.md) for evidence, not agent instructions.
