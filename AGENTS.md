# RustSHT — stable agent rules

## Authority

- Continue in the authorized **primary Luna session**. “Luna Max” and “GPT-6-Luna” denote this current main session, not a request to select, launch, or switch models.
- The main session is the sole production-code writer and runs essential reviews, tests and benchmarks itself. **No subagents**, including benchmark or reviewer agents. **No Sol / Sol High** in any role or as quota fallback.
- At quota/resource limits, preserve and push only verified work if possible, record a precise NEXT, safely clean owned inactive scratch, then stop.

## Sources of truth

1. This file: session policy, correctness, benchmark and Git safety.
2. [Active f64 scalar/spin-1 analysis performance plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md): **only active implementation direction**.
3. [M6 parity ledger](reports/DUCC_PARITY_M6.md) and [rolling progress](reports/PROGRESS.md): measured source-specific results.
4. [Archived parity coverage contract](reports/archive/plans/ducc-performance-parity-continuation-2026-10-27.md), [experiment hygiene](docs/EXPERIMENT_HYGIENE.md), [unit-test layout](docs/TEST_LAYOUT.md) as reference only. Old execution overrides and archived NEXT instructions are historical.

**Owner's durable direction (2026-11-01; supersedes the conflicting B1-only plan):** optimize **f64 scalar analysis (B1) and f64 spin-1 analysis (B3)** against the same pinned DUCC f64 backend, separately; both are active, but investigate one measured hypothesis at a time. Latest B1 f64/DUCC ratios in the 15-pair GL T720 cells are 1.059x/1.131x full q1/q8 and 1.047x/1.032x reduced q1/q8 (reduced cells INCONCLUSIVE); B3 f64/DUCC was 1.21–1.35x in older, source-specific 15-pair cells, so refresh only the needed B3 baseline. Give B1 no more than two bounded attempts at its remaining ordinary recurrence/seed/tail overhead; then concentrate on B3's larger gap, transferring only improvements that survive direct measurement. Do not treat the later-dated B1-only notes as a new owner instruction. **Do not chase f32/f64 timing equality or optimize f32 performance now.** Preserve native f32 correctness and f64/f32 shared algorithm integrity; f32 changes are justified only by an incidental correctness or CI blocker. B0 scalar synthesis and B2 spin-1 synthesis are parked. Use short f64 microbenchmarks, wall-time phase measurements, and actual AVX2/FMA disassembly before coding; hardware perf counters may be unavailable. No broad test/benchmark suites while exploring. The original DUCC-inspired numerical algorithms and their attribution stay intact. Preserve the retired f64-sidecar architecture, public API, numerical guards, and protected T1023 f64 synthesis.

## Numerical and engineering boundaries

- Maintain shared generic algorithms with narrow, justified precision-specific SIMD/numeric leaves; no duplicate maintained scalar/spin recurrences, broad unrelated rewrite, new runtime dependency, GPU/MPI feature, or Python API change.
- Preserve GL/CC/F1, full/reduced mmax, even/odd longitude/Nyquist, spin conventions, coefficients/phase, poles/equator, S/A/S*/A* adjoints, tiny/subnormal/overflow/nonfinite semantics, SIMD tails and poisoned/reused workspace safety.
- Never weaken tolerances, delete independent reference tests, or claim that bitwise parent equality proves mathematical correctness. Distinguish allowed private rounding differences from public numerical failures.
- Unit tests use natural `<module>/tests.rs` with only `#[cfg(test)] mod tests;` in production modules; integration tests remain under `tests/`. Earlier test-layout work is complete.

## Benchmark and acceptance

- Compare exact parent, candidate and pinned native **DUCC 0.42.0** (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) with matched geometry, inputs, compiler/ISA, affinity, threads and warm complete transforms.
- **Diagnosis:** use short f64 B1/B3 stage microbenchmarks and wall-time phase probes, plus available CPU counters and generated AVX2/FMA assembly. Run only affected tests during speculative experiments; do **not** repeat full Rust/Python suites, 16-cell matrices, broad numerical replays or 15-pair campaigns for every hypothesis. Use 3 interleaved f64 whole-call pairs to screen a credible change. **Promotion:** require 15+ exact-source f64 whole-call pairs for affected B1 or B3 full/reduced q1/q8 cells against parent and DUCC f64, uncertainty, provenance, focused f32 guards for shared code, relevant full Rust/Python/CI and exceptional checks, and protected f64 T1023 controls. Callgrind instructions are not hardware cycles.
- Inspect executed SIMD code and normalized useful work to explain gaps. Callgrind instructions are not cycles; separate whole-call time from diagnostic stages and never double-count inclusive costs.
- Preserve analytic, adjoint/sign, exceptional, tiny/subnormal, reset and SIMD-width/tail tests without weakening tolerances. Run focused guards during screening and the relevant complete validation before promotion; revisit independent high-precision analysis only for a demonstrated arithmetic change. Maintain exact source-specific B1/B3 f64/DUCC f64 PASS/GAP/INCONCLUSIVE statuses; report f32 and cross-precision figures only as historical/diagnostic evidence, not active gates.
- The B1/B3 f64 T720 full/reduced q1/q8 checks are an initial performance gate, not the entire supported SHT workload; do not optimize test fixtures at the expense of other grids and orders.
- Keep the executed f64 and native-f32 routes explicit in reports. Never blend source revisions or imply an f32 performance PASS from an f64-only candidate.
- A sidecar timing PASS, generic source syntax, profiling or green CI alone does not prove completion.

## Git, archives and disk safety

- Work on `dev`; inspect local worktrees/unpushed state and reconcile `origin/dev` before changes/pushes. No force-push, history rewrite, hard reset, automatic merge to `main`, or destruction of another agent's changes.
- Commit coherent tested implementation slices and separate behavior-preserving cleanup. Do not flood `dev` with trivial prose-only commits.
- Keep `AGENTS.md` stable and short; keep one concise active M6 plan, a current per-cell ledger and no more than five rolling progress checkpoints. Archive superseded plans and experiments with an index, source SHA, patches/checksums and conclusions. Verify unique evidence before deduplicating or deleting old files.
- Use bounded experiment-owned scratch after checking disk/inodes; preserve verified patches, compact data and provenance before deleting only confirmed inactive owned work. Never clear shared caches, generic `/tmp`, parked worktrees or unpushed code.
