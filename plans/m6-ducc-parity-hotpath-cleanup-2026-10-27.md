# M6 — B1 scalar-analysis performance investigation

**Owner direction (2026-10-31): B1 scalar analysis only.** Diagnose and improve the algorithm's execution cost before returning to B3 or B2. Do not optimize already-accepted B0 scalar synthesis or chase Rust f32/Rust f64 1:1 timing. This is the **only active M6 execution plan**. Read [AGENTS.md](../AGENTS.md), and preserve measurements in the [live ledger](../reports/DUCC_PARITY_M6.md). The [previous three-way-parity plan](../reports/archive/plans/m6-three-way-parity-before-b1-focus-2026-10-31.md) is archived and no longer authoritative.

## Goals and evidence

Target **Rust f32/DUCC f32 ≤1.0x** and **Rust f64/DUCC f64 ≤1.0x**, separately, for B1 on GL T720 full M720/reduced M180, q1/q8. These remain eventual precision-matched performance targets, not a mandate to reach parity in one iteration. **Rust f32/Rust f64 is diagnostic only**, never a parity gate; do not slow f64 to improve the ratio. Optimizing f64 first is explicitly encouraged if a shared algorithmic cost is found.

| GL T720 B1 | Rust f32 / DUCC f32 | Rust f64 / DUCC f64 |
|---|---:|---:|
| Full q1 | 3.030x GAP | 1.310x GAP |
| Full q8 | 2.811x GAP | 1.273x GAP |
| Reduced q1 | 2.612x GAP | 1.278x GAP |
| Reduced q8 | 2.368x GAP | 1.226x GAP |

Current f32 values use parent `0433c215` plus the lazy scaled-state patch with SHA-256 `068828b7e527edaeb6485b6482aca576c1bc9a359cd81fb8b2dad67223c3a5a7`. Candidate/parent 95% intervals are 0.981–1.013 full q1, 0.987–1.004 full q8, 0.961–0.998 reduced q1 and 0.975–0.999 reduced q8. Reduced q1/q8 improve; full q1/q8 are INCONCLUSIVE. f32/DUCC remains above 1.0. f64 is unchanged from its matched 15-pair baseline. See the [lazy-state evidence package](../reports/archive/experiments/m6-b1-lazy-scaled-state-2026-11-01/README.md), prior [scale-mask package](../reports/archive/experiments/m6-b1-scale-mask-compression-2026-11-01/README.md), [tile-32 package](../reports/archive/experiments/m6-b1-precision-tile32-2026-11-01/README.md), and [f64 traversal package](../reports/archive/experiments/m6-b1-two-pair-analysis-unroll-2026-11-01/README.md). Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

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

At exact parent `0433c215`, current-source q1 Callgrind counted 794.30M f32 Ir and 320.34M f64 Ir; the order group accounts for 94.88% f32 and 83.80% f64 inclusively. Matched T720 phase probes put f32 Legendre recurrence/accumulation at 93.5–93.9% of q1/q8 wall time and f64 at 82.4–83.0%; FFT takes the balance. A q8 f64 scheduling probe measured 33.0 μs queue-lock time across 721 jobs (1.3% of its 2.50 ms diagnostic call). Current f32 route counts show 20.0% full and 13.3% reduced scaled pair invocations. Hardware `perf` events remain blocked (`perf_event_paranoid=4`), so Callgrind and route counts do not establish cycle shares. Lazy f32 state setup saves 1.71% Ir and 0.3–2.1% in paired whole-call estimates; reduced q1/q8 are supported while full q1/q8 remain INCONCLUSIVE. f32/DUCC remains 2.37–3.03x GAP, and f64/DUCC remains 1.23–1.31x GAP. The f64 executable `.text` is byte-identical. See the [lazy-state package](../reports/archive/experiments/m6-b1-lazy-scaled-state-2026-11-01/README.md), [scale-mask package](../reports/archive/experiments/m6-b1-scale-mask-compression-2026-11-01/README.md), [profile package](../reports/archive/experiments/m6-b1-profile-410c-2026-10-31/README.md), and [f64 two-pair evidence](../reports/archive/experiments/m6-b1-two-pair-analysis-unroll-2026-11-01/README.md).

**NEXT:** profile non-overlapping current-source regions inside the dominant Legendre stage: normal accumulation, scaled accumulation, recurrence advancement, range normalization, and exceptional-lane fallback. Use the exact T720 f32 fixture and its exact f64 widening. Keep the q8 job-lock result as a small scheduling bound; do not treat it as all scheduler overhead. Select the next change only after these subregions or a typed-block microbenchmark identify a larger avoidable cost. Do not reuse stale `410c9ec` region numbers as current, and do not repeat tile, scale-threshold, factor-fill, sparse-index, lane-scan or recurrence-fusion variants unchanged. Keep B0/B2/B3 parked.
