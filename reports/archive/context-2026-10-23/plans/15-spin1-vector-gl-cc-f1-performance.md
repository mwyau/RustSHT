# Plan 15 — Spin-1 vector SHT performance on GL, CC and F1

**Follow-on (queued after the current scalar-analysis cumulative-ablation):** [Plan 16 — DUCC-equivalent spin-1 recurrence and dataflow](16-ducc-spin1-recurrence-parity.md). This is a separate new investigation; Plan 15 remains complete and its rejected candidates stay rejected.


**Status: COMPLETE (2026-10-21), closeout report: [`reports/plan15-spin1-synthesis-dataflow-closeout-2026-10-21.md`](../reports/plan15-spin1-synthesis-dataflow-closeout-2026-10-21.md).** The Plan 14 scalar full-transform sweep, four principal figures, validation and closeout remain committed on `opt/sht-performance-parity`. Plan 15 completed its spin-1 GL → CC → F1 sequence in the same Luna session and branch.

**Closeout:** The bounded CC/F1 source comparison and two final A/B candidates are recorded in the linked report. Active-ring packing reduced fallback counts but did not yield a measurable whole-transform gain. Pair-outer SIMD traversal removed repeated coefficient broadcasts but regressed q1 synthesis by 23–27% and full-order q8 synthesis by about 5%, consistent with its per-pair lane-state traffic. Both candidates were removed. Accepted GL, CC and F1 changes remain; the large residual is localized to recurrence/spectrum work, with no additional supported production change. Odd-FFT optimization remains excluded per the owner direction below.

The dated checkpoint sections below preserve the implementation sequence. Their earlier forward-looking “next” statements are historical and superseded by this completed closeout.

**Latest F1 synthesis result (`7f47f06f`, 2026-10-21):** the q8 T720 F1 route has 12 ring groups. Reusing spin-1 scalar-converted coefficient rows across those groups reduced matched full-order Rust median 48.37→42.75 ms; all three paired candidate/baseline ratios were 0.844–0.896. Reduced-order and q1 controls are inconclusive. Candidate output is bitwise equal to baseline, with maximum DUCC relative L2 error `8.09e-13`. The bounded next check is whether the same grid-independent conversion reuse helps q8 CC synthesis; retain cache byte/degree limits and safe fallback. See [report and sanitized data](../reports/plan15-spin1-f1-coefficient-reuse-2026-10-21.md).

**Latest CC synthesis result (`be3c48eb`, 2026-10-21):** caching converted coefficients across the 12 exact-T720 CC groups, while retaining static group scheduling, reduced q8 whole-call Rust medians 15.5% full and 18.1% reduced-order; all three pairs improved. Candidate output was bitwise equal to baseline and DUCC relative L2 was at most `3.64e-13`. q1 synthesis and q8 analysis controls were neutral. The remaining q8 ratios are 6.67× full and 7.98× reduced, so proceed to matched CC/F1 recurrence/spectrum and inverse-output attribution. See [report and sanitized data](../reports/plan15-spin1-cc-coefficient-reuse-2026-10-21.md).

**Latest CC scheduling result (`ae3278a1`, 2026-10-21):** at exact T720 CC, q8 dynamic scheduling of 12 ring groups reduced full-order synthesis from 40.027 to 23.596 ms and `mmax=180` from 21.237 to 17.179 ms; all three process pairs improved. The route is gated by cached-work eligibility and group count greater than worker count. A T511 control improved full order with nine groups but left reduced order on the static route. Results are bitwise equal to the prior Rust implementation and DUCC relative L2 error is at most `3.64e-13`. The inverse/writeback phase remained 1.2 ms full and below 0.2 ms reduced, leaving recurrence and spectrum accumulation as the next investigation. See the [report and sanitized distributions](../reports/plan15-spin1-cc-group-scheduling-2026-10-21.md).

**Phase B checkpoint (2026-10-21):** the q8 GL synthesis path reads cached
spin-1 scalar-conversion rows directly across ring groups, avoiding repeated
worker-buffer copies on the paired route. For T720's 23 groups, the source-level
estimate is 365 MiB of coefficient payload copied per transform (about 730 MiB
of source-read/write traffic before cache effects). Worker rows are still
filled before any checked polar/scaled fallback. Release cached/uncached and
poisoned-workspace reuse with changed coefficients pass at T127 (maximum
absolute channel difference `2.17e-18`), along with T720 adjoint,
general-channel, large-finite and no-default synthesis checks. A final-layout
three-way screen confirmed modest q8 gains (about 2–5%) but still had 2–32%
runqueue wait and multimodal q1 timings; keep this finite-check candidate
provisional.

