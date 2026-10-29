# M6 — native-generic SHT execution and DUCC parity

**Status:** active. The release architecture is one shared algorithm family,
monomorphized for native f32 and f64. Runtime f32-to-f64 transform conversion
is temporary technical debt and cannot satisfy the goal.

## Baseline and source inventory

The inspected source baseline is `83dd570d413a9e0c0dd998e1fbfb34f7d8a92650`;
production code is unchanged from `c63cde440d91e3ca2810226163a191199b4ca29d`.
The current tree has no active build or benchmark process. Existing untracked
experiment evidence and registered worktrees are left intact until ownership
and unique-data status are established.

| Classification | Current owner | Direction |
|---|---|---|
| Shared mathematics to retain | `legendre/numeric.rs`, scalar/spin recurrence owners, and `scalar_paired/synthesis.rs` | Keep recurrence equations, operation semantics and checked scaling shared; preserve established f64 seed and normalization behavior. |
| f64 optimized algorithm to genericize | `legendre/scalar_ordered.rs` and `scalar_ordered/kernel.rs` | Preserve order-major reflected-ring traversal and fused ordinary suffix. Genericize geometry, coefficients, SIMD lane operations and output while keeping the same control flow for f32/f64. |
| Precision-specific leaves to retain | `LegendrePrecision`; `FusedSynthesisPrecision` in `scalar_paired/synthesis.rs`; `ShtScalar` typed SIMD hooks | Keep different exponent ranges, safe normal ranges, vector widths and f32/f64 Pulp operations as compile-time policy. Do not put type switches in degree loops. |
| Native f32 route/fallback work to retire | f32 `ShtScalar` overrides in `legendre/mod.rs`, native f32 paired synthesis entry points and per-degree exceptional callbacks | Replace with the shared generic algorithm plus checked precision policy; remove obsolete maintained recurrence/fallback branches after numerical and performance acceptance. |
| f32-to-f64 sidecar to remove | `plan.rs` child `ShtPlan<f64>`; `transform/scalar.rs` promotion/narrowing helpers; `workspace.rs` boxed `F32InternalScalarSynthesisWorkspace`; associated `ShtScalar` dispatch flag | Keep only until native shared scalar synthesis is ready; it is not a valid final backend. |
| Tests and performance evidence | module `tests.rs` files, Rust integration tests, benchmark scripts, `reports/data/` packages | Adapt correctness tests to the shared route. Preserve unique hashes, patches and benchmark provenance; use the archive index and do not delete unowned untracked files. |

`ScalarOrderedScratch<T>` and disjoint spectrum writers already use generic
storage, but the ordered kernel, reflected geometry, public inputs and SIMD
operations are f64-specific. `scalar_paired/synthesis.rs` already provides a
typed f32/f64 fused SIMD abstraction worth reusing. This is a targeted
algorithm generalization, not a mechanical replacement of `f64` tokens.

## Execution sequence

1. **Bounded documentation/archive cleanup.** Keep this plan as the only active
   execution plan. Keep `AGENTS.md` concise, the live ledger per-cell, and
   `PROGRESS.md` to five checkpoints. Move superseded execution plans and the
   long-form M6 history under `reports/archive/`; retain evidence and do not
   alter untracked experiment files of uncertain ownership.
2. **Generic scalar synthesis in isolation.** Start from the exact source
   baseline in a fresh isolated worktree. Genericize the optimized
   order-major/reflected algorithm in `scalar_ordered.rs` and `kernel.rs`.
   Reuse the scale-to-ordinary transition and fused recurrence where safe;
   maintain f32 mantissa/exponent state until conversion is proven safe. Keep
   ordinary-range SIMD work branch-light and use native f32 coefficients,
   Fourier storage, FFT and output.
3. **First proof before dispatch changes.** Compare exact parent, candidate and
   pinned DUCC on GL T720 f32 scalar synthesis: full M720 and reduced M180,
   q1/q8. Include whole transforms and the Legendre stage, recurrence and
   reflected-ring work counts, useful SIMD suffix coverage, f32/f64 adjacent
   controls, and f64 GL T1023 spin-1 synthesis q1/q8. Start with three
   interleaved pairs; extend consequential decisions to at least 15. Validate
   analytic modes, independent references, adjoints, scaling/extremes, poles,
   tails, repeated calls and poisoned-workspace reset.
4. **Switch and retire scalar synthesis machinery.** Once the native shared
   route passes numerical acceptance and provides a credible production
   improvement, dispatch f32 directly to it. Remove the child f64 plan,
   conversion buffers/helpers, conversion guards and obsolete f32-only
   synthesis implementations that no active operation needs. Keep legitimate
   exceptional handling inside the shared algorithm.
5. **Generic scalar analysis.** Share reflected order-major traversal,
   recurrence and accumulation for f32/f64. Keep f32 FFT spectra and
   coefficients native; preserve S/A/S*/A* behavior, weights, scaling, tails
   and workspace reset. Remove replaced f32 callback/fallback recurrences.
6. **Generic spin-1 synthesis and analysis.** Reuse shared scalar-channel and
   reflected-ring strategies. Preserve the top-degree contribution
   (`lmax + 1` for spin-to-scalar synthesis), signs, phases, poles, adjoints
   and q1/q8 scheduling. Remove replaced native-f32 spin fallback algorithms.
7. **Full acceptance and final audit.** Reach Rust/DUCC <= 1.00 independently
   in every focused f32 GL T720 cell, then broaden to T255/T511/T1023, GL/CC/F1,
   full/reduced truncations, odd/even longitude and Nyquist, adjoints and
   representative thread counts. Verify all supported feature builds, Rust
   and integration tests, strict Clippy, formatting and Python API behavior.
   Audit that no conversion sidecar, duplicate maintained f32 recurrence,
   dead precision route or production experiment instrumentation remains.

## Acceptance rules

- DUCC reference: version 0.42.0, commit
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Pin parent/candidate/reference SHAs, flags/ISA, fixtures, affinity, q1/q8
  thread counts and warm-call boundaries. Report complete-transform time and
  per-cell PASS/GAP/INCONCLUSIVE/UNMEASURED status.
- Never weaken numerical tolerances or treat candidate-parent agreement as an
  independent accuracy check. Keep precision-specific scaling mathematically
  safe, including contributions that leave and later re-enter normal range.
- Preserve important f64 behavior and speed, including the GL T1023 spin-1
  synthesis q1/q8 control. Reject repeatable adjacent regressions.
- Keep one live matrix in `reports/DUCC_PARITY_M6.md`; detailed historical
  records belong under `reports/archive/` or the linked evidence packages.
- Work on `dev`, reconcile `origin/dev` before pushing, and commit only
  coherent verified slices. No force-push, history rewrite, `main` merge or
  deletion of unowned work.
