# Resume goal — finish RustSHT Legendre precision convergence (NO SOL)

**Owner handoff: 2026-10-27 01:07 UTC.** This is the **current executable work plan** for the next Luna Max session and supersedes earlier agent-model permissions, old priorities, and instructions to re-evaluate retired experiments. Work against the latest fetched `origin/dev` (at plan creation: `50fa612719465c099da5ace5b0579d3030025909`). **The prior agent exhausted its quota and improperly invoked a Sol reviewer; do not repeat that behavior.**

## Goal and hard constraints

**Deliver a coherent RustSHT Legendre implementation in which f32 and f64 share each mathematically equivalent scalar/spin algorithm, `src/legendre/scalar_f32.rs` and `src/legendre/spin_f32.rs` disappear as standalone algorithm modules, and important f64 correctness and end-to-end performance are preserved.** The disappearance of filenames alone is not completion. Small specialized Pulp f32/f64 SIMD leaves and genuinely distinct precision policies may remain inside appropriately named algorithm/stage modules. Preserve every legitimate scalar-vs-spin and grid/adjoint mathematical distinction.

**ABSOLUTE MODEL BAN for this work: DO NOT invoke Sol, Sol High, a Sol reviewer, a Sol-named agent or a Sol model in any capacity, under any circumstances.** Do not use Sol to check progress, replace quota-limited Luna, read logs, provide architectural advice, or perform any kind of review or fallback. **There is no one-time exception in this assignment.** If Luna quota/resources become unavailable, commit/push the last verified coherent state, preserve a short continuation note, clean scratch, and stop the session cleanly; never switch to Sol. Main Luna Max alone writes production. One previously permitted Luna-only `benchmark` subagent may run focused read-only benchmarks in isolated scratch if available; it is optional and must never delegate to Sol. Otherwise benchmark sequentially in the main session. No other agent/subagent.

**Priority order:** (1) f64 numerical correctness and existing f64 performance wins, (2) f32 numerical correctness including distinct adjoints/fallback/atomicity, (3) meaningful removal of duplicate algorithm bodies and dependencies, (4) f32 performance (a small 1–5% whole-transform trade-off can be justified, cumulative budget tracked). Do not rewrite f64 hot arithmetic just to share f32 source. Do not relax test tolerances, silently change user-facing Rust/Python APIs, or introduce new runtime dependencies.

## Phase 0 — mandatory safe cleanup and reconciliation BEFORE ANY BUILD OR NEW EXPERIMENT

0. Read `AGENTS.md`, this plan, `docs/EXPERIMENT_HYGIENE.md`, the head of `reports/SHARED_KERNEL_REFACTOR_PROGRESS.md`, the candidate ledger, and the latest `reports/PROGRESS.md`. Do not re-read the entire multi-MB historical archive. Fetch `origin/dev`, confirm current HEAD/worktree status, identify active uncommitted changes and unpushed commits, and reconcile with upstream **without force push, hard reset, stash-drop, or overwriting another worker**. Confirm no other production writer; do not run concurrent benchmark and compilation.
1. **Inventory disk before deleting:** `df -h /tmp`, `df -i /tmp`, `df -h .`, `git status --short`, `git worktree list --porcelain`, process list for experiment-owned Cargo/Rust workers, and targeted sizes for known experiment scratch/targets/`.venv`/`UV_CACHE_DIR`/`CARGO_TARGET_DIR`. Distinguish shared toolchain/package caches and other users' worktrees from **confirmed abandoned experiment-owned artifacts**. Do not infer ownership from a generic name.
2. For each stale, **confirmed-owned and inactive** previous experiment scratch, first verify all important source changes, Git commits, compact CSV/JSON data, numerical results, benchmark provenance, and reports are persisted **outside the transient directory**. If anything is unique, save/commit it first. Check no process or agent still uses the paths. Then remove only those exact owned scratch directories, Cargo targets, virtualenvs, temporary benchmark worktrees, obsolete binaries and profiler files; prune only verified stale worktree registrations. **Never** delete unrelated files, user's projects, installed toolchains, active worker state, or the only copy of evidence. No `rm -rf /tmp/*`, blanket `cargo clean` from an uncertain directory, or global `git worktree prune` without checking candidate registrations.
3. Check free bytes/inodes afterward and record a compact initial cleanup checkpoint (owned paths cleaned and reclaimed size, or “nothing safely identifiable” with reason). The GitHub repo tree did **not** contain tracked `target/`/`.venv/`/build directories at creation; local untracked scratch cannot be inspected remotely. Do not claim cleanup happened before verifying it on the host.
4. Remove **stale local WIP artifacts only if experiment-owned and safely recovered**; keep `reports/data` compact authoritative CSV/report evidence and historical Git commits. Avoid deleting unrelated parked worktrees, branches, or historical evidence. `main` is off limits. Do not spend a whole session on cleanup/documentation; once safe, resume numerical implementation.

