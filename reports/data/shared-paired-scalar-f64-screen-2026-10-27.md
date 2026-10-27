# Shared paired scalar traversal — focused f64 timing screen

**Date:** 2026-10-27 01:32 UTC (initial screen; q8 extended to 15 pairs)  
**Candidate:** `a21bd325e9f9cfc86af34355601c3e7428ef5fad`  
**Exact parent:** `32b1a6e6f609663fb7726c51e7dc50570787c91f`  
**Immutable pre-refactor baseline:** `062218639bc2f097f004c617da0ce46a3a4d2941` (not rebuilt).

## Method

Three interleaved exact-parent/candidate pairs per initial cell, GL T720, full mmax (L=M=720), ntheta=721, nphi=1458, even smooth longitude mode, native f64 scalar, q1/q8. The q8 cells were extended to 15 interleaved pairs because the initial three had high host-wait outliers. Each process reported 10 warmed batch measurements (2 transforms per batch; 25 ms target batch); execution order alternated. Fixture SHA-256: alm `3287233ee711f55c89f036cedeb3e73dcee3397b8059444063e048f024496bc0`, map `e5929b5372ec1d08c3b5500993a38b5a8661d4fa26c80cb07f3434a24b40d469`. No DUCC build/reference was measured. Rust 1.99.1 / Cargo 1.99.1, AMD Ryzen 9 5950X, `RUSTFLAGS="-C target-cpu=x86-64-v3 -Aunexpected_cfgs"`; separate release target directories. Binary SHA-256: parent `e47dd6290f4715cb42d76a0f6d130b71739b9eaf02b4188b33289a8ca7584211`, candidate `31e76ecc4c7f25d39c1db8329722c48b23ed96692d6ab76d20674984c4b198c4`.

## Results

| Route | Pairs | Median paired C/P | Range | C slower | Low-wait pairs (≤1% each process) | Low-wait median | Interpretation |
|---|---:|---:|---:|---:|---:|---:|---|
| analysis q1 | 3 | 0.999831 | 0.989356–1.002918 | 1/3 | 0 | nan | wait-affected (10.9%–11.0%) |
| analysis q8 | 15 | 0.975854 | 0.946051–1.100352 | 1/15 | 8 | 0.976677 | candidate faster in 14/15 pairs; one high-wait outlier |
| synthesis q1 | 3 | 0.989713 | 0.790273–1.009177 | 1/3 | 0 | nan | wait-affected (6.3%–7.8%) |
| synthesis q8 | 15 | 1.008852 | 0.834460–1.259579 | 13/15 | 7 | 1.008852 | candidate slower in 13/15; small shift, with high-wait outliers |

At q8, scalar analysis median C/P was `0.975854` (2.41% faster); the 8 pairs with ≤1% host wait for both processes had median `0.976677` (2.33% faster). Scalar synthesis median C/P was `1.008852` (0.89% slower); its 7 low-wait pairs had the same median, with 13/15 pairs slower. This is a small, repeatable synthesis q8 slowdown alongside a larger analysis q8 improvement. Keep this as a per-route trade-off; do not claim a blanket f64 speedup or parity. The source removes the independently maintained f32 paired traversal body while retaining the original f64 seed, SIMD recurrence and normalizer leaves, so the shared traversal is retained; no material f64 regression was established in this screen. q1 is inconclusive because both operations had sustained high worker wait (analysis 10.9–11.0%, synthesis 6.3–7.8%).

All 36 exact-parent/candidate pairs (72 outputs) were byte-identical and finite; all 72 repeat/reset checks passed. The worker does not support poisoned-destination validation, so that check was not run. The adjacent CSV preserves all per-pair batch vectors, order, wait/frequency diagnostics, output hashes and correctness checks.

## Cleanup

This report and CSV are saved outside the unique tmpfs scratch. Remove that scratch and its parent/candidate worktrees immediately after this evidence commit; the shared progress report will record reclaimed space. The first preflight attempt also stopped before measurement on a mistyped SHA and was cleaned separately.
