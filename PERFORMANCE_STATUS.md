# RustSHT — current performance status

**Active direction:** measured f64 B1 scalar-analysis and B3 spin-1-analysis gaps. B1 has one bounded f64 experiment remaining at most; B3 remains primary. f32 performance is deferred. See the [active plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md), [source-specific ledger](reports/DUCC_PARITY_M6.md) and [recent checkpoints](reports/PROGRESS.md).

The latest B3 whole-call screen is `cc60dbdc` (three interleaved GL T720 pairs); its B3 hot-file blobs match current source `be8f72d7`. Ratios are Rust f64 / pinned DUCC 0.42.0 f64; intervals are paired 95% confidence intervals. No new whole-call timing was run for the partial-exception helper screen.

| B3 spin-1 analysis | Full q1 | Full q8 | Reduced q1 | Reduced q8 |
|---|---:|---:|---:|---:|
| Rust / DUCC | 1.252 [1.226, 1.280] GAP | 1.293 [1.179, 1.420] GAP | 1.282 [1.238, 1.326] GAP | 1.277 [1.217, 1.339] GAP |

Earlier 15-pair results at source `31c7cd50` found 1.242–1.285× DUCC, with all four intervals above parity; they are source-specific, not a certification of `cc60dbdc`. The f956320c q1 profile estimates scaled-prefix work at 12.54 ms full / 7.35 ms reduced; disjoint exceptional recovery accounts for 3.10/1.56 ms, recurrence/normalization 2.24/1.36 ms, and masked accumulation 1.89/1.27 ms. The active AVX2 leaf has a 2,696-byte frame and 186 YMM stack references, but PMU is unavailable and these are not proof of spill cost. A sequential split-channel candidate regressed every cell by 19–31%. A leaf-only accumulator comparison measured fused/sequential medians of 454.09/506.39 ns, with bitwise-equal outputs; repeating channel work was 11.5% slower. The partial-exception SIMD helper was 2.835 [2.763, 2.908]× scalar and rejected. The captured-state finite/in-range mask combination improved stable-vector helper time by 13–16%, but its three-pair candidate/parent intervals crossed 1.0 in all four cells. Candidate/DUCC remained 1.234–1.279× GAP. Parent outputs were bitwise identical; repeat/reset checks passed. Estimated whole-call headroom was below 0.8%, not an observed saving. The candidate is rejected and production code is unchanged. NEXT: no more B3 mask microkernels; require a new disjoint cost measurement before another B3 candidate, or use the remaining bounded B1 f64 attempt. See the [mask-screen package](reports/archive/experiments/m6-b3-mask-reduction-screen-2026-11-02/README.md).

B1 current 15-pair results remain: full q1 1.059 [1.002, 1.120] GAP, full q8 1.131 [1.097, 1.166] GAP, reduced q1 1.047 [0.988, 1.110] INCONCLUSIVE and reduced q8 1.032 [0.998, 1.068] INCONCLUSIVE. B0/B2 are protected controls. The pinned DUCC revision is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Other grid, truncation and adjoint cases are not certified at this HEAD.

The intermittent native-f32 nonfinite SIMD-tail issue remains a separate correctness concern; do not resume f32 performance tuning or weaken numerical tolerances.
