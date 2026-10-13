# Fused normalization in the real inverse terminal pass

Date: 2026-10-13. `RealMultipassPlan::inverse` writes its final radix-2 or radix-4 pass to the
caller's output and applies `1/N` at each terminal result. The previous path
wrote an unscaled transform to scratch, then read the full array and copied
scaled values to the caller. The factorization sets `ido == 1` for its last
pass; a debug assertion checks that condition. Two workspace arrays still
alternate across intermediate passes, so workspace size is unchanged. The multipass path runs only when `inverse_normalization_is_safe` accepts the
spectrum. Inputs outside that bound use the recursive residual-corrected
path. The fallback and its high-range numerical behavior are unchanged.

## Validation

`cargo fmt --manifest-path crates/rustsht-fft/Cargo.toml -- --check`,
`cargo test --manifest-path crates/rustsht-fft/Cargo.toml --all-features`, and
`cargo clippy --manifest-path crates/rustsht-fft/Cargo.toml --all-targets
--all-features -- -D warnings` passed. The test run covered 57 unit and
integration tests, including f32/f64 direct DFTs, multipass lengths through
4,096, cancellation, subnormal values, maximum-finite values, and the
inverse-normalization fast and fallback paths. The terminal radix formulas compute each unscaled result before multiplying
by `1/N`, matching the operation order of the former copyback loop. The f64
baseline and candidate checksums match exactly. The f32 checksums agree within
`1e-6 * max(1, |baseline|)`; the direct-reference tests also pass.

## f64 process comparison

The baseline is commit `d6798da`. The candidate source hashes are
`0e1dccfd5be701e7062a112c89ce922bfbcf37f55c004c5234263ea66026db78`
(`src/real_multipass.rs`) and
`e5e2e30d1e91ad431fd5c94bbe7b549ee18320e4930f1721e1e2c7ae6dedc9fb`
(`src/real_multipass/inverse_terminal.rs`). Both binaries used
`benches/rustfft.rs` SHA-256
`6a85531e042175af96e9bcdc07368c15908e46212b7d441d68c57cfb7b9b4cba`,
`rustc 1.99.0 (b940084d7, LLVM 23.1.1)`, and Cargo's optimized `bench`
profile. `RUSTFLAGS`, LTO, and `target-cpu` were unset. The binary uses the
portable x86-64 target with the existing runtime AVX2 dispatch. Baseline and candidate were built in the same temporary worktree path with
the same Cargo target directories. The benchmark ran
`bench-native-real N 1 32`, which measures one transform and 32 serial
transforms with a reused plan and workspace. Inputs and outputs are allocated
outside the timed operations. Each process warms the operation and reports
the median of seven timed samples. Twelve baseline/candidate process pairs
were pinned to logical one CPU core on an x86-64 CPU; build order alternated
within each pair. Table timings are the conventional median of twelve process
medians (the average of the two middle observations for this even sample
count). Raw gains are the median of twelve paired `1 - candidate/baseline`
values; wins count pairs where the candidate time is lower. Positive values
favor the candidate. The R2C measurements are a within-process timing control because the code
change affects C2R only. The adjusted C2R gain is
`1 - (candidate C2R / baseline C2R) / (candidate R2C / baseline R2C)`,
calculated for each process pair. This estimates the C2R change after
accounting for a shared multiplicative change in process speed; it does not
replace the raw timings. One transform, microseconds per transform:

