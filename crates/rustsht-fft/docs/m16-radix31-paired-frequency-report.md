# Paired-frequency accumulation in the radix-31 pass

Date: 2026-10-15. Baseline: M12 commit `af0f93b`.

## Result

The f64 radix-31 `ido == 1` kernel processes two adjacent output-frequency
pairs during each branch traversal. It packs each branch's symmetric input
sum and rotated difference once, then updates both frequency groups. Each
output retains its existing multiply/add order and normalization. The change
adds no plan or workspace storage and keeps the runtime AVX2 fallback.

Six alternating process pairs against M12 improved all N=2049 R2C cells by
7.7–8.1%. C2R improved by 7.6–8.5% by the paired medians; the 1-row C2R
interval crossed parity, while the 32- and 1,024-row intervals were above
parity. N=682 C2C forward transforms improved by 12.4% at 22 and 44 rows, and
inverse transforms improved by 9.9–11.3% at 1, 22 and 44 rows. The N=682
one-row forward result was inconclusive and had a 4.1% slower candidate
median.

## Paired latency against M12

Each process reported seven warmed samples per cell. Six alternating process
pairs ran on an AMD Ryzen 9 5950X, pinned to logical CPU 30. Ratios are
`M12 time / M16 time`, so values above one favor M16. Intervals are
percentile-bootstrap 95% intervals over paired process ratios. Times are
median public-operation microseconds.

| N | Rows | R2C M12 → M16 | R2C ratio (95% interval) | C2R M12 → M16 | C2R ratio (95% interval) |
|---:|---:|---:|---:|---:|---:|
| 2049 | 1 | 35.507 → 32.822 | 1.080 (1.054–1.097) | 41.172 → 38.407 | 1.085 (0.9997–1.162) |
| 2049 | 32 | 36.019 → 33.235 | 1.077 (1.067–1.101) | 38.997 → 36.145 | 1.080 (1.055–1.086) |
| 2049 | 1024 | 36.367 → 33.528 | 1.081 (1.074–1.089) | 39.364 → 36.572 | 1.076 (1.046–1.087) |

| N | Rows | C2C forward M12 → M16 | Ratio (95% interval) | C2C inverse M12 → M16 | Ratio (95% interval) |
|---:|---:|---:|---:|---:|---:|
| 682 | 1 | 4.083 → 4.258 | 0.959 (0.853–1.228) | 5.035 → 4.599 | 1.099 (1.081–1.103) |
| 682 | 22 | 3.999 → 3.553 | 1.125 (1.114–1.139) | 5.052 → 4.554 | 1.113 (1.103–1.117) |
| 682 | 44 | 4.009 → 3.571 | 1.124 (1.118–1.132) | 5.072 → 4.560 | 1.112 (1.107–1.113) |

The N=241 controls were within 0.4% of parity with intervals crossing parity.
N=257 C2R was 0.7% slower at the paired median, with its interval below
parity; N=257 R2C and both N=723 intervals crossed parity. The N=257 C2R
slowdown was below 1%.

The N=2049 plan uses 159,856 workspace bytes in both builds. The N=682 plan
uses 11,904 workspace bytes in both builds. No plan or workspace payload was
added.

## Instruction profile

Callgrind 3.26.0 profiled 100 normal N=2049 C2R operations. M12 recorded
72,501,067 instruction references; M16 recorded 74,173,158. The exclusive
`general_prime_ido1_radix31` count rose from 22,232,829 to 23,902,709.
Instruction references are not cycle or elapsed-time measurements; this
profile did not show an instruction-count reduction.

## Numerical checks and validation

The radix-31 pass-level direct-DFT test passed for forward and inverse
transforms, all tested `l1` layouts, and output guards. The N=682 integration
test matched RustFFT 6.4.1 in both directions. The all-feature workspace suite
passed (101 tests). Formatting, all-target/all-feature Clippy with warnings
denied, and the benchmark build passed. No cross-library performance campaign
was run.

Measurements used Rust 1.99.0 stable, the Cargo bench profile, a portable
compiler target, and runtime AVX2 dispatch. The candidate binary SHA-256 is
`669e93520be31092ddf6c62d19b26f69a2e38a1d29005e35d27e36c448ac8bfd`; the
M12 baseline binary SHA-256 is
`69ce17d85fc9762da1513749f6d7d03e0a456e453fe344b5a94d7c2912cd4eff`.
Six-pair raw samples, configurations, summaries and the compressed Callgrind
profile are in
[`the M16 result directory`](../benches/results/m16-radix31-paired-frequency-accumulation/).
