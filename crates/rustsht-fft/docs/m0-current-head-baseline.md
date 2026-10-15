# M0 current-head FFT baseline

Date: 2026-10-15. Source baseline: `aaadcb0aba9299049cf68eeca0f0dcd851c799b2`.
The benchmark and diagnostic changes in this report do not select a new FFT
route or change a numerical kernel.

The candidate source predates M1–M5. The updated comparison at the accepted
M5 source is in
[`m5-current-head-cross-library.md`](m5-current-head-cross-library.md).

## Method

The measurements use f64 transforms with unnormalized forward operations and
inverse operations normalized by `1/N`. Each plan is constructed once for the
warmed execution samples; a separate setup measurement averages 20 plan and
workspace constructions. Transform latency is the median of seven timed
samples within each process. The reported ratio is
`reference_time / RustSHT-FFT_time`; values above 1 indicate a faster
RustSHT-FFT call. Each comparison has six alternating process pairs pinned to
logical CPU 30. The raw JSONL records include command lines, executable hashes,
process IDs, wall times and load averages before each process. The summary CSV
reports the median paired ratio, its observed range and a 95% percentile
bootstrap interval for the median.

The standard matrix covers 41 lengths, four operations, and row counts 1 and
32. Row counts are capped at 4 when `N > 10000`. The matrix includes the
required real, complex, direct-prime and convolution lengths; controls 98,
147, 4096 and 4097; and the 113² C2C and 137² real cases. The second matrix
uses exact SHT-shaped row counts: C2C lengths 146, 150, 300, 256, 512, 722,
726, 1024, 1442, 1452 and 2904; real lengths 257, 723, 1025, 1443, 2049 and
4097. All six references use the same operation and row shapes in the paired
SHT matrix.

The RustSHT-FFT, RustFFT, DUCC, FFTW and Admiral harnesses generate identical
deterministic inputs. The measured real operations include spectrum packing or
reconstruction and inverse normalization where required. FFTW uses
`FFTW_ESTIMATE | FFTW_PRESERVE_INPUT`; the FFTW and Admiral harnesses use
64-byte aligned, padded rows. All reference runs use one execution thread.
The FFTW path is the system 3.3.10 package because the pinned local source
checkout did not link: its CMake build lacked generated solver tables, and
`autoreconf` is unavailable on this host. The system package's own build flags
were not available. No result below should be attributed to that local source
checkout.

## Build configuration

| Implementation | Revision or version | Build and planning configuration |
| --- | --- | --- |
| RustSHT-FFT | `aaadcb0` | Rust 1.99.0, release bench profile, portable compiler target, runtime AVX2 dispatch |
| RustFFT | 6.4.1 | Same Rust bench binary and compiler; default runtime dispatch |
| DUCC portable | `c1ed01b8fda69d2a378190f0915f1a19762dddbf` | g++ 15.2.0, C++20, `-O3 -DNDEBUG -pthread`, `vectorize=false` |
| DUCC native | `c1ed01b8fda69d2a378190f0915f1a19762dddbf` | g++ 15.2.0, C++20, `-O3 -DNDEBUG -march=native -pthread`, `vectorize=true` |
| FFTW | system 3.3.10 | g++ 15.2.0 harness at `-O3 -DNDEBUG`; system library; `FFTW_ESTIMATE`, input preservation |
| Admiral | `ce67bc00ff9e6de96bf5b75f7a789581105ddb58` | CMake Release, `ADM_TARGET_ARCH=native`, fast math and threads off; estimate planning |

The host is an AMD Ryzen 9 5950X with 16 cores and 32 logical CPUs, running
Linux x86-64. Rust reports LLVM 23.1.1. DUCC and external harness build details
are recorded in [`benches/results`](../benches/results/).

## Numerical comparison

The 35 required lengths produced 92,684 output records per DUCC build.
RustSHT-FFT agreed with pinned DUCC within the comparison tolerance
`atol=rtol=2e-11`: maximum absolute error was `9.095e-13` for the portable
build and `7.958e-13` for the native build. Maximum scaled errors were
`1.245e-13` and `1.705e-13`, respectively. The portable build comparison also
included N=4097, for 107,024 records and maximum absolute and scaled errors
`9.095e-13` and `2.427e-13`.

The six control-length comparisons produced 139,920 output records per
implementation; they include N=4097 as an SHT workload alongside 98, 147,
4096, 12769 and 18769. Maximum absolute error against DUCC was `1.819e-12`
for both planner builds. Comparing RustSHT-FFT with FFTW and Admiral over
those controls gave maximum absolute errors `3.638e-12` and `5.457e-12`.
Every comparison passed the same tolerance. The logs record each record count
and maximum error.

These checks establish agreement for the tested deterministic inputs and
lengths. They do not establish accuracy for every input magnitude or
distribution.

## Execution routes

