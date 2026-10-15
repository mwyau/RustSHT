# Precomputed twiddles for the radix-31 convolution

Date: 2026-10-15. Baseline: M10 commit `b6b963d`.

## Result

The N=682 f64 radix-31 kernel now builds its paired-frequency cosine and sine
vectors before the outer-group loop. It also prepares the scalar-tail
coefficients once per call. Each outer group reuses those twiddles across all
15 symmetric input pairs.

Six alternating process pairs against M10 improved the N=2049, 1,024-row
target by 13.6% for R2C and 11.7% for C2R. Both paired 95% intervals were
above parity. The isolated N=682 C2C control improved in both directions at
22 and 44 rows. The kernel declares 7,440 bytes of fixed local arrays for the
twiddles and input-pair sums; it adds no plan or workspace payload. N=2049
continues to use 190,721 plan bytes and 159,856 workspace bytes.

## Numerical checks and dispatch

Runtime AVX2 detection, the N=682 radix-31 dispatch, the output ordering, and
the inverse safety fallback are unchanged. The kernel uses seven vector pairs
for positive frequencies 1–14 and one scalar tail for frequency 15. Negative
frequency outputs are computed in the same pass.

The direct-DFT unit test checks both directions for `l1` values
`[1, 2, 3, 22, 23]`, inverse scaling, and output guards. The N=682 integration
test compares forward and inverse transforms with RustFFT 6.4.1. The route
test checks AVX2 selection and the high-amplitude inverse fallback. The route
trace records the N=2049 Rader convolution's 682-point plan and its 14 vector
frequencies plus scalar tail. See
[`the route diagnostics`](../benches/results/m11-radix31-route-diagnostics.txt).

## Paired latency against M10

Each process reported the median of seven warmed samples. Six alternating
process pairs were pinned to logical CPU 30. Ratios are `M10 time / M11 time`,
so values above one favor M11. Intervals are 95% bootstrap intervals over the
six paired ratios. Plan and workspace construction were measured separately
from public operation time.

| N | Rows | R2C speedup (95% interval) | C2R speedup (95% interval) |
|---:|---:|---:|---:|
| 2049 | 1 | 1.147 (1.138–1.167) | 1.043 (1.032–1.094) |
| 2049 | 32 | 1.152 (1.141–1.163) | 1.134 (1.065–1.155) |
| 2049 | 1024 | 1.158 (1.148–1.172) | 1.133 (1.120–1.148) |

At 1,024 rows, median public-operation times were 46.457 µs for M11 R2C and
50.318 µs for M11 C2R, compared with 53.641 µs and 57.118 µs for M10.

At 22 and 44 rows, standalone N=682 C2C forward speedups were 1.189
(1.174–1.208) and 1.193 (1.182–1.208); inverse speedups were 1.171
(1.149–1.184) and 1.165 (1.150–1.182). The one-row inverse interval crossed
parity, so it does not establish a gain at that row count.

The N=241, N=257 and N=723 control medians showed no slowdown above 0.3%.
Their paired intervals were wider than the N=2049 intervals. The N=257 and
N=723 captures logged load averages from 5.32 to 6.54, and their intervals
include both gains and slowdowns. These control measurements do not establish
parity under the recorded host load.

Raw samples, configurations, and per-cell summaries are recorded in the
[N=2049 acceptance capture](../benches/results/m11-radix31-target-acceptance-six-pairs.jsonl),
the [N=682 C2C capture](../benches/results/m11-radix31-n682-control-six-pairs.jsonl),
the N=241, N=257 and N=723 control captures, and the
[acceptance summary](../benches/results/m11-radix31-acceptance-summary.csv).

## Instruction profile

Callgrind 3.26.0 recorded 100,508,928 instruction references for 100 normal
N=2049 C2R operations, compared with 137,506,704 for M10, a 26.9% reduction.
The exclusive radix-31 kernel count fell from 63,418,383 references (46.12%
of the M10 total) to 22,232,829 (22.12%). `execute_pass` and `radix11` were the
next largest exclusive functions in M11. These totals include benchmark setup
and invocation costs; instruction references are not elapsed time or cycle
counts.

The [M10 profile](../benches/results/m10-radix31-candidate-c2r-callgrind.out)
and [M11 profile](../benches/results/m11-radix31-candidate-c2r-callgrind.out)
record the compared runs.

## Build and validation

Measurements used an AMD Ryzen 9 5950X under Linux x86-64, Rust 1.99.0 stable,
the release bench profile, a portable compiler target, runtime AVX2, and one
process pinned to CPU 30. The M10 baseline executable SHA-256 is
`4a807ddf1d03beb9d039eecb500355d70a94e48e2388acff46500178abbe7887`. The final
M11 candidate executable SHA-256 is
`54c97e86891dfb6f76ba7f1e3483ad35edb9dbe9ce700f0780373c1998e29926`.

The following checks passed in `crates/rustsht-fft/`:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `TMPDIR=/dev/shm cargo test --workspace --all-features` (100 tests passed)
- `cargo test --release --lib f64_complex_general_prime_radix31_ido1_matches_direct_dfts`
- `cargo test --release --lib m9_c2c_682_selects_avx2_radix31_and_guards_unsafe_inverse_inputs`
- `cargo test --release --test complex f64_682_radix31_transform_matches_rustfft_forward_and_inverse`
- `cargo bench --locked --bench rustfft --no-run`

`/tmp` was a full tmpfs during validation; the complete workspace test passed
with `/dev/shm` as its temporary directory. No M11 cross-library campaign was
run. The M8 cross-library measurements are prior context and are not M11
comparison results.
