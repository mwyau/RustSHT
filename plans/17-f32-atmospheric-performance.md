# Plan 17 — f32 and high-resolution atmospheric SHT performance

**Status: ACTIVE (2026-10-22), Milestone A complete; Milestone B active.** The scalar-analysis cumulative study, [Plan 16 — DUCC spin-1 recurrence parity](16-ducc-spin1-recurrence-parity.md), and the owner-directed scalar Legendre dispatch / paired-FFT / T0–T3072 checkpoint are complete. Milestone A's current f32 baseline is published in [the native benchmark report](../reports/plan17-f32-native-baseline-2026-10-22.md). **Branch:** existing `opt/sht-performance-parity`. **Scope:** current-production CPU scalar spin-0 and vector spin-1 STANDARD f32 performance, with selected f64 high-resolution and scaling controls on GL, CC and F1. Improve measured complete-transform latency and throughput, not an isolated microbenchmark. Plan 17 must not retroactively reopen completed Plans 11, 14, 15 or 16.

## Handoff and stopping discipline

1. The owner-directed scalar dispatch/paired-FFT/full-sweep checkpoint and Plan 16 closeouts are complete. The latest baseline is HEAD `32bb8b4b`; the full f64 scalar sweep is from `819d1f44`, whose scalar/FFT source paths are unchanged at HEAD. Use current HEAD for the f32 baseline, fetch/integrate remote changes safely, and rerun affected checks after any source update. Later explicit owner instructions supersede this queue.
2. Build a **current source** benchmark baseline before any f32 optimization. Do not divide October 15 archival Rust f32 timings by current DUCC or present old numbers as current. Stop investigating any operation whose matched whole-transform results are already competitive or whose remaining gap cannot be attributed.
3. Close each experiment with an accepted improvement or an explained negative result. Continue to the next independently justified milestone rather than waiting for routine approvals. If all hypotheses are closed, conclude Plan 17 rather than inventing work.

## Motivation and evidence, not performance guarantees

- The archived [f32 spin-1 GL T511 screen](../reports/archive/experiments/f32-spin1-current-screen-2026-10-15.md) measured historical Rust warmed q1 synthesis/analysis around 993/1,015 ms and q8 around 147/179 ms, but had **no matched current DUCC** benchmark and predates substantial f64 optimization. Its absolute values are not current baselines.
- The [f32 SIMD analysis report](../reports/archive/experiments/f32-spin1-analysis-simd-2026-10-15.md) found a guarded Pulp analysis gain (approximately 14% q1, 9–10% q8) alongside a possible synthesis regression; do not re-run the same candidate without a new mechanism.
- The [atmospheric-resolution ecTrans study](../reports/archive/experiments/atmospheric-resolution-performance-ectrans-2026-10-16.md) tested T42–T2047, reported recurrence-dominated large operations and field-throughput/workspace limitations. Its older DUCC 0.41.0 Python/native boundaries and shared-host timing are **historical**; use current-source native matched controls to determine which gaps remain.
- The [current scalar f64 GL sweep](../reports/ducc-current-full-sweep-2026-10-21.md) already covers lmax 0–128 plus sampled higher degrees. Do **not** duplicate the exhaustive scalar GL campaign. For f64, test only representative very-high-bandlimit and scaling cases that Plan 16 did not cover, and regressions from newly accepted changes.
- Plan 16 targets **f64 single-field spin-1 recurrence and dataflow**. This successor must not reintroduce its failed SIMD/layout candidates or repeat its q1/q8 T720 matrix without a distinct f32/high-resolution question.

## Milestone A — current f32/native DUCC cross-library baseline and stage evidence

