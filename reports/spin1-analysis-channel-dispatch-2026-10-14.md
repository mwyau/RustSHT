# Spin-1 paired analysis dispatch experiment

## Result

The paired spin-1 GL analysis callback currently dispatches the Q and U
accumulators separately for each degree pair. I tested one Pulp dispatch that
handles both channels sequentially with the same SIMD type. This preserved the
existing per-channel reduction and recurrence order and kept the same SIMD
accumulators live at one time. It added no workspace memory. The corrected candidate did not improve eight-thread latency beyond process
variation. Its paired q8 median was `+0.24%` at T511 and `-0.93%` at T720;
the q8 pair ranges crossed zero. One-thread paired medians fell `1.94%` at
T511 and `0.95%` at T720, but these gains did not transfer consistently to
q8. The candidate was removed because the whole-transform gain was small,
variable, and unsupported at the thread count with the largest absolute
DUCC gap. The production transform is unchanged.

## Profile and hypothesis

The baseline was `dc9f20c` (`Refine high-band synthesis group sizing`). A
collected `5,046,473,554` instruction references across the benchmark's five
timed transforms and setup. `AnalysisPairAccumulation` accounted for 20.0%
inclusive references and `MixedAnalysisPairAccumulation` 8.1%; the paired
SIMD slice setup in the Q/U callback. The two channels still used separate
accumulation passes, which limited register pressure. The paired native
measurements and instruction count show that the dispatch and slice setup do
not account for a material share of transform latency. An initial implementation used `[0, 1].map(...)` inside `WithSimd`. Its
generated code routed the per-degree callback through
references to this path. That temporary version was `14.9x` slower at q1 and
`7.2x` slower at q8. Replacing `.map` with two explicit calls restored normal
latency; the results below describe only the corrected candidate.

## Method

- Native benchmark: `examples/m1b_bench.rs`, reusable plan and workspace,
  `phi0=0.37`, full `mmax`, f64 GL STANDARD analysis, one warm-up and five
  timed operations per process.
- Bands and exact grids: T511 `512x1025`; T720 `721x1443`.
- Five alternating baseline/candidate process pairs per band and thread
  count. q1 processes were pinned to one CPU core; q8 processes to 8 pinned CPU cores. Paired
  execution order alternated.
- CPU: x86-64 CPU, Linux. Rust `1.99.0`, LLVM 23.1.1,
  release `opt-level=3`, 16 codegen units, LTO off, portable Pulp AVX2/FMA
  dispatch, no `target-cpu=native`.
- Candidate source changed `src/legendre/mod.rs`,
  `src/legendre/scalar_paired.rs`, `src/legendre/scalar.rs` and
  `src/spin_transform.rs`; those changes were removed after measurement.
- Process medians and all five inner samples are in
  [`process-medians.csv`](data/spin1-analysis-channel-dispatch-2026-10-14/process-medians.csv).

## Native timings

Values are medians of five process medians, in milliseconds. The paired
percentage is the median of each adjacent process pair's candidate change.

| Band | Threads | Baseline | Candidate | Paired change | Paired range |
|---|---:|---:|---:|---:|---:|
| T511 | 1 | `38.301` | `37.656` | `-1.94%` | `-3.91%` to `+0.35%` |
| T511 | 8 | `12.196` | `12.216` | `+0.24%` | `-1.78%` to `+1.12%` |
| T720 | 1 | `102.023` | `100.841` | `-0.95%` | `-3.92%` to `+1.20%` |
| T720 | 8 | `33.910` | `33.657` | `-0.93%` | `-2.48%` to `+1.04%` |

below the baseline profile. This small reduction did not produce a stable q8
latency gain. The candidate's numerical path used the same per-channel
reducer and the same accumulation order; the changes only combined dispatch
and SIMD setup. For context, the current T720 q8 Python-plan comparison against installed
`ducc0==0.41.0` at source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf` measured RustSHT/DUCC analysis
medians of `25.834/2.962 ms` for scalar and `42.723/5.573 ms` for spin 1. The spin-1 relative-L2 coefficient error was `6.064e-14`. The Python module
was not rebuilt with the rejected dispatch candidate, so these are baseline
context and not candidate timings.

## Numerical validation

The focused channel-reducer test passed for normal-range vectors, mixed
exponents, odd vector tails, and a channel whose unsafe finite inputs take the
scaled fallback while the other channel uses SIMD. Existing spin-1
STANDARD scalar-conversion analysis and adjoint tests passed on GL, CC and F1. Opt-in release DUCC 0.41.0 checks passed for spin 1 and 2 on all three grids
through T511, and exact spin 1 and 2 analysis at T1023. Maximum spin-1 T1023
analysis coefficient absolute error was `8.822e-18`; this test used the pinned
DUCC source revision named above. The candidate passed these numerical checks but did not pass performance
acceptance. Its source changes and focused test were removed; the report and
paired measurements record the result. No public API, tolerance, FFT code or
production kernel changed in the final commit.
