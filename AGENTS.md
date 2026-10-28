# RustSHT — current agent rules

**Authority and read order (updated 2026-10-27 04:46 UTC):** This is the concise active agent file. **FIRST read [M6 — actual hot-loop DUCC parity and cleanup goal](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md)**, then [the parity matrix contract](plans/ducc-performance-parity-continuation-2026-10-27.md), then [experiment hygiene](docs/EXPERIMENT_HYGIENE.md), [current performance status](PERFORMANCE_STATUS.md), the [f64 hotspot evidence plan](plans/f64-ducc-parity-all-cases-2026-10-24.md) as needed, and recent [progress](reports/PROGRESS.md). The prior [NO-SOL architectural restart](plans/resume-legendre-convergence-no-sol-2026-10-27.md) and [precision convergence architecture](plans/final-legendre-precision-convergence-2026-10-26.md) are completed implementation-history references, not instructions to redo stages. **The absolute no-Sol, cleanup, numerical and git rules remain mandatory.** On conflicts, this file and the M6 plan govern the active implementation; the older parity plan supplies the matrix contract.

## Absolute agent/model restriction

**NO SOL AT ALL.** **Never** invoke, spawn, invite, route to, review with, or fall back to **Sol**, **Sol High**, a `reviewer` configured as Sol, or any Sol model/subagent — whether read-only, background, parallel, manual, automatic, at milestones, or when Luna quota expires. The previous “optional Sol” and “one-time explicit authorization” provisions are **revoked for this assignment**. This is a prohibition, not a suggestion. No stealth substitution or model-name workaround.

**Main Luna Max is the only production-code writer.** One previously permitted **Luna-only `benchmark`** subagent is optional for read-only-to-`dev` isolated benchmark work; it must not spawn Sol or any other agent. If the Luna benchmark option is unavailable or would exhaust quota, main Luna should run only essential benchmarks sequentially. Do not add other subagents. **If quota/resources fail, checkpoint, push verified code, clean owned scratch and stop. Never invoke Sol.**

## ACTIVE M6 CONTINUATION OVERRIDE — 2026-10-28: native-f32 families after synthesis blocker

The shared f32/f64 fused scalar-synthesis kernel, 32-ring scheduler and
checked native-f32 packet leaf remain complete. The corrected f32
scaled-prefix-to-fused-suffix trial is a documented algorithmic blocker: at
source `2b531d73`, a useful safe handoff required zero recurrence exponents,
and the trial caused a repeatable adjacent f32 scalar-analysis regression.
Do not repeat that route unchanged. Native-f32 GL T720 scalar synthesis remains
a major gap at **33.37×/34.94×** full and **22.75×/40.42×** reduced q1/q8 against
pinned DUCC.

The first native-f32 scalar-analysis follow-up is now accepted on exact parent
`4770e1c7`. Its bounded native-f32 accumulation leaf improves GL T720 full
analysis to **32.44×/26.73×** and reduced analysis to **24.37×/19.06×** DUCC
q1/q8, with all four cells still GAP. It preserves the scaled callback and
leaves the f64 analysis override unchanged. The protected f64 GL T1023
spin-1 synthesis controls show no meaningful regression; see the candidate
report and exact measurements in the [single M6 ledger](reports/DUCC_PARITY_M6.md).

Continue through native-f32 spin-1 synthesis and analysis before returning to
the remaining f64 gaps. Profile their current work and executed SIMD paths
before choosing a bounded change. Keep scalar synthesis on the scorecard and
resume it after this documented transition blocker if a new work-reduction
hypothesis is measured. M6 remains open.

## Active M6 goal — real DUCC speed parity, assembly/Callgrind, and careful cleanup

**M6 is a NEW, OPEN goal.** The prior session's “goal achieved” referred to completed shared f32/f64 scalar+spin algorithms and module test folders. **It does not mean performance parity or release readiness.** Follow [M6 hotpath execution](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md): (1) current-HEAD validated native DUCC/f64/f32 whole-transform baseline, (2) disjoint and equivalent-work-normalized Callgrind profiler attribution **plus inspected emitted assembly** of executed inner loops, (3) one high-leverage source optimization at a time with exact parent/candidate/DUCC q1/q8 matched tests, (4) per-cell PASS/GAP/INCONCLUSIVE/UNMEASURED parity ledger, then (5) code/Cargo/script and current docs cleanups in **separate behavior-preserving commits**. **Do not stop when profiling or cleanup is complete.** Do not claim parity until supported matched cells pass at latest source.