Before changing production kernels, use the existing native harness (or minimally extend it) for pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` and current Rust HEAD. Verify DUCC's actual f32 computation path, conversions, intermediate precision, scaling, normalization and equivalent thread/configuration semantics. Match `S/A` and two E/B channels for spin 1; report normalized numerical errors using appropriate existing f32 thresholds, not f64 expectations.

Initial representative matrix: T127/T255/T511/T720/T1023, scalar spin 0 plus spin 1, GL and a smaller number of relevant exact CC/F1 controls; f32, q1/q8, analysis/synthesis and full/reduced mmax. Use odd/even longitude as regression controls but do **not** optimize odd FFTs. Avoid a Cartesian-product exhaustive grid: screen a small core set, expand only the cases with a material reproducible excess. Include zero, constant/analytic, high-order, polar, near-Nyquist and `phi0 != 0` correctness fixtures.

Profile the *actual* f32 routes: optimized versus scaled recurrence, SIMD lane utilization, direct-spin versus scalar-converted path, E/B accumulation, coefficient transformation, meridional resampling, longitude row FFT, pack/writeback and worker load. Capture matched-stage timing where equivalent; distinguish the f32-specific bottleneck from a residual that is actually common to f64. Record absolute Rust/DUCC latency and gap in milliseconds, not a ratio alone.

## Milestone B — f32 spin-1 recurrence, numeric representation and SIMD

Milestone A established a substantial recurrence/accumulation cost in current
f32 full transforms: the test-only T720 phase probe attributes about 99% of
wall time to that phase, while the matched full-transform ratios range from
about 8× to 179× across the screened cells. DUCC's f32 public API uses double
intermediates in the scalar recurrence. See the baseline report for timings,
uncertainty, numerical errors and scope limitations.

The first tested f32 spin normal-range skip candidate was not promoted: it
improved selected T720 cases but regressed T255/T511 even when its SIMD scan
was gated off. It passed f32 integration, adjoint and extreme-range tests; the
candidate and matched measurements are summarized in the
[experiment report](../reports/plan17-f32-spin-normal-range-2026-10-22.md).

The accepted first Milestone B change marks `spin_f32::recurrence_step`
`#[inline(always)]`. It removes the outlined vector wrapper from the executed
V3 degree loop and reduced matched T255/T720 GL spin-1 A/S q1/q8 warm latency
by 26.6–44.8%, with bitwise-identical Rust outputs. The comparison, assembly
summary and distributions are in the [inlining report](../reports/plan17-f32-spin-recurrence-inline-2026-10-22.md).
Next, separate scalar lane normalization, recurrence, coefficient visitor /
accumulation and scheduling costs; target only the largest measured component.
The follow-up that forced `normalize_pair_lanes` inline was rejected: it
removed two calls per degree but enlarged the V3 caller about 28% and regressed
seven of eight matched full transforms. See the
[negative A/B report](../reports/plan17-f32-spin-normalizer-inline-2026-10-22.md).
The test-only T720 profile counted 15.9–20.3% of normalizer calls with at least
one rescaled lane, but only 0.39–0.55% of lanes rescaled (one exponent step
each). Counter overhead invalidates its timings; this is frequency evidence
only. Next compare a benchmark-only SIMD range precheck with the scalar lane
scan, including whole transforms and fallback controls. Counts are in the
[negative A/B report](../reports/plan17-f32-spin-normalizer-inline-2026-10-22.md).

Only if Milestone A establishes a material f32 recurrence/accumulation gap:
- Compare f32's executed spin recurrence and normal/scaled SIMD pathways with the best **validated** Plan 16 f64 algorithm; determine which structural optimizations can be translated safely and which require f32-specific scaling or precision handling.
- Check reciprocal/sqrt/trigonometric placement, mixed-precision coefficients, unnecessary conversions and intermediate memory traffic against DUCC's actual f32 implementation. Do not assume widening/narrowing is wasteful until correctness/relative costs are measured.
- Compare SIMD degree×latitude useful work, scalar fallback fraction, tail handling and layout, including emitted instruction widths and spills. Use portable Pulp dispatch, not unconditional x86 intrinsics.
- Prefer a narrow adapter to duplicated algorithms. Do not reorganize stable generic code simply to unify source files. Preserve all pre-existing f32 accuracy, exponent/range and finite-input safety guards; no unsafe fast-math shortcuts.
- Run isolated microbenchmarks **and** complete transforms, q1/q8, on each credible candidate. If common kernel changes affect f64, check that f64 does not regress.

## Milestone C — high-bandlimit and thread-scaling behavior

Test a bounded subset of representative T1023/T1279/T2047 f32 and f64 GL/CC/F1 cases, choosing exact/reduced order and even-smooth longitudes to limit FFT confounding. Include q1/q8 and, **only if available on the host**, q16 across both Ryzen 9 5950X L3 domains. Separate cross-CCD cache effects, per-worker scratch/RSS, oversubscription, scheduling wait, memory bandwidth, cold allocation and first-call cost. The earlier report identified a high-resolution recurrence cost and questionable q16 throughput but did not establish causes in a current source matched comparison.

