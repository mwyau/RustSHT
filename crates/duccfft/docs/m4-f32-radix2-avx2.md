# f32 real inverse radix-two AVX2 kernel

Date: 2026-10-13.

## Kernel and validation

The f32 real inverse radix-two interior now processes four adjacent complex
butterflies per AVX2 vector. It reverses mirrored branch pairs, performs the
sum and difference, applies the pass twiddles, and stores both output
branches. The scalar loop keeps the pass endpoints and remaining interior
values. Runtime AVX2 detection selects the kernel for `ido >= 16`; f64 plans
use their separate two-complex kernel, while f32 plans without AVX2 use the
scalar path. The kernel adds one immutable plan function pointer and no
workspace buffers.

The equations follow the inverse radix-two branch in pinned DUCC source
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The kernel test compares vector
results and untouched endpoints for `ido` 4 through 64, including vector
tails that leave one or more scalar pairs. The public f32 C2R test compares
lengths 128, 512, and 2,048 with independent f64 direct inverses after the
inputs are rounded to f32. The existing extreme-value tests continue to use
the scalar or guarded fallback paths.

| N | Operation | Maximum absolute error | RMS error | Relative L2 error | Scaled maximum error | Nonfinite outputs |
|---:|---|---:|---:|---:|---:|---:|
| 128 | R2C | 2.171e-6 | 7.127e-7 | 8.764e-8 | 5.303e-8 | 0 |
| 128 | C2R | 3.393e-8 | 6.043e-9 | 9.683e-8 | 3.393e-8 | 0 |
| 512 | R2C | 1.250e-5 | 1.694e-6 | 1.042e-7 | 4.976e-8 | 0 |
| 512 | C2R | 3.117e-8 | 3.534e-9 | 1.088e-7 | 3.117e-8 | 0 |
| 2,048 | R2C | 6.340e-5 | 3.751e-6 | 1.149e-7 | 8.019e-8 | 0 |
| 2,048 | C2R | 1.803e-8 | 1.818e-9 | 1.122e-7 | 1.803e-8 | 0 |

Scaled maximum error is maximum absolute error divided by the larger of the
maximum reference magnitude and 1. The full test also checks the existing
general-prime, Bluestein, SHT-length, cancellation, and dynamic-range cases.

## Paired process measurements

The baseline is source `196c9a5`; the candidate is `643b5f9`. Both builds
used the same Rust benchmark source, SHA-256
`76ea65520ac18ab604087c9011c81f61b5f54b597dbfb3b45ad7cf9320c2b665`, and
crate lockfile SHA-256
`e61d5628d09dc7c10e54e15cb5f3aeb5dd5c28392b44692ed153784ad8ca21ba`. The
benchmark binary SHA-256 values were
`0a9634fa5d99eb6ab89b3f81ada225042f49dbce50376b1cf43224deb3678396` for the
baseline and
`b8618201a13aff4376fda4995c3f55f58d8fe73a26cd2a1173a128f1d3cf59b8` for the
candidate.

The machine was an AMD Ryzen 9 5950X with AVX2. Twelve baseline/candidate
process pairs per length were pinned to logical CPU 30; process order
alternated by length and pair. Each process warmed reusable plans and took
seven paired samples. Within a process, the DUCCFFT and RustFFT adapter order
alternated. Single-transform samples repeated `max(262144 / N, 16)` calls;
batch samples repeated `max(floor(262144 / N / 32), 1)` calls to
`forward_batch` or `inverse_batch`. Inputs and output buffers were allocated
outside the timer. Timings cover public validation, real packing or
unpacking, FFT execution, and normalization.

Rust used 1.99.0 (`b940084d7`, LLVM 23.1.1), Cargo 1.99.0, and the default
optimized `bench` profile (`-C opt-level=3`, `-C embed-bitcode=no`,
`-C strip=debuginfo`). No custom `RUSTFLAGS`, LTO, or target CPU setting was
used. RustFFT was 6.4.1 and used the existing RustSHT-style half-length
complex real adapter. The direct C++ DUCC f32 comparisons use separate
portable and native builds and are reported below.

