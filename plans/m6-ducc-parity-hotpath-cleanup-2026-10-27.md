# M6 — Native f32/f64 performance parity

**Owner priority, 2026-10-30:** whole-transform performance parity is the immediate objective. This is the only active M6 direction. See [AGENTS.md](../AGENTS.md) for session, correctness, benchmark and Git rules, and [the live ledger](../reports/DUCC_PARITY_M6.md) for per-cell status. Prior numerical-first work is archived at [m6-before-performance-priority-2026-10-30.md](../reports/archive/plans/m6-before-performance-priority-2026-10-30.md).

## Target

For B0 scalar synthesis, B1 scalar analysis, B2 spin-1 synthesis and B3 spin-1 analysis, on GL T720 full M720/reduced M180 and q1/q8, require each of these per-cell ratios to be at most 1.00:

- native Rust f32 / matched Rust f64;
- native Rust f32 / pinned DUCC f32;
- Rust f64 / pinned DUCC f64.

These are separate gates; one cell cannot offset another. Preserve established f64 performance, including T1023 spin-1 synthesis.

## Current status and order

The original four-backend baseline is source `afd3df285dcb601371bb5b7ba1ec4fdc56ab1645`, pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, three interleaved pairs over all 16 GL T720 cells. All parity gates remain open. `47906d09` adds compact hard-scale vectorization; `a3cd8964` caches sparse fallback indices. B3's one-channel scaled recurrence fusion is integrated at `094d3162`: exact output and 6.3–9.3% whole-call gain versus `95bcbecc` in all four B3 cells, with 15-pair intervals below 1. It still runs 2.64–2.99× DUCC f32. B1 full q8's current-binary control is 1.037 [1.000, 1.112] versus `95bcbecc`, INCONCLUSIVE; the short f64 control is also inconclusive. Details are in the [live ledger](../reports/DUCC_PARITY_M6.md) and [B3 package](../reports/archive/experiments/m6-b3-scaled-recurrence-fusion-2026-10-30/README.md).

1. **B0 scalar synthesis — first, largest observed gap:** the current-source four-backend fixture declines the f64 sidecar and runs an 11.9–19.1x DUCC native-f32 fallback. The archived isolated generic native-f32 candidate measured approximately 1.00–1.26x DUCC on a separate source/fixture. Reproduce both on **identical current-source T720 full/reduced q1/q8 fixtures**; verify executed dispatch and existing numerical, exceptional and reset guards. If validated, integrate the shared native-f32 kernel and benchmark 15+ pairs before retiring the temporary sidecar. Do not mistake an old isolated candidate's timing or accuracy for current production acceptance.
2. **B1 scalar analysis:** after bounded B0 work, resume a materially different, measured recurrence/exceptional-lane hypothesis. Recheck the full-q8 whole-binary control under lower host load; `1.037 [1.000, 1.112]` versus `95bcbecc` is INCONCLUSIVE, not a confirmed regression or non-regression.
3. **B3 spin-1 analysis:** retain the verified 6.3–9.3% fused-recurrence gain, but avoid further fusion without an assembly, code-size and memory-traffic hypothesis: the V3 kernel expanded substantially. Close its remaining 2.64–2.99x DUCC f32 gap with new measured approaches.
4. **B2 spin-1 synthesis:** profile reflected order-major/grouped-ring recurrence and scheduling. It is nearer parity (1.59–1.83x DUCC f32) but still misses all 1:1 gates.

## Working loop

- Use an isolated worktree for risky arithmetic or scheduling changes. Keep runtime f32 inputs, recurrence, SIMD, accumulation, spectra, FFT and outputs native; do not add f32-to-f64 transform promotion or duplicate maintained recurrences.
- Profile the executed path and inspect generated AVX2/FMA and memory behavior before changing it. Optimize whole-call time, including FFT, workspace reset, scheduling and worker overhead. Callgrind instruction references are diagnostic counts, not cycles.
- Screen with three interleaved pairs. Use at least 15 pairs and paired uncertainty for consequential claims. Match geometry, widened f64 fixture, source, binaries, CPU/ISA, affinity and threads. Keep statistically unresolved cells INCONCLUSIVE.
- Record exact source, fixture and binary hashes, route, per-cell times/ratios and numerical guard status. Compare pinned native DUCC 0.42.0.
- Existing analytic, adjoint/sign, exceptional, pole, FFT, workspace-reset and independent-reference tests remain guardrails. Accuracy is adequate to continue performance work; do not resume broad binary128 exploration unless an optimization changes results. Never weaken tolerances or retain a known regression.
- Run formatting, strict Clippy, relevant Rust/Python tests and the protected f64 T1023 control before promotion. Commit coherent verified slices to `dev`, fetch/reconcile before push, and never rewrite history or merge `main`.

## Next

First reproduce the current B0 native-f32 fallback and archived generic candidate on the same T720 fixtures, then test and integrate the validated shared native kernel if the expected large performance gain holds; retain the f64 sidecar until full replacement coverage. Next recheck B1 full q8 under a quiet host and pursue new measured B1/B3 bottlenecks, then B2. Keep all three per-cell parity gates open until 15+ matched pairs support a pass.
