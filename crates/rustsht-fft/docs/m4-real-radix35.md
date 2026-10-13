# Real radix-3 and radix-5 multipass measurements

`d27e611` adds scalar real radix-3 and radix-5 passes adapted from pinned
DUCC's `rfftp3` and `rfftp5`. `RealMultipassPlan` now accepts factor trees
containing only radices 2, 3, 4, and 5. Plans with other factors retain the
validated recursive path; DUCC's existing real complexify and Bluestein
selection conditions remain in place. The AVX2 radix-2/radix-4 interiors run
only when `ido` is even. The measured lengths below use scalar radix-3/5
passes and do not execute those vector kernels. The scalar radix-3/5 loops now
use contiguous input, output, and twiddle slices for each branch; the DUCC
equations, pass order, and normalization remain unchanged.

Odd transforms store both components of their last positive-frequency bin.
Only an even transform stores Nyquist as one real value. Inverse passes keep
the existing `MAX/(4*N)` eligibility check; spectra outside that bound or
with nonfinite values use the residual-corrected recursive path.

## Correctness

The crate tests compare f64 R2C and C2R against direct DFTs at lengths 75, 125,
225, and 1,920. f32 tests compare the short lengths with f64 DFTs after
rounding inputs to f32; the large mixed-radix length 1,920 (`2^7 × 3 × 5`) is
also checked in both directions. Across the f32 cases, the largest relative-L2
error was `1.49e-7`; all outputs were finite. The f32 metrics were:

| N | Operation | Maximum absolute error | RMS error | Relative L2 error | Scaled maximum error | Nonfinite outputs |
|---:|:---:|---:|---:|---:|---:|---:|
| 75 | R2C | 1.879e-6 | 4.806e-7 | 7.784e-8 | 5.074e-8 | 0 |
| 75 | C2R | 3.309e-8 | 9.886e-9 | 1.185e-7 | 3.309e-8 | 0 |
| 125 | R2C | 4.436e-6 | 9.798e-7 | 1.202e-7 | 9.328e-8 | 0 |
| 125 | C2R | 4.995e-8 | 8.228e-9 | 1.295e-7 | 4.995e-8 | 0 |
| 225 | R2C | 3.387e-6 | 1.115e-6 | 1.034e-7 | 3.224e-8 | 0 |
| 225 | C2R | 3.119e-8 | 5.156e-9 | 1.062e-7 | 3.119e-8 | 0 |
| 1,920 | R2C | 4.563e-5 | 3.862e-6 | 1.222e-7 | 4.892e-8 | 0 |
| 1,920 | C2R | 4.260e-8 | 2.497e-9 | 1.488e-7 | 4.260e-8 | 0 |

The pinned DUCC output comparison used revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Its maximum componentwise f64
absolute errors were:

| N | R2C | C2R |
|---:|---:|---:|
| 75 | 1.421e-14 | 2.776e-17 |
| 125 | 4.260e-15 | 2.776e-17 |
| 225 | 7.105e-15 | 1.583e-17 |

The DUCC output comparison used direct C++ plans built with g++ 15.2.0 and
`-O3 -DNDEBUG -march=native`. The Rust output used the optimized `bench`
profile on rustc 1.99.0 with the portable crate target. `dump-native MODE N`
and DUCC's `dump MODE N` used matching f64 input formulas.

## Benchmark method

The baseline is recursive source `e8179e0`; the multipass candidate is
`d27e611`. RustFFT uses the existing real adapter and RustFFT 6.4.1. Direct
DUCC uses the pinned reference. Both portable DUCC (`-O3 -DNDEBUG`) and native
DUCC (`-O3 -DNDEBUG -march=native`) are shown separately. Rust used rustc and
Cargo 1.99.0 (`b940084d7`, LLVM 23.1.1), `opt-level=3`, no custom `RUSTFLAGS`,
no LTO, and no `target-cpu` setting. RustFFT uses its default runtime SIMD
dispatch.

The host was an AMD Ryzen 9 5950X. Every process was pinned to logical CPU 30;
each transform used one thread. Six process samples were collected per
implementation. Implementation order rotated between process samples. Each
process warmed its plan and reported the median of seven timed samples. Plan
construction and workspace allocation used 20 repetitions. The load average
was 3.09 before and 3.21 after the run while the Legendre workload was active,
so these timing comparisons are provisional.

