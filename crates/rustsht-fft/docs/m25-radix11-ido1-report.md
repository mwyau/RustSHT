# M25: terminal radix-11 complex pass

The AVX2 terminal radix-11 pass improves C2C transforms at N=726, 1,452 and
2,904 by 1.43–1.63× relative to M24 across two six-pair captures. All 12
target intervals exclude parity. The largest non-target median slowdown is
3.5% at R2C N=257 in the repeat capture; all non-target median slowdowns are
below 5%.

## Implementation

M25 adds a private f64 AVX2 kernel for radix-11 passes with `ido == 1`. Each
vector handles two outer groups, using the existing radix-11 butterfly. An
odd final outer group uses the scalar pass. Runtime dispatch selects the
kernel only when AVX2 is available and inverse normalization is safe. f32,
unsupported CPUs, and unsafe inverse inputs retain their existing scalar
routes. The kernel adds no plan or workspace storage.

The out-of-line dispatcher is in
[`src/complex/radix11_ido1.rs`](../src/complex/radix11_ido1.rs). Keeping it in
a child module places its machine code after the main complex FFT code in the
linked executable. The N=682 first-pass symbol then shifts 0x210 bytes from
M24; an earlier layout shifted it 0xb00 bytes and coincided with a larger
N=2,049 C2R slowdown. The final staged source patch SHA-256 is
`a08d995064fc04c24a55de912118ec5dadbdd0a814ffed58211cb74d24e818de`.

## Measurement method

The reference executable was built from M24 source commit `9a85e38` and has
SHA-256 `3e9b409212cfea38c207ab3aeb88f00a172771f29ef1a7a4513f933505b69f37`.
The candidate uses the same source plus the M25 patch and has SHA-256
`ec3f0c4837c5b8212dc65377d90057fdd8bb740d6747b0ad8279fe88121479cd`.
Both used Rust 1.99.0 stable, the optimized bench profile, portable x86_64
target settings, and runtime SIMD dispatch.

Two independent six-pair comparisons ran on an AMD Ryzen 9 5950X, with each
process pinned to logical CPU 30. Each covers the 34 current C2C and real FFT
SHT operation and length cells, including scalar and spin-1 row batches. Pair
order alternated candidate-first and reference-first. Each process warmed and
timed seven samples per operation. The summary divides batch operation time
by row count and reports nanoseconds per transform. Workspace creation, plan
creation and workspace bytes are also retained.

The paired speedup is the median of six M24/candidate ratios. Each 95%
interval is the exact 2.5th–97.5th percentile over all 46,656 bootstrap
resamples of those six ratios. All 24 processes exited successfully. No
separate SHT or FFT benchmark process ran during either capture. In the first
capture, one-minute load averages ranged from 1.51 to 2.18; the five-minute
average before it was 3.02 after a separate SHT test had finished. Before the
repeat, one-minute load was 0.66 and five-minute load was 1.40; one-minute
load during the repeat ranged from 0.66 to 1.28.

## Target results

| Operation | Rows | First M24 / M25 (95% interval) | Repeat M24 / M25 (95% interval) |
|---|---:|---:|---:|
| C2C forward N=726 | 361 | 1.626× [1.622, 1.632] | 1.611× [1.569, 1.659] |
| C2C inverse N=726 | 361 | 1.476× [1.470, 1.480] | 1.467× [1.409, 1.492] |
| C2C forward N=1,452 | 721 | 1.602× [1.572, 1.624] | 1.611× [1.546, 1.618] |
| C2C inverse N=1,452 | 721 | 1.456× [1.445, 1.484] | 1.453× [1.414, 1.467] |
| C2C forward N=2,904 | 721 | 1.561× [1.536, 1.574] | 1.553× [1.490, 1.566] |
| C2C inverse N=2,904 | 721 | 1.435× [1.428, 1.443] | 1.425× [1.378, 1.438] |

Candidate and M24 workspace sizes match at 11,968 bytes for N=726, 23,584
bytes for N=1,452 and 46,816 bytes for N=2,904.

## Regression controls

