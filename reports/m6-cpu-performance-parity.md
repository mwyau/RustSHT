# M6 CPU performance parity

Branch: `opt/sht-performance-parity`, based on integrated revision
`a918e80e6bd1488d738518c87d26deb4a9c11d0f`. The campaign follows
`ASTRA_INTEGRATED_REVIEW.md`; performance parity is unproven.

## C01: workspace order-prefix compatibility

A public GL scalar T511 reproducer used identical `512×1024` grids and
`mmax=0`/`511` plans. A low-order workspace passed the full-order plan's
`prepare_workspace(..., 2)`, then `analysis_into` panicked at the prefix
slice in `src/transform.rs:1472` (length two, requested end index 512).

The workspace validator now requires at least `mmax+2` prefix entries in
parallel builds. Incompatible preparation and execution return
`ShtError::BufferLength` before output mutation or capacity changes. A larger
prefix can serve a reduced-order plan. Serial builds have no prefix allocation
and retain compatible cross-order execution.

The public regression covers scalar T511 and spin-1 T255 at q1/q2/q3,
unchanged error outputs and worker allocation, reverse-order synthesis and
analysis, and comparison with a matching workspace. It uses analytic m=0
fields; no new DUCC comparison is required for the capacity check.

Validation on Rust 1.99.0 (2026-10-17):

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: 188 unit, 11 f32 and one public
  compatibility test passed; 13 unit and one f32 test ignored.
- `cargo test --no-default-features`: 150 unit, 11 f32 and one public
  compatibility test passed; seven unit and one f32 test ignored.
- `cargo test --release --test workspace_compatibility`: passed.

Commands used `TMPDIR=/home/albert/RustSHT/target/tmp`. Existing numerical
comparisons in these suites identify DUCC 0.41.0, pinned source
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Python, explicit ignored high-L
and standalone FFT tests were not run for C01; bindings, numerical kernels,
and the standalone workspace did not change.

## Remaining work

C02 FFT extreme-range regressions and scaling; C03 spin fallback regressions;
matched current-source DUCC/SHTns benchmarks; route accounting; f64 synthesis
experiments; f32 widening experiment. The existing review records earlier
benchmarks, which are not current campaign baselines.
