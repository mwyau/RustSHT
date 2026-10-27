# Owner steering — recover the best shared-kernel candidates (2026-10-26 17:13 UTC)

**HISTORICAL/COMPLETED PLAN — not the current task (2026-10-27):** B32 selective f32/setup source is integrated, B36 retained, and B33/B34/B35 unchanged remain rejected. Do **not** restart this recovery campaign. Current implementation instructions are [NO-SOL cleanup-first precision convergence](resume-legendre-convergence-no-sol-2026-10-27.md). **Sol reviewer/model is forbidden for this assignment**, even with previous one-time approval or a quota error.

**Standing reviewer/storage override (owner, 2026-10-26 17:36 UTC):** [Experiment hygiene](../docs/EXPERIMENT_HYGIENE.md) applies at each candidate screen. **Never summon any Sol reviewer or Sol model in this assignment. The former one-time permission is revoked.** Luna `benchmark` retains its earlier narrow authorization. Before experiments use bounded scratch (prefer `/tmp` only if free space/inodes allow), write and verify durable compact results/data/reports outside scratch, then immediately remove ALL experiment-owned compiled baseline/candidate/DUCC builds, local envs, caches, profiler output and temp worktrees. Do not retain experiment binaries for later comparisons, and never clean other agents' environments or the only saved evidence.

**Task:** Review the four recent rejected shared-kernel candidates B32–B35 and selectively integrate the best *substantial* simplifications into current `dev`. The owner clarified the asymmetry: **f64 correctness and performance are the highest priority; f32 correctness is mandatory but maintainability has precedence over a small measured f32 slowdown**. This targeted evaluation takes precedence over the earlier general 0–5% performance trade-off for f64. Work directly on `dev`, not `main`; fetch/reconcile before each push, never force-push. Do not permit another writer on `dev`.

**Important live-state adjustment:** Candidate **B36 (shared scalar recurrence-step equation)** was already reactivated in commit `c908452a5bb9` and documented by `6ce981ec6195` after the 17:02 owner instruction. **Do not revert or attempt to reintroduce B36 as if it were still a rejected candidate.** Start from current HEAD, not the earlier `d679ae42` rejection checkpoint. B36's f32 T720 direct routes measured +1.35–3.24% relative to immediate parent, but the existing six-cell comparison to immutable initial baseline `062218639bc2f097f004c617da0ce46a3a4d2941` was within about 2% per cell. Its optimized f64 Pulp bodies had matching instruction counts/recurrence arithmetic, but that disassembly alone does not prove every f64 whole-transform path is unaffected. Retain B36 subject to the f64 guardrails below.

## 1. Nonnegotiable priority hierarchy

1. **f64 numerical correctness:** no changed formulas, relaxed tolerances, altered adjoint operators, incorrect precision, exceptional/polar behavior, phase, workspace reset, or silent fallback.
2. **f64 whole-transform performance:** preserve established wins and avoid a credible repeatable slowdown in an important f64 scalar or spin workload. *Do not spend a measurable f64 performance margin merely for uniform source code.* Preserve an optimized f64 leaf/route if necessary, and clearly report any existing DUCC gap. Prior owner wording allowing 1–5% regression does **not** authorize accepting an f64 regression under this clarified priority.
3. **f32 correctness:** same invariant protections with native f32 arithmetic and correct f32 exponent/overflow limits; no automatic f64 bridge.
4. **Maintainability and simplicity:** substantially reduce separately maintained numerical implementations, duplicated algorithm logic, special cases and difficult fallback state. A repeatable **0–5% f32 regression** can be worthwhile for a major real reduction; 5–10% requires a stronger case and awareness of cumulative slowdown; >10% ordinarily needs a f32 fast leaf.
5. **f32 performance:** remain competitive where possible, but do not reject an otherwise important shared implementation merely because a critical f32 cell is 1–3% slower. Consider the cumulative f32 slowdown against the immutable baseline, not just each previous commit.

Keep f64 separate SIMD leaves when needed for performance; sharing a common algebraic definition, factor preparation, data model or control path still constitutes valid simplification. One f64-specialized leaf is preferable to two fully duplicated scalar/spin implementations. No dynamic dispatch, trait objects, new allocations or f32->f64 conversions inside hot loops.

