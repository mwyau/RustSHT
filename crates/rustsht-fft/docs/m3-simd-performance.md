# Reachable C2C AVX2 kernels

Date: 2026-10-15. The baseline is M2 commit
`e6cb87c7965130c47e8d06db39978431344c956c`. The candidate adds a vector
prefix with a scalar tail to the reachable C2C AVX2 radix-2, radix-4, and
radix-8 passes. It also adds an f64 radix-4 `ido == 1` kernel that processes
two adjacent outer groups. The accepted f64 radix-8 `ido == 1` kernel keeps
its existing implementation.

## Methods

The benchmark uses Rust 1.99.0 stable, the Cargo bench profile, portable
compiler flags, and one execution thread. The host is an AMD Ryzen 9 5950X;
all paired processes ran on logical CPU 30. The initial load average was
0.55, 0.82, and 1.15. Each process reports the median of seven warmed samples.
Six process pairs alternate baseline and candidate order. Ratios are
`baseline_time / candidate_time`; values above 1 favor the candidate. The
95% intervals are percentile bootstrap intervals over the six paired ratios.

The C2C comparison includes lengths 150, 300, 726, 1452, and 2904 at one row,
32 rows, and the SHT row count for each target. Nested candidates 240, 484, and
1372 use one and 32 rows. Controls 256, 512, and 1024 use one row, 32 rows,
and their SHT row counts. The real wrapper controls use R2C/C2R at 1452 and
1026 with one row, 32 rows, and their SHT row counts. Each operation reuses a
plan and workspace. The harness measures plan and workspace setup separately.

The candidate and baseline workspace byte counts are equal for every paired
C2C cell. The SIMD change adds no plan tables or scratch buffers. Real FFT
wrappers call the scalar C2C path, so their measurements serve as regression
controls for M3.

## Dispatch and tests

The route trace records the AVX2 first-pass prefix for all five target lengths
in both directions. It records the radix-8 out-of-place prefix at N=240 with
`l1=2`, `ido=15`, a 14-element vector prefix, and one scalar tail. N=256
dispatches the new radix-4 `ido == 1` kernel; N=512 and N=1024 dispatch the
existing radix-8 kernel. The trace is in
[`m3-simd-route-diagnostics.txt`](../benches/results/m3-simd-route-diagnostics.txt).

Tests compare complete f32 and f64 transforms against direct DFTs, including
unaligned slices, both directions, odd `ido`, vector tails, and odd/even
`l1`. The inverse safety test uses finite values near the representable limit
and confirms that guarded normalization takes the scalar fallback without
entering the AVX2 route. A separate f32 dispatcher test confirms that
`ido=3`, below the four-lane contract, uses the scalar path. The real N=1452
wrapper has f32/f64 direct-DFT tests; it does not exercise the new C2C SIMD
route.

The read-only Sol High review found no production indexing, twiddle, group
layout, or inverse-scaling defect. It identified four follow-ups: the
unaligned test's first offset depended on allocator alignment; the f32 helper
test called an unsafe kernel below its declared minimum `ido`; real wrapper
timings needed to be labeled scalar controls; and the N=512 batch regression
needed the final six-pair run. The offset is now selected dynamically, the
helper test starts at `ido=4`, and the dispatcher test covers the below-lane
fallback. The odd-`l1` low-level test checks that the final group remains for
the scalar caller.

## M2 comparison

The table reports the SHT-shaped row count for each required target, 32 rows
for nested candidates, and SHT-shaped rows for controls. Times are median
per-transform times in microseconds. The ratio and interval use the six
paired process ratios.

| N | Class | Rows | Forward M2 → M3 (µs) | Ratio, 95% interval | Inverse M2 → M3 (µs) | Ratio, 95% interval |
|---:|---|---:|---:|---:|---:|---:|
| 150 | Target | 73 | 0.994 → 0.872 | 1.140 [1.133, 1.144] | 1.229 → 1.096 | 1.119 [1.118, 1.129] |
| 300 | Target | 73 | 2.051 → 1.725 | 1.189 [1.180, 1.194] | 2.514 → 2.178 | 1.153 [1.135, 1.157] |
| 726 | Target | 361 | 7.067 → 6.398 | 1.110 [1.101, 1.116] | 8.081 → 7.445 | 1.086 [1.073, 1.110] |
| 1452 | Target | 721 | 14.872 → 13.006 | 1.143 [1.141, 1.149] | 16.845 → 15.063 | 1.114 [1.110, 1.124] |
| 2904 | Target | 721 | 31.483 → 26.833 | 1.175 [1.154, 1.184] | 35.275 → 30.912 | 1.144 [1.130, 1.148] |
| 240 | Nested | 32 | 1.426 → 1.076 | 1.323 [1.315, 1.344] | 1.805 → 1.464 | 1.233 [1.215, 1.240] |
| 484 | Nested | 32 | 4.246 → 3.621 | 1.180 [1.158, 1.198] | 4.878 → 4.274 | 1.147 [1.132, 1.169] |
| 1372 | Nested | 32 | 11.302 → 9.914 | 1.141 [1.124, 1.148] | 13.083 → 11.457 | 1.139 [1.125, 1.150] |
| 256 | Control | 128 | 0.740 → 0.429 | 1.735 [1.702, 1.747] | 1.092 → 0.773 | 1.413 [1.394, 1.433] |
| 512 | Control | 256 | 0.894 → 0.890 | 1.008 [0.973, 1.016] | 1.553 → 1.582 | 0.982 [0.977, 0.987] |
| 1024 | Control | 512 | 2.296 → 2.221 | 1.034 [0.992, 1.059] | 3.680 → 3.707 | 0.993 [0.989, 0.998] |