**Subsequent f64 hotspot, after the active f32 work:** GL T720 full-mmax spin-1 analysis q1/q8 remains around 1.23–1.30× pinned DUCC in its latest source-dated measurements. Revisit `src/transform/spin/analysis.rs` reflected-order stage and `src/legendre/scalar_paired/analysis.rs` fused ordinary/scaled recurrence vs DUCC `map2alm_kernel` after the f32 scalar-synthesis algorithm-parity milestone and f32 analysis/spin-1 screens. Count true useful degree×latitude lane/channel operations with thread-safe q8 counters, then inspect executed AVX2/FMA disassembly, spills, branch guards, setup and reductions. Never mistake inclusive source totals or Callgrind instructions for cycles. Protect existing T1023 spin-1 synthesis wins.

**Exit discipline:** No Sol reviewer/model in any role, including quota fallback. When Luna quota runs out, push verified work, save a precise NEXT plus latest perf ledger/profiles summary, cleanup owned scratch and stop. No merge to `main` on green CI. Previous architecture/test-layout steps are finished; don't redo them. Documentation is also a scoped M6 deliverable: use an available technical-writing skill for Markdown quality if installed, but this must neither block the numerical work nor introduce a reviewer/agent. Preserve mathematical provenance and reports; reduce contradictory active front-page guidance instead of removing history.

## HISTORICAL EXECUTION OVERRIDE — f32 DUCC algorithm-parity milestone (2026-10-28; superseded above)

