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

1. **B0 scalar synthesis — integrated native route, parity remains open:** `179835a5` enables the shared order-major/reflected algorithm for f32 and adds exact eight-lane scale and signed-zero basis leaves. On 15 matched GL T720 pairs it cuts the previous fallback by 91–94%. All four cells still miss f32/f64 parity (1.40–1.64x); full/reduced q1/q8 miss DUCC f32 by 1.015–1.325x. Rust f64/DUCC f64 passes in all four cells. Keep the guarded sidecar until broader exceptional/grid coverage is complete, then remove it coherently. Details and caveats are in the [B0 package](../reports/archive/experiments/m6-b0-native-scalar-synthesis-2026-10-31/README.md).
2. **B1 scalar analysis — bounded work complete for now:** a quieter 15-pair full-q8 control versus `95bcbecc` measured candidate/parent `1.060 [1.039, 1.099]`, with bitwise-equal output. Two recurrence-fusion shapes were slower and expanded generated code. Do not repeat them; reopen B1 only for a different measured bottleneck.
3. **B3 spin-1 analysis:** retain the verified 6.3–9.3% fused-recurrence gain. Avoid further kernel growth without assembly/code-size and memory-traffic evidence; its V3 kernel expanded substantially. Close the remaining 2.64–2.99x DUCC f32 gap with a distinct measured approach.
4. **B2 spin-1 synthesis — bounded output-dataflow screen rejected:** a three-pair direct q8 order-column write candidate matched parent output exactly but showed no repeatable whole-call gain: candidate/parent medians were 1.002 full q1, 0.996 full q8, 1.013 reduced q1, and 1.003 reduced q8. Preserve the [screen package](../reports/archive/experiments/m6-b2-scaled-accumulator-screens-2026-10-31/direct-order-output/README.md); do not repeat the scratch-copy change. The existing B2 route remains 1.59–1.83x DUCC f32. Continue only from a materially different profile-backed hypothesis.

## Working loop

- Use an isolated worktree for risky arithmetic or scheduling changes. Keep runtime f32 inputs, recurrence, SIMD, accumulation, spectra, FFT and outputs native; do not add f32-to-f64 transform promotion or duplicate maintained recurrences.
- Profile the executed path and inspect generated AVX2/FMA and memory behavior before changing it. Optimize whole-call time, including FFT, workspace reset, scheduling and worker overhead. Callgrind instruction references are diagnostic counts, not cycles.
- Screen with three interleaved pairs. Use at least 15 pairs and paired uncertainty for consequential claims. Match geometry, widened f64 fixture, source, binaries, CPU/ISA, affinity and threads. Keep statistically unresolved cells INCONCLUSIVE.
- Record exact source, fixture and binary hashes, route, per-cell times/ratios and numerical guard status. Compare pinned native DUCC 0.42.0.
- Existing analytic, adjoint/sign, exceptional, pole, FFT, workspace-reset and independent-reference tests remain guardrails. Accuracy is adequate to continue performance work; do not resume broad binary128 exploration unless an optimization changes results. Never weaken tolerances or retain a known regression.
- Run formatting, strict Clippy, relevant Rust/Python tests and the protected f64 T1023 control before promotion. Commit coherent verified slices to `dev`, fetch/reconcile before push, and never rewrite history or merge `main`.

## Next

B0 native order-major f32 is integrated at `179835a5`; it is 91–94% faster than fallback but remains above both f32 parity targets. Retain its guarded sidecar while completing replacement coverage. B1's quieter full-q8 control and bounded fusion attempts found no promotable gain; keep B3 fusion without kernel growth absent new assembly evidence. B2's direct-output screen was neutral. Return to B0 with a measured source-level hypothesis, then continue B1/B2 only when a different dominant cost is identified. Keep all parity gates open until paired intervals support per-cell passes.
