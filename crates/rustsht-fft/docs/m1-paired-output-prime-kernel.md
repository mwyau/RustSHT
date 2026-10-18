# M1 paired-output general-prime kernel

Date: 2026-10-15. This change pairs the positive and negative frequency
outputs in the existing precomputed-phase C2C general-prime pass. It preserves
the planner, phase-index table, workspace layout, and inverse safety paths.

## Kernel

For an odd radix `p`, output zero is accumulated separately from `x[0]` and
the symmetric input sums `x[j] + x[p-j]`. For each `k = 1..=(p-1)/2`, the
kernel loads one cached phase per branch, forms the symmetric input sum and
difference once, then uses their cosine and sine contributions for both
outputs `k` and `p-k`. `directed_root` retains the existing forward and inverse
sign convention. The outputs are not treated as a conjugate pair; that would
be invalid for general complex input. The pass adds no coefficient table or scratch. It keeps the existing compact
O(p²)-entry byte phase-index table, and workspace byte counts are unchanged. The
previous precomputed-phase arithmetic remains under `cfg(test)` for direct
candidate comparison. The generic prime fallback remains in place for plans
without a precomputed phase table.

## Numerical checks

Direct forward and independently supplied inverse spectra are checked against
direct DFTs for radices 13, 17, 19, 31, 41, 73, 103, and 109. The f32/f64
cases include constants, impulses, Fourier modes, cancellation, sparse data,
near-maximum finite values, and subnormals. The internal kernel test compares
the paired kernel with the previous per-output arithmetic on general,
cancelling, sparse, and near-overflow inputs in both directions. Public inverse tests exercise f32/f64 transforms at lengths 103, 146, 722,
and 1442 with high-dynamic-range spectra. A separate radix-103 case places
large cancelling bins beside small terms. Its exact assertion requires one
minimum subnormal in the public inverse result: the small terms disappear
during prescaling, so a tolerance-only comparison would not detect a dropped
correction. The same observable is checked independently in f32 and f64. The
mixed C2C lengths 146, 722, and 1442 are compared with direct DFT references. The real N=1442 complexified path is checked against independent f32 and f64
direct DFTs. Its inverse also has a high-dynamic-range cancellation case. The
existing real N=1026 coverage remains in the same test. The pinned DUCC output comparison used revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf` and covered 20,871 output records
(41,742 scalar components). Maximum absolute and scaled errors were
`7.958e-13` and `1.137e-13`. The comparison invocation prints
`vectorize=off`; the reference executable is the native DUCC build documented
in [the M0 report](m0-current-head-baseline.md), whose build metadata records
`vectorize=true`. The invocation label is not a rebuild or a change to that
executable. An independent kernel review confirmed the paired-output algebra and the
benchmark gate. Its follow-up also verified that the minimum-subnormal
assertions directly detect a missing prescale correction.

## Measurement method

The baseline is the immediately preceding M0 commit `b508712`; the candidate
contains this paired-output kernel. The existing native benchmark harness and
an added first-call mode use f64. Builds use Rust 1.99.0, LLVM 23.1.1, the
optimized bench profile, and a portable x86-64 target without
`target-cpu=native`. The general-prime pass is scalar; runtime AVX2 dispatch
for other kernels remains available. The host is an x86-64 CPU, with
each process pinned to logical one CPU core. Each warmed comparison uses six
alternating candidate/baseline process pairs, one untimed warm-up and seven
timed samples per process. Ratios are `baseline / candidate`; intervals are
95% percentile bootstrap intervals over the six paired ratios. Inputs and
plans are reused within each process, and each process runs one thread. Load-average telemetry was retained only in local diagnostics. The separate first-call run measures the first public operation after plan,
workspace, and input construction. It excludes process startup, plan creation,
and input allocation. Its warm value is the median of seven calls following
one untimed warm-up. The first-call sample is one execution per process, so
its six-pair intervals are more sensitive to timer and system noise than the
repeated warm measurements.

## Warm C2C results at SHT row counts

Entries are median baseline-to-candidate speedups with 95% paired bootstrap
intervals. The final column gives median public-operation latency in
microseconds as candidate / baseline.

| Length, rows | Forward speedup (95% interval) | Inverse speedup (95% interval) | Forward latency | Inverse latency |
| ---: | ---: | ---: | ---: | ---: |
| 146, 73 | 2.213x (2.206–2.244) | 1.892x (1.871–1.911) | 3.707 / 8.194 | 4.364 / 8.251 |
| 722, 361 | 1.804x (1.732–1.810) | 1.533x (1.529–1.538) | 11.487 / 20.725 | 14.289 / 21.902 |
| 1442, 721 | 2.134x (2.121–2.180) | 1.865x (1.854–1.941) | 56.299 / 119.954 | 63.407 / 118.249 |

For the eight direct-prime lengths, the geometric mean of per-length median
speedups was 1.877x forward and 1.641x inverse at one row, and 1.981x
forward and 1.661x inverse at 32 rows. Every direct-prime target's individual
95% interval was above 1.0x.

## Complexified real N=1442

The real plan complexifies to C2C length 721, which contains the direct
radix-103 pass. Values below use 721 rows, the SHT-shaped row count.

| Operation | Speedup (95% interval) | Candidate / baseline latency | Workspace bytes, candidate / baseline |
| --- | ---: | ---: | ---: |
| R2C | 2.024x (1.997–2.074) | 29.171 / 58.766 µs | 37,952 / 37,952 |
| C2R | 1.988x (1.946–2.005) | 32.127 / 64.033 µs | 37,952 / 37,952 |

## First and warmed calls

The separate one-row run reports candidate / baseline medians. Warm values
are single-operation medians and should not be substituted for the batched
SHT-shaped values above.

| Operation | First call, µs | First-call speedup | Warm call, µs | Warm-call speedup |
| --- | ---: | ---: | ---: | ---: |
| C2C forward, 146 | 5.526 / 9.874 | 1.83x | 3.858 / 8.050 | 2.08x |
| C2C inverse, 146 | 6.903 / 10.355 | 1.68x | 5.551 / 9.222 | 1.84x |
| C2C forward, 722 | 15.229 / 27.196 | 1.78x | 12.624 / 25.869 | 1.86x |
| C2C inverse, 722 | 19.813 / 25.553 | 1.18x | 18.465 / 21.801 | 1.33x |
| C2C forward, 1442 | 68.520 / 150.430 | 2.20x | 54.168 / 118.439 | 2.18x |
| C2C inverse, 1442 | 80.738 / 121.956 | 1.56x | 60.940 / 120.483 | 1.98x |
| R2C, 1442 | 34.450 / 78.674 | 2.41x | 31.825 / 67.984 | 2.17x |
| C2R, 1442 | 49.178 / 71.214 | 1.45x | 36.589 / 59.493 | 1.64x |

The first-call paired interval for C2C inverse length 722 includes 1.0x
(0.81–1.43); its first-call median is not conclusive. All other first-call
intervals in this matrix are above 1.0x.

## Controls, memory, and limits

The designated C2C controls were lengths 256, 512, 1024, and 1452. No control
regressed by more than 5%. At exact SHT row counts, the largest median C2C
regression was forward length 1452 at 1.85%; at one and 32 rows, the largest
was inverse length 256 at about 2.7%. The wider screening matrix also showed
R2C length 256 at 32 rows 3.4% slower (95% interval ratio 0.958–0.976) and
R2C length 103 at 32 rows 6.1% slower by median (0.935–1.003 interval). Those real-transform cells are not C2C controls; the length-103 interval
includes no change and is inconclusive. Workspace bytes are identical before and after the kernel: 4,672 at C2C 146,
12,160 at C2C 722, 26,368 at C2C 1442, and 37,952 at real N=1442. No new
plan table or execution scratch was added. Peak resident memory was not
measured. Plan and workspace construction were measured separately with 20
repetitions; their medians were similar between the two binaries. The
comparison measures standalone FFT operations, not complete spherical
harmonic transforms. Timing coverage is f64 on one CPU; it does not establish
f32 speedups or end-to-end SHT performance.

## Validation and artifacts

The final checks passed:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`: 79 unit and integration tests
- `cargo test --release --test complex --test real`: 61 integration tests
- `git diff --check`

Benchmark artifacts are in [`benches/results`](../benches/results/):

- `m1-direct-config.json`, `m1-direct-paired.jsonl`, and
  `m1-direct-summary.csv`: direct primes and C2C controls at one and 32 rows.
- `m1-sht-config.json`, `m1-sht-paired.jsonl`, and `m1-sht-summary.csv`:
  exact SHT-shaped complex and real row counts.
- `m1-real-1442-config.json`, `m1-real-1442-paired.jsonl`, and
  `m1-real-1442-summary.csv`: real N=1442 at one, 32, and 721 rows.
- `m1-first-call-config.json`, `m1-first-call-paired.jsonl`, and
  `m1-first-call-summary.csv`: first-call and warmed single-operation data.
- `m1-ducc-comparison.txt`: pinned-DUCC output comparison summary. The warmed-operation candidate and baseline binary hashes are recorded in the
JSONL captures. The first-call configuration records its benchmark source
hashes and binary hashes. The structured summary retains pair order, workload configuration, elapsed-time samples, and operation results. Raw commands and host-state captures remain local.
