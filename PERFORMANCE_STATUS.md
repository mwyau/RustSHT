# RustSHT — current performance status

**Active direction:** measured f64 B1 scalar-analysis and B3 spin-1-analysis gaps. B1 has one bounded f64 experiment remaining at most; B3 remains primary. f32 performance is deferred. See the [active plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md), [source-specific ledger](reports/DUCC_PARITY_M6.md) and [recent checkpoints](reports/PROGRESS.md).

The latest exact-current-source B3 screen is `cc60dbdc` (three interleaved GL T720 pairs). Ratios are Rust f64 / pinned DUCC 0.42.0 f64; intervals are paired 95% confidence intervals.

| B3 spin-1 analysis | Full q1 | Full q8 | Reduced q1 | Reduced q8 |
|---|---:|---:|---:|---:|
| Rust / DUCC | 1.252 [1.226, 1.280] GAP | 1.293 [1.179, 1.420] GAP | 1.282 [1.238, 1.326] GAP | 1.277 [1.217, 1.339] GAP |

Earlier 15-pair results at source `31c7cd50` found 1.242–1.285× DUCC, with all four intervals above parity; they are source-specific, not a certification of `cc60dbdc`. A current-source q1 profile attributes 69% (full) and 83% (reduced) of scaled-dispatch time to the scaled prefix. The active scaled AVX2 leaf reserves 2,728 bytes and has 200 YMM stack references. A sequential split-channel candidate regressed every cell by 19–31% and was rejected. No production code changed. The next discriminating check, if B3 resumes, is a leaf-only scaled-prefix accumulator microbenchmark that preserves the shared recurrence; no further full kernel candidate is justified without a helper-level improvement.

B1 current 15-pair results remain: full q1 1.059 [1.002, 1.120] GAP, full q8 1.131 [1.097, 1.166] GAP, reduced q1 1.047 [0.988, 1.110] INCONCLUSIVE and reduced q8 1.032 [0.998, 1.068] INCONCLUSIVE. B0/B2 are protected controls. The pinned DUCC revision is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Other grid, truncation and adjoint cases are not certified at this HEAD.

The intermittent native-f32 nonfinite SIMD-tail issue remains a separate correctness concern; do not resume f32 performance tuning or weaken numerical tolerances.
