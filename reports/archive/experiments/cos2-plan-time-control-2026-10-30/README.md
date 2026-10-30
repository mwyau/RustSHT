# Native-f32 plan-time cosine-squared control

This isolated candidate changes only the cosine-squared operand supplied to the shared generic scalar-synthesis recurrence. It computes `cos(theta)^2` from the original f64 plan angle and narrows that small plan-time constant to f32. Runtime coefficients, recurrence, SIMD, accumulation, Fourier spectrum, FFT and map remain native f32. The f64 generic branch retains its previous cosine-squared expression. No production source was changed on `dev`; no timing was collected from this diagnostic candidate.

The candidate is based on the isolated shared-native scalar-synthesis source at `45380d7e4bd849a33ffef7d91978ee18fd7e4193`. Its exact trace-and-candidate patch is `data/candidate.diff.patch.gz` (decompressed SHA-256 `ebedf095a6dceac343ee98e0465b3702e23a7d86b37bdcc1f0ebf01fc58f0ee5`); file hashes are in `SHA256SUMS`. Decompress and apply it only in a disposable worktree. Applying it is for a disposable worktree only.

## Numerical result

The direct comparison covers 357 single-mode pre-FFT outputs at GL T720, `phi0=0`, `nphi=34`, with the archived binary128 recurrence/output reference. It confirms the baseline parent's selected output bits before comparing the candidate.

| Measure | Existing f32 geometry arithmetic | Plan-time f64 `cos²`, rounded to f32 |
|---|---:|---:|
| Direct cases closer to binary128 | — | 102 / 357 |
| Direct cases farther / unchanged | — | 0 / 255 |
| Correctly rounded direct outputs | 0 / 357 | 0 / 357 |
| Direct RMS absolute error | 2.31854140e-3 | 2.13214617e-3 |
| Direct RMS relative error | 6.62542523e-3 | 5.02144412e-3 |
| Direct maximum absolute / relative error | 9.56558577e-3 / 5.79835575e-2 | unchanged |

For the public single-mode probe, 13,090 map pixels (385 mode/ring rows × 34 longitudes) are compared with the independent long-double map reference. The candidate changes 2,008 pixels: 1,992 move closer and 16 move farther; 11,082 are unchanged. Correctly rounded pixel count stays at 1,704. Maximum absolute error falls from `5.67914588e-3` to `2.66875106e-3`, RMS absolute error from `1.09244758e-3` to `4.48435866e-4`, and aggregate relative L2 error from `3.60792315e-4` to `1.48100666e-4` (58.96% lower). Public `m=0,1,2` outputs are unchanged; the selected changes are in the ordinary higher-order recurrence.

Per-order public relative L2 is:

| Order | Existing | Candidate |
|---:|---:|---:|
| 5 | 5.95748e-4 | 3.41795e-4 |
| 8 | 8.60502e-4 | 2.68827e-4 |
| 16 | 9.29782e-4 | 2.22795e-4 |

This is a substantial selected-fixture improvement, but it is not a correctness pass: some public pixels worsen, no direct output becomes correctly rounded, maximum direct error is unchanged, and no application accuracy tolerance has been set.

## First recurrence divergence and propagation

The plan record comparison finds 24 shared `(m, ring)` rows. The only changed prepared field is `cos²`, in 12 rows (six orders at ring 5 and six at the equator); theta, sine, cosine, sectoral seed, pair coefficients, and other prepared values are bitwise unchanged.

At `m=5`, north ring 5, `cos²` changes from `0x3f7fd6e4` to `0x3f7fd6e5`. The binary128 value is `0.999372803120232096823972339746616469`; absolute operand error falls from `8.24018483e-8` to `2.27972035e-8`. The recurrence starts with identical previous/current state and identical `a,b`. At degree 7, the first changed kernel arithmetic is the leading contraction `(a*cos²+b)`: `0x41264500 → 0x41264501`; the following state update changes from `0xb34c14e6` to `0xb34c14e7`. Both are one-ULP changes, with no recurrence formula or FMA change.

The saved 357-degree `m=5`, ring-5 recurrence table compares each f32 `next` state with the archived binary128 state. By degree 719, the state bits are `0x3f78a7d5 → 0x3f78e1cd`; correctly rounded binary128 is `0x3f78f69a`. Absolute error falls from `1.20191812e-3` to `3.17385191e-4`, and distance falls from 20,165 to 5,325 f32 ULPs. Across the full 357 states, RMS absolute state error falls from `5.19825290e-4` to `1.48874706e-4`, and maximum absolute state error from `1.24105822e-3` to `3.21485990e-4`. Relative RMS state error falls from `1.45808905e-2` to `3.73421940e-3`. ULP distances remain large near cancellation; the absolute and relative error columns are retained in `data/recurrence-csq-comparison.csv.gz`.

