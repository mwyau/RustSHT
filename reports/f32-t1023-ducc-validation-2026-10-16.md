# T1023 f32 scalar and spin-1 validation

The atmospheric f32 screen records both transform directions at T511 and T1023, but its T1023 rows previously lacked same-grid numerical references. This follow-up compares RustSHT f32 scalar and spin-1 STANDARD synthesis and analysis with `ducc0==0.41.0` on the T1023 benchmark grid.

## Method

Both implementations use the rectangular GL grid `1024×2048`, `lmax=mmax=1023`, `phi0=0.37`, and one execution thread. RustSHT workspaces are explicitly prepared with execution capacity 1. Inputs are deterministic integer-derived float32 maps and complex64 coefficients. Spin-1 inputs include both channels and set `m=0` coefficients to real values.

The DUCC source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The reference generator, package version, grid parameters, selected reference values, sums, norms, maxima, and Rust test output are retained in [`reports/data/f32-t1023-validation-2026-10-16`](data/f32-t1023-validation-2026-10-16/). The Rust comparison is an opt-in integration test because each run executes four high-bandlimit transforms.

The test checks eight map locations in each output channel, nine harmonic modes in each analysis channel, and the sums and Euclidean norms of every complete output array. Absolute tolerances are `2.5e-5` for sampled map values and `2e-6` for sampled complex coefficients; relative tolerances are `1e-3` for sums and `3e-4` for norms.

## Results

All sampled values and complete-array summary checks pass. The largest absolute synthesis-map difference among the 24 sampled channel values was `2.18e-5`. Synthesis L2 norms differed from DUCC by `5.5e-7` relative for scalar, `1.82e-6` for spin-1 channel 0, and `1.26e-6` for spin-1 channel 1. Analysis L2 norms differed by `1.15e-6` relative for scalar and less than `7.0e-7` for either spin-1 channel.

This check does not compare every map sample or every coefficient and does not report a full-array relative-L2 error. It adds high-bandlimit coverage for both f32 directions and both spins while keeping the reference fixture small. Existing f32 analytic, adjoint, low-bandlimit, and T511 spin-1 checks remain in place.

## Validation

```text
cargo test -p rustsht --test f32 --all-features --release \
  f32_t1023_scalar_and_spin1_standard_match_ducc_0410 -- --ignored
```

The focused test passed. `cargo fmt --all -- --check` and workspace Clippy passed. The full workspace suite passed with 187 unit tests and 11 f32 integration tests; 13 unit diagnostics and the new T1023 reference test were ignored in that default run. The no-default-feature suite passed with 149 unit tests and 11 f32 integration tests; 7 unit diagnostics and the new T1023 reference test were ignored. The opt-in T1023 test passed separately. Python tests were not run because no bindings changed.

The shared `/tmp` filesystem was full during the first all-feature attempt. Re-running with `TMPDIR=target/rustsht-doc-tmp` passed, including doctests. The no-default-feature command also uses that workspace-local temporary directory. `m1_f32_atmospheric_bench` now declares its existing `parallel` feature requirement, matching the other Rayon benchmark examples, so the no-default test does not try to compile a Rayon-only example.
