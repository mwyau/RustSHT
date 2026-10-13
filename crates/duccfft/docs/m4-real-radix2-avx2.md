# Real inverse radix-two AVX2 measurements

The real multipass inverse uses the radix-two pass for powers of two with an
odd exponent. For lengths 128, 512, and 2,048, the first inverse pass has
`ido` values 64, 256, and 1,024. Its interior loop reads contiguous packed
branches but previously computed each butterfly with scalar loads, repeated
index calculations, and scalar twiddle multiplication.

## Kernel

The f64 inverse radix-two pass now groups input and output branches into
contiguous slices. When the plan detects AVX2 at runtime and `ido >= 16`, the
interior kernel processes two adjacent complex butterflies per 256-bit vector. It reverses the mirrored input pairs, evaluates the radix-two sum and
difference, applies two twiddles, and stores both output branches. The scalar
loop handles the remaining butterfly. Other targets and f32 plans use the
scalar path. The plan stores one optional function pointer and allocates no
additional workspace. The pass equations follow the pinned DUCC revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. A kernel test compares the vector
loop and tail boundary with the scalar equations for `ido` 4 through 64. The
public f64 real-transform test compares R2C and C2R at lengths 128, 512, and
2,048 with independent direct DFTs. The existing f32 length-2,048 test checks
the scalar fallback against an f64 reference.

## Paired baseline comparison

The baseline source was `4b13e38`; the candidate source was `d359ca6`. Both
used the same Rust benchmark source, `rustfft.rs` SHA-256
`6a85531e042175af96e9bcdc07368c15908e46212b7d441d68c57cfb7b9b4cba`, and the
same crate lockfile SHA-256
`e61d5628d09dc7c10e54e15cb5f3aeb5dd5c28392b44692ed153784ad8ca21ba`. The
bench profile used Rust 1.99.0 (`b940084d7`, LLVM 23.1.1), Cargo 1.99.0,
`-C opt-level=3`, `-C embed-bitcode=no`, and `-C strip=debuginfo`. There were
no custom `RUSTFLAGS`, LTO, or `target-cpu` setting. RustFFT was 6.4.1. Each length used 12 baseline/candidate process pairs pinned to logical one CPU core
of an x86-64 CPU with AVX2. Pair order alternated. Every process warmed
the plan and took seven timed samples for R2C and C2R at batch counts 1 and 32. Inputs use the benchmark's fixed `real_value` and `spectrum_value` functions. The samples were collected on 2026-10-13. The C2R control adjustment for each pair is
`((baseline C2R / candidate C2R) / (baseline R2C / candidate R2C)) - 1`. Positive values indicate a C2R gain beyond the matched R2C timing change.

| N | Radix-two pass used | C2R single, baseline / candidate (µs); paired gain; R2C-adjusted gain | C2R batch 32, baseline / candidate (µs per transform); paired gain; R2C-adjusted gain |
|---:|:---:|---:|---:|
| 128 | yes | 0.381 / 0.331; +18.6%; +12.5% (12/12 pairs) | 0.355 / 0.305; +16.5%; +11.3% (8/12 pairs) |
| 256 | no | 0.631 / 0.631; +0.0%; −2.8% (1/12 pairs) | 0.599 / 0.596; −0.3%; −0.5% (5/12 pairs) |
| 512 | yes | 1.493 / 1.313; +13.7%; +12.6% (10/12 pairs) | 1.472 / 1.290; +14.2%; +12.5% (12/12 pairs) |
| 1,024 | no | 2.640 / 2.675; −1.3%; −1.1% (5/12 pairs) | 2.631 / 2.670; −1.4%; −4.9% (0/12 pairs) |
| 2,048 | yes | 6.462 / 5.776; +12.1%; +11.0% (12/12 pairs) | 6.489 / 5.795; +11.9%; +8.5% (12/12 pairs) |
| 4,096 | no | 11.867 / 11.963; −0.9%; −3.7% (0/12 pairs) | 11.968 / 12.079; −1.0%; −4.1% (0/12 pairs) |

The R2C control median changes (`baseline / candidate − 1`) ranged from
−0.6% to +5.1% for single calls and +0.8% to +3.7% for batches. For the
no-radix-two C2R controls, raw single-call medians changed −1.3% to +0.0%,
batch medians changed −1.4% to −0.3%, and R2C-adjusted estimates changed
−0.5% to −4.9%. At 512 and 2,048,
all 12 C2R process pairs favored the candidate in both single and batch
measurements; the R2C-adjusted median gains were 12.5–12.6% at 512 and
8.5–11.0% at 2,048. The paired process records contain 576 unique rows. Baseline and candidate
checksums and workspace sizes match for every length, direction, batch count,
and process pair. The baseline and candidate default-profile benchmark binary
SHA-256 values were
`32e9dd95adce0672853ce387d6b61595622f2cc26881ecb19e923a13300df897` and
`f0fa7b843e527a6041e8aa3b091c0debe43505e0466fa090680566eadac372d8`. Plan
construction and workspace allocation medians pool 24 single-transform R2C
and C2R records from each set of 12 processes:

