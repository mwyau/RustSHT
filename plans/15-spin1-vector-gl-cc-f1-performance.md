# Plan 15 — Spin-1 vector SHT performance on GL, CC and F1

**Status: ACTIVE (2026-10-21), after Plan 14 closeout `f2d9ce47fdf04946ef5cbe478a30b0fc38956f8e`. Owner-authorized.**
The Plan 14 scalar full-transform sweep, four principal figures, validation and closeout are committed and pushed on `opt/sht-performance-parity`. Continue this plan in the same Luna session and branch without another owner prompt. The handoff preserves the scalar baseline and does not expand Plan 15 beyond its stated spin-1 scope.

**Goal:** Improve the measured **single-vector-field spin-1** spherical harmonic transforms—STANDARD E/B synthesis and analysis—against native DUCC, for **Gauss–Legendre (GL), Clenshaw–Curtis (CC), and Fejér-1 (F1)** grids, using mathematically identical work and reliable matched full-transform benchmarks. Prioritize absolute residual latency, correct vector results, reusable memory and practical atmospheric resolutions. A documented absence of a justified optimization is a legitimate outcome.

## 1. Explicit scope and exclusions

- **In scope:** existing f64 spin-1 E/B `S/A` numerical kernels, order/latitude scheduling, FFT and spectrum layout, scalar-conversion recurrence, table use when eligible, and grid-specific CC/F1 meridional preparation. Check `S*` and `A*` for correctness and regressions; profile/optimize their implementation only where the same measured hotspot applies.
- **Three distinct optimization targets:** (a) GL spin-1, (b) CC spin-1 including **conditional** prepared-grid resampling, (c) F1 spin-1 including **required** prepared-grid resampling and phase shift. Do not treat CC/F1 as GL plus different latitude weights.
- **Preserve semantics:** two real map channels in DUCC's component order and complex E/B coefficient channels; physical east/north wind component signs and meteorological divergence/vorticity conventions are separate application-level questions. Avoid inventing a new wind API.
- **Keep operational coverage:** STANDARD vector E/B is the performance priority. Preserve spin-1 `GRAD_ONLY`, scalar `DERIV1` (which uses a spin-1 recurrence) and all defined adjoints; any shared-kernel change must test them. They are controls, not extra standalone optimization campaigns.
- **Out of scope:** optimizing scalar spin-0 CC/F1 generally, spins 2–4, batched fields/fused multi-map APIs (Plan 13), cache-hardware autodetection (Plan 12), replacing the whole FFT subsystem, new dependencies, public API changes or arbitrary rewrite of an algorithm that has not been profiled.

Relevant implemented paths: `src/spin_transform.rs`, `src/meridional.rs`, `src/legendre/`, `src/fft.rs`, `src/grid.rs`, `src/plan.rs`, `src/workspace.rs`, Python `ShtPlan(spin=1)` and its DUCC comparison tests.

## 2. Historical evidence to inspect, not blindly repeat

Read only the relevant historical reports before choosing experiments:

- `reports/archive/experiments/spin1-direct-vs-scalar-conversion-2026-10-14.md`: f64 GL spin-1 scalar-conversion was dramatically faster than the general spin recurrence; the old direct/fused general recurrence prototypes were slower. **Do not resurrect them** without a different measured mechanism.
- `reports/archive/experiments/fused-spin1-analysis-2026-10-15.md`: checked f64 GL q1 fused analysis was accepted; an ungated q8/q16 version had no demonstrated advantage. Preserve the staged-output/fallback safety.
- `reports/archive/experiments/spin1-hemisphere-pair-synthesis-2026-10-14.md`: narrow GL mirrored-latitude optimization applies only in its measured eligible band/thread/layout; it is not an automatic CC/F1 strategy.
- `reports/archive/experiments/spin1-analysis-ring-scale-screen-2026-10-18.md`: q1 shared ring-scale preparation helped modestly; q8 gains were inconclusive.
- `reports/archive/experiments/spin1-analysis-ducc-refresh-2026-10-15.md`: T511 q1/q8 results showed a potential spin-1 deficit, **but compared native Rust to DUCC's Python entrypoint**, with changing host/load and source revisions. Treat these ratios as leads only, never as a current matched parity baseline.
- Inspect any newer accepted spin-1 commits/reports before work. The latest Plan 14 scalar GL improvements are **not automatically spin-1 improvements**.

Cross-check the mathematical contracts in `plans/02-numerics-and-grids.md`, `plans/06-ducc-contract-crosswalk.md`, and the pinned DUCC source. Preserve the dual-channel normalization, parity, negative-order reality, polar cutoffs, scaling and finite-input fallback.

## 3. Phase A — Freeze source and establish native, comparable spin-1 baselines

