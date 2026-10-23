# ACTIVE owner plan — f64 spin-1 parity via order-major fused reflected pairs

**Status:** ACTIVE, 2026-10-23; branch `opt/sht-performance-parity`. **Implementer:** Luna Max. **Target:** reach or exceed pinned native DUCC 0.42.0 f64 spin-1 STANDARD synthesis and analysis on representative GL/CC/F1 full- and reduced-order workloads. Preserve successful f64 scalar CC/F1 algorithms and their near-parity; fix material scalar outliers later without postponing the spin-1 breakthrough. **Do not resume f32.**

This owner directive **supersedes the execution order** in [the earlier scalar CC/F1 performance-first plan](f64-scalar-cc-f1-ducc-parity-implementation-2026-10-23.md) and benchmark-first plans, but does not discard their mathematical constraints, evidence or validated code. No benchmark campaign, GL table threshold study, extra figure series, archival cleanup or final sweep completion is a prerequisite for a new production kernel candidate.

## Verified state and what is not proven

Checked branch head at plan creation: `3b1d546a2b75f0a81c60efa1b190091ba596bd90`. Re-fetch and read current branch before implementing. The recent accepted [spin-1 coefficient fusion](../reports/spin1-paired-coefficient-preparation-2026-10-23.md) and [checked cross-block reuse](../reports/spin1-checked-pair-preparation-reuse-2026-10-23.md) reduced exact CC T720 full-order q8 synthesis to **14.938 ms Rust vs 6.123 ms DUCC, 2.44×**. q1 full `91.566/43.623 ms`, reduced q1 `42.434/18.489 ms` and reduced q8 `6.900/2.651 ms` are also slower. Those are genuine residuals after setup optimization; the recurrence and scaled fallback are the new targets.

Scalar CC/F1 is **not unconditionally ≤1.0×**: selected full-order scalar CC T720 q1 synthesis `1.043×` and q8 `1.014×`; T1023 q8 `1.018×`; F1 T720 q1 `1.011×` and q8 approximately `1.03×`. Scalar CC T720 quarter analysis q1 `0.732×` and selected scalar F1 analyses `0.57–0.75×` are faster. Measurements are mixed-source focused runs with some contention qualifications; no current all-case matrix establishes blanket parity. Since the large scalar deficits were removed, do **not** restart broad scalar benchmarking before attacking 2.44× spin-1.

## DUCC algorithm that actually executes

Reference DUCC `mreineck/ducc`, version 0.42.0, SHA `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`:

