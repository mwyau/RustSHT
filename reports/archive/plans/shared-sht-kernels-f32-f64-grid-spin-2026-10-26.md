> Historical record only. This plan is superseded by [the active native-generic M6 plan](../../../plans/m6-native-generic-ducc-parity.md). Do not follow its old NEXT instructions.

# Shared SHT kernels: f32/f64, scalar/spin, GL/CC/F1

**Current owner assignment (2026-10-27):** Follow [the NO-SOL cleanup-first resume plan](resume-legendre-convergence-no-sol-2026-10-27.md) and current `AGENTS.md` for execution. This earlier milestone document is **historical context** where it says to start a baseline, rebuild completed components, run old performance campaigns, or use optional Sol. **No Sol model or reviewer under any circumstances in this assignment.** The baseline, old `pyspharm` migration and f32 paired-scalar enabling are already completed; do not repeat them.

**Persistent agent/storage policy (2026-10-26 17:36 UTC):** [Experiment hygiene](../../../docs/EXPERIMENT_HYGIENE.md) and `AGENTS.md` take precedence over historical benchmark build-reuse directions. **Sol is absolutely prohibited for this assignment; no one-time exceptions or quota fallbacks.** Use bounded experiment-owned scratch (prefer `/tmp` when capacity permits); after each test/benchmark experiment, save/verify durable report and required data outside scratch, then immediately delete its compiled artifacts, virtualenvs, caches, temporary worktrees and profiler intermediates. Do not fill `/tmp`; never delete other jobs' files or the only copy of required results. Rebuild for later experiments rather than retaining compiled baseline/DUCC objects.

**Latest clarification (17:13 UTC, supersedes the 17:02 symmetric speed trade-off):** f64 numerical correctness and performance are top priorities; do not sacrifice established f64 whole-transform wins to simplify code. f32 correctness remains mandatory, but accept small f32 regressions for substantial sharing. [B32–B35 recovery and candidate ranking](rejected-shared-kernel-candidate-recovery-2026-10-26.md) is the immediate targeted steering; B36's shared scalar step is already present on `dev` since `c908452a` and must not be treated as rejected. Keep f64-specific SIMD leaves as needed.

**Updated owner steering (2026-10-26 14:51 and 17:02 UTC):** [Final module architecture and implementation steering](shared-sht-final-architecture-steering-2026-10-26.md) takes precedence for module layout, removing the Rust `src/pyspharm` namespace, precise task ordering, lightweight validation between small edits, authorized benchmark subagent usage, and final measured complexity reduction. Preserve existing completed `dev` work; do not restart baselines. The separate Python compatibility distribution remains unless separately retired.