**Do not begin until Plan 14 and its requested scalar sweep have finished and pushed.** Record the exact Rust source SHA, pinned DUCC 0.42.0 SHA (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` unless superseded by an explicit owner decision), Rust/GCC flags, runtime SIMD, topology/affinity and benchmark program revisions. Compare native RustSHT calls to native DUCC C++ spin-1 `analysis_2d` and `synthesis_2d`, not Rust against Python overhead.

Extend a suitable existing `m1b_bench` or matched full-transform benchmark adapter only as necessary. Reuse tested fixture generation, batching, pinned cores, outputs and validation code. A spin-1 benchmark must time **two input/output channels** and the **entire E/B triangle** on both sides, with identical map, normalization, phase and coefficient layout; exclude adapter-only conversions and allocations or time them equivalently. Record full end-to-end warm latency and cold/setup separately.

**Representative initial screen for each of GL, CC and F1:**
- T1, T4, T16, T31/32, T63/64/65, T127/128, T159, T255/256, T511, T720, T1023; include T2047 if resources permit, clearly documenting any unmeasured case. Expand around verified discontinuities rather than repeating every scalar Plan 14 configuration.
- Full `mmax=lmax` and reduced `mmax=max(1, lmax//4)` for meaningful bandlimits, avoiding duplicate tiny cases; odd-minimum and even FFT-smooth `nphi` controls at representative points; `phi0` nonzero.
- **Exact and over-resolved latitude grids** with correct distinct minimum exact analysis limits: GL `ntheta >= lmax+1`, CC `ntheta >= lmax+2`, F1 `ntheta >= lmax+1`; include CC just below and just above its **resampling-skip threshold**, without confusing that with the exact-analysis minimum. F1 retains its required phase-shift/resampling path.
- q1 and q8 on matched physical CPUs (and q4/q16 only if capacity/scheduling evidence justifies them). Both STANDARD `A` and `S`, with selected `S*`/`A*`, `GRAD_ONLY` and `DERIV1` regression controls.

Use **three interleaved process pairs**, at least **30 calibrated timed batches per short case** targeted to 20–100 ms per batch, and individually timed long calls over multiple interleaved processes, reporting actual counts, p10/median/p90 and host wait/CPU observations. Validate each case numerically outside the timed region. Require reproducible Rust/DUCC median ratios and **absolute excess milliseconds**, not a universal percentage target. Separate setup, first call, warm call and memory costs. If contention is high, repeat only meaningful ambiguous cases on the same pinned cores. Never present process medians merged as raw batch samples.

Deliver an initial machine-readable `reports/data/plan15-spin1-.../` baseline and a focused report. The baseline must identify *which* grid, operation, order range and thread count actually has the greatest confirmed excess, and whether the difference occurs in CPU work, temporary allocation, scheduling or execution layout.

## 4. Phase B — GL spin-1: current-path diagnosis and bounded improvements

Measure equivalent stages on **executed** optimized paths:

1. Two longitude FFT channels, phase correction, normalization and spectrum layout;
2. E/B ↔ paired-scalar coefficient conversion, any precomputation/weighting and scratch traffic;
3. spin-1 Legendre recurrence/table accumulation, reflected-latitude traversal, SIMD and polar-exclusion behavior;
4. output synthesis packing/inverse FFT, task scheduling and worker scratch reuse.

Use matched DUCC `map2leg/leg2alm` and `alm2leg/leg2map` stages **only when boundaries match**, documenting any mismatch; inspect exact generated SIMD/assembly where necessary. Distinguish serial q1 computational inefficiency from q8 load imbalance, parallel barriers and memory bandwidth. Do not assume a scalar Plan 14 direct-packing trick translates to vector work.

Choose one *highest absolute-time* improvement supported by profiling. Prototype and A/B benchmark against a clean baseline, then either promote with complete correctness evidence or reject/document. Preserve safe scalar-conversion fallback and validated spin-1 optimized gates; do not switch to the already disproven general spin recurrence.

## 5. Phase C — CC spin-1: isolate and optimize conditional meridional processing

Benchmark **CC separately from GL**, at exact and over-resolved latitude counts. Break total time into longitude FFTs, spin-specific channel preparation, per-order meridional `CCResamplingPlan::to_prepared` / `restore_original` as applicable, prepared-latitude Legendre stage, and final layout/FFT. Account for two spin channels.

The existing path uses reflected meridional profiles, spin-dependent parity, quadrature weights, preparation to a CC latitude grid and its exact adjoint. Determine from instrumentation whether:
- the resampling work, FFT shape/planning, repeated zeroing or scatter/copy dominates;
- the prepared grid is larger than necessary for the requested `lmax` or `mmax`;
- repeated E/B channel processing offers a safe shared-data/blocked traversal;
- the **existing** CC resampling-skip path at sufficiently oversampled `ntheta` is actually selected and beneficial.

Compare with DUCC's executed CC path and confirm that the reference and Rust use the same resampling contract. No replacing exact analysis with naive CC quadrature; no phase/parity simplification without analytic and adjoint proofs. Optimize only an identified **CC vector** bottleneck, preserving GL performance and the CC poles.

## 6. Phase D — F1 spin-1: isolate mandatory resampling and half-grid phase

F1 has no pole sample; unlike CC, its meridional preparation includes an extra phase shift and **always** requires the prepared-grid resampling branch in current code. Profile the two spin channels through reflection, FFTs, phase multiplication, padding/weights, prepared Legendre stage and inverse/adjoint path. Measure the difference from CC at comparable effective bandlimit, but never claim equivalent geometries or grid weights.

Investigate whether redundant per-channel FFT/planning work, repeated phase generation, memory transpose, or bounded worker scratch is the largest F1-specific cost. Reuse improvements genuinely shared with CC only if matched F1 A/B evidence supports them. Preserve F1's half-sample offset, spin parity, poles-excluded behavior, normalization and fallback. Any change to shared `src/meridional.rs` must rerun CC and scalar regression controls.

## 7. Phase E — Validation, report, plots and completion

For each accepted code change, require: independent analytic vector E/B basis, pure E/pure B and mixed fields; northern/southern reflected pairs, equatorial singleton and near-polar lanes; forbidden degrees below spin; high `m` and Nyquist handling; exact/over-resolved grids; odd/even `nphi`; nonzero `phi0`; zero, subnormal, near-cancelling, large finite and rejected/nonfinite inputs; SIMD tails; repeated poisoned-workspace reuse; safe fallback after partial vector writes; numerical S/A/S*/A* identities with the proper **real-harmonic inner product** (factor 2 for `m>0`), and no uncontrolled cross-channel contamination.

Check STANDARD, `GRAD_ONLY`, `DERIV1`, all defined adjoints, f32 spin-1, spin 0 and spins 2–4 as regressions, and both CC/F1 preparation/adjoint when modifying shared operators. Test `cargo test --workspace --all-features --release`, release no-default-features, fmt, strict Clippy and Python tests only when touched or at final cross-language handoff. Preserve numerical tolerance; do not loosen it to accept performance changes.

Produce a compact side-by-side matrix by grid × operation × q1/q8 × full/reduced `mmax`, current-source versus DUCC ratios, absolute excess, confidence/spread, correctness and peak workspace/setup costs. Plot **four comparable SVG ratios for each grid** (analysis/synthesis × full/reduced, with q1/q8 curves): 12 plots total, only actual measured even-smooth points; maintain odd-sampling data separately. An additional chart may distinguish CC resampling-active versus skipped, and F1's mandatory preparation. Link all figures from one report and `reports/README.md`; do not force missing samples or misleading interpolations.

A gap is **closed** if correct matched whole-call results no longer justify further optimization, or the residual is isolated and all tested candidate mechanisms are unhelpful. Never impose arbitrary percent-parity criteria or restart broad micro-optimization campaigns. Record rejected candidates, codegen/latency evidence, exact source/toolchain, portable constraints, numerical checks, reproduction and remaining spin-1 opportunities. Remove temporary flags/prototypes, commit/push validated milestones and finalize Plan 15.

## 8. Sequence, concurrent work and branch discipline

1. **Complete:** Plan 14's full scalar sweep, four figures and closeout at `f2d9ce47fdf04946ef5cbe478a30b0fc38956f8e`; Plan 15 activation was pushed at `341661a0256ecab329e5de65ad286f96a8a73914`.
2. **Adapter checkpoint:** native Rust/DUCC spin-1 STANDARD workers and the matched runner are implemented. The T4 GL/CC/F1 A/S q1/q8 smoke passed at `6.19e-16` max relative L2. The full Plan 15 baseline matrix remains pending.
3. **Temporary owner-priority interruption:** before the broader spin-1 baseline, investigate the scalar GL T64–T256 gaps using existing sweep evidence and focused row-FFT/dispatch diagnostics. Do not reopen Plan 14. Resume the Plan 15 GL → CC → F1 baseline sequence after that bounded diagnostic.
4. **Then:** matched native spin-1 GL/CC/F1 baseline; targeted GL optimization; targeted CC optimization; targeted F1 optimization; integrated cross-grid rebenchmark/validation; completion documentation.
4. **Stay deferred:** Plan 12 cache-topology heuristics; Plan 13 fused multi-field batching; spin 2–4 performance work; scalar CC/F1 optimization outside vector-shared code; and any independent odd-FFT rewrite.

Keep working under `opt/sht-performance-parity` unless an explicit later owner instruction changes it. Before every push, inspect the tree and fetch remote changes, preserve all concurrent/unpublished work, rebase **only unpublished** commits when safe, reread updated instructions, rerun affected tests, push normally and verify remote SHA. Never force-push, reset away concurrent work, alter `main`, or silently claim a test passed if it was not run. Use the existing limit of at most one Luna benchmark-only subagent; no Sol reviewer unless specifically authorized.

**Review checkpoint:** The owner can review each GL, CC or F1 milestone after its push without stopping the agent. Continue into the next scoped phase automatically when the previous phase has a validated or well-documented conclusion.
