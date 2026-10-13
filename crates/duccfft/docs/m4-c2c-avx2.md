# M4 C2C AVX2 checkpoint

## Scope

This report measures the first runtime-dispatched AVX2 kernels for complex
transforms. The kernels handle radix 2, 4, and 8 passes in both the in-place
first pass and the out-of-place multipass path. They cover `f32` and `f64`. Unsupported CPUs, other radices, and inverse inputs that require residual
recovery use the scalar path. The measured source revision is `b9ccc38` (`Add runtime AVX2 complex radix
passes`). The full per-process measurements are in
[`m4-c2c-avx2-process-samples.csv`](../benches/results/m4-c2c-avx2-process-samples.csv). That file contains 420 C2C records: three process samples for each
implementation, direction, length, and call count.

## Measurement setup

The host was an x86-64 CPU. All processes were pinned to logical one CPU core. The CPU supports AVX2. The Rust compiler was `rustc 1.99.0
(b940084d7 2026-09-28)`. Rust benchmarks used the Cargo `bench` profile at
`opt-level=3`, with no custom `RUSTFLAGS`, LTO, or `target-cpu` setting. RustFFT
was version 6.4.1 and used its default runtime SIMD dispatch. DUCC source was
pinned to `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, built with g++ 15.2.0. The portable DUCC build used `-std=c++20 -O3 -DNDEBUG -pthread`; the native
build used `-std=c++20 -O3 -DNDEBUG -march=native -pthread`. DUCC's vectorized
planner option was disabled for both runs. This separates the native-target
compiler setting from DUCC's vectorized plan selection. The Rust AVX2 kernels
were compiled with function-level `target_feature` and selected after runtime
CPU detection. Each execution timing is the median of seven timed samples after one warmup. Three separate processes were run per implementation. Inputs were deterministic,
and checksums agreed within output rounding across implementations. Plans, input
buffers, output buffers, and scratch were allocated before timed execution. “Batch 32” means 32
sequential independent calls reusing one plan and workspace; the table reports
nanoseconds per transform. It is not a batched FFT API. Load-average telemetry was retained only in local diagnostics. The CPU was pinned, but host interference
cannot be ruled out. Absolute timings remain provisional.

## C2C timings

Timing tables report `f64`; f32 correctness is covered separately by the SIMD
kernel tests and the large f32 transform regressions. Each value is the median
nanoseconds per transform across the three process samples. Ratios are
`DUCCFFT AVX2 / competitor`; values above 1 mean DUCCFFT was
slower. Single-call timings use the public operation with a warm plan and
reused workspace. C2C has no real packing adapter, so this public-operation
timing is also the relevant FFT execution timing.

### Forward, one transform

| n | DUCCFFT AVX2 | RustFFT | DUCC portable | DUCC native | AVX2 / RustFFT | AVX2 / DUCC native |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 0.240 µs | 0.070 µs | 0.151 µs | 0.150 µs | 3.43× | 1.60× |
| 128 | 0.481 µs | 0.130 µs | 0.331 µs | 0.291 µs | 3.70× | 1.65× |
| 256 | 0.812 µs | 0.291 µs | 0.642 µs | 0.561 µs | 2.79× | 1.45× |
| 512 | 1.784 µs | 0.651 µs | 1.462 µs | 1.423 µs | 2.74× | 1.25× |
| 1024 | 4.358 µs | 1.553 µs | 3.537 µs | 3.066 µs | 2.81× | 1.42× |
| 2048 | 11.562 µs | 3.697 µs | 8.396 µs | 8.095 µs | 3.13× | 1.43× |
| 4096 | 25.979 µs | 8.787 µs | 21.030 µs | 20.058 µs | 2.96× | 1.30× |

### Inverse, one transform

| n | DUCCFFT AVX2 | RustFFT | DUCC portable | DUCC native | AVX2 / RustFFT | AVX2 / DUCC native |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 0.311 µs | 0.090 µs | 0.161 µs | 0.140 µs | 3.46× | 2.22× |
| 128 | 0.661 µs | 0.160 µs | 0.381 µs | 0.310 µs | 4.13× | 2.13× |
| 256 | 1.202 µs | 0.351 µs | 0.742 µs | 0.581 µs | 3.42× | 2.07× |
| 512 | 2.595 µs | 0.782 µs | 1.623 µs | 1.453 µs | 3.32× | 1.79× |
| 1024 | 5.781 µs | 1.773 µs | 3.967 µs | 3.276 µs | 3.26× | 1.76× |
| 2048 | 14.356 µs | 4.188 µs | 8.556 µs | 7.985 µs | 3.43× | 1.80× |
| 4096 | 31.539 µs | 9.728 µs | 21.170 µs | 19.457 µs | 3.24× | 1.62× |

### Forward, sequential batch of 32

| n | DUCCFFT AVX2 | RustFFT | DUCC portable | DUCC native | AVX2 / RustFFT | AVX2 / DUCC native |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 0.208 µs | 0.055 µs | 0.133 µs | 0.137 µs | 3.78× | 1.52× |
| 128 | 0.456 µs | 0.120 µs | 0.310 µs | 0.277 µs | 3.79× | 1.65× |
| 256 | 0.805 µs | 0.281 µs | 0.626 µs | 0.564 µs | 2.87× | 1.43× |
| 512 | 1.798 µs | 0.679 µs | 1.423 µs | 1.405 µs | 2.65× | 1.28× |
| 1024 | 4.385 µs | 1.609 µs | 3.370 µs | 3.038 µs | 2.73× | 1.44× |
| 2048 | 8.817 µs | 3.921 µs | 8.065 µs | 7.410 µs | 2.25× | 1.19× |
| 4096 | 20.855 µs | 8.961 µs | 18.120 µs | 17.192 µs | 2.33× | 1.21× |

### Inverse, sequential batch of 32

| n | DUCCFFT AVX2 | RustFFT | DUCC portable | DUCC native | AVX2 / RustFFT | AVX2 / DUCC native |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 0.291 µs | 0.071 µs | 0.136 µs | 0.130 µs | 4.12× | 2.25× |
| 128 | 0.634 µs | 0.151 µs | 0.364 µs | 0.290 µs | 4.20× | 2.19× |
| 256 | 1.171 µs | 0.338 µs | 0.706 µs | 0.567 µs | 3.47× | 2.06× |
| 512 | 2.509 µs | 0.806 µs | 1.615 µs | 1.452 µs | 3.11× | 1.73× |
| 1024 | 5.875 µs | 1.794 µs | 3.784 µs | 3.297 µs | 3.28× | 1.78× |
| 2048 | 14.184 µs | 4.221 µs | 8.837 µs | 7.915 µs | 3.36× | 1.79× |
| 4096 | 31.760 µs | 9.824 µs | 21.324 µs | 19.877 µs | 3.23× | 1.60× |

## Plan and workspace costs

Plan construction and workspace allocation each used 20 repetitions per
process. The table reports the median process result. Workspace bytes include
initialized reusable buffers; they exclude the plan's roots and allocator
metadata.

| n | Plan construction | Workspace allocation | Workspace bytes |
|---:|---:|---:|---:|
| 64 | 1.706 µs | 0.164 µs | 1280 |
| 128 | 1.842 µs | 0.082 µs | 2304 |
| 256 | 3.536 µs | 0.097 µs | 4352 |
| 512 | 6.844 µs | 0.125 µs | 8448 |
| 1024 | 14.220 µs | 0.186 µs | 16640 |
| 2048 | 27.256 µs | 0.323 µs | 33024 |
| 4096 | 54.220 µs | 0.579 µs | 65792 |

## Results and next optimization

The AVX2 path improved C2C execution over the M3 scalar build by 1.29–1.94×
across the single-call and batch measurements. It remains 2.25–4.20× slower than
default RustFFT and 1.19–2.25× slower than native-target DUCC. The vector path
reduces the scalar gap but does not close the architecture gap. Release assembly was emitted with
`cargo rustc --release --lib -- --emit=asm`. It contains packed AVX2
floating-point instructions in both f32 and f64 pass kernels. It also retains
runtime comparisons for radices 2, 4, and
8 inside the vector pass routines. The four kernels reserve 224–352 bytes of
stack and contain stack-resident vector temporaries. These generated-code
features identify a bounded next experiment: move radix selection outside the
inner loops and specialize the fixed-radix loop nests, then remeasure. The
current assembly supports those observations; it does not establish the exact
wall-time cost of either feature. `cargo fmt --manifest-path crates/duccfft/Cargo.toml -- --check`,
`cargo test --manifest-path crates/duccfft/Cargo.toml --all-targets`, and
`cargo clippy --manifest-path crates/duccfft/Cargo.toml --all-targets --all-features -- -D warnings`
passed. The test run covered 3 SIMD unit tests, 17 complex integration tests,
and 21 real integration tests. This change affects public C2C calls only. Real transforms still use their
recursive scalar execution path. Compare a complete packed-real multipass
prototype with that path before adding real SIMD. A reviewer assessment of the
AVX2 implementation and its benchmark evidence is pending.
