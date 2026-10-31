# M6 — live native-f32/f64 performance ledger

**Target:** for every B0/B1/B2/B3 GL T720 full/reduced q1/q8 cell, each matched whole-transform ratio must be ≤1.00: Rust f32/Rust f64, Rust f32/DUCC f32 and Rust f64/DUCC f64. A `GAP` is a ratio above 1.00; below 1.00 is `INCONCLUSIVE` until its paired interval supports PASS. No current target cell passes all three gates.

The parent reference is source `afd3df285dcb601371bb5b7ba1ec4fdc56ab1645`; pinned DUCC is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Current `dev` includes `47906d09` (compact hard-scale vectorization) and `a3cd8964` (cached sparse scalar-fallback lane indices shared by B1/B3). B0 and B2 retain their original routes. The [three-pair parent baseline package](archive/experiments/current-source-four-backend-baseline-2026-10-30/README.md) keeps the exact original worker and fixture provenance; live B1/B3 rows use the exact `a3cd8964` source, while B0/B2 remain unchanged screens.

Times are medians in milliseconds. B0/B2 use three interleaved pairs (screen); B1/B3 use fifteen interleaved pairs. Candidate/parent intervals are 95% percentile-bootstrap intervals for the median paired ratio, 50,000 resamples. They describe speedup against `47906d09`, not a parity gate. The f32/f64 and f32/DUCC intervals are also in the candidate package.

| Cell | Pairs | Rust f32 / Rust f64 ms (ratio) | Rust f32 / DUCC f32 ms (ratio) | Rust f64 / DUCC f64 ms (ratio) | Candidate / parent (95% interval) | Accuracy and executed route |
|---|---:|---:|---:|---:|---:|---|
| B0 scalar synthesis full q1 | 3 | 320.753 / 15.757 (20.192 GAP) | 320.753 / 19.809 (16.220 GAP) | 15.757 / 20.205 (0.780 INCONCLUSIVE) | — | Existing references pass; native fallback on fixture, sidecar guard declined |
| B0 scalar synthesis full q8 | 3 | 53.853 / 2.468 (21.822 GAP) | 53.853 / 2.995 (18.191 GAP) | 2.468 / 2.975 (0.826 INCONCLUSIVE) | — | Existing references pass; native fallback on fixture, sidecar guard declined |
| B0 scalar synthesis reduced q1 | 3 | 103.952 / 6.582 (15.792 GAP) | 103.952 / 8.711 (11.917 GAP) | 6.582 / 8.661 (0.764 INCONCLUSIVE) | — | Existing references pass; native fallback on fixture, sidecar guard declined |
| B0 scalar synthesis reduced q8 | 3 | 25.876 / 1.047 (23.151 GAP) | 25.876 / 1.355 (19.095 GAP) | 1.047 / 1.363 (0.752 INCONCLUSIVE) | — | Existing references pass; native fallback on fixture, sidecar guard declined |
| B1 scalar analysis full q1 | 15 | 77.350 / 20.899 (3.696 GAP) | 77.350 / 15.594 (4.959 GAP) | 20.899 / 16.096 (1.319 GAP) | 0.789 [0.780, 0.796] PASS vs 47906 | Existing numerical guards pass; bitwise parent output; native-f32 V3 eight-lane path |
| B1 scalar analysis full q8 | 15 | 12.636 / 4.067 (3.191 GAP) | 12.636 / 2.765 (4.586 GAP) | 4.067 / 2.865 (1.415 GAP) | 0.808 [0.770, 0.874] PASS vs 47906 | Existing numerical guards pass; bitwise parent output; native-f32 V3 eight-lane path |
| B1 scalar analysis reduced q1 | 15 | 32.474 / 9.913 (2.751 GAP) | 32.474 / 8.867 (3.635 GAP) | 9.913 / 9.492 (1.182 INCONCLUSIVE) | 0.725 [0.612, 0.979] PASS vs 47906 | Existing numerical guards pass; bitwise parent output; native-f32 V3 eight-lane path |
| B1 scalar analysis reduced q8 | 15 | 3.681 / 1.464 (2.565 GAP) | 3.681 / 1.044 (3.589 GAP) | 1.464 / 1.056 (1.367 GAP) | 0.766 [0.738, 0.784] PASS vs 47906 | Existing numerical guards pass; bitwise parent output; native-f32 V3 eight-lane path |
| B2 spin-1 synthesis full q1 | 3 | 71.887 / 40.324 (1.781 GAP) | 71.887 / 42.611 (1.692 GAP) | 40.324 / 44.102 (0.917 INCONCLUSIVE) | — | Existing references pass; production native f32 |
| B2 spin-1 synthesis full q8 | 3 | 11.066 / 6.857 (1.642 GAP) | 11.066 / 6.163 (1.827 GAP) | 6.857 / 6.481 (1.058 GAP) | — | Existing references pass; production native f32 |
| B2 spin-1 synthesis reduced q1 | 3 | 29.274 / 17.306 (1.674 GAP) | 29.274 / 18.385 (1.592 GAP) | 17.306 / 18.569 (0.932 INCONCLUSIVE) | — | Existing references pass; production native f32 |
| B2 spin-1 synthesis reduced q8 | 3 | 4.912 / 2.928 (1.696 GAP) | 4.912 / 2.825 (1.739 GAP) | 2.928 / 2.782 (1.043 GAP) | — | Existing references pass; production native f32 |
| B3 spin-1 analysis full q1 | 15 | 100.778 / 38.533 (2.625 GAP) | 100.778 / 30.384 (3.247 GAP) | 38.533 / 30.845 (1.237 GAP) | 0.792 [0.767, 0.811] PASS vs 47906 | Existing numerical guards pass; bitwise parent output; native-f32 V3 eight-lane path |
| B3 spin-1 analysis full q8 | 15 | 15.908 / 8.012 (2.113 GAP) | 15.908 / 5.120 (3.324 GAP) | 8.012 / 5.752 (1.428 GAP) | 0.809 [0.783, 0.847] PASS vs 47906 | Existing numerical guards pass; bitwise parent output; native-f32 V3 eight-lane path |
| B3 spin-1 analysis reduced q1 | 15 | 35.329 / 15.258 (2.317 GAP) | 35.329 / 11.927 (2.956 GAP) | 15.258 / 11.931 (1.276 GAP) | 0.768 [0.764, 0.774] PASS vs 47906 | Existing numerical guards pass; bitwise parent output; native-f32 V3 eight-lane path |
| B3 spin-1 analysis reduced q8 | 15 | 5.701 / 2.718 (2.099 GAP) | 5.701 / 1.984 (2.860 GAP) | 2.718 / 1.995 (1.357 GAP) | 0.791 [0.763, 0.800] PASS vs 47906 | Existing numerical guards pass; bitwise parent output; native-f32 V3 eight-lane path |