Inputs use the same deterministic real signal and Hermitian spectrum across
the RustFFT adapter, DUCC harness, and RustSHT-FFT public API. Timing cells are
medians of six process-level medians. The speed ratios are medians of the six
paired process ratios, so they need not equal the ratio of displayed timing
medians. R2C and C2R timings measure the complete public operation, including
the real packing and output conversion used by each library. The batch table
shows per-transform time and the separately measured total time for 32 rows.

## Single transform

Times are in microseconds per public operation.

| N | Operation | Recursive | Multipass | RustFFT | DUCC portable | DUCC native | Speedup vs recursive | Multipass / RustFFT / portable DUCC / native DUCC |
|---:|:---:|---:|---:|---:|---:|---:|---:|---:|
| 75 | R2C | 1.698 | 0.281 | 0.200 | 0.200 | 0.191 | 6.05× | 1.41× / 1.41× / 1.50× |
| 75 | C2R | 1.463 | 0.261 | 0.210 | 0.190 | 0.150 | 5.57× | 1.24× / 1.37× / 1.74× |
| 125 | R2C | 2.610 | 0.481 | 0.321 | 0.340 | 0.291 | 5.41× | 1.50× / 1.43× / 1.65× |
| 125 | C2R | 2.269 | 0.476 | 0.340 | 0.280 | 0.231 | 4.75× | 1.40× / 1.73× / 2.06× |
| 225 | R2C | 6.452 | 0.941 | 0.511 | 0.511 | 0.501 | 6.85× | 1.84× / 1.84× / 1.89× |
| 225 | C2R | 5.149 | 0.862 | 0.541 | 0.541 | 0.421 | 5.98× | 1.59× / 1.60× / 2.05× |

## Batch of 32 transforms

Each cell gives microseconds per transform / total microseconds for the full
32-row call. Speed ratios use the per-transform paired timings.

| N | Operation | Recursive | Multipass | RustFFT | DUCC portable | DUCC native | Speedup vs recursive | Multipass / RustFFT / portable DUCC / native DUCC |
|---:|:---:|---:|---:|---:|---:|---:|---:|---:|
| 75 | R2C | 1.671 / 53.49 | 0.256 / 8.21 | 0.185 / 5.92 | 0.181 / 5.78 | 0.170 / 5.42 | 6.52× | 1.39× / 1.42× / 1.52× |
| 75 | C2R | 1.439 / 46.04 | 0.234 / 7.49 | 0.191 / 6.12 | 0.152 / 4.86 | 0.127 / 4.07 | 6.16× | 1.23× / 1.55× / 1.85× |
| 125 | R2C | 2.589 / 82.85 | 0.457 / 14.63 | 0.308 / 9.87 | 0.315 / 10.06 | 0.270 / 8.63 | 5.66× | 1.49× / 1.45× / 1.70× |
| 125 | C2R | 2.248 / 71.92 | 0.425 / 13.59 | 0.323 / 10.33 | 0.251 / 8.04 | 0.210 / 6.71 | 5.29× | 1.32× / 1.70× / 2.03× |
| 225 | R2C | 6.478 / 207.29 | 0.916 / 29.32 | 0.498 / 15.94 | 0.507 / 16.23 | 0.500 / 15.99 | 7.09× | 1.83× / 1.80× / 1.83× |
| 225 | C2R | 5.131 / 164.18 | 0.837 / 26.78 | 0.525 / 16.81 | 0.507 / 16.24 | 0.413 / 13.21 | 6.14× | 1.59× / 1.65× / 2.03× |

## Plan construction and scratch

Times are medians in microseconds. Scratch bytes report reusable FFT workspace;
DUCC's count is its kernel scratch and excludes the benchmark adapter buffer.
RustFFT's count includes its real adapter's complex work buffer. Multipass
RustSHT-FFT retains both the recursive fallback buffers and the two real pass
buffers. The setup columns use only `mode=r2c`, `rows=1` records: six
process-level samples for each length and implementation. C2R and batch setup
records are excluded.

