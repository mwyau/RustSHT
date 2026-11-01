# RustSHT — current performance status

**Active direction (2026-11-01): B1 f64 scalar analysis, then B3 f64 spin-1 analysis.** B1 is bounded to two evidence-led attempts; B3 has the larger measured gap. See the [active plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md), [source-specific ledger](reports/DUCC_PARITY_M6.md) and [five recent checkpoints](reports/PROGRESS.md). This page is a summary, **not** a complete current-HEAD parity certification.

| GL T720 f64 analysis / DUCC f64 | Full q1 | Full q8 | Reduced q1 | Reduced q8 |
| --- | ---: | ---: | ---: | ---: |
| B1 scalar | 1.059× | 1.131× | 1.047× | 1.032× |
| B3 spin-1 (older source) | 1.213× | 1.346× | 1.236× | 1.295× |

Ratios above 1 mean slower Rust. B1 full q1/q8 are GAP, whereas reduced q1/q8 remain INCONCLUSIVE (95% intervals overlap 1). B3 has four historical GAP cells, but its [15-pair evidence](reports/archive/experiments/m6-b3-scaled-recurrence-fusion-2026-10-30/README.md) is at an older source. The common reference is pinned native DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, with matched f64 workloads; the live ledger records uncertainties and individual provenance.

Preserve accepted B1 `7042ba9` and `8560bca`, shared generic scalar/spin algorithms, protected T1023 f64 spin-1 synthesis, and numerical, exceptional-value, adjoint and reset checks. f32 performance optimization is deferred. The intermittent native-f32 nonfinite SIMD-tail test requires an independent correctness investigation without changing tolerances.

Remaining GL/CC/F1 f64 synthesis, analysis and adjoint cells are **not certified at this HEAD**. The [previous status snapshot](reports/archive/PERFORMANCE_STATUS-before-f64-reset-2026-11-01.md) retains historical f64 synthesis wins/gaps and old phase profiles, but its next-work guidance is superseded. Broaden source-matched f64 coverage after B3.