## 2. Candidate evaluation based on actual changes and saved evidence

**Compare candidates as source changes, not merely benchmark outcomes.** Source SHAs below are recoverable on GitHub; they are *historical* commits against their own exact parents, so **do not cherry-pick them indiscriminately onto today's dev**. Reimplement or port only the useful mechanics after understanding current `numeric.rs`, `spin_factors.rs`, B36, callers and merging conflicts.

| Rank | Candidate | Scope and maintenance evidence | Recorded cost / owner disposition |
|---|---|---|---|
| **1: investigate first** | **B32** `f974d1d5b1ffd7f6e097e4246efdb3b9571369a9`, reverted `b9d91b40` | Generic scaled power/multiplication/normalization for scalar and spin, both precisions. Diff: ~214 old duplicated lines replaced by ~129 new lines including tests; net **−85 source lines** across `numeric.rs`, `scalar.rs`, `scalar_f32.rs`, `spin.rs`, `spin_f32.rs`, `spin_factors.rs`. This is genuine shared mathematics. | Every Rust parent/candidate output bit-identical in 18 three-pair cells; 49 focused tests passed. f32 T720 scalar q1 analysis about +1.4–2.2%, q8 analysis +1.7–2.3%; synthesis q1 +2.0–6.2% across 3 pairs; q8 +1.5–1.9% but host wait high. **f64 T720 scalar synthesis q8 +1.2–1.9% in all 3 low-wait pairs: preserve f64 fast implementation rather than restoring B32 globally.** |
| **2: conditional** | **B35** `467d73ab302eb74047d985a57de4af9f37c75aff`, reverted `046a870f` | Macro emits f32 and f64 native-spin SIMD recurrence-step functions from one operation body. The source diff adds ~67+10 and deletes ~41+42 lines: **net −6 lines** and a sizeable macro interface. It removes one pair of ~40-line duplicate functions but may make maintenance harder. | Correctness and 94 focused spin tests passed. **f64 T1023 spin-1 synthesis q1 slowed 1.247%**, 15 pairs, CI [0.70%, 2.01%] above parity. **Do not reinstate the unchanged shared f64 function.** A variant that retains the original f64 optimized leaf is eligible only if overall duplication/complexity clearly falls. |
| **3: low priority** | **B34** `8e57a108bba1502ec1890b27814cb73c78b68587`, reverted `941c82a0` | Macro for scalar traversal order and three-buffer rotation, leaving the SIMD arithmetic and normalizers in two duplicate blocks. Source diff net **+34 lines**, so it is not a major consolidation. | Correctness passed. Four f32 T720 cells had 15 pairs: analysis +0.56%/+0.81%; synthesis +2.97%/+2.46%. No clear net architectural payoff. **Do not reapply unchanged**, especially now B36 owns the common scalar recurrence step. |
| **4: reject unchanged** | **B33** `75cf5472665249de65c10b70efbcfd4f6e8cf5ed`, reverted `fefaa81e` | Callback-based degree traversal preserves separate inner arithmetic but adds a general closure boundary; source diff of numeric/scalar files yields roughly **+40 source lines**. | Correctness passed, but f64 scalar T720 synthesis/analysis affected as well as f32 (+roughly 1–5%). The abstraction and performance costs outweigh the limited sharing. **Do not restore unchanged**; prefer B36's now-accepted compile-time recurrence-step expression. |

Treat approximate line counts as **diff stats, not a measurement of maintainability**. Review remaining separately maintained equations/functions and actual new macro arguments, interfaces and unsafe/fallback branches. Do not credit moving source text to `numeric.rs` as eliminating it unless the remaining f32/f64 implementations truly share one maintained operation.

## 3. Required evaluation and integration sequence

### A. Short inventory; no new benchmark campaign

