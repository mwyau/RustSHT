# T1023 f32 scalar and spin-1 validation

The atmospheric f32 screen records both transform directions at T511 and T1023, but its T1023 rows previously lacked same-grid numerical references. This follow-up compares RustSHT f32 scalar and spin-1 STANDARD synthesis and analysis with `ducc0==0.41.0` on the T1023 benchmark grid.

## Method

Both implementations use the rectangular GL grid `1024×2048`, `lmax=mmax=1023`, `phi0=0.37`, and one execution thread. RustSHT workspaces are explicitly prepared with execution capacity 1. Inputs are deterministic integer-derived float32 maps and complex64 coefficients. Spin-1 inputs include both channels and set `m=0` coefficients to real values.

The DUCC source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The reference generator, package version, grid parameters, selected reference values, complete-array comparison, sums, norms, maxima, and Rust test output are retained in [`reports/data/f32-t1023-validation-2026-10-16`](data/f32-t1023-validation-2026-10-16/). The Rust comparison is an opt-in integration test because each run executes four high-bandlimit transforms.

The test checks eight map locations in each output channel, nine harmonic modes in each analysis channel, and the sums and Euclidean norms of every complete output array. Absolute comparison bounds are `2.5e-5` for sampled map values and `2e-6` for sampled complex coefficients; relative bounds are `1e-3` for sums and `3e-4` for norms. The sampled map bound follows an exploratory `5e-6` check that failed at `(ring=0, longitude=0)` with a `2.18e-5` absolute difference. The complete-array comparison below records that behavior; the sampled bounds are not a global f32 accuracy criterion.

The opt-in test can write complete RustSHT map and coefficient outputs as little-endian arrays when `RUSTSHT_F32_DUMP_DIR` is set. [`compare-full-output.py`](data/f32-t1023-validation-2026-10-16/compare-full-output.py) regenerates DUCC outputs and reports complete-array relative-L2, RMS absolute, and maximum absolute errors. The temporary arrays were written under ignored `target/`; the compact metrics, rather than 38 MiB of generated arrays, are committed.

## Results

The complete-array relative-L2 errors are:

| Operation | Channel | Relative L2 | RMS absolute | Maximum absolute |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | scalar | `4.87e-5` | `1.15e-6` | `4.75e-5` |
| Scalar analysis | scalar | `1.67e-5` | `1.35e-8` | `1.17e-6` |
| Spin-1 synthesis | 0 | `3.84e-5` | `9.18e-7` | `4.19e-5` |
| Spin-1 synthesis | 1 | `4.19e-5` | `1.04e-6` | `4.42e-5` |
| Spin-1 analysis | 0 | `2.07e-5` | `1.89e-8` | `9.89e-7` |
| Spin-1 analysis | 1 | `2.56e-5` | `1.74e-8` | `9.16e-7` |

The full-array output-norm differences were at most `1.9e-6` relative. These are measured errors, not a pass against an established T1023 f32 accuracy limit: the project has not set that limit. The sampled comparisons also pass their fixture bounds. Existing f32 analytic, adjoint, low-bandlimit, and T511 spin-1 checks remain in place.

## Validation

```text
cargo test -p rustsht --test f32 --all-features --release \
  f32_t1023_scalar_and_spin1_standard_match_ducc_0410 -- --ignored
RUSTSHT_F32_DUMP_DIR=target/f32-t1023-full-output \
  TMPDIR=target/rustsht-doc-tmp cargo test -p rustsht --test f32 \
  --all-features --release f32_t1023_scalar_and_spin1_standard_match_ducc_0410 \
  -- --ignored
TMPDIR=target/rustsht-doc-tmp uv run --locked python \
  reports/data/f32-t1023-validation-2026-10-16/compare-full-output.py \
  target/f32-t1023-full-output \
  > reports/data/f32-t1023-validation-2026-10-16/full-output-comparison.json
```

The focused test passed. `cargo fmt --all -- --check` and workspace Clippy passed. The full workspace suite passed with 187 unit tests and 11 f32 integration tests; 13 unit diagnostics and the new T1023 reference test were ignored in that default run. The no-default-feature suite passed with 149 unit tests and 11 f32 integration tests; 7 unit diagnostics and the new T1023 reference test were ignored. The opt-in T1023 test passed separately. Python tests were not run because no bindings changed.

The shared `/tmp` filesystem was full during the first all-feature attempt. Re-running with `TMPDIR=target/rustsht-doc-tmp` passed, including doctests. The no-default-feature command also uses that workspace-local temporary directory. `m1_f32_atmospheric_bench` now declares its existing `parallel` feature requirement, matching the other Rayon benchmark examples, so the no-default test does not try to compile a Rayon-only example.