| N | Implementation | Plan construction | Workspace allocation | Scratch bytes |
|---:|:---|---:|---:|---:|
| 75 | Recursive RustSHT-FFT | 1.294 | 0.234 | 1,184 |
| 75 | Multipass RustSHT-FFT | 1.456 | 0.192 | 2,384 |
| 75 | RustFFT adapter | 2.020 | 0.042 | 2,400 |
| 75 | DUCC portable | 0.396 | 0.015 | 600 |
| 75 | DUCC native | 0.385 | 0.014 | 600 |
| 125 | Recursive RustSHT-FFT | 1.990 | 0.127 | 1,600 |
| 125 | Multipass RustSHT-FFT | 2.169 | 0.155 | 3,600 |
| 125 | RustFFT adapter | 3.100 | 0.050 | 4,000 |
| 125 | DUCC portable | 0.406 | 0.017 | 1,000 |
| 125 | DUCC native | 0.391 | 0.017 | 1,000 |
| 225 | Recursive RustSHT-FFT | 3.567 | 0.143 | 3,008 |
| 225 | Multipass RustSHT-FFT | 3.900 | 0.202 | 6,608 |
| 225 | RustFFT adapter | 5.035 | 0.073 | 7,200 |
| 225 | DUCC portable | 0.596 | 0.026 | 1,800 |
| 225 | DUCC native | 0.570 | 0.026 | 1,800 |

The new pass tree is 4.75–7.09× faster than the previous recursive execution
for these factors. It remains 1.23–1.84× slower than RustFFT and 1.37–2.06×
slower than DUCC across the measured calls. C2R is within the same order as
R2C for these fixed-radix lengths. This does not isolate residual-correction
costs in the earlier prime-size plans. The new path roughly doubles reusable
scratch relative to recursion because it still allocates the fallback and
multipass buffers together. Plan construction is 0.16–0.33 microseconds slower
than recursion at these sizes.

The paired measurements support using the DUCC-style pass organization for
radix 3 and 5. They do not close the scalar performance gap. The native Rust
benchmark does not expose separate pass or packing timings; its recorded
operation time is the whole public call. RustFFT and DUCC phase fields are
retained in the raw samples for comparison. Further scalar profiling and
workspace reduction remain open before broadening the pass path to generic
primes or beginning additional real SIMD work.

The ratio column gives multipass time divided by the reference time; values
above 1 indicate that multipass is slower. The speedup column gives recursive
time divided by multipass time.

The raw 360-row archive is
[`m4-real-radix35-process-samples.csv`](../benches/results/m4-real-radix35-process-samples.csv).
It records six process-level samples for each implementation, operation,
length, and row count, along with process order, setup, scratch, phase fields,
whole-call timings, and checksums. Recursive and multipass checksums match
exactly for every paired row. C++ checksum output is printed at nine
significant digits; the independent output comparison above uses full
precision.

The benchmark source hashes are `6a85531e042175af96e9bcdc07368c15908e46212b7d441d68c57cfb7b9b4cba`
for the baseline harness and
`bbbcb779effb7c1121f4ad08411eea0207c7ec38eccc75278f57d0fd46784457` for the
candidate harness. The Rust benchmark binary hashes are
`59d1fff792235d376b116f7a88241e80f9c7bfabaec51041f74247119f9b6275`
(recursive) and
`92875c84576b5d1d51be68fba7e4ee1382136dda624399dfb6f23d3a812b1875`
(multipass/RustFFT). The direct DUCC C++ harness source hash is
`cbe53d90e23232bf5abd1ce5fc9236fbe2ff40024effd2df1b90f8498d1dc1fa`; the
portable and native reference binary hashes are
`fe241e7efa57953567675cc67d73d519488923887f9c32d0456e1f5eb1f7e86d` and
`09fe220341903e95788d96983aeddb7652da5a1947ee01ae439b0322fdfce178`.

## Contiguous scalar pass loops

Commit `c804f3f` replaces repeated multidimensional index calculations in the
radix-3 and radix-5 pass loops with contiguous branch slices and stage-local
twiddle slices. The preceding scalar multipass implementation at `d27e611` is
the baseline. This change adds no SIMD code and preserves the pass equations.
The direct f64 DFT tests cover both directions at lengths 75, 125, 225, and
1,920. The f32 test compares both directions at 1,920 with f64 DFTs after
input rounding.

