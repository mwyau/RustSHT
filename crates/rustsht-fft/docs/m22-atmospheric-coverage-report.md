# M22 atmospheric FFT coverage and f32 assessment

Date: 2026-10-16. Candidate source revision: `c48e653978079da8d126757a9362c27b4b64e2c7`.
M22 adds benchmark coverage and no production code changes.

## f32 comparison

The M20 and M21 f32 benchmark executables were built with Rust 1.99.0 stable,
the optimized bench profile, portable target settings and runtime dispatch. The
processes were pinned to logical CPU 30 of an AMD Ryzen 9 5950X. Six alternating
process pairs measured N=723, 1,025, 1,443, 2,049 and 4,097, with seven warmed
samples for one transform and 32 sequential rows. Each M21 process also measured
RustFFT 6.4.1 using the same f32 inputs. The process load averages were 1.40–1.47
before execution. The configurations record the source revisions, executable
hashes, compiler and CPU policy context.

Ratios in the four middle columns are M20 time divided by M21 time. Values above
one favor M21. Intervals are 95% bootstrap intervals over six paired process
ratios. The final column gives the median RustFFT time divided by the M21 time
for one transform; values below one mean RustFFT is faster.

| N | R2C, one row | C2R, one row | R2C, 32 rows | C2R, 32 rows | RustFFT/M21, R2C / C2R |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 723 | 1.003 (1.002–1.010) | 1.004 (1.000–1.006) | 1.003 (0.997–1.036) | 1.012 (1.001–1.028) | 0.072 / 0.088 |
| 1,025 | 0.951 (0.949–0.961) | 0.996 (0.995–1.023) | 0.952 (0.949–0.962) | 0.996 (0.994–1.007) | 0.317 / 0.376 |
| 1,443 | 1.131 (1.129–1.142) | 1.111 (1.108–1.118) | 1.134 (1.130–1.145) | 1.112 (1.110–1.114) | 0.365 / 0.351 |
| 2,049 | 1.003 (1.002–1.008) | 1.006 (1.002–1.061) | 1.004 (1.002–1.006) | 1.005 (1.001–1.010) | 0.245 / 0.258 |
| 4,097 | 1.078 (1.073–1.086) | 1.064 (1.059–1.070) | 1.077 (1.073–1.087) | 1.066 (1.052–1.076) | 0.171 / 0.173 |

The general-prime pass reuse improves f32 N=1,443 by 11–13% and N=4,097 by
6–8%. N=723 and N=2,049 stay near parity. N=1,025 R2C is about 5% slower in
M21 in both workload sizes; C2R remains near parity. This is a measurable f32
regression and limits the claim to the affected larger prime-pass workloads.
The workspace byte counts match between M20 and M21 at all five lengths. M21
does not add workspace storage. RustFFT remains faster than the current f32
implementation at every measured length, by about 2.7–5.9× for N=1,025 through
N=4,097 and 11–14× at N=723.

The f32 benchmark also checks each operation against its RustFFT result. The
release regression test
`f32_large_smooth_prime_bluestein_and_sht_lengths_match_f64_references` covers
N=1,443 and N=4,097 against independent f64 references. Final all-feature f32
tests passed in both debug and release profiles.

Supporting data:

- [M22 configuration](../benches/results/m22-f32-real-pass/m22-f32-real-pass-config.json)
- [Six-pair process records](../benches/results/m22-f32-real-pass/m22-f32-real-pass-six-pairs.jsonl)
- [Paired summary](../benches/results/m22-f32-real-pass/m22-f32-real-pass-summary.csv)
- [Summary script](../benches/summarize_f32_pairs.py)

## Real longitude length coverage

For each triangular truncation `T`, the screen included minimum odd length
`2T+1`, minimum even length `2T+2`, the existing GL benchmark sampling
`2T+3`, and `GridSpec::fft_friendly_nphi(T)`. The friendly count is the smallest
2/3/5/7-smooth length at or above the minimum odd count, bounded by the next
power of two. The set contains 50 unique lengths after deduplication.

The table gives these four length choices and the actual scalar and spin-1 GL
longitude row counts (`T+1` and `2(T+1)`).