The private test diagnostics record each plan's radix, `l1`, `ido`, prime
algorithm, convolution length, and the route entered during forward and inverse
execution. They also record runtime AVX2 branches and the Rader safety fallback.

| Operation and length | Observed route |
| --- | --- |
| C2C 146 | Radix 2 followed by direct prime 73 (`l1=2`, `ido=1`) |
| C2C 722 | Radix 2 and two direct prime 19 passes; no explicit C2C AVX2 branch executed |
| C2C 1442 | Radices 2 and 7 followed by direct prime 103; no explicit C2C AVX2 branch executed |
| C2C 256, 512, 1024 | Runtime AVX2 branches executed in the power-of-two passes; radix-8 `ido=1` AVX2 executed at 512 and 1024 |
| Real 257 | Rader-257 with convolution length 256; subnormal forward and inverse inputs selected the recursive safety fallback to the prior Bluestein route |
| Real 723 | Radix 3 followed by Rader-241 with convolution length 240; subnormal inputs selected the recursive safety fallback |
| Real 1025 | Fixed radix-5 passes and terminal general-prime radix 41; the real AVX2 `ido=1` kernel executed |
| Real 1442 | Complexified to C2C length 721, with direct prime 103 |
| Real 1443 | Fixed radix 3, general-prime radix 13 and terminal radix 37; the real AVX2 `ido=1` kernel executed at radix 37 |
| Real 2049 | Radix 3 followed by Bluestein radix 683 with convolution length 1372 |
| Real 4097 | General-prime radix 17 followed by Bluestein radix 241 with convolution length 484 |
| Real 4096 | Six fixed radix-4 passes; real AVX2 radix-4 branches executed in the first four passes |

The full trace is in
[`m0-route-diagnostics-aaadcb0.txt`](../benches/results/m0-route-diagnostics-aaadcb0.txt).
It includes 98 and 147 interior layouts, 113² C2C and 137² real controls,
and the remaining benchmark lengths.

## Plan and workspace costs

Plan payload counts sum `Vec` capacities and nested vector payloads. They
exclude inline plan fields and allocator metadata. Workspace bytes are the
public reusable scratch payload; they exclude caller input and output arrays.
The benchmark setup time averages 20 constructions and drops.

| Operation and length | Plan construction (µs) | Workspace construction (µs) | Workspace bytes | Retained plan payload (bytes) |
| --- | ---: | ---: | ---: | ---: |
| C2C 146 | 5.70 | 0.17 | 4,672 | 8,948 |
| C2C 722 | 10.31 | 0.16 | 12,160 | 30,246 |
| C2C 1442 | 26.71 | 0.29 | 26,368 | 65,301 |
| C2C 256 | 3.50 | 0.10 | 4,352 | 9,488 |
| Real 257 | 29.73 | 0.88 | 46,032 | 60,488 |
| Real 723 | 38.25 | 0.94 | 56,576 | 68,928 |
| Real 1025 | 18.17 | 0.37 | 28,944 | 34,489 |
| Real 1443 | 24.84 | 0.46 | 40,976 | 43,290 |
| Real 2049 | 73.73 | 1.73 | 137,040 | 129,024 |
| Real 4097 | 81.85 | 1.46 | 129,728 | 127,953 |

The backend timing harnesses report workspace bytes as zero when the library
does not expose caller-owned scratch. This means the harness cannot report
that backend's scratch size; it does not mean that the implementation uses no
memory. Peak resident memory was not measured.

## Paired timing results

Each table entry is the median reference-to-RustSHT-FFT time ratio from six
process pairs. The full CSV gives absolute per-transform medians, observed
paired ranges and bootstrap intervals for every length and mode. A ratio below
1 indicates that RustSHT-FFT was slower for that cell.

### Complex transforms at SHT row counts

| Mode, length, rows | RustFFT | DUCC portable | DUCC native | FFTW | Admiral |
| --- | ---: | ---: | ---: | ---: | ---: |
| C2C forward, 146, 73 | 0.09× | 0.28× | 0.27× | 0.18× | 0.25× |
| C2C inverse, 146, 73 | 0.10× | 0.27× | 0.26× | 0.20× | 0.24× |
| C2C forward, 722, 361 | 0.33× | 0.24× | 0.18× | 0.33× | 0.36× |
| C2C inverse, 722, 361 | 0.29× | 0.24× | 0.17× | 0.32× | 0.35× |
| C2C forward, 1442, 721 | 0.11× | 0.21× | 0.15× | 0.21× | 0.17× |
| C2C inverse, 1442, 721 | 0.11× | 0.21× | 0.14× | 0.20× | 0.16× |
| C2C forward, 256, 128 | 0.40× | 0.86× | 0.78× | 0.41× | 0.65× |
| C2C inverse, 256, 128 | 0.31× | 0.66× | 0.53× | 0.33× | 0.43× |
| C2C forward, 512, 256 | 0.81× | 1.56× | 0.93× | 0.73× | 0.91× |
| C2C inverse, 512, 256 | 0.53× | 1.04× | 0.59× | 0.50× | 0.51× |
| C2C forward, 1024, 512 | 0.71× | 1.51× | 0.78× | 0.70× | 0.79× |
| C2C inverse, 1024, 512 | 0.47× | 1.04× | 0.52× | 0.46× | 0.46× |

