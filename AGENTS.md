# RustSHT agent instructions

## Authority and active work

- Continue in the authorized primary Luna session. Never switch models or use Sol.
- Do not spawn or delegate to agents. The primary session owns implementation, tests and benchmarks.
- Work toward the durable goal: one shared optimized SHT algorithm family with native f32 and f64 execution and verified DUCC parity.
- The only active execution plan is [M6 native-generic DUCC parity](plans/m6-native-generic-ducc-parity.md).
- The live per-cell evidence is [DUCC_PARITY_M6.md](reports/DUCC_PARITY_M6.md); recent checkpoints are in [PROGRESS.md](reports/PROGRESS.md).
- Historical plans and benchmark results are context only. Use [reports/archive/README.md](reports/archive/README.md) to find them; do not follow old NEXT steps.

## Numerical and implementation rules

- f32 transforms must keep f32 input, recurrence, accumulation, FFT and output. Do not use f64 transform plans, promoted runtime arrays or conversion sidecars.
- Share mathematical control flow and recurrence ownership across f32/f64. Use precision-specific policies and typed SIMD leaves where they preserve correctness and measured performance.
- Remove obsolete f32 fallback algorithms and conversion machinery when their shared native replacement is ready. Do not retain compatibility paths for private implementations.
- Preserve public transform operations, conventions, grids, S/A/S*/A* adjoints, poles, extremes, tails and transactional workspace behavior.
- Keep precision-appropriate scaling. Never drop representable contributions, weaken tolerances, remove controls or treat parent equivalence as independent accuracy evidence.
- Keep unit tests under the natural module `tests.rs` with `#[cfg(test)] mod tests;`; integration tests stay in `tests/`.
- Do not add runtime dependencies, public APIs, unrelated FFT rewrites or instrumentation to production hot loops.

## Performance evidence

- Compare against pinned DUCC 0.42.0, commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Pin source and patch hashes, compiler/ISA, fixtures, affinity, thread count and warm-call boundary. Measure whole transforms; profile stages only to explain measured results.
- Start with three interleaved parent/candidate/DUCC pairs. Extend material or ambiguous decisions to at least 15 pairs.
- First focused gate: all 16 native-f32 GL T720 scalar/spin-1 synthesis/analysis
  full/reduced q1/q8 cells; broaden shapes and grids afterward.
- The focused gate is a first milestone, not the full end state; verify the
  broader supported f32/f64 workload matrix before closing M6.
- Record each required cell as PASS, GAP, INCONCLUSIVE or UNMEASURED. Include native precision and conversion route.
- Protect established f64 wins, especially GL T1023 spin-1 synthesis q1/q8. Do not infer parity from one workload or an isolated kernel.

## Git and experiment hygiene

- Work on `dev`. Fetch and reconcile `origin/dev` before pushing. Never force-push, rewrite history, reset shared work or merge into `main`.
- Inspect status, worktrees and live processes before experiments. Do not overwrite or delete untracked work or another worktree whose ownership is unclear.
- Give each experiment a fresh bounded scratch root; keep targets, caches, binaries and profiles there. Check disk and inode capacity first.
- Preserve compact evidence and verify it before removing only experiment-owned inactive scratch. Follow [EXPERIMENT_HYGIENE.md](docs/EXPERIMENT_HYGIENE.md).
- Commit coherent verified slices. If work must stop, record exact source, validation, risks and the next action; do not claim the goal is complete unless every requirement is verified.
- Green tests, profiling, a sidecar timing pass or generic source conversion alone do not establish M6 completion.