| N | R2C baseline → candidate (raw gain; wins/12) | C2R baseline → candidate (raw gain; wins/12) | C2R R2C-adjusted gain (wins/12) |
|---:|---:|---:|---:|
| 64 | 0.176 → 0.190 (−5.83%; 1) | 0.170 → 0.161 (+5.29%; 10) | +10.70% (11) |
| 128 | 0.381 → 0.401 (−5.25%; 0) | 0.330 → 0.316 (+3.07%; 10) | +7.86% (12) |
| 256 | 0.681 → 0.702 (−2.99%; 3) | 0.631 → 0.611 (+3.92%; 10) | +6.80% (12) |
| 512 | 1.623 → 1.648 (−1.23%; 3) | 1.312 → 1.252 (+4.61%; 11) | +5.61% (12) |
| 1,024 | 3.036 → 3.066 (−0.84%; 2) | 2.690 → 2.555 (+5.26%; 12) | +5.76% (12) |
| 2,048 | 7.244 → 7.314 (−0.63%; 4) | 5.811 → 5.651 (+3.10%; 8) | +1.95% (11) |
| 4,096 | 13.250 → 13.276 (−0.23%; 4) | 12.023 → 11.552 (+3.85%; 12) | +3.92% (12) |

Batch of 32 serial transforms, microseconds per transform:

| N | R2C baseline → candidate (raw gain; wins/12) | C2R baseline → candidate (raw gain; wins/12) | C2R R2C-adjusted gain (wins/12) |
|---:|---:|---:|---:|
| 64 | 0.157 → 0.160 (−1.78%; 1) | 0.143 → 0.140 (+2.09%; 12) | +3.67% (12) |
| 128 | 0.365 → 0.372 (−1.62%; 0) | 0.304 → 0.297 (+3.11%; 12) | +4.40% (12) |
| 256 | 0.679 → 0.688 (−1.45%; 3) | 0.613 → 0.593 (+2.96%; 10) | +3.48% (12) |
| 512 | 1.622 → 1.601 (+1.02%; 9) | 1.291 → 1.223 (+4.79%; 11) | +4.12% (11) |
| 1,024 | 2.925 → 2.942 (−0.57%; 4) | 2.676 → 2.562 (+4.78%; 12) | +5.17% (12) |
| 2,048 | 6.996 → 7.036 (−0.30%; 2) | 5.793 → 5.609 (+3.85%; 10) | +3.94% (11) |
| 4,096 | 13.381 → 13.413 (−0.21%; 5) | 12.078 → 11.732 (+4.09%; 11) | +4.17% (11) |

For an equal R2C/C2R mix, each process pair's gain is
`1 - (candidate R2C + candidate C2R) / (baseline R2C + baseline C2R)`; the
reported gain is the median of the twelve paired gains. This gives gains of
0.97–2.10% for one transform and 1.62–2.72% for a 32-row call at lengths
512–4,096. At length 256, the one-transform gain is 0.38% and the batch gain
is 1.29%. The one-transform mix is slower by 0.77–2.82% at lengths 64 and
128; the batch mix is within 0.21% at those lengths. This paired aggregate
can differ from a ratio of sums of the independent table medians; at N=2,048
for one transform, those summaries are +0.97% and +0.69%, respectively. The
R2C control shows small regressions for several single-transform cases. Host-load variation was reviewed locally; exact transient load measurements remain untracked.

## f32 comparison with RustFFT

The f32 baseline and candidate use `f32_real_simd.rs` SHA-256
`76ea65520ac18ab604087c9011c81f61b5f54b597dbfb3b45ad7cf9320c2b665` and
RustFFT 6.4.1's half-length complex real adapter. Both use the default
optimized `bench` profile with no custom Rust flags. Twelve process pairs
were pinned to one CPU core, with baseline/candidate order alternating. Each
process has seven paired samples per length and operation, giving 84 paired
observations. The same deterministic f32-rounded inputs were used for both
builds. Gains are medians of paired `1 - candidate/baseline` values; positive
values favor the candidate. Counts show positive pairs. C2R/RustFFT ratios
are medians of paired per-sample ratios.

