# Spin-1 analysis native-f32 range screen (2026-10-30)

## Scope and provenance

This bounded experiment starts from `9cb2788cd3f18c0f262e24dff6fac58236e5a18d`. It measures two isolated candidates against the same source-matched GL T720 baseline:

- `tile32.patch.gz` changes only the f32 reflected latitude-pair tile to 32; f64 retains 256.
- `range80.patch.gz` keeps the 256 tile and widens only the f32 spin-1 recurrence's ordinary lower bound from `2^-40` to `2^-80`. Fourier-input and accumulated-output guards remain unchanged at the existing precision limits. The f64 range policy is unchanged.

Neither candidate is on `dev`. Both use native-f32 input, recurrence, SIMD, accumulation, spectra, FFT and output. The matched f64 worker receives the same fixture components widened for diagnostic comparison. The pinned comparison worker is native-f32 DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

Each matrix has three interleaved fresh-process pairs, GL T720, full M720/nphi1458 and reduced M180/nphi378, q1/q8, 25 ms warm batches, and `-C target-cpu=x86-64-v3`. Exact fixture hashes, worker hashes, compiler, affinity and pair timings are in the JSON files. Separate q1/q8 dispatch probes confirm Pulp V3 with AVX2/FMA and eight f32 lanes; see `simd-dispatch.json`. Candidate binaries use separate target directories from the baseline.

## Results

All ratios are medians of the three paired process ratios. These are screening results, not 15-pair claims.

| Candidate | Cell | f32 ms | f64 ms | f32/f64 | DUCC f32 ms | f32/DUCC |
|---|---|---:|---:|---:|---:|---:|
| Source baseline | Full q1 | 133.430 | 36.469 | 3.667x | 29.310 | 4.620x |
| Source baseline | Full q8 | 17.046 | 5.629 | 3.009x | 4.137 | 4.136x |
| Source baseline | Reduced q1 | 55.019 | 15.094 | 3.645x | 11.866 | 4.657x |
| Source baseline | Reduced q8 | 7.451 | 2.218 | 3.359x | 1.748 | 4.251x |
| f32 tile32 | Full q1 | 121.026 | 36.779 | 3.238x | 29.120 | 4.154x |
| f32 tile32 | Full q8 | 16.189 | 5.422 | 2.946x | 4.131 | 3.914x |
| f32 tile32 | Reduced q1 | 50.556 | 15.186 | 3.331x | 11.894 | 4.223x |
| f32 tile32 | Reduced q8 | 7.659 | 2.433 | 3.196x | 1.877 | 4.080x |
| f32 range `2^-80` | Full q1 | 119.973 | 37.283 | 3.224x | 30.326 | 3.956x |
| f32 range `2^-80` | Full q8 | 17.212 | 6.093 | 2.825x | 4.432 | 3.884x |
| f32 range `2^-80` | Reduced q1 | 52.344 | 15.216 | 3.442x | 12.066 | 4.340x |
| f32 range `2^-80` | Reduced q8 | 7.885 | 2.471 | 3.126x | 1.901 | 4.129x |

The tile32 candidate misses the `<=2.0x` f64 gate in every cell. The range candidate improves f32 q1 time against its baseline by about 10% full and 5% reduced, but q8 f32 time is slightly slower in both geometries; all four ratios remain above 2.8x. The q8 ratio change is partly affected by slower f64 and DUCC measurements in that later run, so it is not evidence of a q8 f32 speedup. Stop this bounded spin-1 analysis tuning pass here.

Random-fixture distance to DUCC is `1.85e-5` relative L2 / `3.52e-7` maximum absolute (full) and `2.17e-5` / `9.86e-7` (reduced) for the range candidate. These are comparison diagnostics, not independent accuracy acceptance.

## Validation and decision

On the exact range candidate, `cargo fmt --all -- --check` passed. These release tests passed with `--features parallel`:

- `--lib native_f32_spin1_recurrence_accepts_safe_normal_tail_values`
- `--lib spin1_analysis` (three tests, including poisoned workspace extremes)
- `--lib spin1_scalar_fast_analysis_honors_the_polar_order_cutoff`
- `--test f32 f32_spin` (three spin tests, including all-grid operation/adjoint and pinned T511 DUCC analysis)

The range change keeps coefficients and accumulated outputs behind their existing guards; it only admits normal f32 recurrence values down to `2^-80` to the ordinary route. No independent high-degree spin-1 analysis oracle was run, so this patch remains isolated. Do not promote either patch or infer numerical acceptance from random agreement with DUCC.

**NEXT:** leave B3 as a documented timing GAP and proceed to the deferred independent numerical-accuracy phase, using the saved `3bb97c1` trace as diagnostic context. Re-establish any compensation or precision change against an independent reference before proposing production code.

## Reproduction files

- `baseline-9cb2788c.json`, `tile32-9cb2788c.json`, `range80-9cb2788c.json`: source, binary, fixture and pair evidence.
- `simd-dispatch.json`: executed q1/q8 SIMD metadata for the range candidate.
- `run_matrix.py`: deterministic three-pair runner; pass worker paths and CPU affinity as arguments.
- `tile32.patch.gz`, `range80.patch.gz`: isolated, recoverable source changes.
- `SHA256SUMS`: artifact checksums.