A ratio below 1 means M25 was slower. Selected controls with median slowdowns
are listed below for both captures. Several repeat intervals exclude parity.
The largest repeat slowdown is 3.5%, below the 5% control limit used in
earlier FFT milestones.

| Workload | First M24 / M25 (95% interval) | Repeat M24 / M25 (95% interval) | Repeat median change |
|---|---:|---:|---:|
| C2C inverse N=146 | 0.992× [0.981, 0.996] | 0.986× [0.971, 1.003] | +50 ns |
| C2C inverse N=722 | 0.982× [0.964, 0.988] | 0.982× [0.976, 0.995] | +237 ns |
| C2C inverse N=1,024 | 0.995× [0.976, 0.999] | 0.993× [0.988, 1.000] | +26 ns |
| C2R N=723 | 0.979× [0.974, 0.995] | 0.983× [0.973, 1.005] | +226 ns |
| R2C N=257 | 0.975× [0.967, 0.982] | 0.965× [0.956, 0.976] | +66 ns |
| C2R N=2,049 | 0.971× [0.959, 1.003] | 0.974× [0.966, 0.985] | +832 ns |
| C2R N=4,097 | 0.987× [0.976, 0.998] | 0.985× [0.975, 0.993] | +1,275 ns |
| R2C N=4,097 | 0.985× [0.979, 0.995] | 0.989× [0.970, 1.037] | +962 ns |

The N=2,049 C2R plan uses the special N=682 Rader route, which does not call
the new `ido == 1` kernel. Its measured slowdown was 2.9% in the first module
capture and 2.6% in the repeat. An earlier capture with the helper in
`complex.rs` measured a 6.7% slowdown in this cell; placing it in its own
module reduced the observed slowdown. The earlier captures remain preserved
as diagnostics and are not combined with either final summary.

## Numerical validation and review

The AVX2 kernel test compares both directions with direct DFT sums for outer
group counts `l1 = 2, 3, 6, 7, 12, 13, 24, 25`, and checks output guards and
the untouched odd tail. Route tests cover the three target plans, odd-tail
dispatch, high-amplitude inverse fallback and f32 scalar fallback. The full
debug and release test suites each passed 106 tests. Formatting and Clippy
checks passed. A read-only review found no concrete issue in the equations,
scaling, bounds, tail handling, dispatch or fallbacks. The benchmark host
supports AVX2; a non-AVX2 hardware run was not available.

This report compares M25 with M24 and does not repeat the cross-library DUCC,
RustFFT, FFTW or Admiral comparison. Earlier cross-library measurements are
recorded in the [M23 report](m23-final-performance-report.md).

## Supporting data

The final configuration, raw process capture and 34-cell summary are
[`m25-radix11-ido1-module-config.json`](../benches/results/m25-radix11-ido1/m25-radix11-ido1-module-config.json),
[`m25-radix11-ido1-module-six-pairs.jsonl`](../benches/results/m25-radix11-ido1/m25-radix11-ido1-module-six-pairs.jsonl),
and [`m25-radix11-ido1-module-summary.csv`](../benches/results/m25-radix11-ido1/m25-radix11-ido1-module-summary.csv).
The repeat configuration, raw process capture and summary are
[`m25-radix11-ido1-repeat-config.json`](../benches/results/m25-radix11-ido1/m25-radix11-ido1-repeat-config.json),
[`m25-radix11-ido1-repeat-six-pairs.jsonl`](../benches/results/m25-radix11-ido1/m25-radix11-ido1-repeat-six-pairs.jsonl),
and [`m25-radix11-ido1-repeat-summary.csv`](../benches/results/m25-radix11-ido1/m25-radix11-ido1-repeat-summary.csv).
The exact paired-bootstrap summarizer is stored alongside these files. The
initial capture overlapped a separate SHT process during pairs five and six;
the subsequent refactored capture exposed the code-layout control described
above. Both are preserved with their configurations and summaries. The first
module capture's environment record contains a preliminary patch
digest that omitted the new untracked child module; its configuration retains
that captured value and records the corrected staged-patch digest. The repeat
capture contains the corrected digest. Both measured executable hashes are
the same.
