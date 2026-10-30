# Full-array native-f32 analysis accuracy, GL T720

## Scope and provenance

This package measures the current shared native-f32 scalar-analysis and spin-1-analysis paths on every valid output coefficient at full `mmax=720`. It also compares their forward FFT spectra to an independent float64 FFT. Rust production source is `dc1aa4f65109fc93fef8a0595998df464a70a639`; the only Rust modification is the ignored test-only spectrum observer in `fullarray-spectrum-capture.patch.gz`. No production transform code was changed. The diagnostic C++ reference uses GCC binary128 and `libquadmath`.

The deterministic input maps have `ntheta=721`, `nphi=1458`, GL geometry, `phi0=0`, and seeds 11 (scalar), 23 (Q), and 47 (U). All valid orders `m=0..720` and degrees `l=m..720` are compared. q1 and q8 maps, Rust spectra, coefficients, and the captured spin-synthesis inputs/maps are byte-identical; hashes are in `generated/fft-stage-comparison.json`. The B2 synthesis coefficients in this fixture are populated only for `m=0..720`, `l=600..720`; the B1/B3 analysis maps are independent full-field inputs.

The observer and deterministic dump example passed on the stated source. Build settings were Rust `1.99.1 (58cf8f9c3 2026-10-13)`, `--release --features parallel`, `RUSTFLAGS='-C target-cpu=x86-64-v3'`, and `RAYON_NUM_THREADS=1`/`8`. The independent reference used GCC 15.2.0, `-std=c++20 -O2 -march=x86-64-v3 -lquadmath`, on an AMD Ryzen 9 5950X running x86-64 Linux. Retained source and generated evidence are checksummed in `SHA256SUMS`; the diagnostic binary hash and test results are in `PROVENANCE.txt`.

## Full-array coefficient results

The binary128 reference starts from each actual Rust spectrum (normalized by `nphi`), promotes those exact complex inputs, and independently computes the normalized scalar harmonics, weighted ring projection, and spin E/B transpose. This isolates the recurrence and accumulation error conditional on the FFT result. The f64 channels are the same f32 input maps widened exactly before Rust's f64 FFT and analysis; their agreement with binary128 is a control on the mathematical reference and conventions.

| Output | f32 relative L2 | f64 relative L2 | f32 max absolute | f32 max envelope-scaled |
|---|---:|---:|---:|---:|
| Scalar analysis | `1.8647926341e-5` | `5.1919071189e-14` | `4.7205549300e-7` at `(m=4,l=693)` | `1.1562649538e-5` at `(m=4,l=693)` |
| Spin analysis E | `1.8424929229e-5` | `5.2475922558e-14` | `3.5531649947e-7` at `(m=1,l=717)` | `4.4179832429e-6` at `(m=10,l=720)` |
| Spin analysis B | `1.8640131372e-5` | `5.2275317526e-14` | `4.4823250096e-7` at `(m=0,l=682)` | `5.8202954997e-6` at `(m=0,l=682)` |

The envelope-scaled error divides each coefficient error by the larger of the independent coefficient magnitude and the sum of absolute weighted ring contributions. Per-order results for f32 and f64 are in the six `*-full-array.csv` files under `generated/`. No science tolerance has been supplied, so these results characterize error and do not assign numerical acceptance.

The matched f64 controls agree with binary128 to `5.2e-14` relative L2 across the complete arrays. This supports the recurrence normalization, weights, packed harmonic indices, and E/B transpose used by the independent reference for these inputs.

## FFT-stage separation

NumPy 2.3.5 `pocketfft` computes a float64 `rfft` from each exact f32 map widened to f64 and normalizes by `nphi`. On valid orders `m=0..720`, the native Rust f32 spectra differ from this reference by relative L2 `1.409–1.410e-7`; the matched Rust f64 spectra differ by `6.07–6.08e-16`. Stored bins above `mmax` are excluded.

