# M6 — B1 scalar-analysis hot path

**Owner direction (2026-11-02): focus exclusively on B1 scalar analysis.** Improve native f32 against DUCC f32 and native f64 against DUCC f64; f32/f64 timing equality is not a target. B0, B2 and B3 are parked during this investigation. This supersedes the earlier B1-f64/B3-f64 execution order. See [AGENTS.md](../AGENTS.md), the [live parity ledger](../reports/DUCC_PARITY_M6.md), and the [current B1 zipped-loop evidence](../reports/archive/experiments/m6-b1-zip-loop-2026-11-02/README.md).

## Workload and acceptance

Use GL T720, full M720/reduced M180, q1/q8, warm complete transforms including FFT and scheduling. Pin DUCC to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Compare each Rust precision to matching DUCC on the same geometry, fixture, ISA, affinity and thread count. Three interleaved pairs screen; consequential claims require 15+ paired observations, uncertainty, hashes and output comparison. Do not slow f64 or weaken numerical tests to improve a ratio.

Current production head is `5483faa8f3d2035ba505a171885ad23b051d1b8c`. Its B1 `analysis.rs` and `transform/scalar.rs` blobs match `8560bcae`; the source includes the accepted zipped ordinary-loop traversal. The established 15-pair matrix remains 2.10–2.72x for f32 and 1.03–1.13x for f64. The zip change preserves outputs and has no resolved f64 regression. See the [zip-loop evidence package](../reports/archive/experiments/m6-b1-zip-loop-2026-11-02/README.md).

## Profile findings and next work

On the exact B1 source blob, a test-only full-q1 Callgrind profile attributes 263.7M Ir to scaled accumulation and 146.6M Ir to range checking/normalization, versus 22.9M Ir to recurrence advancement; factor materialization is 106.9M Ir nested in scaled accumulation. These are instruction counts, not cycles. A warm phase probe attributes 91–95% of f32 and 76–86% of f64 wall time to combined Legendre work; the separate FFT phase was 5–9% f32 and 13–22% f64 across q/thread cells, so those ranges are not additive. The host blocks `perf stat` and `perf record` (`perf_event_paranoid=4`).

Three isolated f32 candidates were rejected after 15-pair checks: adjacent-factor reuse, recurrence range-check fusion, and lazy hard-factor clearing. Their helper microbenchmarks improved 22% and 5.7% for factor preparation/clearing; all four adjacent/lazy whole-call intervals cross 1.0, while range fusion was tested only in full q8 and was unresolved and slower by point estimate. None is production code. The supplied starting ratios were not reproduced: the exact-source f32 refresh is 1.889–2.729× and the current f64 ledger is 1.032–1.131×. Source-identical runs used different fixtures/affinity observations, so this is not attributed to a code change. Region counts, V3 AVX2/FMA disassembly, focused checks, raw screens and patch hashes are in the [B1 hotspot package](../reports/archive/experiments/m6-b1-hotspot-profile-2026-11-01/README.md).

## Engineering constraints

- Keep the shared generic f32/f64 algorithm and native SIMD/FFT paths; no f32-to-f64 transform backend or duplicate recurrence.
- Use isolated worktrees for risky candidate code. Preserve unrelated dirty worktrees and scratch; never reset or clean shared work.
- Promote only after full relevant Rust/Python checks, strict Clippy, formatting, CI, and protected numerical/performance controls. Push coherent work to the existing `dev` branch after fetching and reconciling remote updates.

## NEXT

**NEXT:** use sampled wall timing on the nonzero GL T720 stream to separate f32 factor/state rebuild, scaled SIMD accumulation and pair range/normalization. Account for sampling overhead and keep FFT/scheduling disjoint. Select a materially different f32 candidate from elapsed-time evidence, screen with three interleaved whole calls, and stop early if no stable gain appears. Keep the 15-pair f64/DUCC comparison as a guard, leave B0/B2/B3 untouched, and do not repeat factor reuse, range fusion, hard-factor lazy clearing, output-scale, no-change-normalizer or seed-decline candidates.