## Phase 1 — scalar-family convergence (finish before spin)

Use the already completed 2026-10-26 23:04 source inventory; **do not commission another broad inventory or historical code hunt**. Initial duplication targets:
- Ordinary scalar traversal: `legendre/scalar.rs` vs `legendre/scalar_f32.rs`.
- Paired scalar traversal: same precision owners, while `legendre/scalar_paired` owns genuine reflected/paired algorithms.

Carry forward the accepted `LegendrePrecision` policy, B36 shared scalar degree equation, selective B32 scaling, shared normalization/factor helpers, native-f32 paired scalar route, and responsibility-based paired/order-major modules. Preserve the optimized f64 ordinary sectoral seed restored in `f44821350d17`; preserve f64 optimized scaling, SIMD and order-major/paired leaves. The alternative shared f64 seed `66145f87`, generic scalar traversal B33/B34, and f64 lane-pair normalizer `e3b9b7fe` were screened/rejected or partially restored. **Do not reapply those exact forms.**

Choose a **coherent algorithm/stage ownership** and consolidate real shared recurrence traversal, state preparation/rotation and valid reusable fallback with compile-time numeric policies and narrow typed SIMD callbacks/leaves. Keep any distinct f32 normal-range skip and atomic paired synthesis transaction. Move correct tests, internal imports and `ShtScalar` trait implementations accordingly. A source move alone is not consolidation. After the actual duplicate algorithm bodies have been retired, remove `src/legendre/scalar_f32.rs` and its `mod scalar_f32` wiring (no compatibility shell, no wholesale renamed `f32.rs` copy). If a f64 fast traversal must remain specialized, document its route/performance reason, but keep it inside scalar mathematical ownership.

**Milestone A:** native f32/f64 scalar S/A/S*/A* work correctly on focused GL/CC/F1 exact, oversampled and reduced shapes, singular/polar/exceptional cases and reused/poisoned scratch, with identified important f64 timing cells protected. Count and report the independently maintained scalar algorithm bodies eliminated.

## Phase 2 — native-spin-family convergence

Targets: `legendre/spin.rs` vs `legendre/spin_f32.rs`: native-spin recurrence traversal and spin-analysis SIMD accumulation. Reuse the accepted shared spin factors, half-angle seed initializer, normalization formula and scaled-basis analysis fallback. Share the mathematically identical recurrence/state/control algorithm while retaining native f64 and f32 Pulp SIMD leaves, ±spin signs/coefficients, precision-specific guards and limited test-only profiling. Keep spin-1/2/other supported spin correctness and S/A/S*/A* adjoints. Do not conflate direct spin and scalar recurrence; do not automatically enable f32 paired-spin or derivative routes without independent tests, transactional fallback and evidence.

After the last duplicate whole-body implementation is replaced or cleanly specialized inside `spin`, remove `src/legendre/spin_f32.rs` and old module references, relocate tests without weakening them, and update `spin_fast.rs`/trait uses as needed.

**Milestone B:** f32/f64 direct spin and relevant converted scalar workflows pass focused mathematical/polar/nonfinite/adjoint/reuse controls; document each remaining specialized SIMD leaf and true precision difference.

## Mandatory test-layout follow-up (separate from numerical source changes)

**Owner decision 2026-10-27 02:19 UTC:** [One Rust unit-test layout](../docs/TEST_LAYOUT.md) is required. **No inline test bodies and no sibling `*_tests.rs` files** in the final Rust crate. For each production source module, use only `#[cfg(test)] mod tests;` with test bodies under its natural `<module>/tests.rs` path; large suites may put narrower test-only child modules under that test folder. Keep root `tests/*.rs` as Cargo integration tests. **Complete the current spin-family numerical consolidation first; only then mechanically migrate the existing internal tests in a separate commit without changing algorithms or numerical assertions.** Compare test counts/feature paths; do not benchmark pure test moves. Never call Sol.

