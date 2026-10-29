# M6 — Native f32/f64 generic execution and verified DUCC parity

**Active owner direction (2026-10-29).** This plan supersedes every earlier M6 implementation override, including requests to optimize an f32-input/f64-internal backend. The prior full M6 plan is preserved as [historical evidence](archive/m6-pre-native-generic-2026-10-29.md); historical NEXT instructions are not active. Read [AGENTS.md](../AGENTS.md) first for session, numerical, and Git rules.

## Durable target and current baseline

Implement the same **optimized mathematical transform algorithms** as Rust generics specialized for native f32 and f64. Both precisions must execute their own native spectral buffers, FFT, Legendre recurrence/accumulation, and output storage. No f32-to-f64 transform-sidecar architecture, nested f64 plan for an f32 request, or second maintained numerical algorithm. Small precision-specific SIMD leaves, numerical scaling policies, and plan-time constants are allowed.

Match or beat pinned native DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` in warm whole-transform time for equivalent operations, independently per required cell (`Rust/DUCC <= 1`, with credible uncertainty); retain independently validated numerical correctness and protected f64 performance. M6 remains OPEN.

**Evidence at decision time (not native-f32 parity):**
- Production GL T720 f32 scalar synthesis currently uses a guarded **f64 child-plan and conversion sidecar**, passing four full/reduced q1/q8 cells at 0.775–0.961x native DUCC. It is **temporary technical debt**, not the desired final architecture. See [production evidence](../reports/data/ducc-parity-m6-2026-10-29/f32-internal-scalar-synthesis-production/README.md).
- Native-f32 scalar analysis and spin-1 transforms remain far from DUCC. Isolated f32-I/O/f64-recurrence analysis and spin diagnostics are near parity in some cases but do **not** prove that a native-f32 instantiation is fast.
- The latest diagnostic scalar-analysis tile-512 change yielded about 1% improvement and was rejected; its subsequent Callgrind attributed 30.3% of process Ir to a checked scaled f64 analysis path and 16.6% to ordinary fused suffix work. These profiles are clues, **not** license to continue optimizing the f64-internal f32 route. See [tile report](../reports/data/ducc-parity-m6-2026-10-29/f32-analysis-tile512-screen-1bd4bd27/README.md) and [profile](../reports/data/ducc-parity-m6-2026-10-29/f32-analysis-recurrence-profile-d507297c/README.md).
- The earlier reflected native-f32 scalar-synthesis patch was rejected on current source due to adjacent and f64 regressions. Native-f32 reflected spin-1 V1–V3 and f64 spin-1 loop tiling also regressed. Do not repeat those exact designs unchanged. Provenance is in the [M6 ledger](../reports/DUCC_PARITY_M6.md).

## Milestone 0 — Bound and archive documentation

Keep [AGENTS.md](../AGENTS.md) short and stable. This file is the **only active implementation plan** for M6. Keep [reports/DUCC_PARITY_M6.md](../reports/DUCC_PARITY_M6.md) as a compact current-source per-cell matrix with links, and [reports/PROGRESS.md](../reports/PROGRESS.md) at five substantive checkpoints.

Move superseded narrative plans and completed/rejected experiment reports into a discoverable archive with a small index: decision, parent/candidate/DUCC SHAs, validation scope, source patch/checksums, benchmark summary, and limitations. Preserve unique evidence and reproducibility. Deduplicate only after checking contents and references. Do not delete independent reference fixtures, necessary tests, unpublished work, or another agent's scratch. Do not rewrite Git history. Make one bounded documentation-cleanup milestone, then return to implementation.

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
