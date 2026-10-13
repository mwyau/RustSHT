# DUCCFFT

DUCCFFT is a standalone Rust crate for one-dimensional real and complex FFTs.
The current revision exposes native f32 and f64 complex transforms using
mixed-radix plans and caller-owned workspaces. Real transforms and optimized
general-prime passes remain under development.

The crate will use reusable plans and caller-owned scratch for f32 and f64
transforms. Its design follows the one-dimensional algorithms in DUCC at
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

The package has a nested Cargo workspace. Run its checks with:

```sh
cargo test --manifest-path crates/duccfft/Cargo.toml
```

The direct C++ DUCC reference harness and RustFFT comparison benchmark are in
`benches/`. Build instructions and benchmark conventions are in
`benches/README.md`.

DUCCFFT is not an official DUCC release. The crate is licensed under
BSD-3-Clause; reference-source notices are in `LICENSE-THIRD-PARTY`.