- Fetch current `origin/dev`, inspect `AGENTS.md`, this plan, current progress and B36. Confirm its accepted status and exact source SHA.
- Inspect candidate diffs B32–B35 and their **already published** parity/timing results; spend at most one concise review checkpoint making a *comparison table* with: unique algorithm bodies eliminated; gross/net source change excluding comments/tests; new macro/trait complexity; f64 affected cells and recorded performance; f32 affected cells and recorded performance; numerical/fallback risk; recommended action.
- Give B32 priority by default. Treat B35 as optional. Defer B33/B34 unless new evidence shows they materially reduce duplication beyond what B36 already achieved. **Do not run new 15-pair A/B screens simply to confirm already established 1–3% slowdowns.**
- Do not promote all candidates because the owner accepts a few percent *f32* slowdown. Select one coherent implementation at a time.

### B. Recover B32's real normalization/power sharing while protecting f64

- Start from the latest accepted `numeric.rs`, `spin_factors.rs`, B36. Reuse the shared policy and checked-exponent handling already present.
- Extract one typed definition for noncritical/shared scaled-power, scaled-product and exponent-normalization mathematics and use it to reduce duplicate f32 scalar/spin and shared factor-preparation logic where valid.
- **Protect f64:** keep the existing f64 hot `scaled_power`, `normalize_value`, `multiply` or equivalent code path when the new generic helper would change f64 codegen or measurably slow f64 end-to-end performance. A short f64-specialized leaf is permitted; avoid a duplicate 200-line normalization subsystem just to preserve 1–2%.
- Consider precision-dependent compile-time specialization and keeping selected f64 helper calls in their existing position; don't add a dynamic strategy switch. Avoid the B32 unchanged code if it repeats the measured f64 T720 scalar synthesis q8 regression.
- f32 native scaling constants/overflow guards must stay correct; preserve f32 normal-range skip path, checked exponent arithmetic, subnormal handling, and existing fallback contracts. Do not swap multiplication versus fused multiply-add or change associativity without mathematical review.
- Verify actual simplification (number of f32/f64 scaling algorithm bodies removed). If there is no meaningful net reduction after protecting f64, reject this particular source strategy and progress elsewhere rather than building a policy framework.
- Run `cargo check --lib` and focused scaling, scalar, native-spin, exceptional and workspace tests on the coherent change. Use saved B32 performance evidence initially. For new affected f64 hot paths, ask Luna `benchmark` for three matched exact-parent/candidate/DUCC q1/q8 pairs, including **GL T720 scalar synthesis q8** and protected **GL T1023 spin-1 synthesis q1/q8**, plus one scalar analysis control. Inspect f64 hot-loop codegen only if an unexpected change appears.
- **Gate:** if the f64 scalar/important spin route reproducibly slows, isolate or revert its affected f64 specialization. Do not discard a useful f32/shared-factor cleanup merely because f64 needs one fast leaf. If f32 is a few percent slower but materially simpler, it can be kept; check cumulative comparison against immutable baseline.

### C. B35 only if it removes meaningful duplicate math after preserving f64

- Evaluate whether B35's 40-line duplicated SIMD step really causes drift/maintenance cost compared with a complicated macro with 7+ method-name parameters. Measure human maintainability, not only total lines.
- **Do not reinstate `467d73ab` unchanged:** its f64 T1023 synthesis q1 slowdown was confirmed at 15 pairs. Either retain the exact current f64 step and share lower-level coefficient preparation/control, or devise a tiny source-generated typed leaf whose f64 matched performance is confirmed.
- If protecting the f64 step means the project still has one native f64 leaf and one native f32 leaf, that can be the *correct final architecture*. Do not force a macro purely for symmetry.
- Implement only if clearly beneficial after B32 and B36, with focused spin f32/f64 tests; use a short f64 T1023 synthesis q1/q8 screen and f32 T720 q1/q8 controls when generated code truly changes. Stop and retain the original f64 function if its confirmed slowdown recurs.

### D. Stop retrying B33/B34 in isolation

- B33 callback traversal and B34 traversal macro are *not* compelling on their own after B36's accepted shared degree-step. They add source/abstraction without sharing scalar arithmetic.
- Do not rerun either unchanged. If later merging the full scalar f32/f64 algorithm naturally removes their remaining duplicated traversal, use that broader implementation and validate as a new integrated candidate; it is not a reason to reintroduce B33/B34's exact old form now.
- Preserve historical source and benchmarks. Update candidate ledger dispositions as “reassessed under asymmetric owner policy,” rather than rewriting old outcomes as invalid.