Each table timing is the median of 84 values (12 process pairs times seven
samples) in nanoseconds per transform. Raw gain is the median of the paired
`baseline C2R / candidate C2R - 1` values. R2C-adjusted gain divides that
ratio by the paired R2C control ratio. The process count reports how many of
the 12 process pairs have a positive median adjusted gain.

| N | R2C single, baseline / candidate | C2R single, baseline / candidate | C2R raw gain | R2C-adjusted gain | Positive processes | R2C batch 32 per transform, baseline / candidate | C2R batch 32 per transform, baseline / candidate | C2R raw gain | R2C-adjusted gain | Positive processes |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 128 | 358.0 / 357.9 | 332.0 / 299.4 | +11.0% | +10.7% | 12/12 | 357.8 / 358.0 | 331.3 / 293.8 | +12.6% | +12.7% | 12/12 |
| 256 | 607.5 / 608.9 | 567.1 / 575.1 | −1.3% | −1.3% | 1/12 | 606.6 / 606.5 | 568.0 / 578.7 | −1.6% | −1.5% | 0/12 |
| 512 | 1,472.0 / 1,472.0 | 1,351.1 / 1,185.0 | +14.1% | +14.2% | 12/12 | 1,478.5 / 1,480.1 | 1,353.6 / 1,196.2 | +13.1% | +13.4% | 12/12 |
| 1,024 | 2,525.7 / 2,515.4 | 2,366.2 / 2,394.0 | −1.3% | −1.6% | 1/12 | 2,528.6 / 2,531.9 | 2,412.9 / 2,394.6 | −0.2% | +0.2% | 6/12 |
| 2,048 | 6,194.4 / 6,171.0 | 5,627.3 / 4,986.0 | +13.0% | +12.9% | 12/12 | 6,205.8 / 6,194.7 | 5,759.4 / 5,040.1 | +14.1% | +14.0% | 12/12 |
| 4,096 | 10,728.0 / 10,748.7 | 10,127.2 / 10,294.4 | −1.6% | −1.4% | 2/12 | 10,825.8 / 10,826.6 | 10,422.4 / 10,378.8 | +0.4% | +0.6% | 8/12 |

The no-radix-two lengths 256, 1,024, and 4,096 show no repeatable single-call
C2R gain. At lengths 128, 512, and 2,048, all 12 paired process medians favor
the candidate for both single and batch calls. Adjusted gains range from
10.7% to 14.2% across those lengths. R2C controls stay within about 0.4% in
the DUCCFFT medians, so the inverse gains are not explained by a matching
change in forward execution.

Candidate whole-public-operation timings are compared with the RustFFT real
adapter below. Timings are microseconds per transform; each ratio is the
DUCCFFT median divided by the RustFFT median. The R2C and C2R operations
include real packing or unpacking.

| N | Single R2C, DUCCFFT / RustFFT / ratio | Single C2R, DUCCFFT / RustFFT / ratio |
|---:|---:|---:|
| 128 | 0.358 / 0.219 / 1.64× | 0.299 / 0.146 / 2.06× |
| 256 | 0.609 / 0.436 / 1.40× | 0.575 / 0.290 / 1.98× |
| 512 | 1.472 / 0.892 / 1.65× | 1.185 / 0.596 / 1.99× |
| 1,024 | 2.515 / 1.811 / 1.39× | 2.394 / 1.253 / 1.91× |
| 2,048 | 6.171 / 3.743 / 1.65× | 4.986 / 2.630 / 1.90× |
| 4,096 | 10.749 / 7.671 / 1.40× | 10.294 / 5.421 / 1.90× |

| N | Batch 32 R2C, DUCCFFT / RustFFT / ratio | Batch 32 C2R, DUCCFFT / RustFFT / ratio |
|---:|---:|---:|
| 128 | 0.358 / 0.219 / 1.63× | 0.294 / 0.146 / 2.01× |
| 256 | 0.607 / 0.439 / 1.38× | 0.579 / 0.293 / 1.98× |
| 512 | 1.480 / 0.886 / 1.67× | 1.196 / 0.593 / 2.02× |
| 1,024 | 2.532 / 1.817 / 1.39× | 2.395 / 1.264 / 1.89× |
| 2,048 | 6.195 / 3.783 / 1.64× | 5.040 / 2.672 / 1.89× |
| 4,096 | 10.827 / 7.779 / 1.39× | 10.379 / 5.488 / 1.89× |

