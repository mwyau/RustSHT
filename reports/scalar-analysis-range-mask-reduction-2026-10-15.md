# Mask reduction in fused scalar analysis

The f64 fused scalar analysis block now combines recurrence range checks as SIMD masks. Each vector lane must be zero or have a magnitude in `[SMALL, BIG]`. The block ANDs the lane masks across vector groups and performs one horizontal reduction after each recurrence update. It previously reduced every SIMD vector group separately. Scalar-tail checks, recurrence values, accumulation order, staged coefficient ownership and scaled fallback are unchanged.

## Native comparison

The baseline is commit `d188b772587ed377fbe5b9772ab31c79b324b4bb`. The candidate change relative to the baseline is recorded in [`candidate-change.txt`](data/scalar-analysis-range-mask-reduction-2026-10-15/candidate-change.txt). Each process used `examples/m1b_bench.rs` for f64 GL STANDARD analysis with a reused plan and workspace, full `mmax`, `phi0=0.37`, one warm-up and five timed calls. The grids were T511 (`512 x 1025`) and T1023 (`1024 x 2049`). Five paired processes ran at q1; twenty ran at T511 q8 and ten at T1023 q8. Process order alternated within each pair.

| Grid | Threads | Baseline median (range) | Candidate median (range) | Paired change (range) | Candidate faster |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | q1 | `19.083 ms` (`19.009–19.173`) | `18.119 ms` (`18.094–18.376`) | `-5.08%` (`-5.45% to -3.33%`) | 5/5 |
| T511 | q8 | `3.454 ms` (`3.331–3.699`) | `3.372 ms` (`3.228–3.564`) | `-2.63%` (`-10.79% to +5.37%`) | 18/20 |
| T1023 | q1 | `150.632 ms` (`150.346–151.150`) | `143.147 ms` (`142.462–144.434`) | `-4.91%` (`-5.38% to -4.17%`) | 5/5 |
| T1023 | q8 | `24.341 ms` (`24.025–27.324`) | `23.877 ms` (`23.407–26.566`) | `-2.48%` (`-14.34% to +9.55%`) | 8/10 |

The q1 result favored the candidate in all ten pairs. q8 medians were 2.5–2.6% lower, but both q8 ranges cross zero. These measurements support the q1 improvement; the q8 effect remains uncertain.

The host was an AMD Ryzen 9 5950X with 16 cores and 32 logical CPUs, one socket and AVX2/FMA. Rust was `1.99.0` with LLVM `23.1.1`; release defaults used portable Pulp runtime dispatch without `target-cpu=native`. Callgrind symbolized the dispatched kernel as `pulp::x86::v3::V3`, which identifies the executed AVX2 path. q1 was pinned to CPU 8; q8 used CPUs 8–15. The cgroup CPU limit was `max 100000`. Host load averages were `0.92, 1.42, 2.46` before and `1.02, 1.41, 2.44` after the paired screen. The CPUs were not isolated.

A single-process T1023 RSS check measured baseline/candidate peaks of `101,396/101,464 KiB` at q1 and `144,672/144,772 KiB` at q8. The kernel adds no workspace storage; these single-process differences do not establish a repeatable memory change.

## Callgrind

The T511 q8 `m1b_bench` process recorded `2,662,738,568` instruction references at baseline and `2,577,533,222` with the candidate, a `3.20%` decrease. The fused kernel accounted for `43.02%` and `41.06%` of those references, respectively. Each profile includes plan/workspace setup and five timed transforms. Instruction references are not cycles or elapsed time. The compressed profiles and summaries are in [`the benchmark data`](data/scalar-analysis-range-mask-reduction-2026-10-15/).

## Current T1023 comparison with DUCC

Five paired processes compared the candidate’s native reused-plan/workspace analysis with `ducc0==0.41.0` through Python `analysis_2d`. The DUCC source revision was `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Both used the GL `1024 x 2049` full-band workload; q1 was pinned to CPU 8 and q8 to CPUs 8–15.

| Threads | RustSHT median (range) | DUCC median (range) | Paired ratio (range) |
| ---: | ---: | ---: | ---: |
| q1 | `143.297 ms` (`142.105–143.661`) | `123.885 ms` (`122.861–124.395`) | `1.16x` (`1.15–1.16x`) |
| q8 | `26.481 ms` (`24.550–27.436`) | `15.824 ms` (`15.744–16.275`) | `1.63x` (`1.56–1.74x`) |

The q8 RustSHT times varied more than DUCC’s. This comparison describes the current setup and does not isolate the patch’s effect on the RustSHT/DUCC ratio. Raw process logs and timing records are in [`the DUCC comparison data`](data/scalar-analysis-range-mask-reduction-2026-10-15/ducc-comparison/).

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `TMPDIR=... cargo test --workspace --all-features` passed: 187 Rust tests and 11 f32 tests; 12 ignored; 0 doctests. A run using the shared `/tmp` completed the Rust and f32 tests but rustdoc could not create its temporary argument file because `/tmp` was full. The passing rerun used a task-local directory on the workspace filesystem.
- `TMPDIR=... cargo test -p rustsht --no-default-features` passed: 149 Rust tests and 11 f32 tests; 7 ignored; 0 doctests.
- The release fused-block tests passed for SIMD tails, scaled-sectoral rejection and late range failure.
- `m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form` passed in release mode for GL, CC and F1. At T2047, the largest analytic map error was `9.958e-12`; the largest selected DUCC sample error was `2.083e-13`.

Python tests were not run because this change does not affect bindings. The accepted change is limited to f64 scalar analysis. Spin-1 analysis, other grids and other thread counts were not benchmarked for this increment.
