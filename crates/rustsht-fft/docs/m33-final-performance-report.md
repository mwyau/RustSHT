# RustSHT-FFT M32–M33 performance report

## Result

The accepted source is M32 at commit `234ab2d30b8d542457398e2f7be17e7e7b9aa25e`. M32 adds a private AVX2 radix-19 kernel for the `(l1, ido) = (38, 1)` pass in complex N=722. M33 produced no accepted code change. Its safety-scan fusion improved the selected real rows, but the full workload showed repeatable forward C2C regressions at N=146 and N=722.

M32 reduced the measured N=722 forward latency from the M31 value of 11.11 µs to 7.258 µs and inverse latency from 13.88 µs to 9.310 µs. The six-pair acceptance comparison gave 1.533× forward speedup (95% interval 1.524–1.611) and 1.487× inverse speedup (1.473–1.495). The `(2, 19)` pass remains scalar; runtime AVX2 dispatch selects the vectorized `(38, 1)` pass. The kernel preserves scalar f32 and unsafe inverse fallbacks.

The independent read-only review found no blocking numerical, memory-safety, dispatch, or scaling issue after a documentation correction. The new exact-source cross-library captures use the accepted M32 executable, SHA-256 `a1036e527f31637d8f0fd9212d1401c2d568155cd1e8dc0faff785aa05333a8c`, built with stable Rust 1.99.0 and portable x86_64 target flags. Each reference comparison used six alternating process pairs, seven warmed samples, one transform thread, and CPU 30 on an AMD Ryzen 9 5950X.

## Accepted-source comparison matrix