At the equatorial ring the selected `cos²` bits change from `0x099bbe75` to `0x099bbe74`; against binary128, the operand absolute error falls from `3.03740326e-40` to `6.36016584e-41`. These controls show that the revised plan constant, not a changed recurrence or accumulation order, causes the measured improvement. The changed leading factor then perturbs every subsequent f32 recurrence state.

## Reproduction and provenance

- Candidate Rust base: `45380d7e4bd849a33ffef7d91978ee18fd7e4193`.
- Pinned DUCC reference source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; archived binary128 and long-double outputs provide the numerical references.
- Rustc: `1.99.1 (58cf8f9c3 2026-10-13)`; release test build; flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; AVX2 trace asserts 8 f32 lanes. The public probe uses workspace capacity 1.
- The exact diagnostic patch is `data/candidate.diff.patch.gz`. It includes test instrumentation and candidate-only changes; none of these source edits were applied to `dev`.
- The full local direct trace SHA-256 is `bd834e729ca198265fa1a6032da995259d30eb4a9fad3a29ef57bed9126cc6fc`; full public-map trace SHA-256 is `d5e7679872608589ed68869657fdb836613f6a5233dc846583be76b9f897a915`. Those single-mode raw traces remain in bounded local scratch; the full dense high-band trace is archived compressed below.
- The test binary SHA-256 is `c806026b2bd11e884de229f5874453cd1e4f97beeb8b472cebd83b0dc70ec13f`.
- `cargo fmt`, `git diff --check`, and `cargo test --release --lib` pass in the isolated candidate (320 passed, 44 ignored). Both ignored direct and public-map trace probes pass. No performance timings were collected from the instrumented binary.

From the repository root, rerun the two comparisons with:

```sh
python3 reports/archive/experiments/cos2-plan-time-control-2026-10-30/tracers/compare_recurrence_csq.py
python3 reports/archive/experiments/cos2-plan-time-control-2026-10-30/tracers/compare_csqonly.py
```

## Dense high-band control

The same candidate was traced on 256 pre-FFT outputs for orders 1–16, degrees 600–720, and north rings 0, 1, 5 and 360. This uses the archived deterministic high-band coefficients, `phi0=0`, the pinned DUCC trace, and the independent binary128 reference cross-checked against long double.

| Error to binary128 | Original f32 `cos²` | Plan-time f64 `cos²`, narrowed to f32 |
|---|---:|---:|
| Outputs closer / farther / unchanged | — | 62 / 2 / 192 |
| RMS absolute error | 9.94269e-6 | 6.40809e-6 |
| Maximum absolute error | 4.50024e-5 | 4.32209e-5 |
| RMS cancellation-scaled error | 1.01733e-3 | 7.15017e-4 |
| Maximum cancellation-scaled error | 5.04414e-3 | 3.09606e-3 |
| Correctly rounded f32 outputs | 0 / 256 | 0 / 256 |

The control improves both RMS measures, but the maximum errors and rounded-output count remain far from an accuracy acceptance. DUCC's maximum absolute error on these same outputs is `1.71407e-9`; the independent binary128 and long-double results differ by at most `1.2544e-16`.

The dense trace also locates the remaining error. Across all 64 order/ring cases the recurrence enters ordinary f32 execution at degree `m`; the scaled-prefix count is zero. For `m=5`, ring 5, the initialized current state is already one ULP from correctly rounded binary128 (`0xb19d1beb` vs `0xb19d1bec`). At degree 7, `cos²`, `a`, `b`, and the leading factor match their rounded reference bits, but the next state differs by two ULP (`0xb34c14e7` vs `0xb34c14e9`). At degree 719 the next state differs by 5,325 ULP. For `m=2`, ring 0, degree 4 instead begins with a one-ULP difference in the leading `a*cos²+b` result and a one-ULP next-state difference, even though each prepared f32 operand matches its separately rounded reference. The earlier replay confirms the AVX2 FMA path matches scalar f32 replay; these differences come from rounded f32 inputs and the sectoral seed, not a different recurrence formula or SIMD result.

Controlled substitutions attribute the dense residual mainly to recurrence state: f64 recurrence with Rust's prepared f32 coefficient pairs has maximum absolute error `1.23702e-7`; f32 recurrence with binary128-rounded coefficient pairs remains at `4.32173e-5`. The accumulation-only output delta with identical f32 terms is at most `3.46918e-8`, far smaller than the recurrence error. These substitutions are diagnostic and are not runtime backends.

## Random full-transform one-change control

The same source and exact archived full/reduced GL T720 fixtures were run with only the `cos²` source switched between the cached plan-time f64 value and the original f32 stable/direct expression. Recurrence-factor preparation stayed unchanged; recurrence, accumulation, Fourier spectra, FFT and map storage were f32. Rust q1 and q8 outputs were bitwise identical in each shape, and both runs passed finite, repeat and poisoned-workspace reset checks.

