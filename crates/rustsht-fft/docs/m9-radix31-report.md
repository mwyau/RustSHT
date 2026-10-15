# AVX2 radix-31 pass in the N=682 convolution

Date: 2026-10-15. Baseline: M8 commit `9acd992`.

## Result

The f64 complex FFT at N=682 now applies an AVX2 kernel to its terminal radix-31
pass (`l1=22`, `ido=1`). This is the complex convolution FFT used by the
N=2049 real Rader-683 transform. The kernel evaluates two positive-frequency
outputs per vector and computes their negative-frequency partners at the same
time. It handles DC separately and evaluates the final positive frequency
with a scalar tail. Six alternating process pairs against M8 improved the N=2049, 1,024-row target
by 24.6% for R2C and 25.5% for C2R. The paired 95% intervals were above parity
in both directions. One-row and 32-row results also improved. The N=682 C2C
control improved at 22 and 44 rows in both directions. The kernel adds no plan or workspace storage. N=2049 retains 190,721 bytes of
plan payload and 159,856 bytes of workspace payload. The N=682 C2C plan uses
28,961 bytes and its workspace uses 11,904 bytes. These counts cover initialized
elements and exclude allocator metadata.

## Kernel and numerical checks

Runtime AVX2 detection selects the kernel only for f64 complex plans of length
682. The specialized pass also requires radix 31, `ido=1`, `l1>1`, and the
precomputed prime phase table. Inverse execution uses the vector kernel only
when deferred normalization passes the existing input-safety check; other
inputs use the scalar pass. The low-level test compares both directions with direct DFT sums for `l1` in
`[1, 2, 3, 22, 23]`, checks inverse scaling, and verifies guard values around
the output slice. An integration test compares N=682 forward and inverse
results with RustFFT 6.4.1. Route tests check runtime AVX2 selection and the
high-amplitude inverse fallback. The N=2049 Rader-683 route and its existing
Bluestein safety fallback are also covered by the full workspace suite. The route trace records 14 positive-frequency outputs in vector pairs and one
scalar tail for each radix-31 pass. The N=2049 Rader convolution uses the same
682-point complex plan. See
[`the route diagnostics`](../benches/results/m9-radix31-route-diagnostics.txt).

## Paired latency against M8

Each process reported the median of seven warmed samples. Six alternating
process pairs were pinned to logical one CPU core. Ratios are `M8 time / M9 time`, so
values above one favor M9. Intervals are 95% bootstrap intervals over the six
paired ratios. Plan and workspace construction were measured separately from
the public operation.

| N | Rows | R2C speedup (95% interval) | C2R speedup (95% interval) |
|---:|---:|---:|---:|
| 2049 | 1 | 1.246 (1.201–1.272) | 1.188 (1.093–1.258) |
| 2049 | 32 | 1.248 (1.235–1.255) | 1.257 (1.228–1.280) |
| 2049 | 1024 | 1.246 (1.238–1.257) | 1.255 (1.226–1.265) |

At 1,024 rows, median public-operation times were 62.552 µs for M9 R2C and
64.818 µs for M9 C2R, compared with 78.071 µs and 81.335 µs for M8. The standalone N=682 C2C measurements isolate the radix-31 pass. At 22 and 44
rows, forward speedups were 1.247 (1.241–1.260) and 1.255 (1.242–1.277);
inverse speedups were 1.365 (1.352–1.398) and 1.361 (1.334–1.393). The
one-row forward interval was wider because one paired sample was an outlier. The N=241, N=257 and N=723 controls had no median regression above 3.7%. N=257
R2C was 3.6% slower by the paired median, with an M8/M9 interval of
0.951–0.982; the interval corresponds to a 1.8–5.1% slowdown. N=257 C2R was
0.8% slower by the median, with an interval crossing parity. N=723 C2R was
1.8% slower by the median, also with an interval crossing parity. Other control
medians were within 1.8% of parity. These one-dimensional FFT measurements do
not establish a whole-SHT speedup. Raw samples, configurations, and per-cell summaries are recorded in the
[N=2049 acceptance capture](../benches/results/m9-radix31-target-acceptance-six-pairs.jsonl),
the [N=682 C2C capture](../benches/results/m9-radix31-n682-control-six-pairs.jsonl),
the N=241, N=257 and N=723 control captures, and the
[acceptance summary](../benches/results/m9-radix31-acceptance-summary.csv).

## Instruction profile

N=2049 C2R operations, compared with 201,895,300 for M8, a 21.2% reduction. The largest exclusive function in M8 was
`radix_general_prime_precomputed_phases` at 110,379,764 references (54.67% of
the total). In M9, `general_prime_ido1_radix31` accounted for 84,962,807
references (53.42%). These totals include benchmark setup and invocation costs;
instruction references are not elapsed time or cycle counts.

the compared runs.

## Build and validation

Measurements used an x86-64 CPU under Linux x86-64, Rust 1.99.0 stable,
the release bench profile, a portable compiler target, runtime AVX2, and one
process pinned to one CPU core. The M8 baseline executable SHA-256 is
`162c081879b49e38a7221d9038ffb5fc82847320bc4cb3f3b2ba43fe2b270156`. The final
M9 candidate executable SHA-256 is
`509de8e5c52b7c97ef1a331a12cf5bc85aa23e7263fa9293675a2a594f1ef4e6`. The following checks passed in `crates/rustsht-fft/`:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `TMPDIR=<local-path> cargo test --workspace --all-features` (100 tests passed)
- `cargo test --release --lib m9_c2c_682_selects_avx2_radix31_and_guards_unsafe_inverse_inputs`
- `cargo test --release --test complex f64_682_radix31_transform_matches_rustfft_forward_and_inverse`
- `cargo bench --locked --bench rustfft --no-run`

`/tmp` was a full tmpfs during validation; the complete workspace test passed
with `<local-path>` as its temporary directory. No M9 cross-library campaign was
run. The M8 cross-library measurements are prior context and are not M9
comparison results.