**Maintainability acceptance update (17:02 UTC):** A measured ~1–5% slowdown is acceptable where a shared implementation delivers substantial, demonstrable maintenance simplification, subject to correctness and a cumulative per-cell baseline comparison. See the [updated performance/maintainability policy](shared-sht-final-architecture-steering-2026-10-26.md#new-owner-performance-vs-maintainability-acceptance-policy-2026-10-26-1702-utc); it supersedes the older strict 'no meaningful regression' requirements below. Do not automatically revert the 1–3% slower generic candidate if it genuinely removes duplicate algorithms; do not accept cosmetic wrapping or cumulative slowdowns blindly.

**Owner direction: 2026-10-26.** Reduce duplicated SHT algorithms, dispatch branches, and special-case maintenance by using Rust compile-time generics and limited precision-specialized SIMD implementations. Extend the successful f64 Legendre paths to f32, and share scalar/spin and grid-independent machinery when their mathematics is identical. **Do not sacrifice numerical correctness or demonstrated performance to maximize shared line count.**

**Branch:** `dev`, starting from the exact initialization baseline `062218639bc2f097f004c617da0ce46a3a4d2941`. Implement, commit, and push directly on `dev`; do not modify `main`. Fetch and reconcile `origin/dev` before each push. Preserve work in existing candidate worktrees and never overwrite parallel work. Keep intermediate commits reviewable; experimental build outputs and environments are transient and must be cleaned after saving durable evidence.

**Owner priority on `dev`:** The 2026-10-26 request prioritizes shared kernels across f32/f64, spins, and grids, superseding the older f64-only optimization priority and f32 pause for work required by this refactor. Preserve other numerical, provenance, testing, performance, and safety requirements. Update `AGENTS.md` minimally to reflect this priority.

## Architecture and scope

Current source:

- `src/legendre/mod.rs`: `ShtScalar` trait and f64/f32 implementations; f32 lacks `supports_paired_scalar`, `supports_scalar_ordered`, and reflected-geometry dispatch. These defaults currently block f32 from the order-major/paired SIMD route.
- `src/legendre/scalar.rs` and `scalar_f32.rs`: separate normalized scalar recurrences and scaling.
- `src/legendre/spin.rs` and `spin_f32.rs`: separate native spin recurrences and numerical scaling.
- `src/legendre/scalar_paired.rs` and `scalar_ordered.rs`: optimized f64 scalar degree-pair and reflected/order-major SIMD machinery. Some helpers are inherently `f64`/`Complex64` and use f64 SIMD lanes.
- `src/legendre/spin_fast.rs`: already-generic spin-1 ↔ scalar coefficient transforms; reuse rather than duplicate.
- `src/spin_transform.rs`, `src/transform.rs`: substantial already-generic orchestration, caching, scheduling, and many benchmark-derived dispatch gates.
- `src/grid.rs`, `src/meridional.rs`: grid definitions and already-generic prepared CC/F1 resampling. GL quadrature is direct; CC/F1 analysis needs prepared-grid resampling with a true adjoint.
- `src/fft.rs`, `src/workspace.rs`: existing precision-generic dataflow and scratch. Do not rewrite low-level FFT primitives without evidence.

**Target layers, not a single all-purpose kernel:**

1. **Grid/operator frontends:** typed `ShtPlan<T>` APIs and operator-specific weighting, phase, quadrature, pole handling, and CC/F1 resampling.
2. **Shared geometry and schedule:** ring/order limits, reflected-pair eligibility, active lane/degree ranges, coefficient caches, worker queues, disjoint spectrum writers and scratch ownership. Grid kind should configure these via small immutable descriptors/capabilities rather than redundant hard-coded branches.
3. **Shared coefficient transforms:** one spin-1 E/B → two scalar-order coefficient adapter, scalar ↔ spin adjoint counterpart, and common per-order factor preparation. Scalar/spin-1 use the *same* scalar Legendre recurrence after conversion where valid; direct-spin recurrence remains available when mathematically required or faster.
4. **Shared numerical algorithms:** factor recurrences, basis initialization, degree-pair transitions, ordinary and scaled/fallback phases, analysis accumulation, synthesis accumulation, and adjoint support written once wherever recurrence and floating-point operation ordering are truly equivalent.
5. **Precision-specific SIMD leaf operations:** compile-time `f32` and `f64` Pulp implementations, eight vs four AVX2 lanes, scaling constants, threshold/bit masks, f32 range protection, and validated fallback semantics. Prefer a small sealed/internal scalar/SIMD operations trait with monomorphization and static dispatch. When Pulp's distinct vector types make a single generic loop awkward or slow, a narrowly scoped macro that *generates two concrete kernels from a single body* is acceptable; avoid macro-generated copies that are difficult to audit. **No trait objects or virtual calls in hot loops; no implicit f32→f64 conversion on the native f32 hot path.**
6. **Existing differences remain explicit:** spin-0 vs direct-spin recurrence, spin-2–4 operators, GL direct analysis vs CC/F1 prepared-meridional resampling, adjoint and weighted analysis vs synthesis, polar/underflow/exceptional cases. Shared architecture must never equate distinct mathematical operators.

## Required implementation sequence

### M0 — Freeze exact baselines and coverage

- At start record remote refs, source SHA, compiler versions, CPU flags, pinned DUCC 0.42.0 SHA `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, existing Rust/Python tests, and current selected performance status. Reuse existing benchmark source and provenance; don't add a new framework or retain compiled binaries across experiments.
- Create a small **operator × precision × grid × route inventory** marking `shared`, `precision specialization`, `grid adapter`, `distinct mathematics`, `old/rejected`, and exact call sites. Include public S/A/S*/A*, spin-0, spin-1 STANDARD/GRAD_ONLY/DERIV1, supported higher spins, full/reduced mmax and q1/q8.
- Record current release-build exact-parent outputs, adjoint dot products, extreme/subnormal/pole checks and reused-workspace behavior. Capture f32 warm end-to-end A/S q1/q8 T255/T511/T720/T1023 before changing it: October 22 f32 timings are historical and not interchangeable with current f64 data.
- Baseline selected f64 performance wins that must not regress: GL T1023 full spin-1 synthesis q1, GL over-resolved T320 full q8, T720 reduced q8, T1023 full q8, selected CC T720/T1023 synthesis, T720 analysis q1/q8. Include fast small-T controls.

### M1 — Simplify shared dispatch/geometry/worker logic without changing kernels

- Extract stable internal kernel/geometry capabilities and operator-mode descriptors from `ShtScalar`, `spin_transform.rs`, `transform.rs`; preserve current route choices initially.
- Consolidate ring/order scheduling, cache-size/eligibility calculation, buffer initialization, transactional fallback, and temporary output writer ownership into single internal helpers across scalar and spin cases **only when preconditions and output semantics match**.
- Keep GL/CC/F1 differences at the quadrature/resampling boundary; feed the same prepared-latitude, order-limit and symmetry descriptors into common Legendre workers. Do not force CC/F1 into GL quadrature.
- First milestone must be structurally simpler, produce identical route counters and bitwise results for unaffected paths, and have no meaningful whole-transform timing regression. Commit/push with before/after diff and source size.

### M2 — Shared scalar numerical recurrence for f32/f64

- Port common scalar coefficient factor, normalized recurrence, binary rescaling, and the analysis/synthesis visitor/accumulation structure from `scalar.rs` and `scalar_f32.rs` to a single typed implementation.
- Put f32/f64 representation-specific exponent thresholds, overflow, subnormal, zero/NaN/Inf handling in a small sealed per-precision numerical policy. Do not use f64 thresholds for f32 or weaken finite checks. Avoid changing evaluation order when possible.
- Retain f64 optimized SIMD ordinary/scaled paths as exact performance references and introduce compile-time f32 SIMD siblings from the shared body. Benchmark emitted AVX2 loops, generated code size and instruction counts: the old f64 inlining breakthrough eliminated intrinsic wrapper calls; generic abstractions must not reintroduce them.
- After validation, remove truly redundant old scalar implementations and unused routes, not before. Preserve DUCC GPL attribution and license headers across moved code.

### M3 — Share scalar/spin-1 prepared coefficient and paired/order-major kernels

- Treat spin-1-to-two-scalar conversion as a front-end adapter feeding the common scalar SIMD engine; verify coefficient/phase signs, degree shifts and both Q/U output channels, including GRAD_ONLY/DERIV1 restrictions and adjoints. Do not equate native spin recurrence with scalar recurrence.
- Generalize the current f64-only paired scalar, reflected geometry, and ordered synthesis contracts to f32 using compile-time specialization. Implement the actual typed operations before enabling `ShtScalar` capability gates for f32.
- Preserve valid order-local/global coefficient cache thresholds based on **actual `size_of::<Complex<T>>()`**, worker capacity, active mmax, grid geometry and scratch availability. Never turn on an unsupported branch merely by returning `true` from a trait method.
- Enable f32 order-major synthesis only for a small, measured whitelist of *eligible geometry/work-shapes* initially. Confirm adapter hit/decline, polar fallback, work per degree/lane and exact output reset on decline; compare end-to-end current f32 versus parent and pinned DUCC. Expand only after regression tests.
- Keep direct f32 spin recurrence as numerical fallback and independent baseline. Avoid a permanent f64-conversion bridge as a default implementation; it may be benchmarked as an optional diagnostic/reference if native f32 lags.

### M4 — Share analysis and adjoint kernels, broaden grids

- Reuse scalar recurrence and converted-spin dataflow for analysis where correct; ensure order-major synthesis wins are not misapplied to analysis or to its transposes. Keep direct-spin when appropriate.
- Keep mathematical S, A, S* and A* distinct and pass adjoint identities using DUCC real-harmonic `m>0` factor-of-two convention. Treat weighted analysis and pole-safe adjoint resampling explicitly.
- Extend precision-generic SHT workers from GL to CC/F1 **after preparing or resampling meridional profiles as required**; verify pole reflection/parity, staggered latitude, weights, phase and Nyquist. Share the post-preparation Legendre engine only.
- Where higher spins share scheduling, scaling and geometry, use common plumbing, but keep their distinct ±spin recurrences and coefficients. No unsupported spin>1 shortcut.

### M5 — Eliminate superseded implementations and special cases

- Inventory all dispatch thresholds, benchmark-specific `lmax` equality/range special cases, fallback routes, duplicate helpers, trait methods, temporary buffers, feature/cfg forks and dead kernels. Each removal needs (a) proof of equivalent coverage/correctness, (b) matched benchmark checks on cells it once protected, and (c) unused-code/call-site verification.
- Reduce long `ShtScalar` per-type impl boilerplate by factoring stable default/shared algorithms, not by making the trait object-safe at the cost of hot-path dispatch.
- Keep distinct routines *when numeric algorithms differ or shared implementations regress*; document the one-sentence reason. Eliminate duplicate source only after both f32 and f64 are running through the intended common code.
- Report total source LOC and duplicated kernel implementations removed; new generic/specialization boilerplate, unsafe blocks, routes and condition count; compile-time and binary-size deltas. A refactor that merely relocates 10,000 lines or adds many modes is not success.

## Regression protocol for every vertical slice

**Never perform a repository-wide mechanical rewrite in one untested commit.** Each slice changes at most one algorithm family or one dispatch/storage layer; compile, validate and benchmark, then checkpoint and continue. One vertically complete family may span a small series of commits, but no persistent red CI.

- Numeric: independent low-degree reference + pinned DUCC; spin-0/1 and distinct higher-spin controls; S/A/S*/A*, GRAD_ONLY/DERIV1 where supported; all supported GL/CC/F1; exact/oversampled, poles/equator and odd/even longitude, phase, full/reduced `mmax`, SIMD tails and q1/q8.
- Failure: tiny/huge, subnormal, overflow and nonfinite input, polar prefix, partial adapter decline after output writes, poison and reset of caller-owned scratch, repeated transforms, workspace capacity less than requested threads, nonparallel builds, platform portability.
- Detect accidental route changes with test-only counters. Preserve bitwise parent output for refactor-only changes that maintain arithmetic order; when an accepted algorithm reorder changes bits, use unchanged tight tolerances plus independent reference/adjoints and record the difference.
- Performance: benchmark fresh-process interleaved exact-parent/candidate/pinned-DUCC, q1 and q8 separately, on quiet fixed cores with identical fixtures, flags and warmup. Start 3 pairs for smoke; use 15 or more for acceptance of affected hot cells. Inspect optimized AVX2 assembly + Callgrind when a generic boundary unexpectedly grows calls, spills, branches or runtime. Compare whole-transform timings and memory/RSS, not solely kernel microbenchmarks or instruction reductions.
- Zero tolerance for new correctness failures or unauthorized public API changes. A repeatable regression of only a few percent may be accepted under the owner's documented maintainability-first policy when the simplification is substantial and the cumulative baseline impact is explicit. If neutral within uncertainty and simpler, favor shared code; if meaningfully slower, retain narrowly specialized leaf while sharing everything else. A prior baseline-failing diagnostic is not a new regression, but must be recorded.
- Use repository checks: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, no-default-features checks/tests as relevant, and Python binding tests when code paths/API affect them. Review GitHub Actions Linux, Windows and macOS for final integration. No new runtime dependencies unless justified.

## Process and deliverables

- Main agent implements sequentially. Use the benchmark subagent only for isolated read-only performance work explicitly described in this request. No need to stop after each milestone for approval. Fetch/reconcile current `origin/dev` before every push; **never force-push**.
- Commit and push recoverable, tested changes directly on `dev` at coherent implementation boundaries. Maintain `reports/SHARED_KERNEL_REFACTOR_PROGRESS.md` with source SHAs, routes changed, tests, performance, deletions and remaining duplication. Keep `reports/PROGRESS.md` within its rolling checkpoint limit. Fetch/reconcile current `origin/dev` before every push; **never force-push**. Do not absorb experimental changes blindly or overwrite existing candidate work.
- If a slice is blocked or fails, revert just that slice, preserve exact evidence in `reports/PERFORMANCE_CANDIDATES.md` or the branch ledger and move to an independent family. No unproven fast-path claims.
- Finish with a consolidated architecture description, changed source inventory, quantified duplication/route reduction, complete supported-operator regression matrix, and all accepted numeric/performance comparisons. Push the tested implementation series directly to `dev`. A green but functionally incomplete stage is not the final deliverable.
