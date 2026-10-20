# DUCC and Rust reference update

The active performance reference is DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, fetched from `mreineck/ducc` branch
`ducc0` on 2026-10-20. The previous benchmark pin remains
`80388904b8cfc32400ba84af8a6a45aabfe60f86` in its archived measurements.

The five SHT, SIMD and constants files checked by the scalar-analysis benchmark
are byte-identical at both revisions. `SHA256SUMS` records their hashes at the
new pin. The newer revision includes upstream pointer-access and packaging
changes elsewhere in the repository; the examined scalar SHT implementation
did not change.

Rust 1.99.1 was installed and selected in `rust-toolchain.toml`. The exact
compiler build and LLVM version are recorded in `rustc-1.99.1-vv.txt`.
