# P01 `with_simd` outline screen

The single mitigation changed only the new scalar transition's `StagedSynthesisScalarTransition::with_simd` attribute from `#[inline(always)]` to `#[inline(never)]`. Both requested release regressions passed after the edit:

- `cargo test --release --lib scalar_synthesis_transition_matches_prepared_reference_with_tail -- --nocapture`
- `cargo test --release --lib scalar_t1023_transition_fallback_matches_scaled_reference_for_extreme_coefficients -- --nocapture`

Six alternating baseline/outlined pairs were completed for each listed cell. Each process exited successfully, recorded five samples per operation, and retained a preflight snapshot. No concurrent build or benchmark process was detected.

| Cell | Operation | Baseline median (ms) | Outlined median (ms) | Median paired O/B ratio | Paired bootstrap 95% interval |
|---|---:|---:|---:|---:|---:|
| T511 scalar q1 | Synthesis | 40.453 | 39.907 | 0.98591 | 0.97789–0.99002 |
| T511 scalar q1 | Analysis | 16.054 | 15.994 | 0.99527 | 0.98510–1.05434 |
| T511 spin1 q1 | Synthesis | 43.565 | 41.751 | 0.96523 | 0.95099–1.02542 |
| T511 spin1 q1 | Analysis | 28.719 | 27.726 | 0.96852 | 0.92767–1.01471 |
| T1023 scalar q1 | Synthesis | 298.690 | 519.095 | 1.74588 | 1.72839–1.74749 |
| T1023 scalar q1 | Analysis | 110.521 | 110.294 | 1.00047 | 0.96609–1.01883 |

The T1023 scalar synthesis median is 74.6% slower than its paired baseline; its paired-ratio range is 1.72050–1.74816. It also forfeits the original P01 gain (previous candidate median 197.335 ms). This mitigation therefore fails the scalar performance requirement, and the T2047 cell was not run. Although T511 spin1's median is below 1.02, its interval crosses 1.02 and the pair range is wide; this screen does not establish that the mitigation resolves that control reliably. The outlined binary emits separate `StagedSynthesisScalarTransition::with_simd` symbols for Scalar, V3 and V4. Its text section is 24,496 bytes larger than the original candidate, so this annotation increased the binary footprint in this build. The source was restored byte-for-byte to the saved candidate file (SHA-256 `8c92399c89f7b02fd6394ce66ea0905b0c68ecdec212c0470332d6efdaed287b`). The outlined source patch, source snapshot, compiled binary and hashes, structured numerical results, and aggregate timings are retained. Raw build, process and host captures remain local. No Git action was taken. This mitigation was reverted; no second variant was attempted.