The candidate C2R/RustFFT ratios, using medians of paired timing ratios, were
2.05, 1.99, and 1.90 for single transforms at lengths 128, 512, and 2,048.
For batches of 32 they were 2.01, 2.02, and 1.89. Candidate R2C ratios were
1.64–1.65 for single transforms and 1.63–1.67 for batches at those lengths.
The kernel reduces the inverse gap but does not remove it.

Each process timed 20 plan constructions and 20 workspace allocations. The
table uses the twelve single-transform process setup values; the benchmark
prints each value with both R2C and C2R samples, so it is counted once per
process. Batch-32 setup values are also present in the raw CSV but are not
included in this table. Scratch bytes are reported by
`RealFftWorkspace::scratch_bytes()`;
the baseline and candidate values match because the kernel adds no workspace
storage.

| N | Plan time, baseline / candidate (µs) | Workspace allocation, baseline / candidate (µs) | Scratch bytes |
|---:|---:|---:|---:|
| 128 | 2.585 / 2.625 | 0.146 / 0.141 | 2,064 |
| 256 | 5.045 / 5.120 | 0.170 / 0.170 | 3,616 |
| 512 | 9.934 / 9.143 | 0.206 / 0.205 | 7,728 |
| 1,024 | 16.236 / 18.185 | 0.231 / 0.251 | 13,888 |
| 2,048 | 32.241 / 32.632 | 0.351 / 0.351 | 30,288 |
| 4,096 | 63.927 / 64.693 | 0.571 / 0.571 | 54,880 |

The raw data are in
[`m4-f32-real-radix2-process-samples.csv`](../benches/results/m4-f32-real-radix2-process-samples.csv).
All 4,032 keys are unique. Matching baseline/candidate records have the same
sample order, repeat count, and checksums.

The Legendre workload shared the host. Load averages at the end of collection
were 3.02, 3.27, and 3.43 for one, five, and fifteen minutes, so the absolute
times remain provisional. The paired controls support retaining
this kernel at 128, 512, and 2,048. Review R25 found no remaining numerical,
safety, bounds, or measurement issue after the ratio correction.

## Generated code

Release disassembly of
`duccfft::simd::x86::f32_real_kernel::inverse_radix2_interior` contains packed
`vmovups`, `vpermpd`, `vaddps`, `vsubps`, `vblendps`, `vmulps`, and shuffle
instructions. The vector loop has one back-edge branch and no helper calls or
stack spills. It uses runtime AVX2 dispatch and emits no FMA instructions.

## Direct C++ DUCC comparison

The direct reference uses `pocketfft_r<float>` from DUCC revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Its adapter copies the real input
to a reusable packed buffer, runs the in-place plan, and unpacks the
nonnegative spectrum. The inverse packs the supplied Hermitian spectrum,
runs DUCC with scale `1/N`, and copies the real output. Input and output
arrays, plan, work buffer, and plan scratch are allocated before timing. The
Rust calls include public validation and caller-owned workspace execution;
the C++ calls include the adapter copies and packing but do not add a Rust
wrapper's validation cost.

All three implementations use the same f32-rounded real and spectrum input
formulas. The C++ input helpers use volatile f32 intermediates to preserve
Rust's operation rounding when the native compiler enables FMA. The native
build may still use FMA inside DUCC's transform kernels. The timed calls
include the real adapter, transform, normalization, and output handling.
Single calls repeat `max(262144/N, 16)` transforms per sample. The 32-row
case repeats `max(floor(262144/N/32), 1)` batches and reports nanoseconds per
row. Inputs and outputs are prepared outside the timer. The C++ adapter
processes rows serially, matching the Rust convenience batch path.

