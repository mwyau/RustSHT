# Complex-to-complex (C2C) across-row SIMD prototype

## Method

The prototype tests single instruction, multiple data (SIMD) lanes across rows. It transposes row-major complex inputs into a reusable sample-major
workspace, with adjacent rows in each SIMD vector. It applies radix-two
decimation-in-time stages with bit-reversed input placement, then transposes the
results back to row-major order. The AVX2 path processes two f64 complex rows
per vector. An odd final row uses scalar arithmetic. The scalar prototype uses
the same layout and stage order. The prototype covers f64 C2C transforms of 256 and 1,024 points, with 2, 3, 4,
8, 16, and 32 rows. Each timed call executes the complete batch operation,
including both transposes. Input restoration, planning, and workspace
allocation occur outside the timed region. Timings are divided by row count for
per-transform comparisons; multiplying by the row count gives the complete
batch-call time. The serial DUCCFFT batch API calls the existing transform once
per row. RustFFT uses its existing plan and scratch once per row. Direct C++
DUCC uses one pinned 1D plan and scratch buffer across the rows. Every process performed seven paired measurements per implementation and
configuration. The implementation order rotated across pairs. Six separate
processes were pinned to logical one CPU core. The benchmark used identical input
values in RustFFT, DUCCFFT, and the C++ reference:

```text
x = index + 1 + 3 * row
real = sin(0.173 * x)
imag = cos(0.113 * (x + 1))
```

The bench checked each forward and inverse output against the serial DUCCFFT
result before timing. The integration test compares both prototype backends
with direct DFTs at lengths 1, 2, 8, 16, 64, 256, and 1,024, including odd row
counts. These checks cover the tested inputs and lengths.

## Build and measurement configuration

The Rust measurements use source commit
`cc01afafc2623907fa3e556a738d20e434d220fe`, the `Cargo.lock` SHA-256
`e61d5628d09dc7c10e54e15cb5f3aeb5dd5c28392b44692ed153784ad8ca21ba`, Rust
1.99.0 (`b940084d7`, LLVM 23.1.1), Cargo 1.99.0, RustFFT 6.4.1, and Cargo's
default bench profile (`opt-level=3`). `RUSTFLAGS`, LTO, and `target-cpu` were
unset. Runtime AVX2 dispatch selected the AVX2 prototype on this x86-64 CPU. The host had host topology omitted. The measurements were pinned
to logical one CPU core while the Legendre workload ran on the shared host. Load-average telemetry was retained only in local diagnostics. The Rust benchmark binary SHA-256 is
`816ff348b71e436b89c65845523193c74c6aff2abbf621f5be2d707607e388f0`. The benchmark source hashes are `423c3b1b6b5dc50773d6c90b9eff95b8b76dd836b54430877dd42e8a6ea8013c`
for `benches/across_row_prototype.rs` and
`91591ce036d4596c8b2bda6462c9d232cb0468ecce509b671aea609ae5aaedf7` for
`benches/prototypes/c2c_across_row.rs`. The C++ harness source was built at repository commit
`d25982e73ff4a9bc2d87483d7afb9228b949d106`. It uses DUCC revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, built with g++ 15.2.0. The
portable build used `-std=c++20 -O3 -DNDEBUG -pthread`; its binary SHA-256 is
`6485c1bd85cd3e67dc929ebeea13fed4bc60fd3a81e646fc4a354a50005698ab`. The
native build added `-march=native`; its binary SHA-256 is
`09fe220341903e95788d96983aeddb7652da5a1947ee01ae439b0322fdfce178`. DUCC's
vectorized planner option was disabled in both builds. The reference harness
source has SHA-256
`cbe53d90e23232bf5abd1ce5fc9236fbe2ff40024effd2df1b90f8498d1dc1fa`. The C++ harness passed one thread to each DUCC plan. The portable and native
build process order alternated across the six samples. Rust execution data contain 4,032 records: 24 configurations × six processes ×
seven pairs × four implementations. Setup data contain six process samples;
each plan and workspace time is the mean of 20 constructions or allocations. The C++ reference data contain six process samples per build and configuration;
each execution time is the median of seven timed calls after a warmup. Raw data
are in:

- [`m4-across-row-process-samples.csv`](../benches/results/m4-across-row-process-samples.csv)
- [`m4-across-row-setup-process-samples.csv`](../benches/results/m4-across-row-setup-process-samples.csv)
- [`m4-across-row-phase-process-samples.csv`](../benches/results/m4-across-row-phase-process-samples.csv)
- [`m4-across-row-ducc-portable-process-samples.csv`](../benches/results/m4-across-row-ducc-portable-process-samples.csv)
- [`m4-across-row-ducc-native-process-samples.csv`](../benches/results/m4-across-row-ducc-native-process-samples.csv)

