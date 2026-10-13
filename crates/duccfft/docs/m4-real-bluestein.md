# Real Bluestein terminal pass

## Method

The real multipass executor uses the existing complex Bluestein plan when a
large odd radix is the final pass (`ido == 1`). The forward pass gathers one
real sequence, runs its complex Bluestein transform, and writes the odd-length
packed half-spectrum. The inverse pass reconstructs the Hermitian complex
spectrum, runs the inverse complex plan, and applies the real transform's
normalization at the terminal pass. Plans, chirps, convolution buffers, and
nested FFT workspaces are reused. The planner selects this path only when the real plan uses the real multipass
executor. The conditional complexify path and inputs that fail the finite
magnitude guards continue through their existing executors. This follows the
`rfft_multipass`, `rfftpass::make_pass`, and `rfftpblue` structure in DUCC
revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The supported terminal
radix condition is `radix >= 135 && ido == 1`. The focused cases are `N=257` with radix 257 and `N=2049` with factors 3 and
683. At N=2049, forward execution applies Bluestein to radix 683 before the
radix-3 pass; inverse execution applies radix 3 before the terminal
Bluestein pass.

## Paired execution timings

Each process used one warm-up and seven timed calls. The table reports medians
from three process-level runs per implementation. The baseline is recursive
DUCCFFT at `a05b2b4860089c67da956ae655790118ac92ea2e`; the candidate is the
terminal Bluestein multipass implementation at
`6ae846d2d90b652d090a3b40dc90d30dd3ea43fc`. Rows 1 and 32 are separate public
calls with a reused plan and workspace; the row-32 value is per transform. Processes were pinned to logical one CPU core on an x86-64 CPU. The Legendre
workload was active during the measurements, so absolute times are
provisional.

| N | Operation | Rows | Recursive DUCCFFT (µs) | Bluestein multipass (µs) | Speedup |
|---:|---|---:|---:|---:|---:|
| 257 | R2C | 1 | 13.365 | 12.393 | 1.08× |
| 257 | R2C | 32 | 13.017 | 11.739 | 1.11× |
| 257 | C2R | 1 | 13.375 | 11.932 | 1.12× |
| 257 | C2R | 32 | 13.460 | 11.943 | 1.13× |
| 2049 | R2C | 1 | 118.502 | 97.291 | 1.22× |
| 2049 | R2C | 32 | 120.149 | 98.495 | 1.22× |
| 2049 | C2R | 1 | 116.178 | 98.604 | 1.18× |
| 2049 | C2R | 32 | 117.558 | 99.411 | 1.18× |

The raw paired samples, checksums, source commits, binary hashes, and load
readings are in
[`m4-real-bluestein-process-samples.csv`](../benches/results/m4-real-bluestein-process-samples.csv). The largest candidate-to-baseline checksum difference was `1.78e-15`.

## Cross-library controls

RustFFT and DUCC controls are from one process per build; each process reports
the median of seven timed calls. The DUCCFFT column reports the median across
three process-level samples from the paired measurements above. RustFFT uses
the existing full-complex adapter for odd lengths. DUCC uses `pocketfft_r`
and the public half-spectrum packing adapter. The table reports
whole-operation time per transform, including the adapters. The controls are
screening measurements rather than repeated process-level comparisons.

| N | Operation | Rows | DUCCFFT (µs) | RustFFT adapter (µs) | DUCC portable (µs) | DUCC native (µs) |
|---:|---|---:|---:|---:|---:|---:|
| 257 | R2C | 1 | 12.393 | 1.223 | 4.759 | 2.826 |
| 257 | R2C | 32 | 11.739 | 1.228 | 4.749 | 2.809 |
| 257 | C2R | 1 | 11.932 | 1.243 | 4.839 | 2.886 |
| 257 | C2R | 32 | 11.943 | 1.220 | 4.775 | 2.840 |
| 2049 | R2C | 1 | 97.291 | 34.775 | 48.190 | 36.539 |
| 2049 | R2C | 32 | 98.495 | 35.269 | 48.787 | 37.660 |
| 2049 | C2R | 1 | 98.604 | 34.996 | 49.263 | 36.698 |
| 2049 | C2R | 32 | 99.411 | 35.417 | 49.265 | 36.799 |

The candidate remains about 10× slower than RustFFT for N=257 and 2.8× slower
for N=2049. At N=2049 it is about 2.0× slower than portable DUCC and 2.6–2.7×
slower than native-target DUCC. The terminal pass reduces the recursive
DUCCFFT time but does not close these gaps.

## Plan and workspace costs

Plan construction and `workspace()` allocation were measured 20 times per
process. Values below are medians across 12 records per length and build
(three process runs, two row counts, and R2C/C2R). Scratch counts initialized
reusable workspace bytes and exclude user buffers.

| N | Executor | Plan construction (µs) | Workspace allocation (µs) | Scratch bytes |
|---:|---|---:|---:|---:|
| 257 | Recursive | 20.597 | 0.783 | 33,472 |
| 257 | Bluestein multipass | 22.135 | 0.813 | 37,584 |
| 2049 | Recursive | 74.271 | 1.716 | 104,256 |
| 2049 | Bluestein multipass | 76.108 | 6.067 | 137,040 |

At N=2049 the new workspace uses 32,784 additional bytes and takes about
4.35 µs longer to allocate. The execution measurements reuse this workspace.

## Numerical checks and build configuration

Independent direct-DFT tests cover f64 R2C and C2R at N=257 and N=2049. The
large-length f32 tests include both lengths and compare against f64 references
after f32 input rounding. Existing high-amplitude Bluestein tests exercise the
magnitude-guard fallback; subnormal tests exercise the eligible multipass
path. A separate output comparison with pinned DUCC gave these maximum
complex-magnitude errors. The scaled R2C error divides the error magnitude by
`max(|DUCC output|, 1)`:

| N | R2C maximum absolute error | R2C maximum scaled error | C2R maximum absolute error |
|---:|---:|---:|---:|
| 257 | `1.61e-14` | `1.61e-14` | `2.78e-17` |
| 2049 | `1.17e-13` | `8.89e-14` | `2.78e-17` |

The paired Rust builds used Rust 1.99.0, Cargo's `bench` profile
(`opt-level=3`), no custom `RUSTFLAGS`, no LTO, and no target CPU setting. RustFFT was 6.4.1 with runtime dispatch. The baseline Rust binary SHA256 is
`4012f24539ce8aaa89182ba38c1bdf2c1da16513eef83b3deb0c4c880068b7b4`; the
candidate binary SHA256 is
`f9e8a5a9c2d74802c20f4c7deb464c01021df5839a9effec926dd49159c73d91`. The DUCC controls use revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`,
g++ 15.2.0, and the scalar planner. The portable build used
`-std=c++20 -O3 -DNDEBUG -pthread`; the native build added `-march=native`. The portable and native binary SHA256 values are
`be83af8bdadd4e53eefdd5ae5c3b14495bb52da5d7339d4ec04abb6c62974689` and
`150fffeaec352b4ea3e7ae76b2ba2ecf17cc49598d345a195f04e8ae7910e245`,
respectively. DUCC vectorization was disabled in both builds. `cargo test --all-features`, all-target/all-feature Clippy with `-D warnings`,
and the crate formatting check passed at the candidate commit. A focused
read-only review of the terminal pass and its benchmark evidence was requested
from the Sol High reviewer; its conclusion is pending.
