# Native-f32 scalar-analysis adaptive schedule: exact-source 15-pair screen

This package records the adaptive-prefix and SIMD-preflight candidate applied
to current `dev` source `f01fb9b161769173397e50dae6c76aae97351c7f`. The source
diff SHA-256 is
`f33008bcbda147b0b194cbc6eeac84cefaed6158d9750bbf6e75f1ed0e09186d`.
The [recoverable patch](candidate.patch.gz) contains the exact source change.

The candidate uses native f32 inputs, sectoral recurrence, paired recurrence,
accumulation, Fourier spectra, FFT and output. It keeps 32-pair tiles until the
sectoral seed reaches ordinary range, then uses 256-pair tiles; an f32 SIMD
preflight also checks the input products and applies safe opposite-parity
cosine weights. f64 keeps the 256-pair tile and existing preparation path.

## GL T720 results

Fifteen fresh-process pairs rotate parent f32, candidate f32, candidate-source
f64 and pinned native-f32 DUCC 0.42.0. Each pair uses the same deterministic
fixture; the matched f64 worker widens those f32 fixture values for diagnosis.
Rust workers use `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; q1 is bound to
CPU 0 and q8 to CPUs 0–7 on the Ryzen 9 5950X. The candidate and parent both
execute the AVX2 V3 backend. The table reports medians across pairs. Ratio
intervals are percentile bootstrap 95% intervals over the 15 paired process
ratios (50,000 resamples).

| Cell | Candidate f32 ms | Matched f64 ms | f32/f64, 95% CI | DUCC f32 ms | f32/DUCC, 95% CI | Status |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 51.343 | 20.468 | 2.510 [2.500, 2.525] | 14.252 | 3.612 [3.588, 3.622] | GAP |
| Full q8 | 8.218 | 3.690 | 2.191 [2.052, 2.285] | 2.342 | 3.424 [3.134, 3.532] | GAP |
| Reduced q1 | 23.546 | 10.222 | 2.309 [2.157, 2.473] | 8.912 | 2.824 [2.544, 3.277] | GAP |
| Reduced q8 | 3.139 | 1.496 | 2.072 [2.001, 2.113] | 1.031 | 2.956 [2.822, 3.082] | GAP, borderline |

Compared with the current-source parent f32 worker, the candidate median is
0.407x / 0.414x / 0.465x / 0.473x for full q1/q8 and reduced q1/q8. The q8
samples were noisy; all four paired-bootstrap intervals remain above the 2x
f64 gate, though reduced q8 is only marginally above it. The candidate remains
well above DUCC in every cell.

The candidate output is not byte-identical to parent f32. Its random-fixture
relative L2 distance from parent is `9.432e-7` full and `1.069e-6` reduced; the
relative L2 distance from DUCC is `1.839e-5` and `2.220e-5`. Those comparisons
are not accuracy acceptance. The selected independent binary128 high-band
measurement for the same adaptive/preflight algorithm is `4.9081e-5` relative
L2 for scalar analysis; no science tolerance is defined, so accuracy remains
OPEN. Existing scalar-analysis analytic, adjoint, exceptional fallback,
workspace-reset and reference tests pass.

## Validation and provenance

- `cargo fmt --all -- --check`: passed.
- `cargo test --release --features parallel --lib scalar_analysis`: 18 passed,
  8 ignored.
- `cargo test --release --features parallel --test f32`: 14 passed, 1 ignored.
- `cargo clippy --release --features parallel --lib -- -D warnings`: passed.
- The f64 worker rebuilt from the candidate source produces identical sampled
  transform checksums to the base-source f64 worker in all four cells. The
  worker hashes and timings are in
  [f64-source-equivalence.json](f64-source-equivalence.json).
- Full pair records, fixture hashes, binary hashes and toolchain provenance
  are in [candidate-screen-15pair-exact-source.json](candidate-screen-15pair-exact-source.json).
- The archived exact-source pair records and fixture hashes preserve the
  measurements. Rebuilding native workers follows the repository
  `scripts/benchmark_ducc_f32_native.py` protocol; no machine-local runner is
  included.

This is a validated functional candidate and a large improvement over the
parent f32 route, but it does not pass the interim performance gate or
establish full-array scientific accuracy. Keep the GAP and accuracy-OPEN
statuses explicit; do not describe it as DUCC parity.