The portable C++ binary used g++ 15.2.0 with
`-std=c++20 -O3 -DNDEBUG -pthread` and DUCC's scalar planner
(`DUCCFFT_DUCC_VECTORIZE=0`). Rust used rustc 1.99.0 and the default optimized
bench profile without target-CPU flags. That Rust binary still uses runtime
AVX2 dispatch for the specialized kernel; “portable” describes its build
target and does not mean scalar execution. The native C++ binary used
`-std=c++20 -O3 -DNDEBUG -march=native -pthread` and DUCC's vectorized planner
(`DUCCFFT_DUCC_VECTORIZE=1`). Its Rust counterpart used
`RUSTFLAGS='-C target-cpu=native'`. The two native binaries target the same
host ISA through different compilers.

The host was an AMD Ryzen 9 5950X. The direct C++ portable and native Rust/C++
samples were collected in one interleaved session. Every process was pinned
to logical CPU 30; there were 12 process samples per length and build, with
seven timed samples per direction and row count. Build order alternated
within each length. Portable RustFFT and DUCCFFT values reuse the earlier R25
campaign, which also had 12 process pairs per length. The portable cross-
library values therefore come from separate campaigns and are unpaired.
load averages at the beginning of the comparison session were 3.75/3.93/3.90;
after the final collection they were 2.48/3.34/3.71. Absolute timings remain
provisional because CPU interference cannot be ruled out. Each cell below is
the median of 84 values (12 processes times seven samples). Ratios are
`DUCCFFT/RustFFT` and `DUCCFFT/direct DUCC`;
values above 1 mean DUCCFFT took longer.

### Portable builds

Single transforms, microseconds per call:

| N | R2C DUCCFFT / RustFFT / DUCC | Ratios FFT/Rust, FFT/DUCC | C2R DUCCFFT / RustFFT / DUCC | Ratios FFT/Rust, FFT/DUCC |
|---:|---:|---:|---:|---:|
| 128 | 0.358 / 0.219 / 0.220 | 1.64×, 1.62× | 0.299 / 0.146 / 0.213 | 2.06×, 1.40× |
| 256 | 0.609 / 0.436 / 0.452 | 1.40×, 1.35× | 0.575 / 0.290 / 0.454 | 1.98×, 1.27× |
| 512 | 1.472 / 0.892 / 0.996 | 1.65×, 1.48× | 1.185 / 0.596 / 0.967 | 1.99×, 1.23× |
| 1,024 | 2.515 / 1.811 / 2.092 | 1.39×, 1.20× | 2.394 / 1.253 / 2.050 | 1.91×, 1.17× |
| 2,048 | 6.171 / 3.743 / 4.561 | 1.65×, 1.35× | 4.986 / 2.630 / 4.508 | 1.90×, 1.11× |
| 4,096 | 10.749 / 7.671 / 9.657 | 1.40×, 1.11× | 10.294 / 5.421 / 9.789 | 1.90×, 1.05× |

Batch of 32 independent rows, microseconds per transform:

| N | R2C DUCCFFT / RustFFT / DUCC | Ratios FFT/Rust, FFT/DUCC | C2R DUCCFFT / RustFFT / DUCC | Ratios FFT/Rust, FFT/DUCC |
|---:|---:|---:|---:|---:|
| 128 | 0.358 / 0.219 / 0.222 | 1.63×, 1.61× | 0.294 / 0.146 / 0.216 | 2.01×, 1.36× |
| 256 | 0.607 / 0.439 / 0.453 | 1.38×, 1.34× | 0.579 / 0.293 / 0.450 | 1.98×, 1.29× |
| 512 | 1.480 / 0.886 / 1.007 | 1.67×, 1.47× | 1.196 / 0.593 / 0.965 | 2.02×, 1.24× |
| 1,024 | 2.532 / 1.817 / 2.106 | 1.39×, 1.20× | 2.395 / 1.264 / 2.072 | 1.89×, 1.16× |
| 2,048 | 6.195 / 3.783 / 4.571 | 1.64×, 1.36× | 5.040 / 2.672 / 4.574 | 1.89×, 1.10× |
| 4,096 | 10.827 / 7.779 / 9.644 | 1.39×, 1.12× | 10.379 / 5.488 / 9.836 | 1.89×, 1.06× |