The sparse-index candidate output matched its `47906d09` parent bit-for-bit in all 120 B1/B3 pairs. Candidate/parent intervals exclude 1 in all eight cells; median whole-call reductions are 19–28%. The exact-source analysis unit suite (36 passed), f32 integration suite (14 passed), strict all-target/all-feature Clippy, protected f64 T1023 regression and Python suite (20 passed) are green. No tolerances changed. This measured improvement still misses parity: median f32/f64 is 2.10–3.70× and f32/DUCC f32 is 2.86–4.96×. The timing host was contended during part of the run, so the full data and bootstrap caveat are preserved in the [candidate package](archive/experiments/current-source-scaled-analysis-profile-2026-10-30/candidate-sparse-fallback-index-15pair/README.md).

## Profile and next measured work

The exact `a3cd8964` q1 profiles record 1.244B B1 Ir and 1.636B B3 Ir, down 15.1% and 17.9% from the `47906d09` parent. The repeated scalar-fallback scan is now eliminated; raw profiles, hashes and runner are in the [sparse-index package](archive/experiments/current-source-scaled-analysis-profile-2026-10-30/candidate-sparse-fallback-index-15pair/README.md). The fused recurrence remains the measured hot area. Next, inspect remaining work inside that block and test a materially different reduction of recurrence/exceptional-lane work; do not repeat lane-scan or coefficient-cache changes.

B0/B2 candidate routes have not changed. B0 still took the native fallback on the measured fixtures; its f32-to-f64 sidecar remains architectural debt. B2 remains the second optimization priority after bounded B1/B3 kernel work.