## Validation/performance cadence

- Source-only relocation or a few precise lines: `cargo check --locked --lib`, fmt; no benchmark. Group related edits, avoid repeated full suites.
- Coherent algorithm family: `cargo check --locked --workspace --all-targets --all-features`, Clippy with `-D warnings`, focused correctness and independent reference/adjointhood tests in both precisions. Include no-default-features where relevant. Small Python binding tests only when their code paths/API change.
- Protected f64 hot-route changes: 3 interleaved exact-parent/candidate (and pinned DUCC when useful) whole-transform q1/q8 pairs for **the affected cells**. Guard examples: GL T1023 spin-1 synthesis q1/q8, T720 spin-1 analysis q1/q8, scalar T720 analysis/synthesis, over-resolved GL and selected CC spin synthesis. Expand to 15+ only if deciding an important ambiguous performance/regression question. Keep f64 specialized fast leaves if a repeatable slowdown appears. Never compare unmatched historical medians as proof of parity.
- At final convergence: broad Rust test/Clippy/format and relevant Python tests, representative GL/CC/F1, full/reduced, exact/oversampled, spin-0/1/higher relevant checks, two distinct adjoints, extreme and poisoned workspace, q1/q8 selected cumulative f64 and f32 benchmarks against immutable pre-refactor SHA `062218639bc2f097f004c617da0ce46a3a4d2941`. Report actual coverage and any known gaps; do not invent a green CI or precise timings.
- `docs/EXPERIMENT_HYGIENE.md` is binding: prefer a bounded owned `/tmp` scratch after bytes/inodes check; preserve+verify compact durable evidence and source SHAs; **immediately delete all experiment-owned builds, environments, caches, temp worktrees and profiling scratch** after each experiment, including failures; confirm reclaimed space. Do not stockpile compiled baseline/DUCC workers.

## Git, progress, token/quota discipline

- Work directly on `dev`; only main Luna writes it. Fetch/reconcile remote before pushes; never rewrite shared history, force-push, delete other agent's edits, or alter `main`. Keep commits coherent (code+focused test evidence; separate durable benchmark results as needed), and push frequently enough that quota exhaustion cannot lose a large unpushed set.
- Avoid verbose hourly reporting, duplicative docs-only commits, speculative refactors, stale status rewrites, and expensive repeated source scans. Update `reports/SHARED_KERNEL_REFACTOR_PROGRESS.md` with a **short new current-state section** and exact SHAs as true milestones occur; limit `reports/PROGRESS.md` to five rolling entries. Existing completed `src/pyspharm` migration and native-f32 paired scalar route are **not** tasks to redo.
- A failed numerical attempt: revert only its candidate source, commit/save minimal evidence of why it failed, and advance to another meaningful algorithm family. Do not get stuck re-profiling a 0.5–1% noise-level effect; do not force-shift work onto a Sol model.
- If the session nears quota or becomes unable to run: **no Sol substitution**. Commit/push verified source, preserve cleanup status and a short `NEXT` handoff with exact remaining functions/tests, and stop; never claim asynchronous work or successful cleanup without verification.

## Completion criteria

1. Neither `src/legendre/scalar_f32.rs` nor `src/legendre/spin_f32.rs` remains, nor a disguised whole-algorithm precision copy under another name.
2. Equivalent scalar and spin mathematical algorithms each have one maintained shared source; specialized typed SIMD and legitimately distinct precision policies are localized with explicit reasons.
3. Scalar/spin S/A/S*/A*, GL/CC/F1, supported spin, polar/extreme/repeated-workspace correctness is established by the tests actually run.
4. Important f64 benchmark wins remain; any f32 small trade-off is explicit and cumulative. Latest source, parent, pinned DUCC and immutable baseline provenance are unambiguous.
5. Source LOC/algorithm body duplication, route complexity and test coverage are meaningfully improved, not simply relocated. No new unnecessary runtime dependency.
6. Experiment artifacts and abandoned owned scratch are cleaned after saving durable evidence. **No Sol invocation.**
7. All verified implementation commits pushed directly to `dev`, `main` unchanged. Final response summarizes code changes, remaining risks, and tests/benchmarks, without inventing a finished release.

**Do the cleanup in Phase 0, then proceed immediately to algorithm-family work without waiting for further guidance.**
