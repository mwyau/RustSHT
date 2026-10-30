# Native-f32 scalar-analysis normal-scale leaf screen

## Scope and provenance

This isolated B1 candidate combines the prior adaptive seed-prefix/SIMD-preflight schedule with one narrow f32 change: normal-range power-of-two scaling adjusts exponent bits directly, while subnormal, overflow, underflow and nonfinite cases retain `numeric::scale_binary_f32`. The candidate still runs native-f32 recurrence, accumulation, spectra, FFT and map I/O. It does not promote transform inputs or state to f64.

Base source is `a53e17877440de962567fc21a5226f786b721d1e`; its source files are identical to the earlier `c921d355` B1 baseline. The adaptive-only comparison parent applies source diff SHA-256 `f33008bcbda147b0b194cbc6eeac84cefaed6158d9750bbf6e75f1ed0e09186d`. The final candidate's uncompressed source diff SHA-256 is `71b791708a76cac5280a9c5a1ab2a6c3d19276539210f609e36d13f256d65162`; recover it from [candidate.patch.gz](candidate.patch.gz), compressed SHA-256 `3287c227e40169403a07d982cb2d2a09a642846047c9508b8081199a345c3a5b`.

The matched four-worker runner is [benchmark.py](benchmark.py), SHA-256 `0c0c3cb44c70f2ebaaf522bf0811f15faefa3ba115f9736b289a921ef82d54c8`. Exact three-pair data, fixtures, worker hashes and pair ordering are in [benchmark-fast-incremental-3pair.json](benchmark-fast-incremental-3pair.json). No timing binaries or duplicate fixture files are retained.

## Matched T720 screen

Three interleaved fresh-process pairs on an AMD Ryzen 9 5950X, Rust 1.99.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, q1 CPU 0 and q8 CPUs 0–7. Each pair included adaptive-only native-f32 Rust, the exact candidate, matched Rust f64, and pinned native-f32 DUCC. All four backends used identical deterministic f32 fixture values; only the diagnostic f64 worker widened those inputs component-wise. DUCC is 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

| GL T720 cell | Adaptive f32 ms | Candidate f32 ms | Matched f64 ms | f32/f64 | DUCC f32 ms | f32/DUCC | Candidate/adaptive | Interim |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| Full q1 | 51.058 | 50.281 | 19.529 | 2.579x | 14.135 | 3.557x | 0.985x | GAP |
| Full q8 | 6.872 | 6.741 | 2.788 | 2.416x | 2.061 | 3.271x | 0.981x | GAP |
| Reduced q1 | 19.025 | 18.695 | 8.272 | 2.260x | 5.789 | 3.229x | 0.983x | GAP |
| Reduced q8 | 2.673 | 2.603 | 1.209 | 2.158x | 0.901 | 2.888x | 0.974x | GAP |

Ratios are medians of the three paired ratios. The exact leaf reduced the adaptive-only candidate by 1.5–2.6%, but every f32/f64 cell still misses the `<=2.0x` interim gate. Do not extend this same scaler micro-optimization. The preceding current-source-parent comparison and earlier bounded tile/range screens remain in the [B1 extended-range package](../native-f32-scalar-analysis-extended-range-2026-10-30/README.md).

The candidate and adaptive-only outputs are byte-identical in all 12 process runs. Candidate relative L2 error against DUCC is `1.8390e-5` full and `2.2204e-5` reduced; these are comparison values, not independent accuracy acceptance. No new binary128, complete-array or application-tolerance acceptance is claimed.

## Validation and decision

- The new exact normal scaler matched the shared bit-level scaler for 16 representative finite, zero, subnormal and nonfinite values across every exponent from `-300` through `300`.
- Scalar-analysis unit module: 38 passed, 8 manual diagnostics ignored.
- f32 integration suite: 14 passed, 1 opt-in T1023 test ignored. This includes T720 scalar analytic modes, adjoint identity and workspace reset.
- `cargo fmt --all -- --check` and `git diff --check` passed.

Keep this source candidate isolated. B1 remains a measured timing GAP; continue to the next family and retain the selected high-degree accuracy status as open.

## Reproduction

Apply `candidate.patch.gz` to the stated base. Build the native-f32 Rust worker and the f64 diagnostic worker with the recorded target flags; use the pinned DUCC worker whose SHA-256 is in the JSON. Set `B1_PARENT_RUST`, `B1_CANDIDATE_RUST`, `B1_F64_RUST`, `B1_DUCC`, `B1_OUT`, `B1_PARENT_PATCH_SHA256` and `B1_PATCH_SHA256`, then run `benchmark.py`. The f64 worker source is preserved in the [B3 preweighted package](../native-f32-spin1-analysis-preweighted-2026-10-30/f64-worker/).