| N | Plan time, baseline / candidate (µs) | Workspace allocation, baseline / candidate (µs) | Workspace bytes |
|---:|---:|---:|---:|
| 128 | 2.753 / 2.747 | 0.399 / 0.341 | 4,128 |
| 512 | 9.009 / 9.002 | 0.619 / 0.613 | 15,456 |
| 2,048 | 34.528 / 34.376 | 1.261 / 1.283 | 60,576 |

The candidate changes no scratch allocation. The raw records are in
[`m4-real-radix2-avx2-process-samples.csv`](../benches/results/m4-real-radix2-avx2-process-samples.csv).

## Cross-library C2R timings

The cross-library run used 12 CPU-pinned processes per implementation and
length, with implementation order rotated. Each process measured whole public
R2C/C2R operations at batch counts 1 and 32. RustFFT used the existing
RustSHT-style real adapter. Direct C++ DUCC used the pinned revision above;
both DUCC plans set `vectorize=false`. Portable DUCC used
`-std=c++20 -O3 -DNDEBUG -pthread`; native DUCC added `-march=native`. The
native comparison uses a different ISA target from the portable Rust build. The C++ harness source SHA-256 was
`cbe53d90e23232bf5abd1ce5fc9236fbe2ff40024effd2df1b90f8498d1dc1fa`. Inputs were identical across implementations. Batch timings report both
microseconds per transform and total microseconds for the 32-transform call. Ratios use the displayed per-transform medians.

| N | Single C2R, DUCCFFT / RustFFT / DUCC portable / DUCC native (µs) | DUCCFFT / RustFFT / portable / native |
|---:|---:|---:|
| 128 | 0.360 / 0.211 / 0.281 / 0.251 | 1.71 / 1.28 / 1.44× |
| 512 | 1.333 / 0.791 / 1.122 / 0.982 | 1.69 / 1.19 / 1.36× |
| 2,048 | 5.846 / 3.657 / 5.240 / 4.559 | 1.60 / 1.12 / 1.28× |

| N | Batch 32 C2R per transform, DUCCFFT / RustFFT / DUCC portable / DUCC native (µs) | Full batch call in the same order (µs) | DUCCFFT / RustFFT / portable / native |
|---:|---:|---:|---:|
| 128 | 0.325 / 0.188 / 0.257 / 0.229 | 10.384 / 6.002 / 8.235 / 7.329 | 1.73 / 1.26 / 1.42× |
| 512 | 1.305 / 0.787 / 1.129 / 0.996 | 41.758 / 25.198 / 36.123 / 31.865 | 1.66 / 1.16 / 1.31× |
| 2,048 | 5.851 / 3.655 / 5.297 / 4.689 | 187.242 / 116.975 / 169.519 / 150.062 | 1.60 / 1.11 / 1.25× |

The candidate closes part of the C2R gap at 512 and 2,048. C2R remains
1.60–1.73× slower than RustFFT in these cases. It is 1.12–1.19× slower than
portable DUCC at 512 and 2,048, and 1.25–1.36× slower than native-target DUCC. The 128-point paired result has wider control variation and is less conclusive. The raw cross-library records contain 576 unique rows and are in
[`m4-real-radix2-cross-library-process-samples.csv`](../benches/results/m4-real-radix2-cross-library-process-samples.csv). The Rust benchmark binary SHA-256 was
`f0fa7b843e527a6041e8aa3b091c0debe43505e0466fa090680566eadac372d8`. The
portable and native DUCC binary SHA-256 values were
`6485c1bd85cd3e67dc929ebeea13fed4bc60fd3a81e646fc4a354a50005698ab` and
`09fe220341903e95788d96983aeddb7652da5a1947ee01ae439b0322fdfce178`. The host also ran the Legendre workload. Load-average telemetry was retained only in local diagnostics. Treat wall-time comparisons as provisional.

## Instruction counts and machine code

and no symbol stripping (`CARGO_PROFILE_BENCH_DEBUG=true` and
`CARGO_PROFILE_BENCH_STRIP=none`). The command ran three warmups followed by
100 normal 2,048-point C2R inverses. Whole-process instruction references fell
from 17,045,640 at baseline to 14,176,291 at the candidate, a 16.8% reduction. These totals include plan construction, workspace setup, and the transforms;
The debug-profile benchmark binary SHA-256 values were
`43e2d6c5e9530c05b78d4c36067186c606525c4a7a37f8f03b38bed5d15c251f` and
`df9a0f55498809e6e97b7e2cfaacbf5155f8979185d2c52d6cc0364edb5832cf`. Disassembly of `duccfft::simd::x86::f64_real_kernel::inverse_radix2_interior`
in the optimized candidate binary contains packed YMM `vmovupd`, `vpermpd`,
`vaddpd`, `vsubpd`, `vblendpd`, `vmulpd`, and `vshufpd` instructions. The
vector loop has one back-edge branch, no helper calls, and no stack-local
spills. It uses runtime AVX2 dispatch and no FMA instructions. The paired C2R gains and instruction reduction support this targeted kernel at
512 and 2,048. The no-radix-two controls show small regressions that need
continued measurement as M4 proceeds. R24 review is pending.
