# M7 real Rader route at N=4097

Date: 2026-10-15. Baseline: M6 commit `2375e0d`.

## Result

The f64 real plan for N=4097 now uses Rader for its terminal radix-241 pass
(`l1=17`, `ido=1`). The Rader convolution length is 240. The plan also
constructs the radix-241 Bluestein fallback with convolution length 484;
the existing safety checks select that route for inputs outside the Rader
range.

Six alternating process pairs met the predeclared target gate at 2,048 rows:
both R2C and C2R improved by more than 5%, with paired 95% intervals above
parity. The route also passed the one- and 32-row checks. The N=257 C2R
control was 2.1% slower by paired median, below the 5% regression threshold.

The change adds 22,400 bytes to the reported plan payload and 4,096 bytes to
each workspace at N=4097. First-call and plan-construction costs are reported
below. These standalone FFT measurements do not establish a whole-SHT speedup.

## Correctness and dispatch

`rader_4097_matches_independent_real_transform_references` compares forward
and inverse results with independently evaluated direct DFT references and
RustFFT 6.4.1. It also checks a high-amplitude constant signal, a high-amplitude
independent inverse spectrum, finite outputs, and workspace reuse after the
extreme-input case. The unit route test verifies ordinary Rader execution,
high-amplitude and subnormal safety fallback, the N=4097 Bluestein fallback
plan, and high-amplitude inverse fallback. A four-thread test shares the plan
across workers with one workspace per worker.

The route trace records Rader-241 in both directions. On the benchmark host,
the nested 240-point complex FFT uses the AVX2 radix-2 first prefix and the
14-vector portion of its radix-8 pass; its scalar tail and radix-3/radix-5
passes execute as scalar code. Runtime feature detection guards those vector
routes. The trace is in
[`m7-rader-4097-route-diagnostics.txt`](../benches/results/m7-rader-4097-route-diagnostics.txt).

The Sol High review identified two test gaps: AVX2 route assertions needed a
runtime feature guard, and the N=4097 inverse safety fallback needed explicit
coverage. Both were fixed. The follow-up review found no remaining actionable
issue with the route, scratch ownership, scaling, or fallback behavior.

## Paired latency against M6

Each process reported the median of seven warmed samples. Six alternating
process pairs were pinned to logical CPU 30. Ratios are `M6 time / M7 time`,
so values above one favor M7. Intervals are 95% bootstrap intervals over the
six paired ratios. Plan and workspace construction were measured separately.

| N | Rows | R2C speedup (95% interval) | C2R speedup (95% interval) |
|---:|---:|---:|---:|
| 4097 | 1 | 2.229 (2.223–2.249) | 2.064 (2.020–2.096) |
| 4097 | 32 | 2.217 (2.212–2.249) | 2.036 (2.032–2.045) |
| 4097 | 2048 | 2.208 (2.201–2.243) | 2.002 (1.971–2.010) |
| 241 | 120 | 1.006 (1.000–1.020) | 1.004 (0.997–1.009) |
| 257 | 128 | 0.991 (0.986–1.004) | 0.979 (0.971–0.987) |
| 723 | 361 | 1.004 (0.999–1.010) | 0.990 (0.985–1.003) |

At N=4097 and 2,048 rows, median operation times were 103.280 µs R2C and
119.729 µs C2R, compared with 228.353 µs and 239.486 µs for M6. The N=257
C2R paired interval indicates a small regression; its paired median remains
inside the 5% limit. The N=723 C2R interval crosses parity. N=241 is the
Bluestein control.

At the 2,048-row target, median plan construction took 94.49–94.52 µs for M7
and 82.46–82.91 µs for M6. Workspace construction took 1.73 µs for M7 and
1.59–1.61 µs for M6. The reported N=4097 plan payload increased from 127,953
to 150,353 bytes; workspace payload increased from 129,728 to 133,824 bytes.
The payload counts initialized elements and exclude allocator metadata.