| Truncation | Minimum odd | Minimum even | GL sampled | FFT-friendly | Scalar rows | Spin-1 rows |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T42 | 85 | 86 | 87 | 90 | 43 | 86 |
| T72 | 145 | 146 | 147 | 147 | 73 | 146 |
| T85 | 171 | 172 | 173 | 175 | 86 | 172 |
| T159 | 319 | 320 | 321 | 320 | 160 | 320 |
| T255 | 511 | 512 | 513 | 512 | 256 | 512 |
| T340 | 681 | 682 | 683 | 686 | 341 | 682 |
| T360 | 721 | 722 | 723 | 729 | 361 | 722 |
| T511 | 1,023 | 1,024 | 1,025 | 1,024 | 512 | 1,024 |
| T639 | 1,279 | 1,280 | 1,281 | 1,280 | 640 | 1,280 |
| T720 | 1,441 | 1,442 | 1,443 | 1,458 | 721 | 1,442 |
| T799 | 1,599 | 1,600 | 1,601 | 1,600 | 800 | 1,600 |
| T1023 | 2,047 | 2,048 | 2,049 | 2,048 | 1,024 | 2,048 |
| T1279 | 2,559 | 2,560 | 2,561 | 2,560 | 1,280 | 2,560 |
| T2047 | 4,095 | 4,096 | 4,097 | 4,096 | 2,048 | 4,096 |
| T4095 | 8,191 | 8,192 | 8,193 | 8,192 | 4,096 | 8,192 |

The 50-length screen ran one process per unique length with one row, seven
warmed samples, and both directions. It recorded plan construction, workspace
construction and bytes, execution time, checksums, affinity and host load. A
compute-heavy RustSHT process was active during this screen; the recorded
one-minute load average was 7.83. These timings are diagnostic route evidence,
not paired performance estimates. T4095 was screened at one row only; its full
4,096/8,192-row workloads were not run.

The screen suggests that sampling choices can select very different routes.
For example, the one-process medians for T799 were 86.0 microseconds at sampled
N=1,601 and 5.7 microseconds at FFT-friendly N=1,600. That ratio is not an
acceptance result because the capture overlapped host load. The raw records
retain each measurement and the associated load value.

Supporting data:

- [Length set and benchmark configuration](../benches/m22_atmospheric_lengths.json)
- [50-length raw screen](../benches/results/m22-atmospheric/m22-atmospheric-length-screen.jsonl)
- [Directional timing, setup and workspace table](../benches/results/m22-atmospheric/m22-atmospheric-length-screen.csv)
- [Screen runner](../benches/screen_atmospheric_lengths.py)
- [Summary script](../benches/summarize_atmospheric_screen.py)

## Meridional complex FFT dimensions

RustSHT's CC/F1 resampling call sites iterate Fourier orders `m=0..=mmax`, so a
truncation `T` supplies `T+1` independent profiles. This row count is distinct
from the longitude row count. At the minimum exact-analysis grids, CC uses
`ntheta=T+2` and F1 uses `ntheta=T+1`; both give original reflected length
`2(T+1)`. The prepared half-length `H` is `good_size_complex(T+1)`, where the
helper searches lengths composed of 2, 3, 5, 7 and 11. The prepared transform
length is `2H`. CC at its minimum exact-analysis grid and F1 both require
resampling, whose full transform length is `4H`. Each order executes four
meridional FFT operations in either direction on this resampling route. Spin
components use the same order loop separately.

| Truncation | Profiles | Original length | Prepared length | Resampling full length |
| ---: | ---: | ---: | ---: | ---: |
| T42 | 43 | 86 | 88 | 176 |
| T255 | 256 | 512 | 512 | 1,024 |
| T720 | 721 | 1,442 | 1,452 | 2,904 |
| T1279 | 1,280 | 2,560 | 2,560 | 5,120 |
| T2047 | 2,048 | 4,096 | 4,096 | 8,192 |

These sizes follow the `CCResamplingPlan` call sites and `good_size_complex`
implementation in the RustSHT source revision documented in
[`rustsht-workloads.md`](rustsht-workloads.md). The M22 real-length screen does
not substitute longitude lengths for meridional transforms. This milestone
derives the meridional workload dimensions but does not add a separate
meridional timing matrix.

## Validation

The following checks passed on the M21 candidate source:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features` — 103 tests passed
- `cargo test --workspace --all-features --release` — 103 tests passed

The suites include independent direct DFT comparisons, analytic cases,
RustFFT comparisons, f32/f64 paths, maximum-finite and subnormal inputs,
cancellation and nonfinite guards, inverse normalization, fallback routes, and
shared-plan execution with independent workspaces. The f32 general-prime
results do not motivate a separate f32 kernel: the same candidate gives clear
gains at N=1,443 and N=4,097, with the N=1,025 R2C slowdown recorded above.
No threading dependency or public API was added.