**FIRST follow the [current M6 algorithm-parity override](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md#active-owner-override--2026-10-28-ducc-algorithmic-execution-parity-first).** The production scalar-synthesis source is the accepted packet-vector leaf at `acc625d7` (current HEAD `2b531d73`), following the accepted 32-ring scheduler. At exact parent `2b531d73`, matched GL T720 Rust/DUCC medians are **33.37×/34.94×** full q1/q8 and **22.75×/40.42×** reduced q1/q8; all four remain **GAP**. A corrected packet suffix candidate moved up to 61.2M terms into fused suffixes but found no safe nonzero-exponent handoff; it was rejected after a repeatable 3.9% adjacent f32 scalar-analysis regression. See the [single M6 ledger](reports/DUCC_PARITY_M6.md) and [screen evidence](reports/data/ducc-parity-m6-2026-10-28/f32-corrected-transition-2b531d73/README.md). Continue toward a measured algorithmic reduction, not another isolated 1–3% shortcut. A single-digit Legendre-stage ratio is an intermediate target, not an M6 completion criterion.

**Do not repeat the completed fused-kernel, task-grain or checked-packet leaf milestones or the rejected shared early-stop visitor, mixed masks, per-vector retries, or zero-exponent suffix unchanged.** Pinned DUCC's `iter_to_ieee`/`calc_alm2map`/`alm2map_kernel` makes a scaled-prefix → one-time scale-correction → bulk fused-SIMD-suffix transition. The guarded native-f32 handoff was measured at exact parent `ef11ce0a`; the split packet traversal and generalized fused kernel increased q1 full Ir by 4.77% and did not improve three of four whole-transform cells. The next high-work hypothesis is synthesis-only reflected-ring reuse: first normalize useful recurrence work against DUCC and design the route around the existing shared recurrence, without changing scalar analysis or f64 dispatch. Do not restore the duplicated reflected f32 candidate that regressed reduced scalar-analysis q8. See the [suffix screen](reports/data/ducc-parity-m6-2026-10-28/f32-zero-exponent-suffix-screen/README.md) and [single M6 ledger](reports/DUCC_PARITY_M6.md).

**Precision diagnostic, not production policy:** DUCC uses internal f64 for its f32 interface. If transition coverage or native-f32 safety differs materially, an isolated f32-I/O/f64-internal benchmark may distinguish precision strategy from other bottlenecks. Do **not** widen production f32 arrays, add a duplicate algorithm, or weaken numerical tolerances. Use exact-parent matched q1/q8 results and independent analytic/extreme/adjoint checks, with protected f64 GL T1023 spin-1 synthesis controls. Single-digit **Legendre-stage** Rust/DUCC is an intermediate goal, not a promised performance outcome; M6 is open until the wider verified parity matrix passes.

**After a meaningful f32 scalar-synthesis algorithmic improvement or documented blocker**, address remaining native-f32 scalar analysis and spin-1 synthesis/analysis gaps, then resume f64 GL T720 spin-1 analysis and other gaps. Preserve the single `reports/DUCC_PARITY_M6.md` ledger. No Sol, no `main` merge, no force push, no uncontrolled scratch deletion, and no claiming parity from isolated benchmarks. Commit/push meaningful verified implementation slices to `dev` and reconcile remote first; the complete numerical, experiment and git safeguards below remain mandatory.

## Completed architectural goal (historical)

The completed refactor shares mathematical algorithms in `src/legendre` while retaining f64 numerical correctness and important f64 performance. There are **no independently maintained `scalar_f32.rs` or `spin_f32.rs` algorithm modules** and no wholesale precision-copy replacements under another filename. Keep small specialized native-f32/f64 Pulp SIMD leaves and genuinely distinct precision policies where needed. One algorithm owner does not mean falsely equating spin/scalar recurrence, GL/CC/F1 quadrature/resampling, S/A/S*/A* adjoints, or direct/paired kernels.

**Completed families (do not redo):** ordinary scalar traversal; paired scalar traversal; native-spin recurrence traversal; spin-analysis SIMD accumulation. The legacy sequence (ordinary scalar → paired scalar → retire `scalar_f32.rs` → native-spin traversal → spin analysis → retire `spin_f32.rs`) is finished. Reuse existing B32 selective scaling, B36 degree step, shared factor/seed/normalization logic and native-f32 paired scalar route. Source-only renames/moves that conceal duplicated algorithms do not satisfy the goal. Do not repeat already completed `src/pyspharm` removal or f32 paired scalar enablement.

Preserve exact f64 scalar seed loop restored by `f44821350d17`, retained f64 SIMD/normalizers and f64 paired/order-major routing. Previous shared f64 scalar seed `66145f87`, checked f64 pair normalizer `e3b9b7fe` (reverted in `0a74927f`), scalar traversal B33/B34, and spin-step macro B35 are **not** to be restored unchanged. Historical failed-source SHA and evidence belong in `reports/PERFORMANCE_CANDIDATES.md`. Do not rerun source-audit/benchmark campaigns without a new need.

**Priority:** (1) f64 correctness and important f64 whole-transform speed, (2) f32 correctness including safe exceptional/polar fallback and exact adjoints, (3) elimination of duplicate maintained algorithms and unnecessary special cases, (4) f32 performance. f32 1–5% slower for substantial sharing can be reasonable **only with cumulative context**; do not automatically accept a measured f64 regression just to reduce lines.

## Phase 0 before implementation: safe cleanup

**First inspect, then clean verified owned artifacts, then implement.** Fetch and reconcile `origin/dev`, `git status`, unpushed local commits, `git worktree list`, remaining Cargo/benchmark processes, exact scratch ownership, `df -h /tmp`, `df -i /tmp` and likely build/venv/cache sizes. Identify abandoned **experiment-owned** scratch, `target/`, venvs, temporary worktrees, reference/DUCC builds and profiler outputs. **Persist/verify** any unique code, benchmark data and reports outside scratch before deletion. Delete only *confirmed owned, inactive* paths, then check reclaimed space. If ownership is uncertain, **do not delete**. Never wipe `/tmp` broadly, global caches/toolchains, user's data, another session's work, or uncommitted/unpushed Git state. GitHub cannot inspect untracked local artifacts; report actual cleanup based on host checks rather than assumptions. Do not spend the full run on housekeeping.

For **every later experiment** create a fresh bounded scratch root (prefer `/tmp` only after free-space/inode checks); put experiment-local `CARGO_TARGET_DIR`, `CARGO_HOME` where used, Python venv/cache, compiled workers, profiler scratch and immutable comparison worktrees inside. **Immediately after saving and verifying** exact commit SHAs, compact machine-readable data, tested cases/results and report in durable storage, delete **all** owned build artifacts/environments/worktrees for that experiment, even after failure/interruption. Do not stockpile reusable compiled parents/DUCC objects; protect evidence before removing its only copy. Coordinate CPU exclusivity during benchmarks. Follow [the full disk-safety policy](docs/EXPERIMENT_HYGIENE.md).

## One unit-test layout (owner 2026-10-27 02:19 UTC)

**No inline Rust unit-test bodies.** Follow [the one required test-module layout](docs/TEST_LAYOUT.md): every production module with unit tests declares only `#[cfg(test)] mod tests;`; put all its unit tests under the natural `<module>/tests.rs` file. A large test suite may subdivide through `<module>/tests/<topic>.rs` imported by its `tests.rs`, without changing the entrypoint rule. Do not use sibling `*_tests.rs`, custom `#[path]` for ordinary unit tests, or a second unit-test convention. Root `tests/*.rs` remains reserved for Cargo integration tests. Do not add `mod.rs` or create new directories except the natural per-module test subdirectory. **The test-layout migration is complete in `055a5058`; do not repeat it.** All assertions, fixtures, cfg gates, numeric tolerances and ignore statuses were preserved. No benchmark is needed for a pure test move. Sol remains banned.

## Numerical and performance guardrails

Preserve native f32/f64 arithmetic and independent analytic/numerical references. Never weaken tolerances or tests for GL/CC/F1, full/reduced `mmax`, exact/oversampled, odd/even longitude/Nyquist, poles/equator, spins 0/1 and relevant higher spins, signed spin conventions, coefficient packing and phase, tiny/subnormal/overflow/nonfinite, SIMD tails, repeated and poisoned workspace, transactional decline/retry, or distinct S/A/S*/A* adjoints. No new runtime dependencies, new GPU/MPI/features, unrelated FFT rewrite or Python import/API changes.

The immutable pre-refactor baseline is `062218639bc2f097f004c617da0ce46a3a4d2941`. Pinned native DUCC 0.42.0 is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Important f64 cells include GL T1023 spin-1 full synthesis q1/q8; GL T720 spin-1 analysis q1/q8; scalar T720 S/A q1/q8; GL oversampled full/reduced spin-1 synthesis; selected CC T720/T1023 spin-1 synthesis. Performance parity across all cases is **not** established; never claim it. Use exact-parent matched measurements for affected hot paths, reporting DUCC comparisons and immutable-baseline context honestly; do not conceal isolated regressions in averages.

**Validation cadence:** tiny semantic-no-op move: narrow build/fmt only. Coherent numerical family: focused f32/f64 references, poles/extremes, fallback, adjoints and scratch controls; checked workspace/Clippy. A changed protected f64 hot path: initial 3 interleaved parent/candidate cases q1/q8 with diagnostics, extend only for consequential ambiguous decisions. Retiring a major family: broaden full Rust/feature/Python validation and selected cumulative screens. Do **not** run a large benchmark or full suite after every helper or a pure file relocation. Keep matched source/test/provenance evidence; never invent benchmark wins or CI results.

## Git and reporting

Work on **`dev` only**, never `main` without explicit owner publication instruction. Fetch/reconcile remote before push; one production writer; no force-push, history rewrite, hard reset, deleting others' work, or clobbering parked experiments. Commit/push **coherent, verified source slices frequently enough** that quota loss cannot strand large unpushed changes. Do not flood the branch with multiple prose-only commits per trivial edit. Save concise milestone evidence in `reports/SHARED_KERNEL_REFACTOR_PROGRESS.md` and a maximum five-entry rolling `reports/PROGRESS.md`, including source SHA, actual tests and measured cells/limitations, algorithm bodies removed and next action. Keep experimental alternatives in Git history/candidate ledger rather than retaining build scratch.

If blocked or near quota, preserve only verified code/evidence, push it, clean owned scratch and leave a compact exact-file/function/test NEXT handoff. Do not promise work after session termination; **do not use Sol**. Resume the next substantial shared algorithm family rather than repeatedly revisiting small f64 normalization experiments.

**Start now with M6: verify current DUCC gap, do work-normalized Callgrind/PC→assembly attribution, implement and benchmark a bounded optimization, then continue across measured f64/f32 gaps while maintaining and cleaning source/docs. Do not claim goal complete from CI or profiling alone.**