Commit `ac8e7547` routes each spin-1 synthesis output channel through the
existing row-parallel inverse-FFT helper when the established geometry and
workspace guards permit it. This changes neither the FFT algorithm nor the
recurrence. At T720, the q8 phase probe reduced the two inverse rows from
7.97 to 2.30 ms. Matched whole-call candidate/baseline ratios were 0.933
full-even, 0.719 full-odd and 0.958 reduced-even. Host wait was elevated, so
use the odd case as a control and treat the even ratios as provisional; the q1
control was unstable and is not claimed. Candidate outputs were bitwise equal
to the before binary, reset/repeat checks passed and maximum relative L2 versus
DUCC was `5.23e-13`. The remaining q8 Rust/DUCC ratio is around 6–8×, and the
recurrence/scheduling stage remains 53.7–56.9 ms. See the [focused report](../reports/plan15-spin1-gl-rowfft-2026-10-21.md)
and [machine-readable A/B data](../reports/data/plan15-spin1-gl-rowfft-2026-10-21/rowfft-ab.json).

**Phase B q8 scheduler checkpoint (2026-10-21):** commit `4e62e200` uses a
bounded dynamic queue for one spin-1 scalar-coefficient ring group at a time,
only on the q8 f64 GL synthesis cache route. On an exact-source recheck at
T720, q8 whole-call latency fell 19–39% against parent in full/reduced order
and odd/even controls; the remaining Rust/DUCC ratio is 3.12–6.53x. The
candidate is bitwise equal to parent and has at most `5.23e-13` relative L2
error versus DUCC. Test-only phase timing still places the largest Rust cost
in recurrence plus scheduling; median worker group durations span 21.5–28.6
ms (1.33x max/min). Elevated runqueue wait and candidate spread make the gain
directional. See the [report](../reports/plan15-spin1-gl-dynamic-scheduling-2026-10-21.md)
and [raw timing distributions](../reports/data/plan15-spin1-gl-dynamic-scheduling-2026-10-21/dynamic-scheduling-ab.json).
The q1 T1023 GL synthesis attribution is complete in the
[focused stage profile](../reports/plan15-spin1-gl-q1-t1023-stage-profile-2026-10-21.md):
the whole-call excess is +141.5 ms (2.18x), and the dominant Rust cost is the
order/recurrence stage. A temporary counter split showed the large fallback
bucket is mostly completed staged recurrence, not replayed degree callbacks.
No GL production change is justified by that profile alone. CC's measured
analysis-resampling improvement and q8 synthesis profile are recorded in
[`reports/plan15-spin1-cc-resampling-2026-10-21.md`](../reports/plan15-spin1-cc-resampling-2026-10-21.md).
The remaining CC synthesis cost is in the general order/recurrence stage;
continue to integrated GL/CC/F1 validation and F1 synthesis attribution.

**Phase D F1 analysis checkpoint (2026-10-21):** commit `29e4b126` extends
the same guarded, reusable order-parallel meridional preparation from CC to
F1 spin-1 analysis, preserving F1 phase shifting. At T720/T1023, full and
quarter order, q8 Rust warm medians improved 22–45% across four matched
whole-transform cases; Rust/DUCC moved from 2.38–3.48x to 1.47–2.48x. The
T1023 full-order phase probe reduced median resampling from 91.0 to 38.7 ms.
Numerical/reset comparisons passed with maximum relative L2 `1.30e-12`.
q1 remains serial; its cross-campaign measurements were noisy and are not
claimed as an effect. q8 scheduling wait was nonzero, so the measured benefit
direction is clear while exact magnitude remains host-sensitive. See the
[report and full timing distributions](../reports/plan15-spin1-f1-analysis-resampling-2026-10-21.md).

**Owner direction (2026-10-21): defer odd-length FFT optimization.** Continue to benchmark and validate odd/even longitude cases as **controls**, but do **not** prototype paired-real-row FFTs, swap in `realfft` or the standalone `rustsht-fft` backend, change prime-radix/FFT planners, or start a C2R/odd-FFT optimization investigation. Do not choose an odd-specific FFT issue as a Plan 15 optimization target even when profiling identifies it. Record the residual and continue to measured **spin-1 vector** bottlenecks outside the FFT engine. Existing FFT behavior and numerical safeguards stay intact. This hold remains until an explicit new owner instruction revokes it.