The largest measured target gain is 18.9% for N=300 forward. The smallest
required-target gain is 8.6% for N=726 inverse. At N=512 with 32 rows, the
median candidate time is 4.6% slower forward and 3.6% slower inverse. These
medians fall inside the predefined 5% control limit. At the SHT row count,
N=512 forward is 0.8% faster and inverse is 1.8% slower. N=1024 inverse is
less than 1% slower at the SHT row count.

The N=1452 and N=1026 real wrapper controls varied by at most 3.3% in either
direction. These changes do not measure the new SIMD route. Full per-row
measurements, paired ratios, ranges, and intervals are in
[`m3-simd-six-pairs-summary.csv`](../benches/results/m3-simd-six-pairs-summary.csv)
and [`m3-simd-six-pairs.jsonl`](../benches/results/m3-simd-six-pairs.jsonl).

## Cross-library comparison

Each comparison used six alternating process pairs and the same SHT row
counts. Ratios are `reference_time / candidate_time`; values below 1 mean the
reference completed the call faster. RustFFT 6.4.1 uses the crate's Rust
benchmark harness. DUCC uses revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`; portable mode uses g++ 15.2.0,
C++20, `-O3 -DNDEBUG -pthread`, with vectorization disabled. Native DUCC adds
`-march=native` and enables vectorization. FFTW is the system 3.3.10 library
with the g++ 15.2.0 harness at `-O3 -DNDEBUG`, `FFTW_ESTIMATE`, and
`FFTW_PRESERVE_INPUT`; package build flags are unavailable. Admiral uses
revision `ce67bc00ff9e6de96bf5b75f7a789581105ddb58`, CMake Release,
`ADM_TARGET_ARCH=native`, fast math and threads disabled, and estimate
planning. The harnesses use one execution thread. Full build details are in
[`m0-current-head-baseline.md`](m0-current-head-baseline.md).

| N | Rows | Direction | RustFFT | DUCC portable | DUCC native | FFTW | Admiral |
|---:|---:|---|---:|---:|---:|---:|---:|
| 150 | 73 | Forward | 0.309 | 0.505 | 0.268 | 0.353 | 0.516 |
| 150 | 73 | Inverse | 0.279 | 0.448 | 0.256 | 0.319 | 0.417 |
| 300 | 73 | Forward | 0.312 | 0.545 | 0.282 | 0.383 | 0.338 |
| 300 | 73 | Inverse | 0.270 | 0.481 | 0.267 | 0.337 | 0.269 |
| 726 | 361 | Forward | 0.247 | 0.553 | 0.397 | 0.378 | 0.345 |
| 726 | 361 | Inverse | 0.234 | 0.527 | 0.357 | 0.349 | 0.294 |
| 1452 | 721 | Forward | 0.256 | 0.590 | 0.254 | 0.449 | 0.271 |
| 1452 | 721 | Inverse | 0.246 | 0.558 | 0.241 | 0.416 | 0.237 |
| 2904 | 721 | Forward | 0.268 | 0.625 | 0.288 | 0.566 | 0.309 |
| 2904 | 721 | Inverse | 0.253 | 0.559 | 0.273 | 0.505 | 0.265 |

The references are faster at every required target length and direction in
this table. DUCC portable is faster by factors of 1.6–2.2, RustFFT by
3.2–4.3, native DUCC by 2.5–4.2, FFTW by 1.8–3.1, and Admiral by 1.9–4.2.
The radix SIMD change reduces source-relative latency but does not close
these cross-library gaps. The tables cover standalone FFT calls, not
complete spherical harmonic transforms.

The paired captures and summaries for each reference are in
[`benches/results`](../benches/results/), with filenames beginning
`m3-simd-vs-`. The candidate matrix runner is
[`benches/m3_native_matrix.py`](../benches/m3_native_matrix.py).
