# Cross-library FFT comparison after M5

Date: 2026-10-15. RustSHT-FFT source: `9cbd354f2e3e2f1debef023cca68b86900904cfe`. This report measures the accepted M5 source before the M6 workspace change. It
updates the M0 library comparison, whose candidate source was
`aaadcb0aba9299049cf68eeca0f0dcd851c799b2`.

## Methods

The standard matrix covers 41 lengths, C2C forward and inverse, R2C and C2R,
and reused-plan row counts of 1 and 32. The two largest lengths use four rows
for batches. Six alternating process pairs ran on logical one CPU core. Each process
reported seven warmed timing samples per operation. The summary reports the
median of the six paired reference-to-RustSHT-FFT ratios, the observed range,
and a 95% bootstrap interval for the median. The exact SHT matrix uses the RustSHT meridional and longitude row counts:
complex transforms at N=146, 150, 300, 256, 512, 722, 726, 1024, 1442, 1452
and 2904; real transforms at N=257, 723, 1025, 1443, 2049 and 4097. It also
uses six alternating process pairs. Each backend uses one execution thread. The timing includes the public real packing, normalization and output
conversion performed by each harness. These rows are sequential; this
campaign does not measure external worker parallelism. RustSHT-FFT and RustFFT use Rust 1.99.0 with the release bench profile and a
portable compiler target. RustSHT-FFT uses runtime AVX2 dispatch where the
plan reaches an AVX2 kernel. RustFFT is version 6.4.1. DUCC uses revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, g++ 15.2.0, and separate portable
(`-O3 -DNDEBUG`) and native (`-O3 -DNDEBUG -march=native`) builds. Admiral uses
revision `ce67bc00ff9e6de96bf5b75f7a789581105ddb58`, CMake Release,
`ADM_TARGET_ARCH=native`, fast math and threads disabled, and estimate
planning. FFTW uses the installed 3.3.10 package with `FFTW_ESTIMATE` and
`FFTW_PRESERVE_INPUT`; the system package build flags are unavailable. The pinned FFTW checkout at
`40e620ec4441d0502e55d73744ef5d29a340a288` built a shared library, but the
benchmark executable could not link because the library lacked generated
solver-table symbols. The FFTW values below are therefore system 3.3.10
results, not measurements from the pinned source revision. The host is an x86-64 CPU running Linux x86-64. Rust reports LLVM
23.1.1. Load-average telemetry was retained only in local diagnostics. The
paired ratios and their intervals show the observed spread. Absolute times
should be treated as host-specific measurements.

## Exact SHT row counts

Each ratio is `reference time / RustSHT-FFT time`; values below 1 indicate that
RustSHT-FFT was slower. The full paired ranges and bootstrap intervals are in
[`m5-current-head-sht-complete-paired-summary.csv`](../benches/results/m5-current-head-sht-complete-paired-summary.csv).

### Complex transforms

| N | Rows | Operation | RustSHT-FFT (µs) | RustFFT | DUCC portable | DUCC native | FFTW | Admiral |
|---:|---:|---|---:|---:|---:|---:|---:|---:|
| 146 | 73 | Forward | 3.62 | 0.20× | 0.64× | 0.62× | 0.39× | 0.56× |
| 150 | 73 | Forward | 0.91 | 0.30× | 0.48× | 0.26× | 0.34× | 0.50× |
| 256 | 128 | Forward | 0.43 | 0.66× | 1.49× | 1.34× | 0.71× | 1.11× |
| 300 | 73 | Forward | 1.79 | 0.30× | 0.53× | 0.28× | 0.37× | 0.33× |
| 512 | 256 | Forward | 0.88 | 0.80× | 1.64× | 0.98× | 0.72× | 0.88× |
| 722 | 361 | Forward | 11.16 | 0.61× | 0.45× | 0.33× | 0.60× | 0.67× |
| 726 | 361 | Forward | 6.46 | 0.24× | 0.55× | 0.39× | 0.38× | 0.34× |
| 1024 | 512 | Forward | 2.23 | 0.70× | 1.48× | 0.79× | 0.68× | 0.77× |
| 1442 | 721 | Forward | 55.50 | 0.23× | 0.44× | 0.34× | 0.45× | 0.36× |
| 1452 | 721 | Forward | 13.12 | 0.25× | 0.58× | 0.27× | 0.44× | 0.27× |
| 2904 | 721 | Forward | 27.01 | 0.27× | 0.61× | 0.29× | 0.54× | 0.30× |
| 146 | 73 | Inverse | 4.26 | 0.18× | 0.53× | 0.50× | 0.39× | 0.48× |
| 150 | 73 | Inverse | 1.10 | 0.28× | 0.45× | 0.26× | 0.31× | 0.42× |
| 256 | 128 | Inverse | 0.78 | 0.43× | 0.93× | 0.75× | 0.47× | 0.61× |
| 300 | 73 | Inverse | 2.21 | 0.27× | 0.48× | 0.26× | 0.33× | 0.27× |
| 512 | 256 | Inverse | 1.57 | 0.51× | 1.05× | 0.60× | 0.49× | 0.51× |
| 722 | 361 | Inverse | 13.84 | 0.46× | 0.37× | 0.27× | 0.50× | 0.56× |
| 726 | 361 | Inverse | 7.38 | 0.24× | 0.53× | 0.35× | 0.35× | 0.30× |
| 1024 | 512 | Inverse | 3.81 | 0.46× | 1.02× | 0.52× | 0.46× | 0.45× |
| 1442 | 721 | Inverse | 60.60 | 0.21× | 0.41× | 0.27× | 0.40× | 0.33× |
| 1452 | 721 | Inverse | 14.95 | 0.25× | 0.56× | 0.24× | 0.42× | 0.24× |
| 2904 | 721 | Inverse | 30.61 | 0.26× | 0.56× | 0.27× | 0.51× | 0.27× |

