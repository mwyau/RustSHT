# RustSHT — active agent instructions

This file is the **single concise entrypoint** for the current agent. Do not treat old prompts, superseded overrides, or historical benchmark reports as new instructions.

## 1. Main session and model policy

- **Continue in the already-running primary Luna session.** The owner has referred to this session as **Luna Max** and **GPT-6-Luna**. These are identifiers for the authorized current main session, **not** instructions to select, launch, verify, rename, or switch models. The session already running is allowed to edit production code, test, benchmark, commit, and push.
- **No Sol whatsoever:** never invoke Sol, Sol High, Sol reviewers, or any Sol-based fallback, whether directly, through tools, or as a subagent.
- **Do not spawn or delegate to any other agent**, including a reviewer or a benchmark worker. Run essential reviews, experiments, and benchmarks yourself in the primary session, unless the owner explicitly changes this rule in a future instruction. Older permissions for optional agents are superseded here.
- If the current session reaches its quota or cannot continue, save verified results, push verified commits if possible, clean only owned inactive scratch, record an exact NEXT handoff, and **stop**. Do not search for a substitute model or promise unattended work.

## 2. Sources of truth and current goal

Read in this order:
1. This file for **session/model, safety, Git, and acceptance rules**.
2. [Current M6 execution plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md) for the active objective and next experiments.
3. [Single M6 performance ledger](reports/DUCC_PARITY_M6.md) and [rolling progress](reports/PROGRESS.md) for the **latest source-specific evidence**.
4. [Parity matrix contract](plans/ducc-performance-parity-continuation-2026-10-27.md), [experiment hygiene](docs/EXPERIMENT_HYGIENE.md), and [test layout](docs/TEST_LAYOUT.md) as needed.

**M6 is open.** Achieve verified DUCC performance parity without sacrificing numerical correctness, shared f32/f64 algorithm ownership, or important existing f64 wins. The earlier shared-algorithm and test-layout refactors are finished; do not restart them. An old report's “current HEAD”, “next”, or “goal complete” is not authoritative over the live plan and ledger.

**Current evidence (2026-10-28, source-specific):**
- Accepted: native-f32 scaled-fallback packet SIMD for scalar synthesis; bounded SIMD accumulation for f32 scalar analysis (about 16–26% faster than its parent).
- Rejected: native-f32 scaled-prefix/fused-suffix variants that regress performance, fail guarded range handling, or harm adjacent analysis. Do not retry these unchanged. Native f32 cannot naively fold large negative recurrence exponents without losing representable contributions.
- Promising but **not promoted**: shared-kernel reflected-ring **f32 scalar synthesis** on earlier parent `49295a89`. The 15-pair isolated screen cut runtime about 40–65% across GL T720 full/reduced q1/q8, but it needs a fresh current-source build, numerical matrix, adjacent f32 controls, and protected f64 controls before any merge into production code.
- The recent native-f32 GL T720 spin-1 synthesis baseline remains about **35–51× DUCC** across full/reduced q1/q8. A **dual-channel reflected spin-1** leaf reusing the existing hemisphere scheduler and shared recurrence is a **hypothesis only**. The unconditional paired-scalar adapter regressed q8 and is rejected. Simply enabling `supports_paired_spin()` will not activate the intended f32 T720 route.
- The next action should follow the most recent M6 plan/ledger and any **verified in-flight work**. Resolve current experiments before starting overlapping ones. Do not confuse an isolated patch or a route census with an accepted production improvement.

Pinned DUCC reference: **0.42.0**, commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Pre-refactor comparison baseline: `062218639bc2f097f004c617da0ce46a3a4d2941`.

## 3. Code and numerical boundaries

- Maintain **shared generic f32/f64 numerical algorithms** with limited, genuinely useful precision-specific SIMD/policy leaves. No duplicated maintained scalar/spin recurrence implementation or broad rewrite. Preserve established f64 scalar seed/normalizer behavior, reflected routes, and T1023 spin-1 synthesis performance.
- Preserve correctness on GL/CC/F1, full/reduced `mmax`, odd/even longitude and Nyquist, spins and signed conventions, S/A/S*/A* adjoints, poles/equator, tiny/subnormal/overflow/nonfinite values, partial SIMD vectors, and repeated/poisoned workspace with transactional decline/reset.
- **Never relax numerical tolerances, remove controls, or claim that parent equivalence establishes independent accuracy.** Use analytic/reference/adjoint/extreme-value tests appropriate to the changed path.
- No new runtime dependency, GPU/MPI feature, unrelated FFT rewrite, new public Python API, or cleanup disguised as an optimization.
- Unit tests live in natural `<module>/tests.rs` with only `#[cfg(test)] mod tests;` in production modules. Cargo integration tests remain in `tests/`. The migration is already complete.

## 4. Performance proof and reporting

- For a candidate, pin the exact **parent SHA, candidate source/patch, and DUCC SHA**; compare identical fixtures, ISA/compiler settings, affinity, warm-call boundaries, and q1/q8 thread settings. Time **whole transforms** and, where needed, the Legendre stage separately.
- Begin with three interleaved matched parent/candidate/DUCC pairs; extend material or ambiguous acceptance decisions to **15+ paired comparisons**. Always report full and reduced GL T720 native-f32 cases affected by the change, adjacent scalar/spin paths, and protected **f64 GL T1023 spin-1 synthesis q1/q8**. Additional shapes and exceptional controls are required when generalizing a route.
- Use useful recurrence/contribution counts, Callgrind and emitted AVX2/FMA code to explain **why** a change works or fails. Do not equate instruction references with cycles, double-count inclusive costs, or treat test-only timing as production timing.
- Reject repeatable regressions and unsafe numerical transitions even if the target workload improves. Keep failed candidates and their evidence reproducible; do not silently change precision policy.
- Maintain **one live per-cell ledger**, `reports/DUCC_PARITY_M6.md`, using exact revision and PASS / GAP / INCONCLUSIVE / UNMEASURED status. Keep `reports/PROGRESS.md` to five substantive checkpoints. Neither green CI, profiling, nor a single promising workload completes M6.

## 5. Git, storage, and exit discipline

- **Work on `dev` only.** Inspect local status/worktrees and `origin/dev`; fetch and reconcile concurrent remote commits before pushing. **No force-push, history rewrite, hard reset, automatic merge into `main`, or destruction of another session's work.**
- Commit and push **coherent, verified implementation slices** frequently, with meaningful source/test/evidence provenance. Avoid a stream of tiny documentation-only commits. Keep behavioral cleanup separate from numerical optimization.
- Give each experiment isolated, size-bounded build/benchmark scratch; check available disk/inodes. Preserve verified source patches, compact numeric results, hashes, and reports before removing **only owned, inactive** build directories/worktrees. Never clear `/tmp` broadly, shared caches, parked work, or unpushed changes. Follow `docs/EXPERIMENT_HYGIENE.md`.
- On blocker/quota: checkpoint actual results, tests, remaining risks and exact next action; push verified work, clean safely, then stop. Do not manufacture benchmark results or defer work to an unauthorized agent.