Use a measured scope: start at T1023; expand to T2047 only for an actual unresolved bottleneck and if run duration/memory are reasonable. Avoid unconditional huge repeated full matrices, new calibration frameworks, speculative GPU/MPI or a memory-expensive full-basis table. For q16 do not assume one L3 domain or sum disjoint caches. Report per-field throughput and strong-scaling where meaningful, with absolute total time and peak workspace memory.

## Milestone D — targeted fixes and broad regression matrix

Prioritize the **largest currently measured absolute latency excess** after stage attribution, separately for f32 spin-1 synthesis, f32 spin-1 analysis, f32 scalar and any justified high-bandlimit scaling case. Work on one cause and one candidate at a time. Explicitly distinguish:
- algorithm/dataflow deficiencies,
- numeric range/fallback overhead,
- parallel task partitioning/NUMA-cache locality,
- spectrum/intermediate copy traffic,
- FFT stage limitations, which remain recorded but **not** an odd-FFT optimization invitation.

Preserve public Rust/Python APIs, current table/recurrence dispatch semantics unless a matched alternative is validated, caller-owned workspaces and scratch bounds. Compare cold/warm and process RSS. Validate the accepted changes on small/boundary grids so a T1023 optimization cannot create a T127 regression. Repeat the representative GL/CC/F1, full/reduced mmax, q1/q8 and f32/f64 controls at final source.

## Benchmark, correctness and source discipline

- Pin source/version and compiler flags/hashes for Rust and DUCC; use identical geometry, phase, normalizations, map/component conventions, fixtures, output resets and thread counts. Rust's strict FP versus DUCC's C++ fast-math and intermediate-precision policies are confounders, not permission to lower accuracy.
- Use reusable plans/workspaces, interleaved fresh-process comparisons, q1 one physical CPU and q8 cores in one L3 domain, if supported. Record core assignments and host contention. Calibrate short calls into 20–100 ms timed batches, with at least 30 valid batches across independent runs for final short-call claims. For long calls use repeated individually timed interleaved processes, state sample count/median/p10/p90 and don't claim statistical certainty under host noise.
- Preserve spin-1 STANDARD E/B, GRAD_ONLY, scalar DERIV1, S/A/S*/A*, GL/CC/F1, both precisions, spins 0–4, odd/even longitude controls, normalization, meridional phase/weights, vector tails, high order, polar cutoff, near-zero/underflow, subnormal and extreme finite tests, invalid input handling and poisoned/changed input workspace reuse. Use independent DUCC plus analytic/adjoint validation; don't merely assert roundtrip.
- Keep changes attributable, small and linear. Run release all-feature/no-default suites, formatting, strict Clippy, and Python binding tests only if bindings change. No new dependencies or public API unless the owner explicitly authorizes; no new FFT engine, no odd FFT campaign, no automatic benchmarking at plan construction.
- Commit and push validated changes to **existing** opt/sht-performance-parity. Before every push inspect status, fetch origin, safely rebase only unpublished commits, reread newer owner instructions/AGENTS/plans, verify tests and branch HEAD. Never force-push, reset concurrent work or change main. At most one Luna benchmark-only subagent; no Sol reviewer without owner authorization.
- Keep sanitized benchmark summary, configurations, implementation decisions and a concise final report versioned; keep oversized raw host telemetry/disassembly untracked. Avoid inventing speedup thresholds, fake parity or accepting an isolated result that regresses the real transform.

## Explicit exclusions and next options

Plan 17 is **single-field** and **CPU** focused. [Plan 12](12-cache-informed-legendre-dispatch.md) (cache-informed dispatch) and [Plan 13](13-fused-batched-sht.md) (true fused multi-field transforms) already describe different, larger projects and **remain deferred** until explicitly authorized. Do not duplicate them here or silently activate them. Plan 17 does not authorize SIMD backends requiring new architecture, GPUs, MPI, odd-FFT improvements, or public batching.

## Closeout

Report the current-source native Rust/DUCC f32 parity matrix, accepted f32/high-resolution speedups and their limits, numerical checks, T1023/T2047 and q1/q8/q16 scaling where measured, regressions prevented, and unresolved gaps. Provide representative reproducible scripts/commands and a clear statement of what should be deferred. Close Plan 17 when its measured, supported candidates have been decided; don't continue through irrelevant speculative optimizations merely to occupy agent runtime.
