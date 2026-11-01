# M6 — B1 scalar-analysis hot path

**Owner direction (2026-11-02): focus exclusively on B1 scalar analysis.** Improve native f32 against DUCC f32 and native f64 against DUCC f64; f32/f64 timing equality is not a target. B0, B2 and B3 are parked during this investigation. This supersedes the earlier B1-f64/B3-f64 execution order. See [AGENTS.md](../AGENTS.md), the [live parity ledger](../reports/DUCC_PARITY_M6.md), and the [current B1 zipped-loop evidence](../reports/archive/experiments/m6-b1-zip-loop-2026-11-02/README.md).

## Workload and acceptance

Use GL T720, full M720/reduced M180, q1/q8, warm complete transforms including FFT and scheduling. Pin DUCC to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Compare each Rust precision to matching DUCC on the same geometry, fixture, ISA, affinity and thread count. Three interleaved pairs screen; consequential claims require 15+ paired observations, uncertainty, hashes and output comparison. Do not slow f64 or weaken numerical tests to improve a ratio.

Current production source is `8560bcae34e0532a230b9a3e0d05cf10380cfccf`; it includes the zipped ordinary-loop improvement, first tested against exact parent `792c8f363ad73cd839a0df73965714b74623ac6e`. The 15-pair B1/DUCC matrix remains 2.10–2.72x for f32 and 1.03–1.13x for f64. The accepted zip change preserves outputs and has no resolved f64 regression. DUCC parity remains a gap. Results and provenance are in the [zip-loop evidence package](../reports/archive/experiments/m6-b1-zip-loop-2026-11-02/README.md).

## Profile findings and next work

Parent Callgrind full-q1 attributes 94.32% of f32 and 81.37% of f64 instructions to B1's symmetric order group; the fused paired block is 86.28%/59.81%, and FFT is 5.68%/18.61%. The zip-loop change reduces whole-call instruction count by 3.42%/4.82%; its source and assembly evidence are archived above. A follow-up nonzero phase probe at source `8560bcae` attributes 91–95% of f32 and 76–86% of f64 instrumented wall time to Legendre plus accumulation. Route counters show 266,685 scaled f32 pair calls at full order and 89,429 at reduced order; sampled exponents are nonzero in 81%/78% of full/reduced work. Hard recovery is uncommon by count (5,601 scalar lanes). f64 records 474/132 typed-seed declines at full/reduced order. These are work counts, not cycle shares. PMU events remain unavailable (`perf_event_paranoid=4`), and Callgrind Ir is not cycles.

An isolated exact f32 output-scale shortcut passed its helper bit-equivalence check and improved that synthetic helper benchmark, but the 3-pair whole-call intervals crossed 1.0 in every cell; the point estimate was slower in three f32 cells and the monomorphized function grew 5.8% with 52 additional branches. It is rejected and remains outside production. See the [scale-path profile and screen](../reports/archive/experiments/m6-b1-scale-output-fast-path-2026-11-02/README.md). The current evidence points to scaled recurrence/accumulation as the main f32 work, and to typed-seed decline/fallback as a possible f64 cost, but it does not assign elapsed cycles to either.

1. Keep the accepted zipped traversal; do not claim a resolved f64 timing gain. Its local correctness and build checks passed. The GitHub CLI is unauthenticated in this session, so workflow status was not refreshed for this diagnostic update.
2. On source `8560bcae`, microbenchmark the scaled pair step and exponent normalization separately for nonzero f32 orders, and isolate f64 typed-seed decline/fallback. Use non-overlapping measurements and inspect actual disassembly.
3. Form one bounded candidate only after a material avoidable cost is measured. Screen all four B1 cells and output correctness; preserve analytic, adjoint, high-order, exceptional, SIMD-tail and workspace-reset behavior.

## Engineering constraints

- Keep the shared generic f32/f64 algorithm and native SIMD/FFT paths; no f32-to-f64 transform backend or duplicate recurrence.
- Use isolated worktrees for risky candidate code. Preserve unrelated dirty worktrees and scratch; never reset or clean shared work.
- Promote only after full relevant Rust/Python checks, strict Clippy, formatting, CI, and protected numerical/performance controls. Push coherent work to the existing `dev` branch after fetching and reconciling remote updates.

## NEXT

**NEXT:** isolate the elapsed cost of scaled pair updates and exponent normalization in f32, and typed-seed decline/fallback in f64. The output-scale shortcut is rejected. Leave B0/B2/B3 untouched, and do not infer cycle savings from route counts or Callgrind Ir.