A separate six-pair one-row campaign measured the first public operation after
plan and workspace setup, followed by seven warmed calls. First-call medians
were 109.026 µs R2C and 114.612 µs C2R for M7, versus 246.360 µs and
234.117 µs for M6. Warmed medians were 101.362 µs and 112.308 µs for M7,
versus 223.588 µs and 232.780 µs for M6. The first-call R2C interval is wider
than the warm interval; both comparisons favor M7.

Raw samples, per-process configuration, and summaries:

- [`m7-rader-six-pair-summary.csv`](../benches/results/m7-rader-six-pair-summary.csv)
- [`N=4097 paired samples`](../benches/results/m7-rader-4097-six-pairs.jsonl)
- [`N=241 paired samples`](../benches/results/m7-rader-241-six-pairs.jsonl)
- [`N=257 paired samples`](../benches/results/m7-rader-257-six-pairs.jsonl)
- [`N=723 paired samples`](../benches/results/m7-rader-723-six-pairs.jsonl)
- [`first-call summary`](../benches/results/m7-rader-4097-first-call-six-pairs-summary.csv)
- [`first-call paired samples`](../benches/results/m7-rader-4097-first-call-six-pairs.jsonl)
- [`screening summary`](../benches/results/m7-rader-screen-summary.csv)

## Comparison with other FFT libraries

The N=4097, 2,048-row target was measured in six alternating process pairs
against RustFFT 6.4.1, DUCC revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf` in portable and native builds,
system FFTW 3.3.10, and Admiral revision
`ce67bc00ff9e6de96bf5b75f7a789581105ddb58`. Each process reported the median
of seven warmed samples. Ratios are `reference time / M7 time`; values above
one favor M7.

| Reference | R2C ratio (95% interval) | C2R ratio (95% interval) |
|---|---:|---:|
| RustFFT 6.4.1 | 0.592 (0.585–0.598) | 0.512 (0.509–0.523) |
| DUCC portable | 1.022 (0.956–1.032) | 0.887 (0.873–0.893) |
| DUCC native | 0.727 (0.707–0.736) | 0.624 (0.617–0.630) |
| FFTW 3.3.10 | 0.626 (0.624–0.636) | 0.517 (0.515–0.518) |
| Admiral | 0.615 (0.606–0.616) | 0.545 (0.537–0.547) |

RustFFT, native DUCC, FFTW, and Admiral were faster in both directions. The
portable DUCC R2C interval includes parity; its C2R result favors DUCC. These
comparisons describe only the measured N=4097 operation and row count.

The merged target summary is
[`m7-crosslib-sht-targets-summary.csv`](../benches/results/m7-crosslib-sht-targets-summary.csv).
Raw samples and build configurations are recorded for
[RustFFT](../benches/results/m7-crosslib-rustfft-6.4.1-six-pairs.jsonl),
[portable DUCC](../benches/results/m7-crosslib-ducc-c1ed01b8-portable-six-pairs.jsonl),
[native DUCC](../benches/results/m7-crosslib-ducc-c1ed01b8-native-six-pairs.jsonl),
[FFTW](../benches/results/m7-crosslib-fftw-system-3.3.10-six-pairs.jsonl), and
[Admiral](../benches/results/m7-crosslib-admiral-ce67bc00-six-pairs.jsonl).

## Build and validation

Measurements used an AMD Ryzen 9 5950X under Linux x86-64, Rust 1.99.0
(`rustc 1.99.0`, LLVM 23.1.1), the release bench profile, a portable compiler
target, and runtime AVX2. The M6 benchmark executable SHA-256 is
`4fd81a13a3564d47fbf7c982b78e53d74aa1ecd9ef8c9ed1ef50e64600393192`. The
M7 executable SHA-256 is
`028a36f5cd4a32f8c5d180341623b27c0ece10e709e44ac85bee81714a268401`; a
post-change `cargo bench --locked --bench rustfft --no-run` rebuild reproduced
that hash.

The following checks passed in `crates/rustsht-fft/`:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features` (94 passed)
- `cargo test --release --lib m7_real_4097_uses_rader_and_preserves_bluestein_fallback`
- `cargo test --release --test real rader_4097` (2 passed)
- `cargo bench --locked --bench rustfft --no-run`
