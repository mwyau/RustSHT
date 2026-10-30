# Native-f32 breadth-port candidate checkpoint (2026-10-30)

This archive preserves the initial three-family screens, later candidate routes on `dev`, and exact-source follow-up evidence. `matrix.csv`, `matrix.json`, `followup-matrix.csv`, and `followup-screen.json` are historical three-pair screens. The source-correct B1 route and preceding 15-pair B2 synthesis screen are summarized in [`current-dev-followup.json`](current-dev-followup.json); the B1 patch is [`scalar-analysis-current-dev.patch.gz`](patches/scalar-analysis-current-dev.patch.gz). B1, B2 and B3 candidate code is now on `dev`, but independent high-degree accuracy remains open and these screens do not close numerical acceptance or final DUCC parity.

The current-dev follow-up records B1 and the preceding B2 candidate; the later B3 screen is indexed in [`b3-spin1-analysis-current.md`](b3-spin1-analysis-current.md). B1 remains a 7.00–10.55x f64 timing GAP. The integrated B2 source is commit `db93f14268e0320cc10d5e0e800f0143f41d657a`: its exact final source passes the interim f32/f64 <=2x gate in all four T720 cells in a three-pair screen (1.55–1.73x medians); the preceding kernel source passed in 15 pairs. Current-source f64 T1023 output is bitwise equal to parent. The final-source timing matrix, per-pair values, binaries, fixtures, DUCC distances and patch are recorded in [`b2-spin1-synthesis-integrated-final-t720-3pair.json`](b2-spin1-synthesis-integrated-final-t720-3pair.json); the f64 T1023 check is in [`b2-spin1-synthesis-integrated-t1023-f64-control.json`](b2-spin1-synthesis-integrated-t1023-f64-control.json). B3 now uses the shared scaled SIMD accumulator and improves about 2.5–3x, but remains 3.97–5.13x matched f64 in the four-cell three-pair screen. Its T511 DUCC reference passes; independent T720 spin-1 accuracy remains open.

## Historical 3-pair follow-up status (superseded)

These screens used three interleaved process pairs on GL T720. Ratios are shown separately against same-source Rust f64 and native-f32 DUCC. B2 spin-1 synthesis meets the interim <=2.0x f64 gate in all four cells, but misses the <=1.5x stretch. B1 scalar analysis and B3 spin-1 analysis remain far above the interim gate. The current B1/B2 values and B2's 15-pair confidence intervals are in [`current-dev-followup.json`](current-dev-followup.json).

| Family | Cell | Rust f32 ms | Rust f64 ms | f32/f64 | DUCC f32 ms | f32/DUCC | Independent accuracy |
|---|---|---:|---:|---:|---:|---:|---|
| Scalar analysis | Full q1 | 461.610 | 19.612 | 23.54x | 14.550 | 31.77x | UNMEASURED |
| Scalar analysis | Full q8 | 63.461 | 3.023 | 20.99x | 2.232 | 28.21x | UNMEASURED |
| Scalar analysis | Reduced q1 | 138.536 | 8.210 | 16.87x | 5.927 | 23.41x | UNMEASURED |
| Scalar analysis | Reduced q8 | 19.842 | 1.220 | 16.26x | 0.975 | 20.32x | UNMEASURED |
| Spin-1 synthesis | Full q1 | 67.687 | 38.882 | 1.74x | 42.160 | 1.61x | UNMEASURED |
| Spin-1 synthesis | Full q8 | 10.524 | 6.371 | 1.65x | 5.942 | 1.77x | UNMEASURED |
| Spin-1 synthesis | Reduced q1 | 27.745 | 16.638 | 1.65x | 18.119 | 1.53x | UNMEASURED |
| Spin-1 synthesis | Reduced q8 | 4.448 | 2.855 | 1.55x | 2.673 | 1.71x | UNMEASURED |
| Spin-1 analysis | Full q1 | 506.474 | 37.495 | 13.51x | 32.333 | 16.07x | UNMEASURED |
| Spin-1 analysis | Full q8 | 72.530 | 6.161 | 11.98x | 5.091 | 15.90x | UNMEASURED |
| Spin-1 analysis | Reduced q1 | 174.224 | 15.230 | 11.19x | 13.122 | 13.10x | UNMEASURED |
| Spin-1 analysis | Reduced q8 | 25.837 | 2.414 | 10.70x | 2.204 | 13.43x | UNMEASURED |

Candidate source diffs are B1 `a7c617785c3e312bd6bdf18c66f7b545e66f18fbcbdedc8a1b93f20cb8776c3a` on base `8fb2e1fd`; B2 `37115d60240f6d378c6a1341b0efeaec06ecc6fd2c81570ec7d8daa9a6b0a5bf` on base `bb5e5c04`; B3 `8a2180a0f26db10c27cfb7199852661c7de0464256e558e8243552e48b081670` on base `bb5e5c04`. Exact binary and fixture hashes, process-pair medians, tests and patch checksums are in `followup-screen.json` and `SHA256SUMS`.

### Measured blockers in the historical candidates

