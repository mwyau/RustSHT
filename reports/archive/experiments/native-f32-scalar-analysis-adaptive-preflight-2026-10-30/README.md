# Native-f32 scalar-analysis adaptive schedule and SIMD preflight screen

## Scope

This isolated candidate combines two source changes on current dev base c921d3550c6a6610f39c54b0798c03d9ad4b47eb:

1. Keep 32-pair tiles through the f32 sectoral-seed scale-sensitive prefix, then process the suffix at the existing 256-pair scratch capacity.
2. Vectorize f32 input range checks and opposite-spectrum cosine weighting, retaining a scalar tail and the existing exceptional fallback behavior.

The mathematical recurrence, f32 accumulation formula, FFT, and output precision are unchanged. The candidate was not promoted because every T720 cell still misses the interim <=2x matched-f64 target. An independent selected high-band measurement is now recorded below; no science tolerance was supplied.

The recoverable source patch is [adaptive-schedule-simd-preflight.patch.gz](adaptive-schedule-simd-preflight.patch.gz), SHA-256 52d3557bd004ff53a4da32bdbd3d4d8bacae0eac61fd5b6ae896296b06f82efa. Decompress it before applying with `git apply`. RustSHT f32 and f64 binary hashes, DUCC hash, exact fixtures and per-pair data are in [adaptive-simd-t720-3pair.json](adaptive-simd-t720-3pair.json). A fresh pre-candidate f64 baseline on the same c921d355 source is in [fresh-current-source-f64-baseline.json](fresh-current-source-f64-baseline.json). No fixture copies or benchmark binaries are stored.

## T720 result

Three interleaved fresh-process pairs on AMD Ryzen 9 5950X with target-cpu x86-64-v3. Each comparison held CPU binding constant across its Rust and DUCC workers; q1 and q8 thread counts are recorded per case. Native f32 fixtures were widened only for the matched f64 worker. DUCC is pinned at 0.42.0 revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b.

| Geometry | f32 ms | matched f64 ms | f32/f64 | DUCC f32 ms | f32/DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 52.650 | 20.411 | 2.577x | 14.577 | 3.623x |
| Full q8 | 7.772 | 3.562 | 2.182x | 2.338 | 3.329x |
| Reduced q1 | 19.199 | 8.559 | 2.261x | 6.360 | 3.283x |
| Reduced q8 | 2.817 | 1.341 | 2.104x | 0.971 | 2.916x |

The f32/f64 and f32/DUCC ratios are separate measurements. All four f32/f64 cells are GAP. The deterministic random-fixture relative L2 to DUCC is 1.8390e-5 full and 2.2204e-5 reduced; those are comparison metrics, not independent accuracy acceptance.

## Independent high-band accuracy measurement

The exact `c921d355` plus adaptive/preflight patch was run with the binary128 harness and direct Fourier reference in the [cross-family accuracy package](../native-f32-cross-family-accuracy-2026-10-30/README.md). The plan uses GL T720 with full `mmax=720`; the reference checks 11 selected orders (`0,1,2,5,8,16,64,180,360,719,720`) and degrees `600..720`, 1,092 complex coefficients per output. This is a selected high-band check, not a complete coefficient-array check.

| Output | q1 f32 relative L2 | q8 f32 relative L2 | f64 relative L2 | Maximum f32 absolute error |
|---|---:|---:|---:|---:|
| Scalar analysis | `4.9080808751e-5` | `4.9080808751e-5` | `8.7567204919e-14` | `3.8298466149e-7` at `(m=0,l=676)` |
| Spin analysis E | `4.8309101503e-5` | `4.8309101503e-5` | `1.0192069518e-13` | `3.5506046142e-7` at `(m=1,l=717)` |
| Spin analysis B | `5.3312543504e-5` | `5.3312543504e-5` | `1.0124263860e-13` | `4.4823250096e-7` at `(m=0,l=682)` |

The f32 fixture and scalar/spin-analysis outputs are byte-identical between q1 and q8. The reference includes an independent direct Fourier projection, but this output comparison does not split the Rust forward-FFT residual from recurrence/accumulation error. No science tolerance is defined, so these values are measured errors, not accuracy PASSes. Machine-readable metrics, fixture/output hashes and harness hashes are in [accuracy-independent-fullorder.json](accuracy-independent-fullorder.json). Spin-synthesis accuracy is not measured by this run.

## Diagnosis and decision

The adaptive schedule does not remove enough f32 scale-sensitive work to close the remaining gap. The SIMD preflight/weighting leaf also failed to improve the whole-transform screen; the exact run's q1 full and reduced medians remained 2.577x and 2.261x f64. The remaining blocker is the native-f32 scaled recurrence/accumulation path and its scale-state transitions, not a f64 conversion route. This screen does not attribute a precise time share to each operation.

Keep the candidate isolated. Do not continue scalar-analysis tile, hard-lane, or preflight micro-tuning in the same form. B1 performance remains GAP; its selected high-degree accuracy is measured but acceptance remains OPEN.

Tests passed: see [test-results.txt](test-results.txt). Independent high-degree, exceptional-magnitude and broad geometry accuracy checks were not run for this candidate.

## Checksums

See [SHA256SUMS](SHA256SUMS) for package checksums. The patch applies to the listed base commit, not arbitrary later source.