## Batch execution timings

Values are median microseconds per transform. Rust values pool the 42 paired
samples from six processes; each C++ value is the median of six process-level
medians. Each timed operation processes the number of rows in the second
column. DUCC refers to direct C++ plans. Ratios are summarized below; values
above one mean the across-row prototype is slower.

| N | Rows | Direction | DUCCFFT serial batch | RustFFT rows | DUCC portable | DUCC native | Across-row AVX2 |
|---:|---:|---|---:|---:|---:|---:|---:|
| 256 | 2 | Forward | 0.787 | 0.286 | 1.040 | 0.591 | 1.130 |
| 256 | 2 | Inverse | 1.137 | 0.351 | 1.175 | 0.606 | 1.159 |
| 256 | 3 | Forward | 0.768 | 0.274 | 1.042 | 0.578 | 1.237 |
| 256 | 3 | Inverse | 1.119 | 0.347 | 1.167 | 0.601 | 1.264 |
| 256 | 4 | Forward | 0.764 | 0.258 | 1.040 | 0.583 | 0.911 |
| 256 | 4 | Inverse | 1.106 | 0.336 | 1.173 | 0.605 | 0.937 |
| 256 | 8 | Forward | 0.731 | 0.252 | 1.026 | 0.567 | 0.906 |
| 256 | 8 | Inverse | 1.081 | 0.332 | 1.160 | 0.601 | 0.946 |
| 256 | 16 | Forward | 0.745 | 0.259 | 1.029 | 0.566 | 1.152 |
| 256 | 16 | Inverse | 1.091 | 0.340 | 1.162 | 0.596 | 1.201 |
| 256 | 32 | Forward | 0.729 | 0.261 | 0.833 | 0.565 | 1.354 |
| 256 | 32 | Inverse | 1.077 | 0.346 | 0.940 | 0.598 | 1.408 |
| 1,024 | 2 | Forward | 3.837 | 1.518 | 4.656 | 3.629 | 5.643 |
| 1,024 | 2 | Inverse | 5.272 | 1.749 | 5.162 | 3.441 | 5.816 |
| 1,024 | 3 | Forward | 3.824 | 1.514 | 4.462 | 3.555 | 6.136 |
| 1,024 | 3 | Inverse | 5.248 | 1.740 | 5.138 | 4.163 | 6.329 |
| 1,024 | 4 | Forward | 3.820 | 1.514 | 4.705 | 3.455 | 5.686 |
| 1,024 | 4 | Inverse | 5.251 | 1.751 | 5.077 | 3.535 | 5.891 |
| 1,024 | 8 | Forward | 3.823 | 1.527 | 3.369 | 3.209 | 5.753 |
| 1,024 | 8 | Inverse | 5.237 | 1.759 | 3.784 | 4.031 | 5.954 |
| 1,024 | 16 | Forward | 3.809 | 1.579 | 3.343 | 4.239 | 6.774 |
| 1,024 | 16 | Inverse | 5.231 | 1.817 | 3.786 | 4.696 | 6.986 |
| 1,024 | 32 | Forward | 3.837 | 1.556 | 3.362 | 4.314 | 8.149 |
| 1,024 | 32 | Inverse | 5.236 | 1.789 | 3.793 | 4.723 | 8.290 |

Across the six row counts, prototype-to-reference timing ratios span:

| N | Direction | Across-row / DUCCFFT | Across-row / RustFFT | Across-row / DUCC portable | Across-row / DUCC native |
|---:|---|---:|---:|---:|---:|
| 256 | Forward | 1.19–1.86× | 3.53–5.18× | 0.88–1.63× | 1.56–2.40× |
| 256 | Inverse | 0.85–1.31× | 2.79–4.07× | 0.80–1.50× | 1.55–2.36× |
| 1,024 | Forward | 1.47–2.12× | 3.72–5.24× | 1.21–2.42× | 1.55–1.89× |
| 1,024 | Inverse | 1.10–1.58× | 3.33–4.63× | 1.13–2.19× | 1.48–1.76× |

At N=256, the median inverse time improves over the serial DUCCFFT batch path
for four and eight rows. The other N=256 cases and every N=1,024 case are
slower. For each direction, the geometric mean across 504 paired Across-row
runtime AVX2/serial DUCCFFT elapsed-time ratios is 1.5195× for forward and
1.1185× for inverse, or 52.0% and 11.8% more elapsed time. The prototype is
2.79–5.24× slower than RustFFT and 1.48–2.40× slower than native-target DUCC
across the reported medians. The complete 32-row public-operation times, in microseconds per batch call,
are:

