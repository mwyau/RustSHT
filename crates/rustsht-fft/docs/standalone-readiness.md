# Standalone use and RustSHT integration

## Independent build

RustSHT-FFT is an isolated Cargo workspace under `crates/rustsht-fft`. It uses stable
Rust 1.99 and has no dependency on the RustSHT crate, the root workspace, C++,
or a runtime FFT backend. The runtime dependencies are `num-complex` and
`num-traits`; RustFFT is a development-only reference dependency. From the repository root, build and validate the crate with:

```sh
cargo test --manifest-path crates/rustsht-fft/Cargo.toml --all-features
cargo clippy --manifest-path crates/rustsht-fft/Cargo.toml --all-targets --all-features -- -D warnings
cargo fmt --manifest-path crates/rustsht-fft/Cargo.toml --all -- --check
cargo run --manifest-path crates/rustsht-fft/Cargo.toml --example basic
```

The example at `examples/basic.rs` exercises an f64 real transform and an f32
in-place complex transform.

## Execution contract

Create a plan for a positive length, then allocate its workspace once for each
concurrent execution context. Keep the immutable plan shared and give each
worker its own mutable workspace. Reuse both across rows. Transform methods
validate buffer lengths and return `FftError`; normal execution reuses the
workspace and does not allocate. The flat batch methods are serial convenience
loops and reuse one workspace for each row. Forward transforms are unnormalized negative-exponent DFTs. Inverse transforms
use the positive exponent and include `1/n` normalization. A real spectrum has
`n / 2 + 1` bins. Real inverse ignores imaginary DC and even-length Nyquist
values. Plans accept arbitrary positive lengths subject to available address
space and allocation; zero length returns `FftError::InvalidLength`. The crate runtime-dispatches eligible x86 AVX2 kernels and retains scalar
fallbacks. Distributed builds do not require `target-cpu=native`. The
architecture is portable Rust, but the current performance report measures
the native AVX2 path only on x86-64 x86-64 CPU. See
[`api-design.md`](api-design.md) for planner thresholds and
[`m5-sht-performance-report.md`](m5-sht-performance-report.md) for supported
workloads and measured limits.

## Future RustSHT integration steps

Integration is intentionally not part of this branch. To evaluate a later
integration in RustSHT:

1. Remove the nested `[workspace]` stanza from `crates/rustsht-fft/Cargo.toml` and
   add `"crates/rustsht-fft"` to the root workspace members in the same change.
2. Add `rustsht_fft = { path = "crates/rustsht-fft" }` to the RustSHT package
   dependencies. Keep the new package's own dependency list limited to its
   standalone FFT requirements.
3. Replace the longitude FFT adapter with a reusable `RealFftPlan<T>` and
   caller-owned `RealFftWorkspace<T>`. Preserve one workspace per concurrent
   worker; do not put mutable scratch in the plan or share it across Rayon
   tasks. Retain RustSHT's current rejection of imaginary DC and even-length
   Nyquist components, mapping violations to its existing error. RustSHT-FFT's
   real inverse intentionally ignores those endpoint imaginary parts, so a
   direct adapter replacement would otherwise change RustSHT behavior.
4. Replace CC/F1 meridional RustFFT plans with `ComplexFftPlan<T>` only after
   matching each existing forward/inverse normalization and scratch lifetime.
5. Run the existing analytic harmonic, direct DUCC, adjoint, grid and
   round-trip suites without weakening tolerances. Compare the complete scalar
   and spin-1 SHT operations at the current longitude sampling before changing
   any sampling policy.
6. Re-run end-to-end T72–T720 benchmarks. The FFT row timings in the M5 report
   are not full SHT timings and do not establish an integration speedup. The nested workspace is deliberate while RustSHT-FFT remains isolated. Do not
merge the crate into the RustSHT workspace merely to build or test it.

## Release status

The crate is functionally implemented and independently testable, but remains
performance-incomplete. The M5 report identifies prime real transforms and
large C2C general-prime passes that are still slower than RustFFT or DUCC on
some target lengths. The measured Rader implementation is a benchmark-only
prototype. The package keeps `publish = false`; separate publication should
wait until those performance gaps and the integration-level SHT results have
been reviewed.