## 4. Precision-specific performance and correctness gates

**f64—strict priority, no accepted repeatable degradation in important cells:**
- Protected f64 GL T1023 full spin-1 synthesis q1/q8 (latest source approximately 88.97 ms / 14.17 ms; exact current comparisons needed when code changes).
- GL T720 full spin-1 analysis q1/q8, T320/T720/T1023 overresolved spin-1 synthesis q8, CC T720/T1023 spin-1 synthesis, full/reduced mmax.
- **f64 scalar** GL T720 scalar synthesis q1/q8, T255/T720 scalar analysis and representative CC/F1 controls (because B32/B33 previously affected scalar). Don't assume f64 optimized spin-1 paths represent f64 scalar paths.
- Judge f64 on per-case whole transforms, not averages and not DUCC parity alone; retain numerical accuracy and established wins. Distinguish weak n=3 directional signals from a reproducible result. If a candidate's critical f64 gate is suspicious, first remove f64 changes or restore the f64 leaf; use longer runs only if resolving ambiguity materially affects acceptance.

**f32—maintainability-prioritized, cumulative budget:**
- GL scalar f32 T255/T720 A/S q1/q8, reduced M180 T720, GL spin-1 T720 A/S q1/q8; representative CC/F1 controls where impacted.
- +0–5% relative to the exact parent is acceptable for a large reduction in independently maintained algorithm bodies; +5–10% conditional on larger benefit; >10% normally use a specialized f32 leaf. Cumulative loss against immutable `06221863` matters: B36 already has measured immediate-parent f32 costs but still sits near baseline in selected cells.
- Keep f32 numerical tolerances, finite/extreme/polar handling, adjoints, safe output initialization, repeat/reset and spin restrictions unchanged. No blanket precision conversion.

**Testing/CPU:** For minor code edits run small build checks only; run focused numerical tests when integrating each family; run a three-pair performance screen only when affected whole-transform f64/f32 routes change. Use Luna benchmark subagent read-only in short-lived isolated worktrees with CPU exclusivity and the mandatory save-then-immediate-cleanup procedure; do not keep compiled workers after the experiment. Broad Rust/Python/CI matrix only at major stage/final integration. Do not burn 15-pair campaigns on a minor f32 slowdown already accepted by the owner.

**Benchmark interpretation:** Exact-parent and immutable-initial baseline comparisons serve different purposes. Historical noninterleaved baseline medians are **context**, not statistically matched proof of a zero-cumulative regression. Use matched remeasurements for any consequential f64 acceptance decision; avoid false precision. Record measured losses, not a claim of “performance improvement.”

## 5. Completion / handoff

- Choose the best *integrable* B32/B35 subset or explicitly reject each after source-aware assessment. Do not integrate candidates for cosmetic source sharing.
- Keep B36 as accepted source and base future work on current `dev`.
- Commit tested source changes directly on `dev`; separate each logical slice and reconcile remote before every push. Do not create side PRs or modify `main`.
- Record in `reports/SHARED_KERNEL_REFACTOR_PROGRESS.md` and `reports/PERFORMANCE_CANDIDATES.md`: candidate source SHAs, what was reused versus rewritten, old/new algorithm body count and real code complexity, f64 guardrail results, f32 accepted performance trade-offs, exact current source and parent SHAs, focused tests performed, and any unresolved risks.
- Provide a short final decision matrix: **integrated / integrated only f32 or setup / kept f64 specialized / rejected**, with a one-sentence reason and quantitative measures. After the targeted review and integration, resume the broader final architecture and module-consolidation work; do not get stuck repeatedly re-profiling the same four rejected candidates.

**Immediate instruction to Luna Max:** Read this steering after fetching the latest `origin/dev`; verify B36 is present; inspect B32 and B35 first; implement the smallest substantial B32 consolidation that leaves critical f64 hot paths intact; evaluate B35's real cost/benefit; leave B33/B34 retired unless broader work inherently supersedes them. Correctness and f64 whole-transform performance are highest priority; f32 shares more code even when a few percent slower.
