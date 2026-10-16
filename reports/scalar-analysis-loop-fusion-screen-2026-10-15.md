# Scalar analysis recurrence/accumulation loop-fusion screen

## Candidate

The candidate combined each f64 scalar analysis accumulation loop with the recurrence update for the next degree pair. It reused the loaded basis and cosine vectors. It retained the existing scaled-recurrence fallback and staged coefficient writes. A Pulp AVX2 disassembly showed vectorized accumulation and recurrence updates without YMM/XMM stack spills. The fused function's object code grew from 0xd27 to 0x1307 bytes. The candidate source patch is preserved in [`candidate.patch`](data/fused-scalar-analysis-loop-fusion-2026-10-15/candidate.patch). The patch was removed from the implementation after the end-to-end screen. The only source change retained is a test input of 21 latitude lanes, which exercises an odd SIMD-vector remainder and a scalar tail in the existing fused analysis path.

## Method

The benchmark used `examples/m1b_bench.rs` for f64 scalar STANDARD GL analysis with full `mmax`, `phi0=0.37`, `ntheta=lmax+1`, and `nphi=2*lmax+3`. Each process warmed the transform and recorded five timed analyses with a reused plan and workspace. The baseline source was commit `5ccd8072807b381dda3edbad4c62b2e48ae45a7a`; the candidate applied the preserved patch to that source. Both binaries used portable Pulp runtime dispatch and RustFFT without `target-cpu=native`. Two screens each used five alternating baseline/candidate process pairs at q1 and q8 for T511 and T1023. Processes were pinned to one CPU core at q1 and 8 pinned CPU cores at q8. The runner held `target/rustsht-benchmark.lock`; `/usr/bin/time` recorded process elapsed time and peak RSS. Rust was `1.99.0 (b940084d7 2026-09-28)` with LLVM `23.1.1`. The host was an x86-64 CPU running Linux. The baseline executable SHA-256 was `e5c72dc27a28c6c784735e176dc1a07ae3035920c35d985eb547cfd671ca0687`; the candidate executable SHA-256 was `50d50fc595e3fdf02dc37e09014cfc562e8d0cc33585d85496220c8e3d27c300`. Each cell reports medians of five process medians. Paired change is the median of candidate/baseline changes for each process pair; negative values favor the candidate. The range is the minimum to maximum paired change. Process-level raw samples and peak RSS are retained in [`the data directory`](data/fused-scalar-analysis-loop-fusion-2026-10-15/).

## Results

| Bandlimit | Threads | Screen | Baseline (ms) | Candidate (ms) | Paired change | Paired range |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | First | 20.307 | 19.724 | -2.79% | -3.64% to +0.24% |
| T511 | 8 | First | 3.904 | 3.661 | -6.22% | -25.56% to +31.50% |
| T1023 | 1 | First | 161.226 | 153.277 | -4.49% | -5.44% to +39.32% |
| T1023 | 8 | First | 28.246 | 26.125 | -3.90% | -11.21% to +18.41% |
| T511 | 1 | Repeat | 20.201 | 19.782 | -2.27% | -6.92% to +2.21% |
| T511 | 8 | Repeat | 4.909 | 3.803 | -15.93% | -33.66% to +29.59% |
| T1023 | 1 | Repeat | 160.665 | 171.937 | +7.96% | -23.89% to +15.46% |
| T1023 | 8 | Repeat | 28.937 | 27.160 | -2.37% | -13.13% to +8.32% |

The q1 T511 median favored the candidate in both screens, with paired ranges crossing zero. The q1 T1023 result changed direction between screens. All q8 ranges crossed zero and were broad. Load-average telemetry was retained only in local diagnostics. The measurements do not establish a repeatable whole-transform speedup, so the candidate did not pass the performance gate.

## Validation

With the candidate present, `cargo fmt --all -- --check`, workspace Clippy with all targets/features and warnings denied, and `cargo test --workspace --all-features` passed (182 Rust tests and 10 f32 tests; 12 ignored). `cargo test -p rustsht --no-default-features` passed (144 Rust tests and 10 f32 tests; 7 ignored). The opt-in release `m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form` test passed for GL, CC and F1. DUCC references used `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The retained 21-lane test passed after removing the candidate. Python tests were not run because no Python bindings changed. No candidate recurrence/accumulation implementation remains in the source tree.