- **B1 scalar analysis:** f32 does not override `fused_scalar_analysis_pair_block_preweighted` or the paired-input preparation hooks in `ShtScalar`; `transform/scalar.rs` therefore declines the fused block and uses per-degree recurrence callbacks. The isolated SIMD normalizer change does not port the optimized f64 order block. The next implementation is the shared fused analysis block, with precision-specific SIMD leaves.
- **B2 spin-1 synthesis (historical candidate):** the GL T720 route uses the generic order-major/reflected scalar kernel with native f32 SIMD, spectrum, FFT and output. Its random-map relative L2 distance to DUCC is about `2.81e-4` full and `2.70e-4` reduced; this does not establish accuracy. The protected f64 T1023 control for the exact integrated source is recorded separately in [`b2-spin1-synthesis-integrated-t1023-f64-control.json`](b2-spin1-synthesis-integrated-t1023-f64-control.json).
- **B3 spin-1 analysis:** a generic two-channel scaled recurrence candidate reduces the screen from the initial 15–24x to 10.7–13.5x f64, but remains a GAP. Lane-wise exponent restoration follows the vector product work and remains a major cost. The f64 production route does not yet dispatch through this new kernel, so this is not yet a shared maintained f32/f64 implementation. Independent accuracy is unmeasured.

The current candidate tests are summarized in `followup-screen.json`; they cover existing analytic, reset and focused recurrence behavior, not the missing high-degree independent reference and adjoint matrix.

## Initial candidate screen (superseded by the follow-up above)

Each table cell is **matched f32/f64 ratio; separately measured f32/DUCC ratio**.

| Family | Full q1 | Full q8 | Reduced q1 | Reduced q8 |
|---|---:|---:|---:|---:|
| scalar-analysis | 23.54x; 31.77x | 20.99x; 28.21x | 16.87x; 23.41x | 16.26x; 20.32x |
| spin1-synthesis | 29.53x; 26.11x | 27.32x; 35.37x | 13.64x; 29.25x | 28.02x; 28.22x |
| spin1-analysis | 24.18x; 30.42x | 21.12x; 27.33x | 17.85x; 22.09x | 15.19x; 20.39x |

Whole-call times are ms. The f32/f64 numerator and denominator use the source-matched paired screen; the f32/DUCC value comes from its separate matched-input screen. Both use three fresh-process pairs, CPU 0 for q1 and CPUs 0–7 for q8. f64 controls consume the same f32 fixture values widened component-wise to f64. Rust candidates use Pulp V3 AVX2/FMA; f32 recurrence, accumulation, Fourier storage and FFT remain f32. Source, patch, binary and fixture hashes are in `matrix.json`; raw pair results are in `raw/`. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Numerical status

All candidate outputs were finite and repeat/reset checks passed in the recorded workers. The f32/DUCC maximum relative-L2 values are approximately 1.83e-5 / 2.21e-5 (scalar analysis full/reduced), 2.14e-4 / 1.95e-4 (spin-1 synthesis), and 1.85e-5 / 2.17e-5 (spin-1 analysis). These are random-fixture cross-implementation measurements; independent high-degree, analytic and adjoint acceptance for the exact candidate source remains **UNMEASURED**. No accuracy PASS is claimed.

## Initial-screen blockers (superseded by measured follow-up above)

- **Scalar analysis:** the typed ordinary-range block is reached only when scaled initialization fits ordinary f32. Checked scaled traversal remains on the exceptional path. The measured gap is 16.3–23.5x f64; fallback costs have not been separated enough to justify another compensation kernel.
- **Spin-1 synthesis:** the candidate includes prepared dual-channel recurrence and a bounded SIMD scaled-accumulation path. This change did not materially reduce whole-call time: the paired f32/f64 gap is 13.6–29.5x, with reduced q1 best. Measure transition/fallback work before choosing another kernel.
- **Spin-1 analysis:** safe normal seed materialization enters the fused ordinary block, while scaled seeds and later range declines remain in checked f32 traversal. The 15.2–24.2x gap shows seed handoff alone is insufficient; quantify scaled-prefix and fallback coverage before another attempt.

## Historical protected f64 control (superseded)

An earlier B2 candidate was compared with clean parent `bb5e5c04` on GL T1023 full spin-1 synthesis. The immediately preceding candidate has a 15-pair control in [`current-dev-followup.json`](current-dev-followup.json). For the final `db93f142` source, one matched process per q again produced bitwise-identical maps; q1/q8 time ratios were 1.014/1.019 and are only sanity checks. Full current-source outputs and binary hashes are in [`b2-spin1-synthesis-integrated-t1023-f64-control.json`](b2-spin1-synthesis-integrated-t1023-f64-control.json).

## Reproduction and disposition

Patches apply to the listed base source revisions, not necessarily current `dev` after future changes. The committed `pair-data.json` preserves per-pair timings and numerical errors without host telemetry or local paths. Exact source revisions, binary hashes, and fixture hashes are summarized in `matrix.json`. Full unsanitized worker captures remain local and untracked. Candidate patches are preserved under `patches/`; this archive is not a production merge.

Patch archives are gzip-compressed byte-for-byte copies of `git diff`; unpack with `gzip -dk patches/<family>.patch.gz`. Their uncompressed SHA-256 is recorded as each family's `patch_sha256` in `matrix.json`.
