# Reusing input pairs in the radix-31 convolution

Date: 2026-10-15. Baseline: M9 commit `c4b0fd5`.

## Result

The N=682 f64 radix-31 kernel now treats the radix as a compile-time constant
and computes the 15 symmetric input-pair sums and rotated differences once per
outer group. The frequency loop reuses those values for each pair of output
frequencies. The transform signs, output ordering, and normalization are
unchanged.

Six alternating process pairs against M9 improved the N=2049, 1,024-row target
by 10.8% for R2C and 8.3% for C2R. Both paired 95% intervals were above parity.
The isolated N=682 C2C control improved at 22 and 44 rows in both directions.
The kernel uses two 15-element local arrays, totaling 480 bytes of f64 complex
values. It adds no plan or workspace payload: N=2049 uses 190,721 plan bytes
and 159,856 workspace bytes.

## Numerical checks and dispatch

Runtime AVX2 detection and the N=682 radix-31 dispatch are unchanged from M9.
The kernel still handles DC separately, computes positive and negative
frequency outputs together, and evaluates frequency 15 with a scalar tail.
Inverse inputs outside the deferred-normalization safety range use the scalar
pass.

The direct-DFT unit test covers both directions and `l1` values
`[1, 2, 3, 22, 23]`, including inverse scaling and output guards. The N=682
integration test compares forward and inverse transforms with RustFFT 6.4.1.
The route test checks AVX2 selection and the high-amplitude inverse fallback.
The N=2049 route trace records the same 14 vector frequencies and one scalar
tail as M9. See
[`the route diagnostics`](../benches/results/m10-radix31-route-diagnostics.txt).

## Paired latency against M9

Each process reported the median of seven warmed samples. Six alternating
process pairs were pinned to logical CPU 30. Ratios are `M9 time / M10 time`,
so values above one favor M10. Intervals are 95% bootstrap intervals over the
six paired ratios. Plan and workspace construction were measured separately
from public operation time.

| N | Rows | R2C speedup (95% interval) | C2R speedup (95% interval) |
|---:|---:|---:|---:|
| 2049 | 1 | 1.131 (1.114–1.149) | 1.128 (1.107–1.195) |
| 2049 | 32 | 1.130 (1.111–1.139) | 1.097 (1.091–1.101) |
| 2049 | 1024 | 1.121 (1.108–1.130) | 1.090 (1.088–1.099) |

At 1,024 rows, median public-operation times were 53.926 µs for M10 R2C and
57.322 µs for M10 C2R, compared with 60.328 µs and 62.700 µs for M9.

At 22 and 44 rows, standalone N=682 C2C forward speedups were 1.165
(1.156–1.216) and 1.162 (1.150–1.327); inverse speedups were 1.130
(1.126–1.278) and 1.133 (1.121–1.188). Wider upper bounds reflect variation
in some paired samples; all four intervals were above parity.

The N=241, N=257 and N=723 controls had no median slowdown above 1.7%. N=241
C2R was 1.7% slower by the median, with an interval below parity. N=257 C2R
was 0.7% slower by the median, with an interval crossing parity. N=241 R2C
was 1.3% slower by the median, with an interval crossing parity. Both N=723
directions improved by less than 1.5%. The N=257 R2C control improved 3.4%.
These one-dimensional FFT measurements do not establish a whole-SHT speedup.

Raw samples, configurations, and per-cell summaries are recorded in the
[N=2049 acceptance capture](../benches/results/m10-radix31-target-acceptance-six-pairs.jsonl),
the [N=682 C2C capture](../benches/results/m10-radix31-n682-control-six-pairs.jsonl),
the N=241, N=257 and N=723 control captures, and the
[acceptance summary](../benches/results/m10-radix31-acceptance-summary.csv).

## Instruction profile

Callgrind 3.26.0 recorded 137,506,704 instruction references for 100 normal
N=2049 C2R operations, compared with 159,052,105 for M9, a 13.5% reduction.
The exclusive radix-31 kernel count fell from 84,962,807 references (53.42% of
the M9 total) to 63,418,383 (46.12%). These totals include benchmark setup and
invocation costs; instruction references are not elapsed time or cycle counts.

The [M9 profile](../benches/results/m9-radix31-candidate-c2r-callgrind.out) and
[M10 profile](../benches/results/m10-radix31-candidate-c2r-callgrind.out)
record the compared runs.

## Build and validation

Measurements used an AMD Ryzen 9 5950X under Linux x86-64, Rust 1.99.0 stable,
the release bench profile, a portable compiler target, runtime AVX2, and one
process pinned to CPU 30. The M9 baseline executable SHA-256 is
`509de8e5c52b7c97ef1a331a12cf5bc85aa23e7263fa9293675a2a594f1ef4e6`. The final
M10 candidate executable SHA-256 is
`4a807ddf1d03beb9d039eecb500355d70a94e48e2388acff46500178abbe7887`.

The following checks passed in `crates/rustsht-fft/`:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `TMPDIR=/dev/shm cargo test --workspace --all-features` (100 tests passed)
- `cargo test --release --lib f64_complex_general_prime_radix31_ido1_matches_direct_dfts`
- `cargo test --release --lib m9_c2c_682_selects_avx2_radix31_and_guards_unsafe_inverse_inputs`
- `cargo test --release --test complex f64_682_radix31_transform_matches_rustfft_forward_and_inverse`
- `cargo bench --locked --bench rustfft --no-run`

`/tmp` was a full tmpfs during validation; the complete workspace test passed
with `/dev/shm` as its temporary directory. No M10 cross-library campaign was
run. The M8 cross-library measurements are prior context and are not M10
comparison results.
