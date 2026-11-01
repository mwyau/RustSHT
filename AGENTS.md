# RustSHT — stable agent rules

## Authority

- Continue in the authorized **primary Luna session**. “Luna Max” and “GPT-6-Luna” denote this current main session, not a request to select, launch, or switch models.
- The main session is the sole production-code writer and runs essential reviews, tests and benchmarks itself. **No subagents**, including benchmark or reviewer agents. **No Sol / Sol High** in any role or as quota fallback.
- At quota/resource limits, preserve and push only verified work if possible, record a precise NEXT, safely clean owned inactive scratch, then stop.

## Sources of truth

1. This file: session policy, correctness, benchmark and Git safety.
2. [Active B1 scalar-analysis performance plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md): **only active implementation direction**.
3. [M6 parity ledger](reports/DUCC_PARITY_M6.md) and [rolling progress](reports/PROGRESS.md): measured source-specific results.
4. [Archived parity coverage contract](reports/archive/plans/ducc-performance-parity-continuation-2026-10-27.md), [experiment hygiene](docs/EXPERIMENT_HYGIENE.md), [unit-test layout](docs/TEST_LAYOUT.md) as reference only. Old execution overrides and archived NEXT instructions are historical.

**Owner's durable direction:** maintain a shared, optimized SHT implementation with native f32/f64 arithmetic, SIMD, FFT and storage. **Sole immediate focus: B1 scalar analysis.** Investigate where the extra execution time comes from with focused microbenchmarks, cycle profiles and hot-path disassembly. **Optimize f64 first when a shared algorithm/dispatch bottleneck is measurable**, otherwise tackle the dominant f32 scaled recurrence/accumulation cost. Compare Rust f32 with DUCC f32 and Rust f64 with DUCC f64 separately. **Rust f32/f64 timing is diagnostic only, not a 1:1 acceptance gate**; never slow f64 to improve it. B1 currently measures 3.59–4.96x DUCC f32, f64 1.18–1.37x DUCC f64 (one inconclusive) and f32/f64 2.57–3.70x. B0 scalar synthesis is accepted/parked; B3 and B2 are parked until B1 has a profile-backed, bounded result. Preserve numerical guards, the protected f64 T1023 path, the retired-sidecar architecture and source-specific evidence. Do not repeat rejected B1 fusion/tile/scale, B2 accumulator or direct-output variants unchanged.

## Numerical and engineering boundaries

- Maintain shared generic algorithms with narrow, justified precision-specific SIMD/numeric leaves; no duplicate maintained scalar/spin recurrences, broad unrelated rewrite, new runtime dependency, GPU/MPI feature, or Python API change.
- Preserve GL/CC/F1, full/reduced mmax, even/odd longitude/Nyquist, spin conventions, coefficients/phase, poles/equator, S/A/S*/A* adjoints, tiny/subnormal/overflow/nonfinite semantics, SIMD tails and poisoned/reused workspace safety.
- Never weaken tolerances, delete independent reference tests, or claim that bitwise parent equality proves mathematical correctness. Distinguish allowed private rounding differences from public numerical failures.
- Unit tests use natural `<module>/tests.rs` with only `#[cfg(test)] mod tests;` in production modules; integration tests remain under `tests/`. Earlier test-layout work is complete.

## Benchmark and acceptance

- Compare exact parent, candidate and pinned native **DUCC 0.42.0** (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) with matched geometry, inputs, compiler/ISA, affinity, threads and warm complete transforms.
- **Diagnosis:** use short representative B1 microbenchmarks, perf cycle/counter profiles and generated assembly. Run only changed-path tests for speculative candidates; do **not** spend exploration time on the full Rust/Python suites, all 16 cells, extensive numerical replays or repeated 15-pair campaigns. Screen promising changes with 3 interleaved whole-call pairs. **Promotion:** require 15+ whole-call pairs for affected B1 full/reduced q1/q8 f32/f64 vs exact parent and corresponding DUCC, paired uncertainty, source/fixture/binary hashes, relevant full Rust/Python/CI and exceptional regression checks, and protected f64 T1023 controls. Instruction counts are not cycles.
- Inspect executed SIMD code and normalized useful work to explain gaps. Callgrind instructions are not cycles; separate whole-call time from diagnostic stages and never double-count inclusive costs.
- Preserve existing analytic, adjoint/sign, exceptional and reset tests and tolerances. Use focused guards during screening and the relevant complete validation before integration; revisit binary128 only for a demonstrated arithmetic issue. Maintain exact source-specific evidence and precision-matched DUCC PASS/GAP/INCONCLUSIVE statuses. Keep f32/f64 as a diagnostic observation, **not** a PASS/GAP; B0 accepted/parked does not mean f32/f64 parity.
- The 16-cell GL T720 matrix is a first gate, not the full supported f32/f64 parity scope.
- Current-source evidence must identify each f32 route; do not blend native production results with conversion-based diagnostics.
- A sidecar timing PASS, generic source syntax, profiling or green CI alone does not prove completion.

## Git, archives and disk safety

- Work on `dev`; inspect local worktrees/unpushed state and reconcile `origin/dev` before changes/pushes. No force-push, history rewrite, hard reset, automatic merge to `main`, or destruction of another agent's changes.
- Commit coherent tested implementation slices and separate behavior-preserving cleanup. Do not flood `dev` with trivial prose-only commits.
- Keep `AGENTS.md` stable and short; keep one concise active M6 plan, a current per-cell ledger and no more than five rolling progress checkpoints. Archive superseded plans and experiments with an index, source SHA, patches/checksums and conclusions. Verify unique evidence before deduplicating or deleting old files.
- Use bounded experiment-owned scratch after checking disk/inodes; preserve verified patches, compact data and provenance before deleting only confirmed inactive owned work. Never clear shared caches, generic `/tmp`, parked worktrees or unpushed code.
