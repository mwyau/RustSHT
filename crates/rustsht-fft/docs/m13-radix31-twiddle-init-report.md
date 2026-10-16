# Radix-31 twiddle initialization experiment

Date: 2026-10-15. Baseline: M12 commit `af0f93b`.

## Result

The experiment replaced zero-initialized local radix-31 twiddle arrays with
fixed-size `MaybeUninit` arrays. The kernel writes every cosine, sine and tail
entry before reading it through `assume_init_ref()`. This reduced the
latency improvement. The M12 implementation remains in use. The candidate binary was built with Rust 1.99.0 stable and the Cargo bench
profile for a portable target with runtime AVX2 dispatch. Measurements ran on
an x86-64 CPU, pinned to logical one CPU core. Each process reported seven
warmed samples per cell; six process pairs alternated candidate and baseline. Ratios are `M12 time / M13b time`, so values above one favor M13b. Intervals
are percentile-bootstrap 95% intervals over paired process ratios.

## N=2049 real transforms

Times are median public-operation microseconds. Every paired interval crosses
parity.

| Rows | R2C M12 → M13b | R2C ratio (95% interval) | C2R M12 → M13b | C2R ratio (95% interval) |
|---:|---:|---:|---:|---:|
| 1 | 35.692 → 35.441 | 1.001 (0.998–1.028) | 38.958 → 38.261 | 1.019 (0.961–1.110) |
| 32 | 35.936 → 35.553 | 1.006 (0.998–1.027) | 39.188 → 39.015 | 1.004 (0.988–1.018) |
| 1024 | 36.325 → 36.087 | 1.002 (0.996–1.026) | 39.846 → 39.265 | 1.014 (0.982–1.025) |

## N=682 complex controls

The N=682 C2C forward and inverse cells at 1, 22 and 44 rows also had paired
95% intervals that crossed parity. At 22 and 44 rows, paired median ratios
ranged from 1.004 to 1.009 for forward transforms and 1.004 to 1.007 for
inverse transforms. The one-row forward median favored M12, with a ratio of
0.976 and a wide interval of 0.802–1.036.

## Instruction profile

references fell from 72,501,067 for M12 to 68,378,046 for M13b (5.7%). The
exclusive `memset` count fell from 4,420,105 to 230,128. The exclusive
`general_prime_ido1_radix31` count changed from 22,232,829 to 22,299,789. These instruction counts do not measure cycles or native elapsed time.

## Validation and artifacts

The direct-DFT kernel test passed for forward and inverse transforms. Clippy
with all targets and features and warnings denied passed, as did the benchmark
build. The candidate was formatted with `cargo fmt`. The full crate test suite
was not run for this rejected experiment. The candidate source change is in the gzip-compressed
[`candidate patch`](../benches/results/m13-radix31-twiddle-init/candidate.patch.gz),
based on M12 commit `af0f93b`. Its binary SHA-256 was
`99cad798e8a8545285c6ae1420d3121d4963919cbb6a40214d0c25a33051caba`; the
baseline binary SHA-256 was
`69ce17d85fc9762da1513749f6d7d03e0a456e453fe344b5a94d7c2912cd4eff`. Six-pair raw samples, configurations and summaries for the real-transform
[`the M13 result directory`](../benches/results/m13-radix31-twiddle-init/).
