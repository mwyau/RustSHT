# Tile-128 SIMD-prefix correctness checks

## Source

The checks used the isolated SIMD-prefix synthesis candidate based on source
commit `739275d20987fec35506916cb682f961e8a7ffaf`. The candidate
`src/legendre/synthesis_fast.rs` SHA-256 was
`b4a849f0fbac0537cf13a203462d75f233cdb5d0a11ef4ce9ac0d956b0521d10`; the
SIMD-prefix patch and performance status are recorded in the
[screen report](simd-prefix-screen-t2047-q1.md).

## Validation

`cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
The all-features release workspace suite passed: 202 unit tests, four extreme
FFT tests, 11 f32 tests, and one workspace-compatibility test. Fourteen unit
tests and one f32 test were ignored. The release workspace suite also passed
with `--no-default-features` with the same counts.

The opt-in scalar high-bandlimit test passed with all features and with
`--no-default-features`. It checked T511, T1023, and T2047 on GL, CC, and F1
against analytic fields and selected DUCC 0.41.0 outputs. The commands were:

```sh
cargo test --release --lib transform::tests::m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form -- --ignored --exact --nocapture
cargo test --no-default-features --release --lib transform::tests::m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form -- --ignored --exact --nocapture
```

At T2047, GL maximum absolute errors were `2.092e-13` for the selected DUCC
row, `9.953e-12` for the analytic map, and `4.175e-14` for analysis
coefficients. Candidate unit tests also passed for scaled-prefix transition,
exponent rebasing, unsafe range fallback, mixed polar tiles, and staged polar
fallback.

The all-features and no-default-features test executables had SHA-256 values
`9d3d6e2f2e1e63551a8e4f5687716bed353248b4aaed7c670d30134826b9dba3` and
`fbec307e9dc6d82e5d503b870d8ed37a462c6d97acf90e920493f85decc326b6`,
respectively. Rust and Cargo 1.99.0 were used.

## Performance status

These checks add no performance measurement. The earlier T2047 one-process
screen compared SIMD prefix with the scalar-prefix intermediate. Its RustSHT
median was 1,143.564 ms versus 1,367.195 ms, but the DUCC series failed the
fixed CV and drift limits. The screen is inconclusive and does not compare
this source with the clean production baseline. The SIMD-prefix source remains
experimental and is not routed in production.

The six-pair T1023 and T2047 q1 results belong to a different scalar-prefix
source, SHA-256
`edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`. Those
results do not establish performance for the SIMD-prefix source tested here.
