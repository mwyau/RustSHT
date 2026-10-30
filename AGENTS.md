# RustSHT — stable agent rules

## Authority

- Continue in the authorized **primary Luna session**. “Luna Max” and “GPT-6-Luna” denote this current main session, not a request to select, launch, or switch models.
- The main session is the sole production-code writer and runs essential reviews, tests and benchmarks itself. **No subagents**, including benchmark or reviewer agents. **No Sol / Sol High** in any role or as quota fallback.
- At quota/resource limits, preserve and push only verified work if possible, record a precise NEXT, safely clean owned inactive scratch, then stop.

## Sources of truth

1. This file: session policy, correctness, benchmark and Git safety.
2. [Active native-f32/f64 M6 plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md): **only active implementation direction**.
3. [M6 parity ledger](reports/DUCC_PARITY_M6.md) and [rolling progress](reports/PROGRESS.md): measured source-specific results.
4. [Archived parity coverage contract](reports/archive/plans/ducc-performance-parity-continuation-2026-10-27.md), [experiment hygiene](docs/EXPERIMENT_HYGIENE.md), [unit-test layout](docs/TEST_LAYOUT.md) as reference only. Old execution overrides and archived NEXT instructions are historical.

**Owner's durable direction:** one shared optimized SHT algorithm family specialized for **native f32 and f64** arithmetic, SIMD, FFT and storage. **Immediate order:** implement scalar analysis, spin-1 synthesis and spin-1 analysis using the optimized f64 algorithmic paths, as native f32. First measure corresponding f64 T720 whole-call times; interim gate is **f32/f64 <=2.0 individually per full/reduced q1/q8 cell** (stretch <=1.5). This is **not** a DUCC parity or numerical-accuracy PASS. Defer detailed scalar-synthesis high-degree compensation until all three families have native candidate routes and interim measurements. Keep the temporary production f32-to-f64 sidecar only until an independently correct native replacement is ready; do not expand it. Remove private legacy f32 routes without compatibility obligations only after validated coverage. Preserve numerical exceptional/polar/scaled handling and the public API.

## Numerical and engineering boundaries

- Maintain shared generic algorithms with narrow, justified precision-specific SIMD/numeric leaves; no duplicate maintained scalar/spin recurrences, broad unrelated rewrite, new runtime dependency, GPU/MPI feature, or Python API change.
- Preserve GL/CC/F1, full/reduced mmax, even/odd longitude/Nyquist, spin conventions, coefficients/phase, poles/equator, S/A/S*/A* adjoints, tiny/subnormal/overflow/nonfinite semantics, SIMD tails and poisoned/reused workspace safety.
- Never weaken tolerances, delete independent reference tests, or claim that bitwise parent equality proves mathematical correctness. Distinguish allowed private rounding differences from public numerical failures.
- Unit tests use natural `<module>/tests.rs` with only `#[cfg(test)] mod tests;` in production modules; integration tests remain under `tests/`. Earlier test-layout work is complete.

## Benchmark and acceptance

- Compare exact parent, candidate and pinned native **DUCC 0.42.0** (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) with matched geometry, inputs, compiler/ISA, affinity, threads and warm complete transforms.
- Screen with 3 interleaved pairs; use 15+ for consequential promotion. Report per-cell q1/q8, full/reduced f32 plus adjacent controls, exact source/fixture/binary hashes and credible uncertainty. Protect f64 GL T1023 spin-1 synthesis q1/q8 and established f64 wins.
- Inspect executed SIMD code and normalized useful work to explain gaps. Callgrind instructions are not cycles; separate whole-call time from diagnostic stages and never double-count inclusive costs.
- A breadth-first native-f32 candidate must pass baseline analytic, adjoint/sign, exceptional and reset checks **without weakened tolerances**. Document known high-degree numerical GAPS explicitly; do not promote them as accepted production routes. After all three families are instantiated, finish independent high-degree numerical convergence and final native-f32 <=1.0x DUCC parity. Maintain one live per-cell PASS/GAP/INCONCLUSIVE/UNMEASURED ledger, with **f32/f64 interim status separate from DUCC and accuracy**.
- The 16-cell GL T720 matrix is a first gate, not the full supported f32/f64 parity scope.
- Current-source evidence must identify each f32 route; do not blend native production results with conversion-based diagnostics.
- A sidecar timing PASS, generic source syntax, profiling or green CI alone does not prove completion.

## Git, archives and disk safety

- Work on `dev`; inspect local worktrees/unpushed state and reconcile `origin/dev` before changes/pushes. No force-push, history rewrite, hard reset, automatic merge to `main`, or destruction of another agent's changes.
- Commit coherent tested implementation slices and separate behavior-preserving cleanup. Do not flood `dev` with trivial prose-only commits.
- Keep `AGENTS.md` stable and short; keep one concise active M6 plan, a current per-cell ledger and no more than five rolling progress checkpoints. Archive superseded plans and experiments with an index, source SHA, patches/checksums and conclusions. Verify unique evidence before deduplicating or deleting old files.
- Use bounded experiment-owned scratch after checking disk/inodes; preserve verified patches, compact data and provenance before deleting only confirmed inactive owned work. Never clear shared caches, generic `/tmp`, parked worktrees or unpushed code.
