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

1. **B1 scalar analysis:** continue with a new measured hypothesis and recheck the full-q8 current-binary control under lower host load. The B1 f32 analysis entrypoints remain unchanged by `094d3162`; do not claim performance non-regression while that interval crosses 1. B3's scaled-path fusion is measured and integrated; its V3 kernel grew substantially, so avoid another fusion without an assembly/code-size hypothesis.
2. **B2 spin-1 synthesis:** profile the reflected order-major/grouped-ring path and optimize measured recurrence or scheduling cost.
3. **B0 scalar synthesis:** optimize the isolated generic native-f32 kernel, validate it, then remove the f32-to-f64 production sidecar and obsolete fallback routes as one coherent change.

## Working loop

- Use an isolated worktree for risky arithmetic or scheduling changes. Keep runtime f32 inputs, recurrence, SIMD, accumulation, spectra, FFT and outputs native; do not add f32-to-f64 transform promotion or duplicate maintained recurrences.
- Profile the executed path and inspect generated AVX2/FMA and memory behavior before changing it. Optimize whole-call time, including FFT, workspace reset, scheduling and worker overhead. Callgrind instruction references are diagnostic counts, not cycles.
- Screen with three interleaved pairs. Use at least 15 pairs and paired uncertainty for consequential claims. Match geometry, widened f64 fixture, source, binaries, CPU/ISA, affinity and threads. Keep statistically unresolved cells INCONCLUSIVE.
- Record exact source, fixture and binary hashes, route, per-cell times/ratios and numerical guard status. Compare pinned native DUCC 0.42.0.
- Existing analytic, adjoint/sign, exceptional, pole, FFT, workspace-reset and independent-reference tests remain guardrails. Accuracy is adequate to continue performance work; do not resume broad binary128 exploration unless an optimization changes results. Never weaken tolerances or retain a known regression.
- Run formatting, strict Clippy, relevant Rust/Python tests and the protected f64 T1023 control before promotion. Commit coherent verified slices to `dev`, fetch/reconcile before push, and never rewrite history or merge `main`.

## Next

Profile the remaining B1 recurrence/exceptional-lane work and try a materially different, isolated optimization. Recheck the uncertain B1 full-q8 control under a quieter host. Keep B3's 15-pair gain and open DUCC parity gaps visible; proceed to B2 and B0 after bounded B1 work.