### Native x86 builds

Single transforms, microseconds per call:

| N | R2C DUCCFFT / RustFFT / DUCC | Ratios FFT/Rust, FFT/DUCC | C2R DUCCFFT / RustFFT / DUCC | Ratios FFT/Rust, FFT/DUCC |
|---:|---:|---:|---:|---:|
| 128 | 0.320 / 0.201 / 0.199 | 1.59×, 1.60× | 0.267 / 0.132 / 0.209 | 2.02×, 1.28× |
| 256 | 0.541 / 0.409 / 0.399 | 1.32×, 1.36× | 0.507 / 0.273 / 0.380 | 1.86×, 1.34× |
| 512 | 1.286 / 0.836 / 0.877 | 1.54×, 1.47× | 1.036 / 0.585 / 0.766 | 1.77×, 1.35× |
| 1,024 | 2.207 / 1.722 / 1.843 | 1.28×, 1.20× | 2.081 / 1.188 / 1.778 | 1.75×, 1.17× |
| 2,048 | 5.410 / 3.554 / 3.710 | 1.52×, 1.46× | 4.468 / 2.525 / 3.593 | 1.77×, 1.24× |
| 4,096 | 9.432 / 7.304 / 7.748 | 1.29×, 1.22× | 9.384 / 5.251 / 7.376 | 1.79×, 1.27× |

Batch of 32 independent rows, microseconds per transform:

| N | R2C DUCCFFT / RustFFT / DUCC | Ratios FFT/Rust, FFT/DUCC | C2R DUCCFFT / RustFFT / DUCC | Ratios FFT/Rust, FFT/DUCC |
|---:|---:|---:|---:|---:|
| 128 | 0.319 / 0.200 / 0.201 | 1.59×, 1.59× | 0.262 / 0.133 / 0.209 | 1.97×, 1.25× |
| 256 | 0.534 / 0.411 / 0.402 | 1.30×, 1.33× | 0.507 / 0.277 / 0.383 | 1.83×, 1.33× |
| 512 | 1.298 / 0.822 / 0.884 | 1.58×, 1.47× | 1.045 / 0.584 / 0.772 | 1.79×, 1.35× |
| 1,024 | 2.206 / 1.727 / 1.869 | 1.28×, 1.18× | 2.156 / 1.203 / 1.816 | 1.79×, 1.19× |
| 2,048 | 5.427 / 3.612 / 3.707 | 1.50×, 1.46× | 4.527 / 2.557 / 3.624 | 1.77×, 1.25× |
| 4,096 | 9.545 / 7.410 / 7.683 | 1.29×, 1.24× | 9.447 / 5.343 / 7.469 | 1.77×, 1.26× |

In the native-build measurements, DUCCFFT is about 1.28–1.59× slower than
RustFFT for R2C and 1.75–2.02× slower for C2R. Direct native DUCC is close to
native RustFFT for R2C and faster than DUCCFFT at every measured length. The
f32 inverse AVX2 kernel lowers the C2R time at selected radix-two stages; it
does not remove the broader real-inverse execution gap.

### Setup and storage

Rust setup entries are medians of 12 process-level medians of 20 plan or
workspace creations. `RustFFT adapter setup` includes both directional plans,
the complex transform buffer, FFT scratch, twiddle allocation, and twiddle
generation. The direct C++ entries are medians of 12 process-level medians of
20 individually timed operations; destruction occurs after each timed
interval. DUCC's plan scratch and the adapter's reusable packed-data buffer
are separate. The Rust workspace bytes are from
`RealFftWorkspace::scratch_bytes()`.