The mixed-prime C2C calls have the largest measured gaps. At N=1442, the
RustSHT-FFT forward and inverse medians were 115.4 and 123.9 µs per row;
RustFFT measured 12.8 and 13.0 µs, and native DUCC measured 17.7 and 16.3 µs.
The route trace identifies the direct radix-103 stage as the dominant bounded
target for the paired-output experiment.

### Real transforms at SHT row counts

| Mode, length, rows | RustFFT | DUCC portable | DUCC native | FFTW | Admiral |
| --- | ---: | ---: | ---: | ---: | ---: |
| R2C, 257, 128 | 0.40× | 1.63× | 0.96× | 0.96× | 0.52× |
| C2R, 257, 128 | 0.40× | 1.54× | 0.91× | 0.76× | 0.43× |
| R2C, 723, 361 | 0.35× | 1.24× | 0.88× | 0.65× | 0.78× |
| C2R, 723, 361 | 0.33× | 1.17× | 0.82× | 0.63× | 0.74× |
| R2C, 1025, 512 | 1.01× | 0.82× | 0.61× | 1.05× | 0.97× |
| C2R, 1025, 512 | 0.86× | 0.64× | 0.46× | 0.88× | 0.82× |
| R2C, 1443, 721 | 0.77× | 0.36× | 0.30× | 0.49× | 0.67× |
| C2R, 1443, 721 | 0.64× | 0.32× | 0.23× | 0.40× | 0.59× |
| R2C, 2049, 1024 | 0.41× | 0.56× | 0.43× | 0.48× | 0.32× |
| C2R, 2049, 1024 | 0.41× | 0.56× | 0.43× | 0.48× | 0.32× |
| R2C, 4097, 2048 | 0.27× | 0.46× | 0.33× | 0.29× | 0.28× |
| C2R, 4097, 2048 | 0.25× | 0.44× | 0.31× | 0.26× | 0.27× |

The paired ratios show the 1025 forward real operation near RustFFT, FFTW and
Admiral on this machine. The 1443, 2049 and 4097 real calls are slower than
each reference in both directions. N=1442 and N=1452 real calls use complexify
plans, so their results do not measure the native real-prime passes.

## Artifacts

- [`m0-complete-paired-summary.csv`](../benches/results/m0-complete-paired-summary.csv): all standard and SHT-shaped paired cells, timing medians, ratios, spread and bootstrap intervals.
- [`m0-complete-paired-current-vs-rustfft-6.4.1.jsonl`](../benches/results/m0-complete-paired-current-vs-rustfft-6.4.1.jsonl), [`m0-complete-paired-current-vs-ducc-c1ed01b8-portable.jsonl`](../benches/results/m0-complete-paired-current-vs-ducc-c1ed01b8-portable.jsonl), [`m0-complete-paired-current-vs-ducc-c1ed01b8-native.jsonl`](../benches/results/m0-complete-paired-current-vs-ducc-c1ed01b8-native.jsonl), [`m0-complete-paired-current-vs-fftw-system-3.3.10.jsonl`](../benches/results/m0-complete-paired-current-vs-fftw-system-3.3.10.jsonl) and [`m0-complete-paired-current-vs-admiral-ce67bc00.jsonl`](../benches/results/m0-complete-paired-current-vs-admiral-ce67bc00.jsonl): six process pairs for each 41-length standard matrix.
- `m0-sht-exact-paired-current-vs-*.jsonl` and `m0-sht-paired-current-vs-*.jsonl`: exact SHT row counts and the FFTW/Admiral SHT process captures.
- [`m0-ducc-portable-output-comparison-aaadcb0.txt`](../benches/results/m0-ducc-portable-output-comparison-aaadcb0.txt), [`m0-ducc-native-output-comparison-aaadcb0.txt`](../benches/results/m0-ducc-native-output-comparison-aaadcb0.txt), [`m0-external-output-comparison-aaadcb0.txt`](../benches/results/m0-external-output-comparison-aaadcb0.txt), and the two `m0-control-output-comparison-*.txt` files: full output comparisons.
- `m0-*-build-info.txt`: DUCC, FFTW and Admiral build settings.
- [`paired_processes.py`](../benches/paired_processes.py), [`summarize_pairs.py`](../benches/summarize_pairs.py), [`external_fft.cpp`](../benches/reference/external_fft.cpp) and [`compare.py`](../benches/reference/compare.py): reusable collection, summary and output comparison tools.

The reported timings cover standalone one-dimensional FFT calls. They do not
measure complete RustSHT spherical harmonic transforms.
