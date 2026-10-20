# Spin analysis ring-scale hoisting experiment

## Result

A prototype cached the GL spin-1 analysis factor
`(ring_weight / sin(theta)) / nphi` in immutable plan metadata, so the paired
analysis recurrence could reuse it across harmonic orders. The factors used the
same `f64` to scalar conversion, latitude sine conversion and ordered
divisions as the previous ring loop. Focused recurrence tests, the T255 DUCC
fixture and the opt-in T511 GL/CC/F1 DUCC and roundtrip test passed. The
prototype did not improve whole-transform latency and was removed.

## Measurements

The baseline was commit `23d15ea`; the candidate was that source plus the
uncommitted scale vector. Both used `examples/m1b_bench.rs`, f64 GL spin-1
STANDARD analysis, full `mmax`, `phi0=0.37`, reusable plans and workspaces,
one warm-up and five timed calls per process. Each process ran the q1 and q8
operations using the same q8-sized workspace. Five alternating process pairs
were collected for T255, T511 and T720; T127 used three pairs. Processes were
8 pinned CPU cores. The grid was `ntheta=lmax+1`, `nphi=2*lmax+3` (T127
128x257, T255 256x513, T511 512x1025, T720 721x1443). Results are median
process timings and ranges in milliseconds:

| Band | Threads | Baseline | Cached scale | Median change |
| --- | ---: | ---: | ---: | ---: |
| T127 | 1 | `1.062 [1.051–1.066]` | `1.078 [1.045–1.081]` | `+1.51%` |
| T127 | 8 | `0.496 [0.489–0.582]` | `0.488 [0.479–0.562]` | `-1.61%` |
| T255 | 1 | `6.912 [6.846–6.982]` | `6.910 [6.870–6.981]` | `-0.03%` |
| T255 | 8 | `2.662 [2.594–2.811]` | `2.683 [2.623–2.719]` | `+0.79%` |
| T511 | 1 | `38.683 [38.515–39.328]` | `39.199 [38.919–41.072]` | `+1.33%` |
| T511 | 8 | `12.369 [12.104–13.917]` | `12.386 [12.249–13.209]` | `+0.14%` |
| T720 | 1 | `104.572 [104.137–104.686]` | `106.279 [104.884–107.314]` | `+1.63%` |
| T720 | 8 | `34.550 [33.674–36.436]` | `34.377 [34.164–36.999]` | `-0.50%` |

The q8 paired differences varied in sign and overlapped their process ranges. The candidate added `8 * ntheta` bytes to each applicable f64 plan (5.6 KiB
at T720). Measured plan construction rose by roughly 1–7 microseconds from
T127 through T720. T127 selected the existing table path, so the cached values
were not used there. The slight T511/T720 q1 regressions and lack of q8 gain do
not justify retaining the extra plan state.

## Validation and environment

`spin1_gl_paired_analysis_matches_full_latitude_recurrence` and
`spin1_gl_paired_analysis_falls_back_for_extreme_inputs` passed. The static
`ducc0==0.41.0` spin T255 reference test passed, as did the opt-in release
T511 spin test on GL, CC and F1. No tolerance, transform convention or FFT
code changed. DUCC wall-clock comparisons were not repeated because the
candidate did not produce a RustSHT speedup; the DUCC tests are correctness
checks, not timing comparisons. The host was an x86-64 CPU running Linux. Rust was
`1.99.0 (b940084d7 2026-09-28)` with LLVM 23.1.1. The release profile used
`opt-level=3`, 16 codegen units and no LTO; Pulp used portable runtime dispatch
prevented hardware-counter collection. Benchmark output with all paired raw
samples is in [`paired-process-samples.txt`](../../data/spin-analysis-ring-scale-hoist-2026-10-14/paired-process-samples.txt). Reproduce the native run with:

```sh
cargo build --release --example m1b_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 511 8 analysis GL STANDARD
```

The same command applies to the other bandlimits by replacing `511`.
