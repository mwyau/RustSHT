# Four-frequency accumulation in the AVX2 radix-31 pass

Date: 2026-10-15. Baseline: M17 commit `f3f03a3`.

## Result

The f64 AVX2 radix-31 `ido == 1` pass now accumulates four adjacent output-
frequency pairs in its main branch traversal, then accumulates the remaining
three pairs in a tail traversal. The seven frequency pairs therefore need two
branch traversals per outer value instead of three. The scalar endpoint,
frequency order, inverse scaling, dispatch, portable fallback and workspace
layout are unchanged. Six alternating process pairs favor M18 on the repeated-row workloads. For
N=682 C2C with 22 and 44 rows, paired median ratios show 2.4–2.8% lower
latency in both directions, with 95% intervals above parity. For N=2049 real
transforms with 1,024 rows, R2C and C2R improve 1.5% and 1.2%, respectively,
with intervals above parity. The single-row forward C2C measurement is noisy:
its interval crosses parity. It does not support a directional claim. The
N=241, 257 and 723 controls remain within 1% of parity by paired median. Workspace sizes are unchanged.

## Paired latency against M17

Each process reports the median of seven warmed samples per cell. Six
alternating process pairs ran on an x86-64 CPU, pinned to logical one CPU core. Ratios are `M17 time / M18 time`; values above one favor M18. Intervals
are percentile-bootstrap 95% intervals over paired process ratios. Times are
public-operation microseconds.

| N | Rows | R2C M17 → M18 | Ratio (95% interval) | C2R M17 → M18 | Ratio (95% interval) |
|---:|---:|---:|---:|---:|---:|
| 2049 | 1 | 31.900 → 31.414 | 1.020 (0.998–1.034) | 34.476 → 34.034 | 1.008 (0.883–1.099) |
| 2049 | 32 | 31.830 → 31.376 | 1.013 (1.007–1.040) | 34.978 → 34.701 | 1.009 (1.001–1.032) |
| 2049 | 1024 | 32.185 → 31.710 | 1.015 (1.002–1.035) | 35.743 → 35.303 | 1.010 (1.007–1.024) |

| N | Rows | C2C forward M17 → M18 | Ratio (95% interval) | C2C inverse M17 → M18 | Ratio (95% interval) |
|---:|---:|---:|---:|---:|---:|
| 682 | 1 | noisy; interval crosses parity | 0.927 (0.747–1.258) | 4.398 → 4.298 | 1.025 (1.018–1.037) |
| 682 | 22 | 3.347 → 3.267 | 1.028 (1.023–1.039) | 4.370 → 4.282 | 1.024 (1.015–1.030) |
| 682 | 44 | 3.377 → 3.287 | 1.028 (1.024–1.042) | 4.379 → 4.280 | 1.025 (1.017–1.028) |

Across the nine real control cells at N=241, 257 and 723, paired median
ratios range from 0.990 to 1.004. Their confidence intervals remain inside
0.982–1.022, well within the 5% regression limit. Workspace sizes are
unchanged: 159,856 bytes at N=2049, 11,904 bytes at N=682, 35,120 bytes at
N=241, 46,032 bytes at N=257 and 53,536 bytes at N=723.

## Code generation

The candidate helper is 0x13c9 bytes, compared with 0x1804 bytes in M17. Its stack frame is 512 bytes smaller. The main accumulation loop keeps its
eight AVX2 output accumulators in YMM registers; the stack references in that
loop load the precomputed cosine and sine tables. No accumulator spill/reload
was evident in the inspected loop. This is a qualitative disassembly check,
not a cycle or instruction-count profile.

## Numerical checks and validation

The radix-31 pass-level direct-DFT test passed for forward and inverse
transforms, all tested `l1` layouts and output guards. The full
`cargo test --workspace --all-features` run passed 160 unit tests and 10
integration tests; nine opt-in tests were ignored, and doc tests passed. `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo fmt --all -- --check`, `git diff --check` and the release benchmark
Build passed. Tests used an isolated temporary directory after the default temporary
storage was exhausted. An evidence audit confirmed all 60 benchmark processes exited
successfully and that the recorded hashes, alternating order and 18 summary
rows match the raw captures. Planner dispatch and workspace ownership were
not changed. Measurements used Rust 1.99.0 stable, the Cargo bench profile, a portable
compiler target and runtime AVX2 dispatch. The candidate binary SHA-256 is
`a97674f383732a07ee860887f5407c12659874ee70eb8d9ab0aa058d968dca17`; the M17
baseline binary SHA-256 is
`9527a7b7d241fb5fc157851a81f3c2ff79dec101e5fb22f391b24ab45c824ffc`. Six-pair raw samples, configurations and summaries are in
[`the M18 result directory`](../benches/results/m18-radix31-four-frequency/). The exact SHT cross-library comparison remains documented in
[`m17-cross-library-report.md`](m17-cross-library-report.md).