| N | Build | DUCCFFT plan / workspace (µs) | RustFFT adapter setup (µs) | Direct DUCC plan / scratch allocation / work allocation (µs) | Direct DUCC plan scratch / work bytes | DUCCFFT workspace bytes |
|---:|---|---:|---:|---:|---:|---:|
| 128 | Portable | 2.625 / 0.141 | 1.934 | 0.471 / 0.025 / 0.026 | 512 / 512 | 2,064 |
| 256 | Portable | 5.120 / 0.170 | 3.767 | 0.581 / 0.030 / 0.030 | 1,024 / 1,024 | 3,616 |
| 512 | Portable | 9.143 / 0.205 | 6.092 | 1.022 / 0.040 / 0.040 | 2,048 / 2,048 | 7,728 |
| 1,024 | Portable | 18.185 / 0.251 | 12.304 | 1.358 / 0.050 / 0.050 | 4,096 / 4,096 | 13,888 |
| 2,048 | Portable | 32.632 / 0.351 | 25.674 | 2.445 / 0.080 / 0.080 | 8,192 / 8,192 | 30,288 |
| 4,096 | Portable | 64.693 / 0.571 | 51.689 | 3.642 / 0.140 / 0.140 | 16,384 / 16,384 | 54,880 |
| 128 | Native | 2.555 / 0.151 | 1.638 | 0.446 / 0.020 / 0.020 | 512 / 512 | 2,064 |
| 256 | Native | 4.969 / 0.161 | 3.672 | 0.551 / 0.030 / 0.030 | 1,024 / 1,024 | 3,616 |
| 512 | Native | 8.101 / 0.186 | 5.926 | 0.872 / 0.040 / 0.040 | 2,048 / 2,048 | 7,728 |
| 1,024 | Native | 15.990 / 0.240 | 12.093 | 1.418 / 0.110 / 0.050 | 12,768 / 4,096 | 13,888 |
| 2,048 | Native | 36.584 / 0.361 | 25.323 | 2.215 / 0.196 / 0.080 | 25,056 / 8,192 | 30,288 |
| 4,096 | Native | 62.784 / 0.571 | 50.239 | 3.537 / 0.391 / 0.140 | 49,632 / 16,384 | 54,880 |

The direct C++ work buffer is one N-float array reused serially across rows;
output arrays are caller storage and are not counted as scratch. Batch setup
measurements are retained in the raw records. The native DUCC planner uses
larger aligned plan scratch for N≥1,024.

The direct DUCC timing records are
[`m4-f32-ducc-portable-process-samples.csv`](../benches/results/m4-f32-ducc-portable-process-samples.csv)
and
[`m4-f32-ducc-native-process-samples.csv`](../benches/results/m4-f32-ducc-native-process-samples.csv).
The Rust native-target records are
[`m4-f32-rust-native-process-samples.csv`](../benches/results/m4-f32-rust-native-process-samples.csv).
Each direct C++ file has 2,016 unique rows; the Rust native file has 2,016.
The direct C++ binaries were built from source hash
`501f659379908ad935cd4334e8aa10cce1e5ae9057c0ffdf8d693d0cefcbc919` and have
SHA-256 `73d0dadf702faf8a01231ac5085398ecf0215bacc9d8d1a679cfa0247816b068`
(portable) and
`c4a4e81b4f10c3954c92258a0b4244a0f15a9454aa479965dfdfab4d9d5d858f`
(native). The Rust native benchmark binary SHA-256 is
`6526addbc977011cb4ef8b058ba8933821430f296b66d1888eb4f6789a560a1f`.
The benchmark source SHA-256 is
`76ea65520ac18ab604087c9011c81f61b5f54b597dbfb3b45ad7cf9320c2b665`; the
crate lockfile SHA-256 is
`e61d5628d09dc7c10e54e15cb5f3aeb5dd5c28392b44692ed153784ad8ca21ba`. Raw
CSV SHA-256 values are `dfe61b3b4b3e9d459fe401ea6175980cf48734e05cbda4f1236685c8922d95e6`
(portable DUCC),
`2e8d7af99cea7cd80fc7ee8b71b733180c0478d57b8aa2897f257b18819faed6`
(native DUCC), and
`39b29a13e70376a1f3f7a21da689c6e3db4c98ab420443b8f5314ea80ea52f23`
(native Rust).
