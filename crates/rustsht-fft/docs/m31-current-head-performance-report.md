# M31 current-head FFT baseline

Date: 2026-10-16. Source revision: `cc6a626aede65c7bfdc9ebb0ab91cca5972fab94` (M30).

## Method

The release benchmark compared the current RustSHT-FFT source with RustFFT 6.4.1 in the same executable. It covered all 34 C2C and real operation/length cells at their current SHT row counts. Six process pairs alternated candidate-first and reference-first order; each process used one transform thread and seven warmed samples per operation. Real timings include public packing, reconstruction and normalization. Ratios are RustFFT latency divided by RustSHT-FFT latency; values below one mean RustFFT was faster. Intervals are deterministic percentile-bootstrap 95% intervals from 5,000 resamples of the six paired process ratios. Both commands used binary SHA-256 `1170d4ff79fe99557cc9e3c04c586bf6d76dd2a108ad4793f23038419112c00d`, built with Rust 1.99.0 stable, the optimized bench profile, a portable x86_64 target and runtime AVX2 dispatch. Processes ran on one CPU core of an x86-64 CPU. The observed `amd-pstate-epp` governor and energy preference were `performance`, with boost enabled and a CPU-frequency telemetry omitted policy maximum. Load-average telemetry was retained only in local diagnostics. All processes exited successfully.

## Latency relative to RustFFT

Times are median microseconds per transform at the listed row count. The complete 34-cell data, process outputs and setup measurements are in the result directory.

| Operation | N | Rows | RustSHT-FFT | RustFFT | RustFFT / RustSHT-FFT (95% interval) |
|---|---:|---:|---:|---:|---:|
| C2C forward | 722 | 361 | 11.105 | 6.877 | 0.619 (0.612–0.633) |
| C2C inverse | 722 | 361 | 13.879 | 6.492 | 0.468 (0.458–0.474) |
| C2C forward | 726 | 361 | 2.027 | 1.552 | 0.766 (0.729–0.785) |
| C2C inverse | 726 | 361 | 3.060 | 1.725 | 0.564 (0.536–0.577) |
| C2C forward | 1,442 | 721 | 26.157 | 12.767 | 0.488 (0.487–0.493) |
| C2C inverse | 1,442 | 721 | 28.081 | 13.026 | 0.464 (0.461–0.464) |
| C2C forward | 1,452 | 721 | 4.327 | 3.306 | 0.764 (0.717–0.773) |
| C2C inverse | 1,452 | 721 | 6.430 | 3.651 | 0.568 (0.545–0.573) |
| C2C forward | 2,904 | 721 | 9.522 | 7.185 | 0.755 (0.714–0.778) |
| C2C inverse | 2,904 | 721 | 13.689 | 7.764 | 0.567 (0.538–0.576) |
| R2C | 723 | 361 | 10.068 | 4.710 | 0.468 (0.453–0.470) |
| C2R | 723 | 361 | 10.945 | 4.750 | 0.434 (0.417–0.438) |
| R2C | 2,049 | 1,024 | 31.840 | 36.881 | 1.158 (1.116–1.170) |
| C2R | 2,049 | 1,024 | 35.280 | 36.809 | 1.043 (1.039–1.050) |
| R2C | 4,097 | 2,048 | 83.935 | 60.693 | 0.723 (0.719–0.726) |
| C2R | 4,097 | 2,048 | 97.690 | 60.840 | 0.623 (0.617–0.626) |

N=1,442 has the largest complex latency: RustSHT-FFT is 2.05× slower than RustFFT forward and 2.16× slower inverse. At N=722, RustSHT-FFT is 2.14× slower inverse and 1.62× slower forward. At N=2,904 inverse, it is 1.76× slower. For real transforms, it is 2.14–2.30× slower at N=723 and 1.38–1.61× slower at N=4,097. At N=2,049, RustSHT-FFT is slightly faster in both real directions. The largest absolute costs are real C2R N=4,097 at 97.690 µs, real R2C N=4,097 at 83.935 µs, and C2C inverse N=1,442 at 28.081 µs. Small C2C N=146 has the largest relative gap, but its 3.6–4.3 µs transform cost is lower than the N=722 and N=1,442 targets.

## Confirmed routes and memory

The route trace shows the source-selected factorization and runtime dispatch:

| Workload | Current route |
|---|---|
| C2C N=722 | `[2,19,19]`; first radix-2 prefix uses AVX2; radix-19 passes `(l1, ido) = (2,19)` and `(38,1)` use the scalar general-prime path. |
| C2C N=726 | `[2,3,11,11]`; AVX2 radix-2 prefix, radix-3 pass, odd-`ido` radix-11 pass and terminal radix-11 pass are selected. |
| C2C N=1,442 | `[2,7,103]`; the terminal radix-103 `ido=1` pass uses the f64 AVX2 kernel and plan-level root cache. |
| C2C N=1,452 and 2,904 | `[4,3,11,11]` and `[8,3,11,11]`; AVX2 radix-3 and radix-11 kernels cover the reachable passes. |
| Real N=723 | Rader radix 241 with a 240-point complex convolution; the safety fallback uses Bluestein with convolution length 512. |
| Real N=4,097 | Rader radix 241 with a 240-point complex convolution; the safety fallback uses Bluestein with convolution length 484. |

The route/memory diagnostic reports retained heap payload and workspace bytes. Selected current RustSHT-FFT values are: C2C N=722, 30,390/12,160 bytes; N=1,442, 107,061/26,368 bytes; N=2,904, 102,000/46,816 bytes; real N=723, 69,744/53,536 bytes; and real N=4,097, 150,785/133,824 bytes. Retained plan counts exclude allocator metadata and inline plan-object storage. RustFFT workspace bytes are present in the CSV; comparable retained-plan payload was not collected. Plan and workspace construction were timed separately. For example, C2C N=722 plan construction was 10.294 µs for RustSHT-FFT and 49.023 µs for RustFFT; workspace payload was 12,160 and 36,128 bytes. At N=1,442 plan construction was 28.687 and 36.739 µs, with workspace payloads of 26,368 and 23,072 bytes. At real N=4,097 plan construction was 93.359 and 466.558 µs, with workspace payloads of 133,824 and 345,488 bytes. A separate fresh-process diagnostic measured one first transform and seven warmed calls after plan, workspace and input construction. Its raw records cover the same 34 operations. These are unpaired observations and are not used in the ranking; some first-call and warmed estimates, particularly at N=722, were higher than the row-batch measurements.

## Profile evidence and next target


The N=1,442 AVX2 radix-103 kernel accounts for 63.3% of forward and 57.8% of inverse profile instruction references. Its plan-cache route is accepted; the earlier Rader-103 and paired-outer experiments are documented as rejected. The next complex candidate targets the scalar radix-19 transforms at N=722. The real N=723 and N=4,097 paths follow.

## Other-library comparisons and checks

M19 and M23 contain DUCC, FFTW and Admiral comparisons for earlier source revisions. M24–M30 changed complex kernels, so those complex ratios do not describe this source revision. The M32 acceptance capture will compare the radix-19 candidate with native and portable DUCC, FFTW and Admiral on selected workloads. The real-transform implementation matches M21. Before this measurement, `cargo fmt --all -- --check`, `cargo test --all-features` (108 tests) and all-target/all-feature Clippy with warnings denied passed. `cargo bench --bench rustfft --no-run` passed. The route diagnostic passed. Release tests were not repeated for this documentation and measurement milestone; M30's report records the release suite for the unchanged production source. The full records are in [`M31 results`](../benches/results/m31-current-head-cross-library/). M19 documents the M18-source baseline.