| N | Direction | DUCCFFT | RustFFT | DUCC portable | DUCC native | Across-row AVX2 |
|---:|---|---:|---:|---:|---:|---:|
| 256 | Forward | 23.32 | 8.37 | 26.66 | 18.08 | 43.34 |
| 256 | Inverse | 34.47 | 11.07 | 30.09 | 19.12 | 45.05 |
| 1,024 | Forward | 122.77 | 49.80 | 107.59 | 138.05 | 260.77 |
| 1,024 | Inverse | 167.56 | 57.25 | 121.38 | 151.15 | 265.29 |

## Setup and scratch

Rust plan and workspace times are medians across six processes of means over 20
operations. C++ DUCC plan and buffer allocation medians pool the six row counts
and both directions (72 process/configuration records per length and build),
with each record based on its 20-repetition setup measurement. Workspace sizes
are reusable bytes; plan tables are excluded. RustFFT plan setup constructs
both directions with one planner, and its workspace size is the larger of the
forward and inverse scratch lengths.

| N | Implementation | Plan time | Workspace allocation, 2 rows | Scratch, 2 rows | Workspace allocation, 32 rows | Scratch, 32 rows |
|---:|---|---:|---:|---:|---:|---:|
| 256 | DUCCFFT | 3.77 µs | 0.19 µs | 4,352 B | 0.09 µs | 4,352 B |
| 256 | RustFFT | 4.82 µs | 0.04 µs | 4,096 B | 0.04 µs | 4,096 B |
| 256 | Across-row prototype | 3.61 µs | 0.15 µs | 8,192 B | 4.14 µs | 131,072 B |
| 256 | DUCC portable C++ | 1.21 µs | 0.13 µs | 4,096 B | 0.13 µs | 4,096 B |
| 256 | DUCC native C++ | 0.75 µs | 0.10 µs | 4,096 B | 0.10 µs | 4,096 B |
| 1,024 | DUCCFFT | 13.98 µs | 0.47 µs | 16,640 B | 0.19 µs | 16,640 B |
| 1,024 | RustFFT | 21.05 µs | 0.13 µs | 16,384 B | 0.13 µs | 16,384 B |
| 1,024 | Across-row prototype | 13.85 µs | 0.49 µs | 32,768 B | 19.56 µs | 524,288 B |
| 1,024 | DUCC portable C++ | 2.09 µs | 0.10 µs | 16,896 B | 0.10 µs | 16,896 B |
| 1,024 | DUCC native C++ | 2.17 µs | 0.13 µs | 16,896 B | 0.13 µs | 16,896 B |

The transposed workspace uses `N × rows` complex f64 values. Its allocation
cost and scratch size therefore increase with batch size, while the serial
plans reuse one transform workspace for every row.

## Phase profile and generated code

The profile times packing into the transposed layout, FFT stages, and unpacking
back to row-major data. Values are median nanoseconds per transform. Instrumented
phase sums differ from uninstrumented whole-call medians by up to about 4.5%
in the listed cases.

| N | Rows | Direction | Pack | Butterfly stages | Unpack |
|---:|---:|---|---:|---:|---:|
| 256 | 2 | Forward | 176 | 839 | 136 |
| 256 | 2 | Inverse | 176 | 872 | 136 |
| 256 | 32 | Forward | 549 | 560 | 241 |
| 256 | 32 | Inverse | 542 | 607 | 241 |
| 1,024 | 2 | Forward | 1,205 | 4,125 | 536 |
| 1,024 | 2 | Inverse | 1,190 | 4,341 | 546 |
| 1,024 | 32 | Forward | 3,749 | 3,039 | 1,317 |
| 1,024 | 32 | Inverse | 3,732 | 3,244 | 1,222 |

For 32 rows, the two layout conversions account for about 56–63% of measured
time. At N=1,024, 32 rows, the AVX2 butterfly stages take about 3.0–3.2 µs per
transform, while packing and unpacking take 5.0–5.1 µs. This measured data
movement explains why the faster vectorized stages do not produce a faster
whole operation. The release binary contains a separate `stage_avx2` symbol. Its inner loop
uses packed YMM `vmulpd`, `vaddsubpd`, `vaddpd`, and `vsubpd` instructions for
two complex rows. The odd-row tail uses scalar operations. The kernel has no
FMA dependency and is selected only after runtime AVX2 detection.

## Decision

Keep the across-row code as a benchmark prototype. Do not route production
batch calls through it: it loses to serial DUCCFFT in 22 of the 24 median
configuration cells, remains 2.79–5.24× slower than RustFFT, and uses up to
524,288 bytes of workspace for 32 rows at N=1,024. The pack and unpack costs
dominate at larger batches. Existing within-transform AVX2 passes already
vectorize the contiguous butterfly dimension, so adding row lanes does not
remove enough arithmetic to repay the format conversions.
