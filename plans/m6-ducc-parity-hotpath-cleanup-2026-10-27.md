# M6 — Native f32/f64 generic execution and verified DUCC parity

**Active owner direction (2026-10-29).** This plan supersedes every earlier M6 implementation override, including requests to optimize an f32-input/f64-internal backend. The prior full M6 plan is preserved as [historical evidence](../reports/archive/plans/m6-pre-native-generic-2026-10-29.md); historical NEXT instructions are not active. Read [AGENTS.md](../AGENTS.md) first for session, numerical, and Git rules.

## Durable target and current baseline

Implement the same **optimized mathematical transform algorithms** as Rust generics specialized for native f32 and f64. Both precisions must execute their own native spectral buffers, FFT, Legendre recurrence/accumulation, and output storage. No f32-to-f64 transform-sidecar architecture, nested f64 plan for an f32 request, or second maintained numerical algorithm. Small precision-specific SIMD leaves, numerical scaling policies, and plan-time constants are allowed.

Match or beat pinned native DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` in warm whole-transform time for equivalent operations, independently per required cell (`Rust/DUCC <= 1`, with credible uncertainty); retain independently validated numerical correctness and protected f64 performance. M6 remains OPEN.

**Inspected source baseline:** `83dd570d413a9e0c0dd998e1fbfb34f7d8a92650`;
production code is unchanged from `c63cde440d91e3ca2810226163a191199b4ca29d`.
The isolated generic implementation and follow-ups are based on parent
`b6a96aa7`.
The source inventory classifies the current implementation as follows:

| Classification | Current owner | Direction |
|---|---|---|
| Shared mathematics | `legendre/numeric.rs`, scalar/spin recurrence owners, `scalar_paired/synthesis.rs` | Retain shared equations and checked scaling; preserve f64 seed and normalization behavior. |
| f64 optimized algorithm to genericize | `legendre/scalar_ordered.rs`, `scalar_ordered/kernel.rs` | Preserve order-major reflected-ring control flow and fused ordinary suffix; genericize geometry, arithmetic, SIMD and output. |
| Precision-specific leaves | `LegendrePrecision`, paired `FusedSynthesisPrecision`, typed `ShtScalar` SIMD hooks | Retain f32/f64 exponent ranges, vector widths and typed Pulp operations as compile-time policy. |
| Native f32 transform fallbacks | f32 `ShtScalar` overrides and f32-only paired-synthesis entry points | Replace with shared generic algorithms; remove obsolete routes only after validation. |
| Conversion sidecar | f64 child in `plan.rs`, conversion helpers in `transform/scalar.rs`, boxed buffers in `workspace.rs` | Remove after native shared synthesis is correct and measurably useful. |
| Tests and benchmark evidence | module tests, integration tests, benchmark scripts and `reports/data/` | Adapt tests to shared routes; preserve source hashes, patches and unique evidence. |

`ScalarOrderedScratch<T>` and disjoint spectrum writers are generic already,
but the ordered kernel, reflected geometry and SIMD operations are f64-specific.
The paired synthesis implementation has a typed f32/f64 SIMD abstraction to
reuse. This is a targeted generalization, not a mechanical `f64` replacement.

**Evidence at decision time (not native-f32 parity):**
- The first isolated generic scalar-synthesis candidate is now measured. Its
  15-pair native-f32 GL T720 ratios are `1.355`, `1.338`, `1.054`, and `1.134`
  versus DUCC for full/reduced q1/q8. It remains a GAP and did not change
  production dispatch. The vector-rescale leaf and forced-inline follow-up were
  also rejected after three-pair screens. Exact patches, paired data, work
  counts, Callgrind, and the high-degree numerical caveat are indexed in the
  [isolated synthesis record](../reports/archive/experiments/native-generic-scalar-synthesis-2026-10-29/README.md).
- An exact-zero shared-basis restore shortcut improved the generic parent by
  about 6–7% across the 15-pair native-f32 screen. Full T720 remains 1.24–1.27x
  DUCC; reduced q1 is INCONCLUSIVE near parity and reduced q8 is too noisy.
  A precision gate removed the earlier q1 f64 slowdown; its protected f64 q8
  result remains INCONCLUSIVE. Skipping the four FMAs after zero restoration
  did not establish a further gain. The near-pole independent-accuracy issue
  remains open; none of these patches is promoted. Full matrices and hashes
  are in the [isolated synthesis archive](../reports/archive/experiments/native-generic-scalar-synthesis-2026-10-29/README.md).
- A suffix-inlining follow-up did not improve the generic candidate: the
  original f32 suffix already emits direct AVX2 FMA with a small stack frame.
  The exact three-pair delta screen and assembly excerpt are archived with the
  candidate record; do not repeat this attribute change unchanged.
- The generic candidate passed the release lib suite, but the high-degree
  zonal probe reports `0.018705474` in f32 versus `0.018541995` from the
  independent f64 recurrence near the pole. This diagnostic has no tolerance
  assertion; the paired m=0 leaf below now carries the independent zonal check.
- A precision-specific AVX2 leaf that restored each scaled f32 lane by
  adjusting exponent fields and falling back for exceptional lanes also
  regressed all four T720 synthesis cells (1.112–1.623x DUCC). Its scaler
  consumed 36.9% of q1 Callgrind instruction references; this is not a cycle
  estimate. Do not repeat per-lane exponent restoration as the next candidate.
- Production GL T720 f32 scalar synthesis currently uses a guarded **f64 child-plan and conversion sidecar**, passing four full/reduced q1/q8 cells at 0.775–0.961x native DUCC. It is **temporary technical debt**, not the desired final architecture. See [production evidence](../reports/data/ducc-parity-m6-2026-10-29/f32-internal-scalar-synthesis-production/README.md).
- Native-f32 scalar analysis and spin-1 transforms remain far from DUCC. Isolated f32-I/O/f64-recurrence analysis and spin diagnostics are near parity in some cases but do **not** prove that a native-f32 instantiation is fast.
- The latest diagnostic scalar-analysis tile-512 change yielded about 1% improvement and was rejected; its subsequent Callgrind attributed 30.3% of process Ir to a checked scaled f64 analysis path and 16.6% to ordinary fused suffix work. These profiles are clues, **not** license to continue optimizing the f64-internal f32 route. See [tile report](../reports/data/ducc-parity-m6-2026-10-29/f32-analysis-tile512-screen-1bd4bd27/README.md) and [profile](../reports/data/ducc-parity-m6-2026-10-29/f32-analysis-recurrence-profile-d507297c/README.md).
- The earlier reflected native-f32 scalar-synthesis patch was rejected on current source due to adjacent and f64 regressions. Native-f32 reflected spin-1 V1–V3 and f64 spin-1 loop tiling also regressed. Do not repeat those exact designs unchanged. Provenance is in the [M6 ledger](../reports/DUCC_PARITY_M6.md).

**Latest scalar-synthesis diagnostic:** the first-order residual update was
integrated through the test-only scaled prefix and ordinary suffix. Across
three interleaved whole-transform rounds it cost `2.22–2.53x` DUCC in the four
T720 scalar cells and left the random-map error at about `1.9e-4`, so this
all-order route is rejected. It lowered a 208-value high-degree recurrence
replay to `1.49e-6` relative RMS against the same f32 setup, but did not correct
the f32-versus-DUCC transform difference. Earlier setup screens measured
amplification from f32-rounded geometry, recurrence factors and the sectoral
seed; direct source comparisons found matching formulas and nearest-f32
tables. The full source patch, fixture hashes, binaries and paired results are
in the [residual integration record](../reports/archive/experiments/native-generic-scalar-synthesis-2026-10-29/residual-recurrence-integration/README.md).
The original twofold callback remains rejected at `17.24x` ordinary-loop cost;
see the [compensated recurrence record](../reports/archive/experiments/native-generic-scalar-synthesis-2026-10-29/compensated-recurrence-probe/README.md).

**NEXT:** in a bounded `m=1–16,l=600–720` replay, isolate rounding in `cos²`,
recurrence factors and the sectoral seed against DUCC's double constants. Test
high/low native-f32 constant leaves with residual propagation limited to those
low orders; require a material accuracy gain at a feasible AVX2 cost before
another whole-transform run. Keep production dispatch unchanged.

## Milestone 0 — Bound and archive documentation (completed in `8e888e9a`)

Keep [AGENTS.md](../AGENTS.md) short and stable. This file is the **only active implementation plan** for M6. Keep [reports/DUCC_PARITY_M6.md](../reports/DUCC_PARITY_M6.md) as a compact current-source per-cell matrix with links, and [reports/PROGRESS.md](../reports/PROGRESS.md) at five substantive checkpoints.

Superseded plans, the previous full ledger and experiment decisions are now
indexed under `reports/archive/`. Untracked experiment artifacts were left
untouched because ownership/uniqueness was not established. Continue with
implementation; do not repeat this cleanup milestone.

## Milestone 1 — Genericize fast order-major/reflected scalar synthesis

Start with:
- `src/legendre/scalar_ordered.rs`
- `src/legendre/scalar_ordered/kernel.rs`
- `src/legendre/scalar_paired/synthesis.rs`
- `src/legendre/numeric.rs`, `src/legendre/mod.rs`
- `src/transform/scalar.rs`, `src/plan.rs`, `src/workspace.rs`

The present fast scalar-ordered/reflected kernel is specialized for f64: `Complex64`, `f64` scratch, and `S::f64s`. It already has some generic infrastructure (`ScalarOrderedScratch<T>`, disjoint spectrum output, typed fused-synthesis precision). **Genericize its order-major scheduling, north/south reflection, scaled-prefix/bulk-recurrence/accumulation algorithm once**, with small typed scalar/SIMD leaves for f32/f64. Prefer monomorphized static dispatch. Do not copy an independent f32 recurrence or add runtime precision switches in hot loops.

Use native f32 coefficients, f32 FFT and Fourier buffers, f32 recurrence/accumulation, and f32 map writes throughout the f32 transform. No promotion of runtime f32 inputs or whole recurrence to f64. Plan-time numerical constants may use higher precision when justified.

**Critical numerical risk:** f32's narrower exponent range makes a literal f64-range substitution unsafe. Keep mantissa/exponent state and checked exceptional/sectoral/polar handling as needed, without prematurely flushing a contribution that can later be represented. Preserve correct adjoints, normalization and reset. Performance may differ significantly from the existing f64 kernel; eight f32 AVX2 lanes do not prove parity. Inspect real generated AVX2/FMA and the scaled/ordinary counts.

First prototype in an isolated current-source worktree. Measure native-f32 GL T720 scalar synthesis full M720 and reduced M180, q1 and q8, against exact source parent and native-f32 DUCC. Validate analytic/sectoral, adjoints, extremes/subnormal, tails and poisoned/reset workspace. Reject unsafe results. If slow, attribute useful work, scaled prefix, loop structure and instruction cost **before** adding another optimization.

**Current state:** the generic route and its follow-ups remain isolated.
Fifteen-pair screening established a GAP in both full T720 cells; reduced q1
is near parity and q8 remains inconclusive. The precision-gated exact-zero
restore helps the generic parent but does not close full-grid gaps. The
per-lane AVX2 scaler and suffix-inlining changes were rejected. The ordinary
suffix emits direct AVX2 FMA; its inspected f32 V3 specialization uses eight
YMM lanes and has no vector-state spills.

The latest paired f32 recurrence leaf integrates high/low plan constants and
compensated runtime state into the guarded `m=0` recurrence. It reduces the
T720 zonal maximum absolute error to `3.19e-8`, but full q1/q8 remain
`1.256/1.272x` DUCC, random-transform relative-L2 error is unchanged, and the
leaf does not establish parity. An accuracy-only band screen places m=0 at
`5.83e-8`, nonzero order bands at `1.32e-4–2.10e-4`, and degree 600–719 at
`2.55e-4`. The intersection screen places `m=1–32,l=600–719` at
`3.25e-4` relative L2; `l=720` is `4.28e-4` for only 32 coefficients per
channel and has maximum absolute error `3.25e-5`. Ringwise errors peak at
polar-end rings 5/717 and equator ring 360; order subdivisions put `m=5–8`
at `4.61e-4` relative L2 and `m=9–16` at `3.42e-4` maximum absolute error.
A test-only observer found zero scaled-prefix pair updates in all 736 ring
batches for `m=1–16`; each batch began the ordinary f32 suffix at degree `m`.
A second replay matched the SIMD output bitwise and measured relative RMS
contributions of `1.6448e-4` from recurrence rounding and `1.0311e-6` from
accumulation rounding when f32 setup values were held fixed. Sequential
one-at-a-time f64 substitutions measured geometry sensitivity at `9.00e-4`,
paired-factor sensitivity at `1.51e-4`, sectoral seed at `2.18e-5`, and
coefficient preparation at `1.95e-6` relative RMS. These are not additive
error terms; equator accumulation is locally material. Next compare the exact
geometry and factor-preparation sequence with pinned native-f32 DUCC. Keep
production dispatch unchanged; do not promote this leaf or the conversion
sidecar. See the [paired zonal-leaf
record](../reports/archive/experiments/native-generic-scalar-synthesis-2026-10-29/zonal-double-single-runtime/README.md).

## Milestone 2 — Replace and REMOVE obsolete scalar synthesis paths

After the new native generic scalar-synthesis route passes correctness and its measured acceptance gate, make a coherent production replacement and delete the now-unnecessary private implementations:
- f32-owned nested `ShtPlan<f64>`, `F32InternalScalarSynthesisWorkspace`, conversion buffers, promotion/writeback, and sidecar-specific range guards;
- superseded f32 scalar-synthesis visitor/dispatch paths and private compatibility wrappers no longer used by any supported operation;
- redundant tests targeting removed private implementation details, while retaining/strengthening public mathematical and exceptional tests.

**No backward compatibility is required for private f32 implementations or their bitwise rounding.** Preserve the public f32 API and mathematical/error contracts. Numeric exceptional-case handling remains mandatory but belongs inside the new shared native algorithm, not a second legacy transform. Do not leave broken intermediate commits on `dev`. If parity is not yet achieved, keep the conversion sidecar identified as removal debt only until a correct replacement is ready; do not continue optimizing or generalizing it.

## Milestone 3 — Native generic scalar analysis, then spin-1

Apply the same shared native-precision execution strategy to scalar analysis. Reuse reflected-ring traversal, order-major scheduling, fused recurrence/accumulation and typed SIMD, preserving exact S/A/S*/A* distinctions. The previous direct-f32-spectrum/f64-recurrence diagnostic achieved near DUCC parity in places, **but it is not a native-f32 result**. Do not extend that diagnostic as production architecture. Profile the **native f32** instantiation; replace/delete old analysis callback and fallback machinery only when all necessary behavior is reimplemented.

Then genericize optimized spin-1 synthesis **and** analysis using shared dual scalar-channel/reflected work (and the required `lmax + 1` scalar degree for spin-1 preparation), with correct E/B↔Q/U signs, normalization, poles, exceptional values and adjoints. Preserve proven f64 wins. Do not reuse rejected f32 V1–V3 reflected-packet or f64 large-register tile designs unchanged. Remove old private f32 general-spin transform fallbacks only after verified replacement. Do not delete a necessary checked numerical fallback.

## Acceptance, CI and reporting

Focused matrix: GL T720 spin 0/1 × synthesis/analysis × full/reduced `mmax` × q1/q8 = **16 native-f32 cells**. Report each individually with complete Rust and pinned-DUCC milliseconds, paired ratio/uncertainty, actual executed native precision/route, numerical references, reset and adjoint checks, exact source/compiler/fixture hashes, and PASS/GAP/INCONCLUSIVE/UNMEASURED. Existing production 4/4 conversion-backend passes do **not** count toward native-f32 architectural completion.

Start with three interleaved matched pairs; extend consequential acceptance to 15+. Include adjacent f32 controls and protected **f64 GL T1023 spin-1 synthesis q1/q8**. Confirm compiler settings, CPU affinity, ISA and benchmark boundaries. Keep counters/profilers out of timed binaries. Never mistake Callgrind Ir for cycles or infer speed from kernel-only numbers.

Extend validated native correctness/performance to representative T255/T511/T1023, GL/CC/F1, full/reduced and oversampled, odd/even longitude/Nyquist, adjoints and q2/q4 as warranted. Run formatting, Clippy, feature configurations, Rust integration and Python tests. Record pre-existing build failures separately; do not add new ones or relax tolerances.

Make bounded verified implementation and documentation commits on `dev`, reconcile remote before each push, **never force-push or merge `main`**, and preserve scratch ownership. Keep a precise NEXT on interruption. Do not claim M6 complete from refactoring, isolated speedups or green CI alone.