To isolate how FFT input error affects the coefficient result, the same binary128 recurrence/projection is evaluated with either the captured f32 spectra or the matched f64 spectra from the exact same f32 map. Only the Fourier input changes:

| Projection | f32-spectrum vs f64-spectrum reference relative L2 | Maximum absolute displacement | f32 output error with f32 / f64 spectrum reference |
|---|---:|---:|---:|
| Scalar analysis | `1.3378151544e-7` | `1.3885698152e-9` | `1.8647926341e-5` / `1.8649522718e-5` |
| Spin analysis E | `1.3396349148e-7` | `1.5024783760e-9` | `1.8424929229e-5` / `1.8426464664e-5` |
| Spin analysis B | `1.3367228202e-7` | `1.4522328155e-9` | `1.8640131372e-5` / `1.8642155464e-5` |

The FFT-input substitution moves the reference projection by roughly `1.34e-7` relative L2. The native-f32 analysis output remains about `1.84–1.86e-5` from the reference, and its total relative-error norm changes by only `0.008–0.011%` when the spectrum is replaced. On this fixture the recurrence/accumulation path dominates the coefficient error; the FFT is measurable but secondary. The exact per-order substitution tables are in `generated/*-fft-input-sensitivity.csv`; f32 outputs compared against the f64-spectrum reference are in `generated/*-f32-vs-f64-spectrum-reference.csv`.

## q1/q8 and interpretation

All 20 captured q1/q8 input and output files listed in the FFT-stage JSON have equal SHA-256 hashes. This is a deterministic consistency result for this fixture, not evidence that other schedules, grid families, exceptional inputs, or candidate implementations agree.

Full-array f32/f64 and FFT-stage error are now measured for B1 and B3, but no accuracy bound has been approved. Existing small-grid f32 spin adjoint tests remain applicable; this package does not add an independent high-band adjoint identity or exceptional-magnitude reference. B2 still needs a full-map binary128 result and independent extreme-magnitude coverage. Do not promote a compensation or change tolerances based on these aggregate measurements.

## Reproduction

From a clean checkout at the Rust source SHA above:

```sh
trace_dir=$(mktemp -d -t rustsht-fullarray-accuracy.XXXXXX)
git apply <(gzip -dc reports/archive/experiments/native-f32-cross-family-full-array-accuracy-2026-10-30/fullarray-spectrum-capture.patch.gz)
cp reports/archive/experiments/native-f32-cross-family-accuracy-2026-10-30/phasec_fullorder_accuracy_dump.rs examples/
RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=1 cargo run --release --features parallel --example phasec_fullorder_accuracy_dump -- "$trace_dir/q1"
PHASEC_FULLARRAY_DIR="$trace_dir/q1" RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=1 cargo test --release --features parallel --lib phasec_capture_fullarray_analysis_spectra -- --ignored --nocapture
RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=8 cargo run --release --features parallel --example phasec_fullorder_accuracy_dump -- "$trace_dir/q8"
PHASEC_FULLARRAY_DIR="$trace_dir/q8" RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=8 cargo test --release --features parallel --lib phasec_capture_fullarray_analysis_spectra -- --ignored --nocapture
g++ -std=c++20 -O2 -march=x86-64-v3 reports/archive/experiments/native-f32-cross-family-full-array-accuracy-2026-10-30/fullarray_analysis_reference.cpp -lquadmath -o "$trace_dir/fullarray_reference"
"$trace_dir/fullarray_reference" "$trace_dir/q1"
python3 reports/archive/experiments/native-f32-cross-family-full-array-accuracy-2026-10-30/fullarray_fft_stage.py --q1 "$trace_dir/q1" --q8 "$trace_dir/q8" --out "$trace_dir/fft-stage-comparison.json"
```

The generated map binaries are not retained in the repository. The committed per-order tables and FFT summary are compact derived evidence; the exact commands above regenerate them from the deterministic inputs.
