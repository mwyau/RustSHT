# M26: odd-`ido` radix-11 AVX2 tail

The isolated radix-11 tail kernel improved all six targeted C2C cells relative
to M25. Median speedups ranged from 1.028× to 1.068×, and every exact paired
95% interval was above parity. Across the 28 non-target cells, the largest
median slowdown was 2.42%. Workspace sizes matched in all 34 cells.

## Implementation

The f64 AVX2 radix-11 pass handles the even prefix of `ido` in its existing
kernel. For odd `ido >= 3`, an out-of-line helper handles the final inner
index by packing corresponding values from two adjacent outer groups into
the AVX2 lanes. If `l1` is odd, the last outer group remains scalar. The
existing N=682 `ido=31` kernel, inverse safety fallback, f32 path and CPUs
without AVX2 retain their previous routes. The change adds no plan or
workspace storage.

The helper uses the radix-eleven equations from DUCC `fft1d_impl.h` at source
revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; its source retains the
DUCC copyright and BSD-3-Clause notice. The benchmarked production patch SHA-256
is `a33e4fbf0c830b93540c324ec2f27e85c4a8b6e7ad901d4fe7c4d4c3100ca087`.
The final tested source patch, which also includes two test-coverage additions,
is preserved with the result data; its SHA-256 is
`89c8352d07daafe09108173cb6d621578707c42f76b1a00a3314d60c28b84796`.

## Measurement method

The reference executable uses M25 source commit `e6b0101` and has SHA-256
`ec3f0c4837c5b8212dc65377d90057fdd8bb740d6747b0ad8279fe88121479cd`. The
candidate executable has SHA-256
`76f2404f5c454e32985520030bd12626c8f1979f96f12a2633515b0a3e9ca71b`. Both
used Rust 1.99.0 stable, the optimized bench profile, portable x86_64 target
settings and runtime SIMD dispatch.

Six alternating process pairs compared 34 exact C2C and real SHT FFT-target
operation/length cells, including one-transform and scalar/spin-1 row batches.
Each process warmed and timed seven samples per operation with one transform
thread. The process ran on an AMD Ryzen 9 5950X pinned to logical CPU 30. All
12 processes exited successfully. No separate SHT or FFT test or benchmark
process overlapped the capture. The initial one-minute load average was 2.60
and ranged from 2.60 to 4.04 during the capture.

Speedup is the median of six M25/candidate process-pair ratios. Each interval
is the exact 2.5th–97.5th percentile over 46,656 bootstrap resamples of those
six ratios. The table reports median nanoseconds per transform for the
candidate and reference.

## Target results

| Operation | Rows | Speedup (95% interval) | Candidate ns | M25 ns |
|---|---:|---:|---:|---:|
| C2C forward N=726 | 361 | 1.068× [1.047, 1.074] | 2,729.572 | 2,906.529 |
| C2C inverse N=726 | 361 | 1.032× [1.025, 1.039] | 3,795.659 | 3,925.183 |
| C2C forward N=1,452 | 721 | 1.065× [1.042, 1.071] | 5,696.083 | 6,070.015 |
| C2C inverse N=1,452 | 721 | 1.033× [1.017, 1.039] | 7,850.930 | 8,106.488 |
| C2C forward N=2,904 | 721 | 1.049× [1.028, 1.061] | 12,303.069 | 12,952.569 |
| C2C inverse N=2,904 | 721 | 1.028× [1.015, 1.037] | 16,578.732 | 17,077.350 |

The median workspace size is unchanged in all 34 cells, including 11,968
bytes at N=726, 23,584 bytes at N=1,452 and 46,816 bytes at N=2,904. Fourteen
of the 28 non-target cells had median ratios below parity. The largest
non-target slowdown was 2.42% for C2C inverse N=150 (0.976× [0.973, 0.982]);
no non-target median slowdown exceeded 2.5%.

## Numerical validation and diagnostics

The radix-11 pass test compares both directions with direct DFT sums and
checks untouched output regions for even and odd `ido`, odd `l1`, and the
scalar `(l1, ido) = (1, 3)` case. The SHT meridional transform test adds N=363
to its independent DFT cases. Route tests check tail selection, the scalar
odd-group tail, unsafe inverse fallback and the f32 route. The final debug and
release workspace suites each passed 106 unit and integration tests.

The full nested FMA accumulation trial slowed N=1,442 C2C by about 15–18%.
Splitting cosine and sine accumulators measured near parity, with intervals
crossing parity; neither FMA candidate was retained. Two earlier tail-kernel
variants showed code-layout slowdowns in unrelated controls. The selected
out-of-line helper isolates the tail code and keeps `SimdKernels` unchanged.
Those candidates remain diagnostic and are not combined with the final
measurement.

## Reproduction and supporting data

`cargo fmt --all -- --check`, all-target/all-feature Clippy with warnings
denied, and the debug and release all-feature workspace tests passed. The
release suite used `/dev/shm` for its target and temporary directories because
`/tmp` was full. A non-AVX2 hardware run and a new cross-library performance
campaign were not part of this change.

The final configuration, raw process capture, 34-cell summary and paired
bootstrap summarizer are in
[`the isolated result directory`](../benches/results/m26-radix11-odd-tail/).
The tested source patch and its SHA-256 are recorded there. Earlier in-module
and dispatch-refined tail variants, and both rejected FMA variants, remain in
their result directories with configurations, raw captures, source patches
and available summaries for comparison.
