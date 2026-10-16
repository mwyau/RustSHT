# Three-frequency accumulation in the AVX2 radix-31 pass

Date: 2026-10-15. Baseline: M16 commit `004faf3`.

## Result

The f64 AVX2 radix-31 `ido == 1` pass now accumulates three adjacent
output-frequency pairs during each branch traversal, then stores all six
frequency groups. The remaining vector pair and scalar endpoint retain their
existing paths. The operation order within each frequency, inverse scaling,
dispatch, fallback and workspace layout are unchanged.

Six alternating process pairs favor the candidate in every N=2049 real
transform cell. For the SHT-shaped 1,024-row case, R2C improves 3.9% and C2R
improves 3.2% by paired median; all six N=2049 confidence intervals are above
parity. At N=682, all five multirow or inverse C2C cells have 3.7–5.3% lower
latency and intervals above parity. The one-row forward C2C interval crosses
parity. Controls at N=241, 257 and 723 stay within 1% of parity by paired
median. No plan or workspace bytes were added.

## Paired latency against M16

Each process reports the median of seven warmed samples per cell. Six
alternating process pairs ran on an AMD Ryzen 9 5950X, pinned to logical CPU
30. Ratios are `M16 time / M17 time`; values above one favor M17. Intervals
are percentile-bootstrap 95% intervals over paired process ratios. Times are
public-operation microseconds.

| N | Rows | R2C M16 → M17 | Ratio (95% interval) | C2R M16 → M17 | Ratio (95% interval) |
|---:|---:|---:|---:|---:|---:|
| 2049 | 1 | 33.058 → 31.839 | 1.039 (1.024–1.057) | 40.942 → 34.124 | 1.185 (1.039–1.212) |
| 2049 | 32 | 33.235 → 31.781 | 1.044 (1.028–1.068) | 36.210 → 34.827 | 1.040 (1.037–1.047) |
| 2049 | 1024 | 33.378 → 32.116 | 1.040 (1.021–1.068) | 36.630 → 35.348 | 1.033 (1.018–1.048) |

| N | Rows | C2C forward M16 → M17 | Ratio (95% interval) | C2C inverse M16 → M17 | Ratio (95% interval) |
|---:|---:|---:|---:|---:|---:|
| 682 | 1 | 3.807 → 3.667 | 1.078 (0.921–1.248) | 4.544 → 4.354 | 1.046 (1.016–1.052) |
| 682 | 22 | 3.550 → 3.364 | 1.056 (1.026–1.081) | 4.541 → 4.378 | 1.038 (1.018–1.048) |
| 682 | 44 | 3.564 → 3.378 | 1.054 (1.028–1.074) | 4.551 → 4.372 | 1.042 (1.018–1.051) |

The N=241 control medians are 0.6% slower in both real directions, with
intervals crossing parity. N=257 is within 0.9% of parity. At N=723, the
R2C median is 0.9% slower with its interval below parity; C2R is 0.7% slower
with an interval crossing parity. These small control changes are below the
5% regression limit. Workspace sizes are unchanged: 159,856 bytes at N=2049,
11,904 bytes at N=682, 35,120 bytes at N=241, 46,032 bytes at N=257 and
53,536 bytes at N=723.

## Code generation

The candidate radix-31 helper is 0x1804 bytes, compared with 0x1820 bytes in
M16. Its stack frame is 64 bytes larger, consistent with the changed local
twiddle-table layout. The inspected hot loop contains AVX2 multiply/add/sub
operations; no accumulator spill/reload was evident. This is a qualitative
disassembly check, not a cycle or instruction-count profile.

## Numerical checks and validation

The radix-31 pass-level direct-DFT test passed for forward and inverse
transforms, all tested `l1` layouts and output guards. The complete crate
workspace all-feature suite passed (101 tests). All-target/all-feature
Clippy passed with warnings denied, formatting check passed, and the release
benchmarks compiled. No changes were made to planner dispatch or scratch
ownership.

Measurements used Rust 1.99.0 stable, the Cargo bench profile, a portable
compiler target and runtime AVX2 dispatch. The candidate binary SHA-256 is
`9527a7b7d241fb5fc157851a81f3c2ff79dec101e5fb22f391b24ab45c824ffc`; the
M16 baseline binary SHA-256 is
`669e93520be31092ddf6c62d19b26f69a2e38a1d29005e35d27e36c448ac8bfd`.
Six-pair raw samples, configurations and summaries are in
[`the M17 result directory`](../benches/results/m17-radix31-three-frequency/).
The current-source cross-library measurements are documented in
[`m17-cross-library-report.md`](m17-cross-library-report.md).