The complete 34-cell matrix for each of RustFFT 6.4.1, native SIMD DUCC, portable DUCC, FFTW 3.3.10, and Admiral is in [`m32-accepted-source-cross-library-summary.csv`](../benches/results/m33-rader-fusion/m32-accepted-source-cross-library-summary.csv). The accompanying JSONL files record raw process output, binary hashes, setup times, scratch bytes, checksums, and host load. DUCC is pinned at `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; native DUCC used planner vectorization. Admiral is pinned at `ce67bc00`.

The table reports the accepted RustSHT-FFT median and `reference / RustSHT-FFT` latency ratios. A ratio above 1 means RustSHT-FFT was faster for that row.

| Operation | RustSHT-FFT | RustFFT 6.4.1 | DUCC native | DUCC portable | FFTW 3.3.10 | Admiral |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| C2C forward, N=722 | 7.278 µs | 0.933× | 0.500× | 0.683× | 0.928× | 1.026× |
| C2C inverse, N=722 | 9.344 µs | 0.691× | 0.398× | 0.551× | 0.741× | 0.828× |
| R2C, N=723 | 10.119 µs | 0.466× | 1.156× | 1.639× | 0.881× | 1.050× |
| C2R, N=723 | 10.933 µs | 0.435× | 1.066× | 1.515× | 0.819× | 0.975× |
| R2C, N=2049 | 31.631 µs | 1.172× | 1.201× | 1.569× | 1.329× | 0.884× |
| C2R, N=2049 | 35.178 µs | 1.048× | 1.091× | 1.446× | 1.214× | 0.816× |
| R2C, N=4097 | 84.475 µs | 0.724× | 0.891× | 1.237× | 0.757× | 0.746× |
| C2R, N=4097 | 99.171 µs | 0.613× | 0.745× | 1.064× | 0.616× | 0.656× |

At N=723, the accepted source is 2.15× slower than RustFFT for R2C and 2.30× slower for C2R, while it is faster than native SIMD DUCC in both directions. At N=4097, RustSHT-FFT is 1.38× slower than RustFFT for R2C and 1.63× slower for C2R; it is 12% and 34% slower than native DUCC, respectively. At the N=2049 regression control, it is faster than RustFFT, DUCC, and FFTW, while Admiral is faster. These are standalone FFT measurements; they do not establish an end-to-end SHT speedup.

## M33 real-prime investigation

The accepted planner routes N=723 and N=4097 through Rader radix 241 with a 240-point complex convolution. N=2049 remains on the radix-683 Rader route with a 682-point inner transform. The route and retained-plan memory output is in [`m32-accepted-route-memory-diagnostics.txt`](../benches/results/m33-rader-fusion/m32-accepted-route-memory-diagnostics.txt).

Callgrind profiles isolate public real operations, multipass execution, the Rader helper, and the convolution region. They identify the inner complex transforms and convolution as the largest profiled regions. For N=723 C2R, the 100-operation captures report 25.66 million instruction references for the public operation, 21.41 million for the multipass region, 18.83 million for Rader, and 16.06 million for convolution. The isolated 240-point C2C profiles report 2.10 million references for forward and 2.93 million for inverse over 100 calls; the N=723 route invokes each three times per real transform. For N=4097 C2R, the corresponding public, multipass, Rader, and convolution captures report 230.63, 210.59, 106.68, and 91.01 million references. The radix-241 stage invokes each 240-point transform 17 times per operation.

Callgrind records instruction references; these counts do not measure elapsed cycles or phase latency. The Rader helper profile includes the 240-point convolution plus gather/scatter, permutation, scaling, and wrapper work. The available captures do not assign separate wall-clock times to real packing, output reconstruction, pointwise multiplication, or final writeback. Hardware performance counters were unavailable on this host (`perf_event_paranoid=4`). The raw profiles are in [`profiles/`](../benches/results/m33-rader-fusion/profiles/).

## Rejected M33 candidate

The final M33 v3 candidate fused forward Rader safety-bound checks and collected inverse safety bounds during Rader spectrum packing. It added no scratch and did not change planner routes. In the six-pair 34-cell comparison against accepted M32, it improved the selected real rows:

| Operation | M32 | M33 v3 | M33 speedup, 95% interval |
| --- | ---: | ---: | ---: |
| R2C, N=723 | 10.087 µs | 9.570 µs | 1.054× (1.034–1.089) |
| C2R, N=723 | 11.025 µs | 9.868 µs | 1.119× (1.033–1.147) |
| R2C, N=2049 | 31.515 µs | 30.033 µs | 1.049× (1.038–1.054) |
| C2R, N=2049 | 34.732 µs | 32.068 µs | 1.086× (1.057–1.116) |
| R2C, N=4097 | 84.035 µs | 81.413 µs | 1.035× (1.028–1.037) |
| C2R, N=4097 | 98.902 µs | 90.611 µs | 1.086× (1.082–1.106) |

The full matrix also measured N=146 C2C forward at 3.855 µs versus 3.616 µs (0.939× reference/candidate ratio, 95% interval 0.935–0.950), a 6.5% slowdown on the paired ratio scale. N=722 C2C forward was 7.930 µs versus 7.250 µs (0.916×, 0.884–0.957), a 9.1% slowdown on that scale. The inverse N=722 row was at parity. The independent read-only review recommended rejecting the candidate because the two forward regressions were material; unchanged instruction counts and relocated machine code did not explain away the measured latency losses. The v3 source diff hash was `8845bead2e211dffc709f2cb13c908f4332754329bec60b45b10d59f666f7ee0`.

The v1 and v2 safety-scan fusion screens also remain in the results directory. Their N=1025 and N=1443 regressions led to the v3 Rader-only scope. No M33 source changes remain in the branch. Candidate/reference six-pair data, five cross-library matrices, first-call diagnostics, and profiles are indexed in [`benches/results/m33-rader-fusion/README.md`](../benches/results/m33-rader-fusion/README.md).

## Setup and memory

| Plan | Retained plan payload | Workspace scratch | Median plan construction | Median workspace construction |
| --- | ---: | ---: | ---: | ---: |
| C2C N=722 | 30,390 bytes | 12,160 bytes | about 10.3 µs | about 0.15 µs |
| Real N=723 | 69,744 bytes | 53,536 bytes | about 36.5 µs | about 1.1 µs |
| Real N=2049 | 191,153 bytes | 159,856 bytes | about 105 µs | about 2.6 µs |
| Real N=4097 | 150,785 bytes | 133,824 bytes | about 93.5 µs | about 1.8 µs |

Plan payload comes from `m0_retained_plan_bytes()` and excludes allocator bookkeeping. Construction medians are from the accepted-source paired captures. The M33 v3 candidate used the same workspace sizes as M32.

A separate first-call diagnostic records the first public operation after plan, workspace, and input construction, followed by seven warmed operations. It contains one observation per process and is not paired acceptance evidence: [`m33-first-call-diagnostic.txt`](../benches/results/m33-rader-fusion/m33-first-call-diagnostic.txt).

## Numerical and release validation

The accepted source passed direct DFT tests for both N=722 directions, including the complete two-pass transform, odd tails, guard regions, extreme finite values, and inverse normalization. The all-feature suites also cover f32/f64, Rader numerical paths, small residuals, shared immutable plans, and concurrent calls with separate workspaces.

For N=722, 723, 2049, and 4097, the independent comparison against native and portable DUCC covered 26,571 output records (53,142 real and imaginary components) for C2C forward/inverse and R2C/C2R. The maximum absolute difference from DUCC was `4.547e-13`; the comparison used `rtol=2e-11` and `atol=2e-11`. The same source was compared against FFTW 3.3.10 and Admiral `ce67bc00`; maximum absolute differences were `4.547e-13` and `1.137e-12`. Raw outputs are [`native DUCC`](../benches/results/m33-rader-fusion/m32-accepted-native-ducc-numerical-comparison.txt) and [`portable DUCC`](../benches/results/m33-rader-fusion/m32-accepted-portable-ducc-numerical-comparison.txt).

The following checks passed on the accepted M32 source:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo test --workspace --all-features --release`

Each test suite passed 111 tests: 37 unit, 2 across-row, 32 complex, and 40 real tests; doctests contained no cases. The host supports AVX2. Runtime AVX2 selection and scalar/f32 and exceptional-input fallbacks were tested; a non-AVX2 host was not available.

## Remaining work and recommendation

The largest measured gaps are real R2C/C2R at N=723 and N=4097 against RustFFT, plus N=4097 against native DUCC. The profiles point to repeated 240-point complex transforms in the N=723 and N=4097 Rader routes. M33 did not change those inner transforms, and its scan fusion was rejected. Further changes should first measure elapsed time for the Rader subphases; Callgrind instruction references alone do not establish that a proposed rewrite will reduce latency.

RustSHT-FFT is suitable for a bounded, optional SHT integration experiment that measures end-to-end GL transforms with the current standalone source. It is not ready for a production switch. The integration experiment must include analytic fields, independent DUCC comparisons, adjoint identities, setup and workspace costs, and the actual target grid and batch sizes. No SHT code was changed in this session.