- `src/ducc0/sht/sht.cc:190` `make_ringdata` pairs north/south reflected rings and sorts by sine; `:649` `alm2leg` chooses STANDARD spin-1 scalar-converted execution when `lmax>=500` and `abs(sin(theta))>=1e-4`; near poles use direct spin. `:473` `spin1to0` converts both E/B rows through degree `lmax+1` once per order; `:705–717` calls `inner_loop_a2m` twice, one per scalar channel, over **reflected** pairs.
- `src/ducc0/sht/sht_inner_loop.h:1383` `fill_a2m` selects active reflected pairs, pads SIMD lanes and initializes parity accumulators; `:426` `iter_to_ieee` advances scaled recurrence; `:459` `alm2map_kernel` executes ordinary recurrence; `:1425` `extract_a2m` combines even/odd parity only after degree loop and reconstructs both hemisphere outputs. `sht.cc:851` has the corresponding spin-1 analysis, with scalar passes and `spin0to1`.
- Cross-reference [Sol's executable-path, assembly and fallback review](../reports/f64-ducc-independent-review-2026-10-22.md). DUCC uses approximate scaling policies; **do not adopt them by weakening RustSHT's stricter exceptional-value safety.**

Rust already implements `spin1_to_scalar`, the order-major atomic queue (`src/spin_transform.rs:1645`), checked dual-channel preparation, scaled-transition visitors (`src/spin_transform.rs:2730+`), and `FusedHemispherePairBlock` (`src/legendre/scalar_paired.rs:5410+`). But T720/T1023 q8 ordinary spin-1 generally evaluates separate physical hemispheres; the existing paired hemisphere dispatch (`src/spin_transform.rs:55` and `src/spin_transform/hemisphere.rs`) is gated to **GL T319–511, full order, q4–q8**. The general `FusedPreparedSynthesisPairBlock` (`src/legendre/scalar_paired.rs:5258`) shares two channel recurrence but works on physical latitude blocks rather than globally active reflected pairs.

Sol's source/assembly work estimates around **20 vector FMAs per degree pair for two physical-hemisphere shared streams** versus **12 for DUCC's two scalar-channel reflected passes**. This is an *arithmetic hypothesis*; not a measured speedup. q8 worker occupancy was already eight; the bottleneck is not a four-worker scheduling problem.

## Experiments already rejected: do not repeat

The [2026-10-23 rejected reflected-row report](../reports/spin1-reflected-scalar-row-adapter-negative-2026-10-23.md) contains TWO negative experiments:

1. Route each converted scalar channel **separately** through the scalar ordered reflected recurrence, with polar fallback. T720 q8 GL/CC/F1 each remained ~1.00× the parent; the initial whole-sphere and quarter-order variants violated numerical tolerances.
2. A follow-up **two independent ordered scalar channels** took CC T720 q8 synthesis from `15.056 ms` to `24.029 ms` (1.596× slower) despite passing numerics. This is decisive: simply invoking the scalar engine twice is **not** a productive port.

An adjacent F1 reflected-dispatch trial took T720 q8 from `14.367 ms` to `51.569 ms` and was removed. Earlier Plan 16 parity-only separation and one-channel all-ring variants also lost; Plan 15 loop reversal/compaction and transform-wide cache variants were rejected. Do not broaden an old hemisphere-eligibility switch or reapply two separate scalar transforms.

**The new experiment must share one recurrence state across both converted components and across both reflected hemispheres**, under existing checked numerical preflight. It must not add new transform-wide coefficients or a second whole-spectrum pass.

## M1 — FIRST production breakthrough candidate: fused reflected two-channel per-order engine

### 1.1 Integrate into existing order-major work ownership

Retain `synthesize_spin1_order_major_queue` (`src/spin_transform.rs`) and its per-order worker-owned `q_row/u_row` and unique-order spectrum scatter. Implement a **private, checked per-order helper** for spin-1 STANDARD f64 whose input is one converted E/B order and existing prepared recurrence coefficients/factors. Do not introduce a competing whole-transform scheduler or round-trip via the public scalar SHT. Keep the accepted fused coefficient preparation and cross-block reuse.

### 1.2 Pack globally active mirrored latitude pairs

For each order `m`, identify reflected north/south ring indices and their active limits using the validated `theta_sin/theta_cos`, `ring_order_limits` and actual grid geometry:

- Pack **active mirrored pairs** in 64-lane (or existing SIMD-block) groups, with exact ring indices retained for scatter. Pack by actual order eligibility; do not reject an entire block because a few pairs have polar/scale exceptions. Match DUCC's `make_ringdata`/`fill_a2m` active-pair idea, including CC endpoints, F1 pole exclusion, even/odd `ntheta` and a singleton equator. Avoid transform-size per-order allocation; use bounded reusable worker scratch.
- Separate into **ordinary eligible** (proven finite/nonzero seed and coefficients), **checked scaled-transition candidates**, and **original spin direct/polar or unsafe-conversion fallback**. DUCC uses `sin>=1e-4` to choose conversion; Rust's ordinary fused path requires `sin>=0.01`. Those are distinct numerical eligibility boundaries; do not change the latter just because DUCC accepts the former. Do not compact into a large duplicated staging buffer when indices alone suffice.

### 1.3 Implement ONE fused recurrence over reflected pairs

Adapt the existing `FusedHemispherePairBlock` mathematics within an **order-major** kernel and reuse the `FusedPreparedSynthesisPairBlock` checked/normalization mechanisms:

- One sectoral seed and **one** recurrence state per northern-latitude SIMD lane, not separate north/south recurrences and not two independent first/second scalar SHTs.
- Update both converted complex channels using the same recurrence; accumulate **even and odd parity** for each channel and real/imag parts, then reconstruct north/south outputs from `even ± odd` *only after accumulation*. Apply `1/sin(theta)`, phase and Nyquist convention exactly once in the correct existing stage.
- Explicitly address eight real/imag parity accumulators and AVX2 register pressure. First measure a small direct fused variant and inspect actual spills/register traffic. If spilled, try a **bounded degree-tile or safe accumulator staging strategy** that reuses recurrence state, or reorder operations to shorten live ranges. Do **not** split into a second full recurrence over the same latitudes; an extra load/store tradeoff needs a targeted measurement, not guesswork.
- Run checked seed/coeff/sum preflight and protect exceptional-value classification. If a late scale or product check fails, discard this **order's staged output** and recompute through the validated existing original/checked fallback; never keep partially accumulated hemisphere contributions.
- Reconstruct active mirrored rings with proper direction/phase and validate no duplicates or omissions, especially polar/CC poles and F1 offset. Preserve order-major output ownership and parallel worker scratch.

**This differs materially from the rejected adapters:** those performed two scalar recurrences or retained per-hemisphere work. The candidate shares recurrence across **both channels and both hemispheres** while preserving the successful order-major ownership and bounded scratch.

### 1.4 Do not profile indefinitely

Before implementing, at most one **small untimed** T720 full/reduced route-count check may quantify ordinary-success, scaled-success, early/late decline and scalar visitor *weighted by lane×degree*. The point is to choose where to attach M1, not to launch another long benchmark campaign. Existing Sol diagnostics already show substantial fallback worker-sums. If the new shared fused path only handles a small fraction of work, prioritize M2 scaled handling early.

The **next substantive implementation commit must modify actual production `src/` code and contain tests for the new fused work path or a concrete numerical blocker**. No another report-only or coefficient-cache-only cycle.

## M2 — Scaled-prefix integration and fallback: eliminate repeat work

After the first ordinary fused reflected-pair candidate, address the important **scaled/polar** work, not just setup:

- Adapt Rust's existing `StagedSynthesisPairTransition` (`src/legendre/scalar_paired.rs:953+`) so scaled prefixes on active reflected pairs can enter the **same shared two-channel SIMD recurrence** once safe. Reuse DUCC's `iter_to_ieee` concept, but preserve Rust range checks, normal/subnormal distinctions and fallback semantics. Avoid first fully computing a failed block and then redoing its entire range with a scalar degree visitor.
- Treat `sin≈1e-4..0.01` as a **candidate region for a checked shared scale→ordinary route**, never automatically as safe. Keep near-polar original spin recurrence for truly unsupported conversion/ring limits. Count how often each fallback is actually taken in *one short diagnostic*, and keep fallback cost proportional to exceptional rings rather than entire 64-ring groups.
- No relaxed tolerance, approximate zeroing of significant finite terms, loss of m=0 or phase, or changed GRAD_ONLY/DERIV1/spin 2–4 behavior. Stage and commit successful branches atomically; a failed conversion reverts to original E/B original spin calculation.

## M3 — Spin-1 analysis after a successful synthesis mechanism

Sol's T1023 CC/F1 q8 stage report measured **Rust recurrence/accumulation ~51.7/52.1 ms**, while Rust CC/F1 prepared-grid resampling ~16.2/16.4 ms was already similar to DUCC's ~17.1/18.0 ms. Thus analyze `src/spin_transform.rs::analyze_spin_scalar_fast_order` and GL-only reflected-pair analysis route, DUCC `leg2alm_internal` / `inner_loop_m2a` / `spin0to1`.

Implement a matched reflected-pair analysis prototype **only if the recurrence phase remains the dominant measured excess**. Two converted channels, parity-weighted north/south combinations, one shared scalar Legendre recurrence and `spin0to1` transpose must remain mathematically correct; check S/A adjoints, CC/F1 prepared sampling, q1/q8, full/reduced and low-order. Do not optimize the already competitive spin-1 resampler or FFT.

## M4 — Tiny performance decision matrix and remaining scalar outliers

For a quick early directional smoke: T720 CC full and quarter spin-1 STANDARD synthesis at q1/q8, one matching F1 T720 full q8 and GL T720 full q8; optionally T1023 CC full q8 as a held-out check **after a gain**. Compare old Rust/candidate Rust/pinned DUCC absolute warmed latencies and relative error. If the direction is negative and the algorithmic mechanism is sound, diagnose ONE execution issue (SIMD spills, fallback frequency or scatter) and revise or revert. Do not spend multiple hours on a negative candidate.

For promotion, verify a few independently interleaved processes with calibrated short batches and real long-call counts, plus neighboring degrees/reduced order and exceptional cases. Report 3 comparisons: candidate/base, candidate/DUCC and **remaining absolute excess milliseconds**. A small but real gain is allowed; however prioritize major recurrence changes until the 2.44× gap falls. Avoid benchmark charts and exhaustive T0–T3072 scans at this stage.

Before claiming overall scalar CC/F1 at-parity, eventually recheck the small q1 scalar synthesis excesses (~1–4%) and high/reduced orders with representative focused cells. **Do not hold spin-1 implementation hostage to that bookkeeping.** Complete benchmark campaign/figures, GL 20 MiB policy, deep repository cleanup and f32 remain nonblocking/deferred.

## Validation, safety and workflow

- Retain `S/A/S*/A*`; spin-1 STANDARD, GRAD_ONLY, DERIV1; scalar GL/CC/F1 baseline; poles, equatorial singleton, longitude phase `phi0 != 0`, odd/even `nphi`, `m=0`, full/reduced `mmax`, SIMD tails, subnormals, extreme finite and Inf/NaN, poisoned workspace/reset, late fallback and concurrent q1/q8. Shared code must preserve other supported spins/f32. Existing numerical tolerances do not change.
- Native DUCC 0.42.0 pinned SHA above; record exact Rust source, CPU ISA/compiler flags, q1 physical core and q8 one-CCD affinity, source/result provenance and host contention. Short tests: calibrated 20–100 ms batches (>=30 across independent runs) for *accepted* performance claims; use shorter screens for discarded candidates; do not invent statistical significance or claim full parity from one workload.
- Stable Rust, existing Pulp SIMD, bounded workspace, proper GPL attribution for translated DUCC. No FFT rewrite/RealFFT/paired-odd expansion, new table/cache system, public API rewrite, batch API, spin2–4 performance campaign, f32 work, or unrelated Python changes.
- Before each push, `git fetch origin`, check branch/untracked/concurrent files, reconcile safely and rebase only unpublished local work if safe. No force-push/hard resets/main changes. Push small buildable implementation commits with tests and concise evidence. No competing benchmark jobs or more than one benchmark-only subagent.
- Use `cargo fmt`, strict Clippy, relevant release/all-feature and true package-only no-default checks; target DUCC/analytic/adjoint tests for every change. Do not delete rejected experiment evidence or owner-approved `pyspharm-rs` integration.

**Success criteria:** a demonstrated **material whole-transform reduction in spin-1 recurrence time** from a new fused mirrored-pair/transition algorithm, with unchanged correctness, reproducible progress toward **≤1.0× DUCC** on representative q1/q8 full/quarter CC/F1/GL, and spin-1 analysis follow-up when synthesis improves. If a candidate fails, preserve a compact negative result and immediately try a distinct source-grounded recurrence mechanism rather than another benchmark infrastructure milestone. Do not describe an untested mechanism as a proven speedup.