### Real transforms

| N | Rows | Operation | RustSHT-FFT (µs) | RustFFT | DUCC portable | DUCC native | FFTW | Admiral |
|---:|---:|---|---:|---:|---:|---:|---:|---:|
| 257 | 128 | R2C | 2.17 | 0.58× | 2.24× | 1.33× | 1.32× | 0.75× |
| 723 | 361 | R2C | 11.11 | 0.43× | 1.48× | 1.07× | 0.80× | 0.96× |
| 1025 | 512 | R2C | 6.01 | 1.02× | 0.82× | 0.62× | 1.06× | 0.96× |
| 1443 | 721 | R2C | 18.25 | 0.77× | 0.36× | 0.30× | 0.49× | 0.68× |
| 2049 | 1024 | R2C | 90.29 | 0.41× | 0.55× | 0.43× | 0.47× | 0.31× |
| 4097 | 2048 | R2C | 227.55 | 0.27× | 0.46× | 0.33× | 0.28× | 0.28× |
| 257 | 128 | C2R | 2.40 | 0.54× | 2.00× | 1.20× | 1.21× | 0.69× |
| 723 | 361 | C2R | 12.21 | 0.39× | 1.37× | 0.96× | 0.74× | 0.86× |
| 1025 | 512 | C2R | 7.30 | 0.86× | 0.64× | 0.46× | 0.88× | 0.82× |
| 1443 | 721 | C2R | 22.19 | 0.64× | 0.32× | 0.24× | 0.40× | 0.59× |
| 2049 | 1024 | C2R | 91.89 | 0.40× | 0.55× | 0.43× | 0.46× | 0.31× |
| 4097 | 2048 | C2R | 239.57 | 0.25× | 0.44× | 0.31× | 0.25× | 0.27× |

The largest remaining gaps in these row counts are complex N=1442, real
N=1443, 2049 and 4097. The route trace identifies direct radix-103 work at
C2C N=1442, native real-prime passes at N=1443, and Bluestein at real N=2049
and 4097. These measurements do not establish a complete spherical harmonic
transform speedup.

## Full matrix and numerical comparisons

The standard comparison contains 328 matched cells per reference, with
medians, paired ranges and 95% bootstrap intervals in
[`m5-current-head-complete-paired-summary.csv`](../benches/results/m5-current-head-complete-paired-summary.csv). The five raw paired captures are
[`RustFFT 6.4.1`](../benches/results/m5-current-head-paired-vs-rustfft-6.4.1.jsonl),
[`DUCC portable`](../benches/results/m5-current-head-paired-vs-ducc-c1ed01b8-portable.jsonl),
[`DUCC native`](../benches/results/m5-current-head-paired-vs-ducc-c1ed01b8-native.jsonl),
[`system FFTW 3.3.10`](../benches/results/m5-current-head-paired-vs-fftw-system-3.3.10.jsonl)
and [`Admiral`](../benches/results/m5-current-head-paired-vs-admiral-ce67bc00.jsonl). Each capture records command lines, executable hashes, process order, affinity
and load averages. The current candidate agreed with pinned DUCC over 232,604 output records at
`atol=rtol=2e-11`. Maximum absolute/scaled errors were `1.819e-12`/`9.113e-13`
for portable DUCC and `1.819e-12`/`1.076e-12` for native DUCC. The same
232,604 records agreed with system FFTW at `3.638e-12`/`9.353e-13` and Admiral
at `5.457e-12`/`1.549e-12`. The comparison outputs are in
[`m5-current-head-output-comparison-portable.txt`](../benches/results/m5-current-head-output-comparison-portable.txt)
and [`m5-current-head-output-comparison-native.txt`](../benches/results/m5-current-head-output-comparison-native.txt). The route and memory diagnostic is
[`m5-current-head-route-diagnostics.txt`](../benches/results/m5-current-head-route-diagnostics.txt). It records plan stages, radix, `l1`, `ido`, prime algorithm, convolution
length, route entry, AVX2 eligibility and the subnormal Rader fallback. For
example, real N=257 selects Rader with convolution length 256 and retains
Bluestein length 525 for unsafe inputs; real N=723 selects Rader-241 with
convolution length 240 and retains a length-512 fallback. The diagnostic reports workspace payloads of 46,032 bytes at real N=257 and
57,376 bytes at real N=723. It counts initialized workspace elements, not
allocator metadata or peak resident memory. The external libraries do not
expose comparable caller-owned workspace payloads. The sequential matrix
measures no 2-, 4- or 8-worker execution throughput.
