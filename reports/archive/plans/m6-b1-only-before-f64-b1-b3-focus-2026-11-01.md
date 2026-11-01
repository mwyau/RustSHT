# Superseded B1-only M6 plan — archived 2026-11-01

**Historical only.** Its B1-only and f32-performance NEXT instructions are retired. Use the [active f64 B1/B3 plan](../../../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md).

---

# M6 — B1 scalar-analysis performance investigation

**Owner direction (2026-10-31): B1 scalar analysis only.** Diagnose and improve the algorithm's execution cost before returning to B3 or B2. Do not optimize already-accepted B0 scalar synthesis or chase Rust f32/Rust f64 1:1 timing. This is the **only active M6 execution plan**. Read [AGENTS.md](../AGENTS.md), and preserve measurements in the [live ledger](../reports/DUCC_PARITY_M6.md). The [previous three-way-parity plan](../reports/archive/plans/m6-three-way-parity-before-b1-focus-2026-10-31.md) is archived and no longer authoritative.

## Goals and evidence

Target **Rust f32/DUCC f32 ≤1.0x** and **Rust f64/DUCC f64 ≤1.0x**, separately, for B1 on GL T720 full M720/reduced M180, q1/q8. These remain eventual precision-matched performance targets, not a mandate to reach parity in one iteration. **Rust f32/Rust f64 is diagnostic only**, never a parity gate; do not slow f64 to improve the ratio. Optimizing f64 first is explicitly encouraged if a shared algorithmic cost is found.

| GL T720 B1 | Rust f32 / DUCC f32 | Rust f64 / DUCC f64 |
|---|---:|---:|
| Full q1 | 2.842x GAP | 1.136x GAP |
| Full q8 | 2.654x GAP | 1.132x GAP |
| Reduced q1 | 2.424x GAP | 1.142x GAP |
| Reduced q8 | 2.181x GAP | 1.111x GAP |

Current B1 numbers compare exact parent `80959a30d6965a6b0765aabbab7a37d16be402d2` with the f64 SIMD sectoral-seed candidate over fifteen interleaved pairs, against matching pinned DUCC 0.42.0 workers. The f64 candidate improves whole calls by 8.3–10.6%, with bitwise-equal outputs; its candidate/parent 95% intervals are 0.871–0.929 full q1, 0.885–0.905 full q8, 0.906–0.929 reduced q1 and 0.894–0.919 reduced q8. The f32 route and V3 kernel are byte-identical to the parent; all four candidate/parent intervals include 1.0. Both precisions remain above matching DUCC. See the [f64 seed-initialization evidence package](../reports/archive/experiments/m6-b1-f64-simd-seed-initialization-2026-11-01/README.md), plus the historical [f32 two-pair package](../reports/archive/experiments/m6-b1-f32-two-pair-unroll-2026-11-01/README.md), [lazy-state package](../reports/archive/experiments/m6-b1-lazy-scaled-state-2026-11-01/README.md), [scale-mask package](../reports/archive/experiments/m6-b1-scale-mask-compression-2026-11-01/README.md), [tile-32 package](../reports/archive/experiments/m6-b1-precision-tile32-2026-11-01/README.md), and [f64 traversal package](../reports/archive/experiments/m6-b1-two-pair-analysis-unroll-2026-11-01/README.md). Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

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

On parent `80959a30`, the f64 seed Callgrind region measured 36.25M instructions; 29.08M (80.23%) came from 195,204 scalar `scaled_power` calls. The SIMD initializer reduces that region to 1.80M instructions, and a pinned 105-lane T720 seed microbenchmark is 6.5–7.6x faster. This accounts for the measured 8.3–10.6% f64 whole-call improvement; f64 remains 1.11–1.14x DUCC. Hardware `perf` is unavailable (`perf_event_paranoid=4`), so Callgrind counts do not establish cycle shares.

For f32 full q1, isolated Callgrind gates recorded 34.0M instructions in seed setup, 153.4M in the ordinary two-pair body, 261.0M in scaled accumulation, 22.3M in recurrence advancement and 43.9M in range normalization. These regions were measured separately and are not elapsed-time shares or additive whole-call costs; f32's V3 kernel remains byte-identical to parent and f32 timing is unresolved. **NEXT:** use pinned wall-time microbenchmarks to measure f32 scaled accumulation and normalization against matching DUCC f32, then isolate the next f64 ordinary-loop cost. Keep B0/B2/B3 parked; do not infer cycles from Callgrind or claim parity.
