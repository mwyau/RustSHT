# RustSHT performance checkpoint (facts, not instructions)

**Branch:** `opt/sht-performance-parity`. **Reviewed at:** `96ffd14`, 2026-10-19. Verify current HEAD before relying on this checkpoint.

For the **only active implementation direction**, see [Plan 11](plans/11-ducc-equivalent-scalar-analysis.md).

- f64 scalar GL analysis has integrated ordinary and mixed/scaled SIMD recurrence, guarded insignificant-prefix advancement, active-ring compaction, dynamic harmonic-order scheduling and FMA accumulations.
- `a707780` changed ordinary SIMD coefficient accumulation to FMA. Its Rust-only short screen generally favored FMA, but did **not** establish new matched DUCC parity.
- `96ffd14` prepared cosine-square and scaled-correction scratch in reusable workspaces and reduced per-pair lane-validity horizontal reductions to per-block checks. Matched-codegen inspection found smaller stack footprint and fewer YMM stack operands. However its full-transform T720 q1/q8 screen **regressed** compared with `a707780`; T300 reduced-mmax q1 improved and q8 was near neutral. No generalized speedup established.
- Prior Rust/DUCC stage results suggested a roughly 2× recurrence/accumulation deficit, but used different phase boundaries. There is no defensible current strict equivalent-work performance ratio.
- Mathematical and fallback safety remain mandatory; the latest codegen report records the source changes, numerical errors, reverted experiment and measurement limitations.

**Evidence:** [latest SIMD codegen follow-up](reports/archive/experiments/scalar-analysis-codegen-followup-2026-10-19.md), [earlier AVX2 report](reports/archive/experiments/scalar-analysis-avx2-codegen-2026-10-19.md), [stage profile](reports/archive/experiments/scalar-analysis-stage-profile-2026-10-18.md). The long pre-cleanup handoff is [archived](reports/archive/performance-status-2026-10-19-pre-cleanup.md). Historic reports are evidence, not standing targets.
