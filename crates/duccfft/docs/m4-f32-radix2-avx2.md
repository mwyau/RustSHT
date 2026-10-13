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
workspace buffers. The equations follow the inverse radix-two branch in pinned DUCC source
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
candidate. The machine was an x86-64 CPU with AVX2. Twelve baseline/candidate
process pairs per length were pinned to logical one CPU core; process order
alternated by length and pair. Each process warmed reusable plans and took
seven paired samples. Within a process, the DUCCFFT and RustFFT adapter order
alternated. Single-transform samples repeated `max(262144 / N, 16)` calls;
batch samples repeated `max(floor(262144 / N / 32), 1)` calls to
`forward_batch` or `inverse_batch`. Inputs and output buffers were allocated
outside the timer. Timings cover public validation, real packing or
unpacking, FFT execution, and normalization. Rust used 1.99.0 (`b940084d7`, LLVM 23.1.1), Cargo 1.99.0, and the default
optimized `bench` profile (`-C opt-level=3`, `-C embed-bitcode=no`,
`-C strip=debuginfo`). No custom `RUSTFLAGS`, LTO, or target CPU setting was
used. RustFFT was 6.4.1 and used the existing RustSHT-style half-length
complex real adapter. The direct C++ DUCC timing harness currently uses
double precision, so this f32 change is compared with RustFFT and its own
paired baseline; the table does not claim an f32 C++ DUCC timing. Each table timing is the median of 84 values (12 process pairs times seven
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
change in forward execution. Candidate whole-public-operation timings are compared with the RustFFT real
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
2.05, 1.99, and 1.90 for single transforms at lengths 128, 512, and 2,048. For batches of 32 they were 2.01, 2.02, and 1.89. Candidate R2C ratios were
1.64–1.66 for single transforms and 1.63–1.67 for batches at those lengths. The kernel reduces the inverse gap but does not remove it. Each process timed 20 plan constructions and 20 workspace allocations. The
table reports the median of the twelve process medians. Scratch bytes are
reported by `RealFftWorkspace::scratch_bytes()`;
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
[`m4-f32-real-radix2-process-samples.csv`](../benches/results/m4-f32-real-radix2-process-samples.csv). All 4,032 keys are unique. Matching baseline/candidate records have the same
sample order, repeat count, and checksums. The Legendre workload shared the host. Load-average telemetry was retained only in local diagnostics. The paired controls support retaining
this kernel at 128, 512, and 2,048. Review R25 is pending.

## Generated code

Release disassembly of
`duccfft::simd::x86::f32_real_kernel::inverse_radix2_interior` contains packed
`vmovups`, `vpermpd`, `vaddps`, `vsubps`, `vblendps`, `vmulps`, and shuffle
instructions. The vector loop has one back-edge branch and no helper calls or
stack spills. It uses runtime AVX2 dispatch and emits no FMA instructions.