| Shape | Original f32 `cos²` relative L2 to DUCC | Plan-time f64 `cos²` relative L2 to DUCC | Relative-L2 change | Maximum absolute error, old → candidate |
|---|---:|---:|---:|---:|
| Full M720 | 1.83217e-4 | 1.70306e-4 | −7.05% | 5.20408e-4 → 2.85864e-4 |
| Reduced M180 | 1.81741e-4 | 1.60901e-4 | −11.46% | 4.68373e-4 → 2.77132e-4 |

The full map has 255,806 pixels move closer to DUCC, 211,761 farther, and 583,651 unchanged. The reduced map has 74,923 closer, 46,212 farther, and 151,403 unchanged. These controls show the selected aggregate improvement generalizes to these random inputs, while individual pixels still move in both directions. DUCC f32 uses a double Legendre recurrence; there is no independent high-precision full-map oracle in this experiment. The random results measure proximity to DUCC, not an absolute science error bound.

## Decision and NEXT

The plan-time `cos²` value is a verified, one-change numerical improvement on selected modes, dense high-band outputs, and both random full-transform shapes. It removes the prior `cos²` operand mismatch in the traced ordinary recurrence, but seed initialization and accumulated native-f32 recurrence rounding still dominate the dense residual. There is no supplied science tolerance, no dense output becomes correctly rounded, and random full-map results have no independent high-precision oracle. Keep the candidate isolated and unpromoted.

Phase C's dense and random controls are complete. Return to the breadth-first B1 scalar-analysis candidate: inspect its executed loop work and SIMD path against DUCC and the matching f64 baseline, then make one bounded performance change or document a measured blocker before moving to B2/B3. Defer further scalar-synthesis compensation until that family work is complete.

## Dense and random evidence provenance

- Rust candidate base: `45380d7e4bd849a33ffef7d91978ee18fd7e4193`; complete isolated candidate plus dense-trace patch SHA-256 (uncompressed): `87d02ad7262c90123123d48e0ac63909704330e691ccbf7f22473097c57fef8c`. The original control patch remains at `data/candidate.diff.patch.gz`; the complete trace state is `data/dense-high-band/candidate-full-validation.patch.gz`.
- The one-change f32-`cos²` replay patch is `patches/disable-plan-cos2-only.patch.gz`. Test instrumentation and a non-test no-op trace guard are separately available in `patches/dense-trace-test-overlay.patch.gz` and `patches/non_test_trace_guard.patch.gz`.
- Pinned DUCC source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; DUCC f32 worker binary SHA-256 `acc82387af2c42aed53b25678e3872157ed3fb6e5d5874322badc472a1d35d5d`. The binary128 dense reference and source DUCC trace are preserved in the numerical-causality archive linked above.
- Final Rust AVX2 example binary SHA-256 `33329cde79244c96527263df14a817021bfa09261002afcd139235cef5ada9c8`; f32 uses eight lanes. The original-f32-`cos²` control binary SHA-256 is `7ee06f33900a6fec49f1da87c11f575c75d41af75ef2897b1e2bfe1ad26bfb6c`. Dense test binary SHA-256 is `6968d2d6658c0610fc0645c87a0b18e409a93f3b35e4136a6d6d81e60732704a`.
- Rustc `1.99.1 (58cf8f9c3 2026-10-13)`, flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, host AMD Ryzen 9 5950X. Candidate execution uses native f32 recurrence, accumulation, spectra, FFT and map outputs; f64 is used only to prepare the small plan-time `cos²` constant.
- `cargo fmt --check`, `git diff --check`, `cargo build --release --example ducc_f32_full_transform_parity`, and `cargo test --release --lib` pass in the isolated candidate (320 passed, 45 ignored). The dense trace probe passes. No timing claim was made from these runs.
- The candidate dense raw trace is preserved compressed at `data/dense-high-band/rust-native-f32-dense-trace.csv.gz`; derived recurrence, pair, precision-control, accumulation, output and long-double tables are alongside it. `data/random-field/one-change-random-field-results.json` records all input/output hashes and per-cell numerical metrics. Checksums are in `SHA256SUMS`.

The dense comparator is `reports/archive/experiments/numerical-causality-ducc-0.42.0-2026-10-29/tracers/compare_dense_band.py`; it consumes the saved Rust trace plus the pinned DUCC and binary128 traces from that package. The public single-mode comparisons remain reproducible with:

```sh
python3 reports/archive/experiments/cos2-plan-time-control-2026-10-30/tracers/compare_recurrence_csq.py
python3 reports/archive/experiments/cos2-plan-time-control-2026-10-30/tracers/compare_csqonly.py
```