| N | R2C single | C2R single | R2C batch 32 | C2R batch 32 | C2R/RustFFT ratio, single; batch 32, baseline → candidate |
|---:|---:|---:|---:|---:|---:|
| 64 | −0.04% (39/84) | +3.13% (82/84) | −0.57% (14/84) | +3.19% (83/84) | 1.891 → 1.829; 1.894 → 1.842 |
| 128 | −0.17% (32/84) | +2.40% (84/84) | −0.32% (29/84) | +2.45% (83/84) | 2.019 → 1.969; 2.020 → 1.962 |
| 256 | −0.61% (31/84) | +3.10% (77/84) | −0.18% (35/84) | +2.94% (76/84) | 2.002 → 1.939; 1.984 → 1.927 |
| 512 | −0.03% (39/84) | +2.99% (84/84) | −0.23% (33/84) | +2.81% (84/84) | 2.010 → 1.942; 2.003 → 1.944 |
| 1,024 | −0.10% (39/84) | +3.40% (81/84) | −0.70% (26/84) | +2.76% (79/84) | 1.900 → 1.860; 1.893 → 1.859 |
| 2,048 | −0.63% (21/84) | +2.53% (80/84) | −0.68% (22/84) | +2.74% (80/84) | 1.929 → 1.856; 1.912 → 1.847 |
| 4,096 | −0.49% (26/84) | +5.26% (78/84) | −0.23% (36/84) | +5.08% (75/84) | 1.971 → 1.863; 1.968 → 1.879 |

The f32 R2C/RustFFT ratio improves slightly at six of seven lengths, although
raw R2C times vary by less than 0.7%. The C2R/RustFFT ratio improves at all
lengths; it falls from 1.89–2.02× to 1.83–1.97× for single transforms and
from 1.89–2.02× to 1.84–1.96× for batches. Control-adjusted C2R gains are
2.8–5.0% for single transforms and 2.7–5.0% for batches, with at least
75/84 positive pairs at every length. The f32 comparison does not include a new direct C++ DUCC run. The pinned
DUCC comparison and direct C++ timing tables are in
[`m4-f32-radix2-avx2.md`](m4-f32-radix2-avx2.md). The f64 DUCC comparison is
in [`m4-real-multipass.md`](m4-real-multipass.md).

## Generated code and disposition

In the portable release benchmark binary, the `RealMultipassPlan<f64>::inverse`
symbol shrank from 739 to 653 bytes. `execute_inverse_pass<f64>` remained
4,301 bytes, and the new out-of-line terminal helper is 1,076 bytes. The
inverse method calls the helper for the final `ido == 1` pass; it no longer
contains the output normalization copy loop. These symbol sizes describe code
layout and do not measure execution time. An earlier inline version grew `RealMultipassPlan<f64>::inverse` to 1,837
bytes. I replaced it with the out-of-line helper; those preliminary timings
are excluded from the tables. The tables use binaries rebuilt from the same
worktree path and Cargo target directories. The fused terminal write removes the full-length output copy from the safe
inverse path without changing scratch allocation or the residual-corrected
fallback. The f64 paired timings show small forward-control regressions at
short lengths and a 1.0–2.1% gain for a single R2C/C2R pair and 1.6–2.7% for
a 32-row pair at lengths 512–4,096. The f32 C2R/RustFFT ratios improve across
the full tested set. Keep the change while M4 profiling continues; the
remaining f32 C2R gap is about 1.83–1.97× RustFFT. Raw process records:

- [f64 baseline/candidate process samples](../benches/results/m4-real-terminal-output-pass-process-samples.csv)
- [f32 baseline samples](../benches/results/m4-real-terminal-output-pass-f32-baseline-process-samples.csv)
- [f32 candidate samples](../benches/results/m4-real-terminal-output-pass-f32-candidate-process-samples.csv)

The f64 raw file has SHA-256
`322654419bec1acb063c35745ffa5500ea8f222eca4ef7188bb5577309d11f82`. The
f32 baseline and candidate files have SHA-256
`022e765ccc54fcc52037b86e07912182a8e68a068b173fe6a6435617177a1a1b` and
`c1ec1778aafd4f078edc5b9b72244c5003685965a94f6a8b0865c1e6889c1e53`.