**Goal:** Improve the measured **single-vector-field spin-1** spherical harmonic transforms—STANDARD E/B synthesis and analysis—against native DUCC, for **Gauss–Legendre (GL), Clenshaw–Curtis (CC), and Fejér-1 (F1)** grids, using mathematically identical work and reliable matched full-transform benchmarks. Prioritize absolute residual latency, correct vector results, reusable memory and practical atmospheric resolutions. A documented absence of a justified optimization is a legitimate outcome.

## 1. Explicit scope and exclusions

- **In scope:** existing f64 spin-1 E/B `S/A` numerical kernels, order/latitude scheduling, FFT and spectrum layout, scalar-conversion recurrence, table use when eligible, and grid-specific CC/F1 meridional preparation. Check `S*` and `A*` for correctness and regressions; profile/optimize their implementation only where the same measured hotspot applies.
- **Three distinct optimization targets:** (a) GL spin-1, (b) CC spin-1 including **conditional** prepared-grid resampling, (c) F1 spin-1 including **required** prepared-grid resampling and phase shift. Do not treat CC/F1 as GL plus different latitude weights.
- **Preserve semantics:** two real map channels in DUCC's component order and complex E/B coefficient channels; physical east/north wind component signs and meteorological divergence/vorticity conventions are separate application-level questions. Avoid inventing a new wind API.
- **Keep operational coverage:** STANDARD vector E/B is the performance priority. Preserve spin-1 `GRAD_ONLY`, scalar `DERIV1` (which uses a spin-1 recurrence) and all defined adjoints; any shared-kernel change must test them. They are controls, not extra standalone optimization campaigns.
- **Out of scope:** optimizing scalar spin-0 CC/F1 generally, spins 2–4, batched fields/fused multi-map APIs (Plan 13), cache-hardware autodetection (Plan 12), **all odd-length real-FFT optimization or backend substitution (including paired-real rows and native/custom FFT integration)**, replacing the whole FFT subsystem, new dependencies, public API changes or arbitrary rewrite of an algorithm that has not been profiled.

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
- Full `mmax=lmax` and reduced `mmax=max(1, lmax//4)` for meaningful bandlimits, avoiding duplicate tiny cases; odd-minimum and even FFT-smooth `nphi` **benchmark/validation controls only (no odd-FFT tuning)** at representative points; `phi0` nonzero.
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
2. **Adapter checkpoint:** native Rust/DUCC spin-1 STANDARD workers and the matched runner are implemented. The T4 GL/CC/F1 A/S q1/q8 smoke passed at `6.19e-16` max relative L2. Phase A then completed 170 GL/CC/F1 geometry cases, 680 warm timing rows and 2,040 numerical comparisons with no failures; see [`reports/plan15-spin1-native-baseline-2026-10-21.md`](../reports/plan15-spin1-native-baseline-2026-10-21.md) and its sanitized data.
3. **Temporary owner-priority interruption — complete:** focused scalar GL T64–T256 row-FFT and table/recurrence controls are recorded in [`reports/scalar-t64-256-performance-discontinuities-2026-10-21.md`](../reports/scalar-t64-256-performance-discontinuities-2026-10-21.md). The boundary causes and residual synthesis gap are documented; Plan 15 now resumes at Phase B GL attribution.
4. **Completed:** compare executed CC/F1 synthesis recurrence and spectrum accumulation with DUCC and test the remaining concrete dataflow hypotheses. No new source change was justified; the measured residual and rejected mechanisms are documented in the closeout report.
5. **Stay deferred:** Plan 12 cache-topology heuristics; Plan 13 fused multi-field batching; spin 2–4 performance work; scalar CC/F1 optimization outside vector-shared code; and **all odd-length FFT improvements**, including paired-real rows, `realfft` migration, and standalone `rustsht-fft` integration. Keep odd/even benchmark controls.

Keep working under `opt/sht-performance-parity` unless an explicit later owner instruction changes it. Before every push, inspect the tree and fetch remote changes, preserve all concurrent/unpublished work, rebase **only unpublished** commits when safe, reread updated instructions, rerun affected tests, push normally and verify remote SHA. Never force-push, reset away concurrent work, alter `main`, or silently claim a test passed if it was not run. Use the existing limit of at most one Luna benchmark-only subagent; no Sol reviewer unless specifically authorized.

**Review checkpoint:** The owner can review each GL, CC or F1 milestone after its push without stopping the agent. Continue into the next scoped phase automatically when the previous phase has a validated or well-documented conclusion.
