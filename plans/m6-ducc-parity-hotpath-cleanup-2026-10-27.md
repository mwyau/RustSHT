# M6 — B1 scalar-analysis performance investigation

**Owner direction (2026-10-31): B1 scalar analysis only.** Diagnose and improve the algorithm's execution cost before returning to B3 or B2. Do not optimize already-accepted B0 scalar synthesis or chase Rust f32/Rust f64 1:1 timing. This is the **only active M6 execution plan**. Read [AGENTS.md](../AGENTS.md), and preserve measurements in the [live ledger](../reports/DUCC_PARITY_M6.md). The [previous three-way-parity plan](../reports/archive/plans/m6-three-way-parity-before-b1-focus-2026-10-31.md) is archived and no longer authoritative.

## Goals and evidence

Target **Rust f32/DUCC f32 ≤1.0x** and **Rust f64/DUCC f64 ≤1.0x**, separately, for B1 on GL T720 full M720/reduced M180, q1/q8. These remain eventual precision-matched performance targets, not a mandate to reach parity in one iteration. **Rust f32/Rust f64 is diagnostic only**, never a parity gate; do not slow f64 to improve the ratio. Optimizing f64 first is explicitly encouraged if a shared algorithmic cost is found.

| GL T720 B1 | f32/f64 (diagnostic) | f32/DUCC f32 | f64/DUCC f64 |
|---|---:|---:|---:|
| Full q1 | 3.564x | 4.796x | 1.310x |
| Full q8 | 3.337x | 4.419x | 1.273x |
| Reduced q1 | 2.903x | 3.779x | 1.278x |
| Reduced q8 | 2.665x | 3.373x | 1.226x |

These are the latest simultaneous 15-pair GL T720 screen: native f32 candidate, native f64, and precision-matched DUCC. The f32 candidate uses source `ecb66f10` plus the result-short-circuit patch in the [2026-11-01 profile package](../reports/archive/experiments/m6-b1-analysis-normalizer-result-short-circuit-2026-11-01/README.md). The ratios are geometric paired means; all four f32/DUCC and f64/DUCC cells remain GAP. A separate 15-pair f64 parent/candidate screen is INCONCLUSIVE in all cells and bitwise equal. Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Investigate before optimizing

1. **Confirm actually executed paths.** Follow f32 and f64 B1 dispatch and work allocation through Fourier FFT, Legendre/paired recurrence, normalization, scaled/ordinary accumulation, tails and reductions. Read matched DUCC code; identify exact hot loops.
2. **Measure cycles, not just instructions.** Use short `perf stat`/`perf record` profiles (cycles, instructions, cache/branch misses when available) and inspect `objdump`/`llvm-objdump` or `cargo asm` for AVX2/FMA, divisions, scalar fallback, register spills, redundant scaling, code size and memory pressure. If perf is restricted, use targeted timings plus assembly. Keep host load and CPU affinity controlled; do not interpret Callgrind Ir as elapsed time.
3. **Build representative focused microbenchmarks** to isolate sectoral initialization, recurrence advancement, normal and exceptional-lane accumulation, FFT, scratch movements and worker scheduling, for f32 and f64 and DUCC where practical. Use real nonzero T720 data, warm reuse and enough iterations for signal. Account for non-overlapping phase costs; do not add nested inclusive timings as if independent.
4. **Find the 2–3x-plus extra cost.** Compare f64 versus DUCC f64 for a possible shared optimization first if the profile supports it; then explain f32-specific overhead against the faster f64 path. Measure useful work per degree/ring, fallback frequency, SIMD width and actual scale transitions. Do not assume unchanged f32 and f64 paths.
5. **Test one causal hypothesis at a time** in an isolated worktree: predicted cost, narrow change, actual hot-loop microbenchmark, focused correctness guard, and three interleaved warm whole-call pairs against the exact parent. Reject weak changes quickly. Do not repeat rejected recurrence-fusion, sparse-lane-scan, scaling-threshold, tile or output-copy variants without new evidence.

## Validation budget and parked families

During exploration **do not run** complete Rust/Python suites, all 16 cells, repeated 15-pair matrices or extensive binary128 studies. Run only directly affected correctness tests and microbenchmarks. Before promoting a verified improvement, run 15+ matched B1 full/reduced q1/q8 pairs for native f32/f64 and pinned DUCC, including source/fixture/binary hashes, paired uncertainty and output controls; then run formatting, strict Clippy, relevant full Rust/Python/CI, numerical/exceptional regression checks and protected f64 T1023 spin-1 synthesis. Do not weaken tests or alter APIs; retain shared native precision algorithms.

- **B0 scalar synthesis:** accepted/parked. Both precision-matched DUCC ratios pass; its 1.03–1.18x f32/f64 ratio is not a target. Do not restore the f64 child sidecar.
- **B3 spin-1 analysis:** park the proven 6–9% scaled fusion; no further work now.
- **B2 spin-1 synthesis:** park the remaining 1.6–1.8x DUCC gap and rejected accumulator/output-copy variants; no further work now.

## NEXT

At parent `410c9ec`, warm phase probes put Legendre recurrence plus accumulation at 94–97% of Rust f32 time and 82–91% of Rust f64 time; FFT took 3–6% and 8–18%, respectively. The profile combines recurrence with accumulation and hardware `perf` is blocked on this host. The accepted f64 two-pair traversal remains active. The new f32 result-classification short-circuit improves all four T720 cells by 3.1–3.7% against its exact parent, with 15-pair confidence intervals below 1 and zero candidate/parent relative L2 in every pair. The f32/DUCC gap remains 3.37–4.80x; f64/DUCC remains 1.23–1.31x. The f64 candidate/parent intervals include 1.00 in all cells. See the [profile package](../reports/archive/experiments/m6-b1-profile-410c-2026-10-31/README.md), [f64 two-pair evidence](../reports/archive/experiments/m6-b1-two-pair-analysis-unroll-2026-11-01/README.md), and [f32 normalizer screen](../reports/archive/experiments/m6-b1-analysis-normalizer-result-short-circuit-2026-11-01/README.md). **NEXT:** profile the remaining f32 scaled recurrence and accumulation path with a bounded, source-matched probe; test one new measured hypothesis. Do not repeat the rejected factor, fusion, tile or sparse-index variants, infer cycles from Callgrind Ir, or change B0/B2/B3.
