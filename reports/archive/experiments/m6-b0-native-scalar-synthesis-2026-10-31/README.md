# B0 native-f32 scalar synthesis route — 2026-10-31

**Result:** the f32 implementation now uses the shared reflected, order-major scalar synthesis kernel directly. The isolated candidate is based on `d4a7fa1552af08eecb69b252c31221b19e256ccb`; its source patch SHA-256 is `6e797e1af92ed911ea216cfa5d7fe38c0948d8d07e0bfd5d85da4dae2369e699` (`results/candidate.patch`). It executes native f32 recurrence, SIMD, spectral storage, FFT, and map output. No runtime f32-to-f64 promotion is used.

## Route and archived-candidate comparison

At the base source, f32 did not override `ShtScalar::supports_scalar_ordered()`, so GL synthesis bypassed `scalar_ordered::synthesize_dispatched` and used the native-f32 general ring-group fallback. The current T720 fallback screen measured 299.1/49.2/96.3/24.2 ms for full q1/q8 and reduced q1/q8. Enabling the existing generic dispatch selected the same `scalar_ordered::synthesize_batch` algorithm used by f64, with V3/AVX2 and eight f32 lanes.

The archived isolated candidate was based on older source `b6a96aa7161a64f678b614d73478496f50f0551e`; its best 15-pair screen reported 1.249/1.256/0.995/1.082 f32/DUCC ratios. That result did not transfer unchanged. On current `d4a7fa15` fixtures, route enablement alone screened at 1.12–1.65× DUCC. Adapting its two measured f32 leaves recovered most of the archived timing: an exact eight-lane f32 scale-`-80` restore and a proven signed-zero basis shortcut for scale epochs at or below `-2`. The zero shortcut leaves the recurrence state intact; the bound is `2^-60 × 2^-160 = 2^-220`, below half the least f32 subnormal. Exact scaler and tail tests cover the specialized leaf.

The production f64 sidecar and its conversion workspace/routing code were not removed in this slice. The new path is selected at compile time through the existing generic route; f32 does not acquire an f64 child transform.

## Matched GL T720 results

The baseline is the current d4 production f32 fallback (3 interleaved pairs). Candidate, matching Rust f64, pinned DUCC f32, and pinned DUCC f64 use the same archived GL fixtures and warm whole-call runner (15 interleaved pairs for the candidate rows). Times are median milliseconds. Ratios are paired median ratios with 95% percentile-bootstrap intervals over 50,000 resamples; see `results/metadata.json` and the four per-cell JSON files for every pair, dispatch record, fixture hash, and binary hash.

| Cell | Fallback f32 | Candidate f32 | Rust f64 | DUCC f32 | DUCC f64 | f32/f64 | f32/DUCC32 | f64/DUCC64 | Candidate/fallback |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 299.1 | 24.66 | 15.10 | 19.23 | 19.39 | 1.640 [1.625, 1.645] GAP | 1.285 [1.278, 1.292] GAP | 0.780 [0.773, 0.787] PASS | 0.082 [0.081, 0.083] |
| Full q8 | 49.2 | 4.00 | 2.63 | 3.04 | 3.09 | 1.560 [1.539, 1.652] GAP | 1.325 [1.305, 1.400] GAP | 0.816 [0.789, 0.830] PASS | 0.078 [0.072, 0.079] |
| Reduced q1 | 96.3 | 8.75 | 6.27 | 8.60 | 8.63 | 1.396 [1.392, 1.410] GAP | 1.015 [1.005, 1.033] GAP | 0.726 [0.722, 0.744] PASS | 0.087 [0.085, 0.089] |
| Reduced q8 | 24.2 | 1.67 | 1.13 | 1.42 | 1.42 | 1.479 [1.414, 1.525] GAP | 1.143 [1.063, 1.215] GAP | 0.799 [0.785, 0.825] PASS | 0.064 [0.063, 0.068] |

The route cuts current fallback time by 91–94%. B0 still misses the native-f32/Rust-f64 and native-f32/DUCC-f32 1:1 goals in all four cells; reduced q1 is close but its interval remains above 1.00. Rust f64/DUCC f64 passes in all four cells. q8 uncertainty is wider under host variation, but each f32 parity interval remains above 1.00.

The f32 runner reported V3/AVX2, eight lanes, native RustSHT, finite results, repeat consistency, and reset consistency in every pair. Exact source, binary and fixture hashes are in `results/metadata.json`. CPU: AMD Ryzen 9 5950X; q1 used the first allowed CPU and q8 the first eight allowed CPUs. Rust 1.99.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, parallel feature. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Numerical and regression scope

Formatting, strict Clippy, all release Rust tests and Python API tests passed. No tolerance was changed. Rust tests: 325 passed and 38 ignored; integration tests: 19 passed; doc tests: none. Python: `uv sync --python 3.11` and `uv run --no-sync pytest python/tests`, 20 passed.

A deterministic output-only comparison on the same f32/f64/DUCC fixtures found candidate-vs-DUCC relative L2 of `1.8676e-4` full and `1.8914e-4` reduced. The previous fallback was `1.7234e-4` full and `2.7255e-4` reduced: the candidate is slightly farther from DUCC on the full random fixture and closer on the reduced fixture. Candidate-vs-fallback relative L2 is `2.4882e-4` full and `3.2712e-4` reduced. These are one-run deterministic fixture comparisons, not a new tolerance claim.

The existing near-pole high-degree diagnostic still prints `0.018705474` for the native-f32 order-major result versus `0.018541995` from its independent f64 recurrence. The test remains diagnostic-only and no accuracy threshold was added or weakened. The current owner direction treats existing f32 accuracy as adequate for continued performance development; this report does not call the probe an independent accuracy PASS.

The protected f64 GL T1023 spin-1 synthesis control compared the exact d4 parent and candidate binaries on bitwise-matched outputs. q1 (3 pairs): 95.78/95.89/129.61 ms parent/candidate/DUCC; candidate/parent median 1.001, pair range [0.991, 1.026]. q8 (15 pairs): 21.07/19.90/25.57 ms; candidate/parent median 0.976 with bootstrap interval [0.881, 1.008]. All workers passed repeat/reset checks. This shows no established f64 regression; q8 timing is not a claimed gain. Fixture, binary and per-pair details are in `results/t1023-metadata.json` and the sanitized q1/q8 paired summary JSON files.

## Reproduction and files

`results/run_t720_pairs.py` is the paired runner. It accepts the matched baseline directory, parent binaries, candidate root and output directory as arguments; fixture hashes for every case and precision are in `results/metadata.json`, and fixture binaries are not duplicated here. `results/run_accuracy_probe.py` records the output-only comparison. The T1023 fixture hashes match the existing deterministic generator at `reports/data/ducc-parity-m6-2026-10-29/f32-spin1-dual-channel-pair-screen/sources/make_f64_t1023_fixture.py`.