The A/B comparison used the same `rustfft.rs` source in both builds (source
SHA-256 `bbbcb779effb7c1121f4ad08411eea0207c7ec38eccc75278f57d0fd46784457`).
Rust 1.99.0 (`b940084d7`, LLVM 23.1.1) and Cargo 1.99.0 used the optimized
`bench` profile (`opt-level=3`), with no custom `RUSTFLAGS`, LTO, or target CPU
flag. Runtime AVX2 dispatch for radix-2/radix-4 was the same in both binaries.
The machine was an AMD Ryzen 9 5950X. Every process was pinned to logical CPU
30 and ran one execution thread. Six process samples interleaved baseline and
candidate order; odd samples ran the baseline first, and even samples ran the
candidate first. Each sample warmed once and timed seven executions. Plan
construction and workspace allocation each used 20 repetitions. The host
load average after collection was 3.13, 2.98, and 2.87. The Legendre workload
shared the host, so these timings remain provisional.

The benchmark used the public real operations with deterministic
`real_value(index, row)` samples and `spectrum_value(index, n, row)` inverse
spectra. A batch contains 32 sequential row transforms with one reused plan
and workspace. Time values below are per transform. Batch totals are shown in
parentheses. Paired latency reductions are the medians of
`1 - candidate / baseline` across the six process pairs. They need not equal
the latency reduction calculated from the displayed medians; these percentages
are not speedup percentages, which would use `baseline / candidate - 1`.

| N | Operation | Single baseline → candidate, µs | Paired latency reduction | Batch 32 baseline → candidate per transform, µs (batch total, µs) | Paired latency reduction |
|---:|:---:|---:|---:|---:|---:|
| 225 | R2C | 0.932 → 0.667 | 28.9% | 0.914 → 0.635 (29.255 → 20.329) | 30.5% |
| 225 | C2R | 0.871 → 0.681 | 21.9% | 0.842 → 0.659 (26.941 → 21.085) | 21.6% |
| 1,920 | R2C | 7.589 → 6.547 | 14.2% | 7.262 → 6.183 (232.373 → 197.858) | 14.8% |
| 1,920 | C2R | 6.332 → 5.696 | 10.3% | 6.378 → 5.725 (204.110 → 183.215) | 10.5% |

The median plan construction/workspace allocation times were 4.667/0.565 µs
for the baseline and 5.246/0.673 µs for the candidate at N=225. At N=1,920
they were 35.099/1.782 µs and 33.581/1.935 µs. Each setup summary uses only
`mode=r2c`, `rows=1` records: six process samples per implementation and
length. Reusable workspace size was unchanged at 6,608 bytes for N=225 and
56,816 bytes for N=1,920.

The baseline and candidate checksums match for every operation and length.
They cover the first output value; full-array direct-reference tests provide
the correctness check. In release f64 disassembly, the
`execute_forward_pass` symbol fell from 10,255 to 9,249 bytes. Static
instruction counts for that whole symbol fell from 25 to 19 `imul`
instructions and from 2,160 to 1,954 total instructions. The symbol still
contains pass dispatch and cold bounds-check paths, so these counts do not
attribute a runtime share to the radix-3/5 inner loops. The contiguous-slice
change reduced median public latency by 10–31% in the four measured cases,
without reducing workspace size. Using the earlier N=225 control medians in
the tables above, the new candidate is 1.30/1.30/1.33× slower than RustFFT,
portable DUCC, and native DUCC for R2C, and 1.26/1.26/1.62× slower for C2R.
Those are ratios of independently measured medians. No RustFFT or DUCC control
at N=1,920 was collected in this focused A/B run. The results support keeping
the scalar loop change before further real SIMD work.

Raw records for all 96 process samples are in
[`m4-real-radix35-slices-process-samples.csv`](../benches/results/m4-real-radix35-slices-process-samples.csv).
The baseline and candidate benchmark binary SHA-256 values are
`92875c84576b5d1d51be68fba7e4ee1382136dda624399dfb6f23d3a812b1875` and
`691222ff38f890c02e9436368ca0291e18a07b6db07c57780ef3435a733eef26`.
