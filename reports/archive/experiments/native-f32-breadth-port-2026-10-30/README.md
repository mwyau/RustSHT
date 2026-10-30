# Native-f32 breadth-port candidate checkpoint (2026-10-30)

All three requested transform families now have isolated native-f32 candidate implementations and 3-pair GL T720 screens. None approaches the <=2.0x matching f64 interim gate. These are candidate measurements, not production results. No candidate was promoted. The f32/DUCC error columns are comparisons to DUCC, not independent accuracy validation.

## Per-cell screen

Each table cell is **matched f32/f64 ratio; separately measured f32/DUCC ratio**.

| Family | Full q1 | Full q8 | Reduced q1 | Reduced q8 |
|---|---:|---:|---:|---:|
| scalar-analysis | 23.54x; 31.77x | 20.99x; 28.21x | 16.87x; 23.41x | 16.26x; 20.32x |
| spin1-synthesis | 29.53x; 26.11x | 27.32x; 35.37x | 13.64x; 29.25x | 28.02x; 28.22x |
| spin1-analysis | 24.18x; 30.42x | 21.12x; 27.33x | 17.85x; 22.09x | 15.19x; 20.39x |

Whole-call times are ms. The f32/f64 numerator and denominator use the source-matched paired screen; the f32/DUCC value comes from its separate matched-input screen. Both use three fresh-process pairs, CPU 0 for q1 and CPUs 0–7 for q8. f64 controls consume the same f32 fixture values widened component-wise to f64. Rust candidates use Pulp V3 AVX2/FMA; f32 recurrence, accumulation, Fourier storage and FFT remain f32. Source, patch, binary and fixture hashes are in `matrix.json`; raw pair results are in `raw/`. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Numerical status

All candidate outputs were finite and repeat/reset checks passed in the recorded workers. The f32/DUCC maximum relative-L2 values are approximately 1.83e-5 / 2.21e-5 (scalar analysis full/reduced), 2.14e-4 / 1.95e-4 (spin-1 synthesis), and 1.85e-5 / 2.17e-5 (spin-1 analysis). These are random-fixture cross-implementation measurements; independent high-degree, analytic and adjoint acceptance for the exact candidate source remains **UNMEASURED**. No accuracy PASS is claimed.

## Bounded blockers

- **Scalar analysis:** the typed ordinary-range block is reached only when scaled initialization fits ordinary f32. Checked scaled traversal remains on the exceptional path. The measured gap is 16.3–23.5x f64; fallback costs have not been separated enough to justify another compensation kernel.
- **Spin-1 synthesis:** the candidate includes prepared dual-channel recurrence and a bounded SIMD scaled-accumulation path. This change did not materially reduce whole-call time: the paired f32/f64 gap is 13.6–29.5x, with reduced q1 best. Measure transition/fallback work before choosing another kernel.
- **Spin-1 analysis:** safe normal seed materialization enters the fused ordinary block, while scaled seeds and later range declines remain in checked f32 traversal. The 15.2–24.2x gap shows seed handoff alone is insufficient; quantify scaled-prefix and fallback coverage before another attempt.

## Protected f64 control

The B2 candidate was compared with clean parent `bb5e5c04` on GL T1023 full spin-1 synthesis, using the same f64 inputs and three interleaved pairs at each thread count. All output hashes were bitwise identical. q1 candidate/parent paired ratios were 1.0024, 1.0037, 1.0015 (median 1.0024). q8 ratios were 0.9654, 1.1272, 1.0068 (median 1.0068); the middle pair is noisy, so q8 is **INCONCLUSIVE**. This is a protection screen, not 15-pair performance evidence. Binary and input/output hashes are in the protected T1023 entry of local captures; the compact result is summarized in `matrix.json`.

## Reproduction and disposition

Patches apply to the listed base source revisions, not necessarily current `dev` after future changes. The committed `pair-data.json` preserves per-pair timings and numerical errors without host telemetry or local paths. Exact source revisions, binary hashes, and fixture hashes are summarized in `matrix.json`. Full unsanitized worker captures remain local and untracked. Candidate patches are preserved under `patches/`; this archive is not a production merge.

Patch archives are gzip-compressed byte-for-byte copies of `git diff`; unpack with `gzip -dk patches/<family>.patch.gz`. Their uncompressed SHA-256 is recorded as each family's `patch_sha256` in `matrix.json`.
