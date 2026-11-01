# M6 — B1 scalar-analysis hot path

**Owner direction (2026-11-02): focus exclusively on B1 scalar analysis.** Improve native f32 against DUCC f32 and native f64 against DUCC f64; f32/f64 timing equality is not a target. B0, B2 and B3 are parked during this investigation. This supersedes the earlier B1-f64/B3-f64 execution order. See [AGENTS.md](../AGENTS.md), the [live parity ledger](../reports/DUCC_PARITY_M6.md), and the [current B1 zipped-loop evidence](../reports/archive/experiments/m6-b1-zip-loop-2026-11-02/README.md).

## Workload and acceptance

Use GL T720, full M720/reduced M180, q1/q8, warm complete transforms including FFT and scheduling. Pin DUCC to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Compare each Rust precision to matching DUCC on the same geometry, fixture, ISA, affinity and thread count. Three interleaved pairs screen; consequential claims require 15+ paired observations, uncertainty, hashes and output comparison. Do not slow f64 or weaken numerical tests to improve a ratio.

Current exact parent is `792c8f363ad73cd839a0df73965714b74623ac6e`. The zipped ordinary-loop candidate changes only SIMD slice traversal in `accumulate_two_normal_scalar_analysis_pairs`: f32 improves three cells with paired intervals below one, full q1 is inconclusive; f64 changes are inconclusive in all four cells and show no resolved regression. DUCC parity remains a gap. Results and provenance are in the linked evidence package.

## Profile findings and next work

Parent Callgrind full-q1 attributes 94.32% of f32 and 81.37% of f64 instructions to B1's symmetric order group; the fused paired block is 86.28%/59.81%, and FFT is 5.68%/18.61%. The candidate Callgrind run reduces whole-call Ir by 3.42%/4.82%. AVX2/FMA disassembly shows redundant per-vector slice bounds and repeated stack base loads in the parent ordinary kernel; zipped traversal emits one loop guard with the same FMA sequence. Hardware PMU events are unavailable (`perf_event_paranoid=4`), so Callgrind Ir is not cycles. Current evidence does not split recurrence, ordinary/scaled accumulation and exceptional fallback into elapsed-time shares; continue that diagnosis without double-counting inclusive phases.

1. Keep the zipped-loop change if current CI and focused numerical checks remain green; refresh the four-cell timing on a quieter host before claiming an f64 gain.
2. Profile representative nonzero f32/f64 T720 routes to separate ordinary recurrence/accumulation, scaled accumulation, exponent normalization, exceptional fallback, FFT and scheduling. Use paired microbenchmarks and disassembly; use PMU counters only if available.
3. Form one distinct, profile-backed B1 candidate at a time. Recheck all four B1 cells and output correctness; preserve analytic, adjoint, high-order, exceptional, SIMD-tail and workspace-reset behavior.

## Engineering constraints

- Keep the shared generic f32/f64 algorithm and native SIMD/FFT paths; no f32-to-f64 transform backend or duplicate recurrence.
- Use isolated worktrees for risky candidate code. Preserve unrelated dirty worktrees and scratch; never reset or clean shared work.
- Promote only after full relevant Rust/Python checks, strict Clippy, formatting, CI, and protected numerical/performance controls. Push coherent work to the existing `dev` branch after fetching and reconciling remote updates.

## NEXT

The zipped traversal is ready for `dev` after the final source/ledger reconciliation. Then profile scaled and exceptional B1 costs separately and repeat f64 whole-call measurements under quieter conditions. Leave B0/B2/B3 untouched.
