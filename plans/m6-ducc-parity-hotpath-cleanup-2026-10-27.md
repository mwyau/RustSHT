# M6 — Native f32/f64 performance parity

**Owner priority, 2026-10-30:** whole-transform performance parity is the immediate objective. This is the only active M6 direction. See [AGENTS.md](../AGENTS.md) for session, correctness, benchmark and Git rules, and [the live ledger](../reports/DUCC_PARITY_M6.md) for per-cell status. Prior numerical-first work is archived at [m6-before-performance-priority-2026-10-30.md](../reports/archive/plans/m6-before-performance-priority-2026-10-30.md).

## Target

For B0 scalar synthesis, B1 scalar analysis, B2 spin-1 synthesis and B3 spin-1 analysis, on GL T720 full M720/reduced M180 and q1/q8, require each of these per-cell ratios to be at most 1.00:

- native Rust f32 / matched Rust f64;
- native Rust f32 / pinned DUCC f32;
- Rust f64 / pinned DUCC f64.

These are separate gates; one cell cannot offset another. Preserve established f64 performance, including T1023 spin-1 synthesis.

## Current status and order

The exact-source four-backend baseline is source `afd3df285dcb601371bb5b7ba1ec4fdc56ab1645`, DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, with three interleaved pairs over all 16 GL T720 cells. All f32 parity gates remain open. `47906d09` adds compact hard-scale vectorization for B1/B3; `a3cd8964` caches active scalar-fallback indices. Exact-source 15-pair B1/B3 results show bit-identical outputs and 19–28% lower whole-call time than `47906d09`; absolute ratios remain 2.10–3.70× Rust f64 and 2.86–4.96× DUCC f32. A shared-host contention caveat and complete per-cell intervals are recorded in the [live ledger](../reports/DUCC_PARITY_M6.md) and [candidate package](../reports/archive/experiments/current-source-scaled-analysis-profile-2026-10-30/candidate-sparse-fallback-index-15pair/README.md).

1. **B1 scalar analysis and B3 spin-1 analysis:** the sparse fallback lane-list change is integrated. Profile the residual fused recurrence block and identify the actual next instruction/work bottleneck; prefer changes to useful recurrence work or exceptional-lane execution over further bookkeeping. Use 15-pair whole-call confirmation and preserve the profile-based rejection history.
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

Inspect the residual fused B1/B3 recurrence work after `a3cd8964`, then test one materially different measured optimization. The lane-list change is 15-pair verified, bit-identical and integrated; its parity gaps remain open. Continue bounded B1/B3 work, then B2 and B0.
