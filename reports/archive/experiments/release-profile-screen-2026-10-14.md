# Release compiler profile screen

## Result

No release-profile change was accepted. Thin LTO improved the repeated
T511 q1 scalar-analysis median by 5.3%, but the paired q8 spin-1 synthesis
results did not reproduce a gain: a fresh five-pair run had a 6.1% higher
candidate median, with mixed paired outcomes. Spin-1 q8 analysis was flat. At T720, q8 spin-1 synthesis was unchanged and analysis varied by about 1%. The one-codegen-unit profile consistently slowed q8 spin-1 synthesis. The
default release profile remains at 16 codegen units with LTO disabled.

## Method

The baseline executable was built from the production source at `fbbe483`;
the only later changes through `ec18235` were documentation. Candidate
executables used the same source and input generation. `examples/m1b_bench`
measured reused plans/workspaces, one warm-up and five timed transforms per
process with `requested-only`. Native Rust runs used GL, full `mmax`,
`phi0=0.37`, f64 and exact grids `512x1025` at T511 and `721x1443` at T720.
q1 processes were pinned to one CPU core; q8 processes to 8 pinned CPU cores. Paired process
order alternated between baseline and candidate. The host was shared. CPU was x86-64 CPU under Linux. Rust was
`1.99.0 (b940084d7 2026-09-28)`, LLVM 23.1.1. All builds used
`opt-level=3`, portable Pulp runtime dispatch (AVX2/FMA on this host), and no
`target-cpu=native`. The profile variants were built independently with:

```sh
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_LTO=false \
  CARGO_TARGET_DIR=<temporary-path> cargo build --release --example m1b_bench
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 CARGO_PROFILE_RELEASE_LTO=thin \
  CARGO_TARGET_DIR=<temporary-path> cargo build --release --example m1b_bench
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_LTO=thin \
  CARGO_TARGET_DIR=<temporary-path> cargo build --release --example m1b_bench
```

The exact per-process timed samples for the five-pair thin-LTO confirmation
at T511 are in
[`process-samples.txt`](../../data/release-profile-screen-2026-10-14/process-samples.txt). Additional process-median screens are summarized below. They are useful for
rejecting large regressions; small changes are not treated as repeatable
without the confirming sample set.

## Thin LTO results

The table shows medians across five process medians from the confirmation
run, in milliseconds. Paired reduction is the median of
`(baseline - candidate) / baseline` across process pairs.

| Workload | Baseline | Thin LTO | Median change | Paired reduction |
| --- | ---: | ---: | ---: | ---: |
| T511 scalar analysis, q1 | 23.714 | 22.460 | -5.3% | +5.0% |
| T511 spin-1 synthesis, q8 | 18.756 | 19.900 | +6.1% | +1.1% |
| T511 spin-1 analysis, q8 | 12.586 | 12.622 | +0.3% | -0.8% |

Thin-LTO screening on the same workload showed T511 q1 spin-1 synthesis
medians of `58.444 ms` baseline and `56.253 ms` candidate, while q1 spin-1
analysis was `38.523` and `38.899 ms`. q8 scalar synthesis medians were
`12.886` and `12.639 ms`; its three-process scalar-analysis screen was
`7.463` and `7.113 ms`. These changes do not establish a broad gain. At T720
q8, spin-1 synthesis medians were `46.875` and `46.881 ms`; spin-1 analysis
medians were `34.096` and `33.692 ms`. Individual process results varied in
both directions. No DUCC latency comparison was repeated for these build
profiles. The thin-LTO T511 release suite passed the pinned DUCC 0.41.0 scalar
T511/T1023/T2047 GL/CC/F1 analytic regression and the spin-1/2 T511
GL/CC/F1 DUCC, round-trip and coefficient checks. The source revision for
DUCC fixtures is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Numerical errors
matched the established results: scalar T2047 maximum map error was
`9.958e-12`, spin-1 GL T511 DUCC analysis maximum coefficient error was
`4.970e-18`, and synthesis-analysis coefficient error was `1.255e-14`.

## Separate codegen-unit screens

At T511 q8, one codegen unit with LTO off had spin-1 synthesis process
medians `18.323, 18.800, 20.123 ms`; the candidate medians were
`23.147, 23.320, 22.931 ms`. q8 spin-1 analysis medians were
`12.386, 12.017, 12.266 ms` versus `12.448, 14.633, 13.055 ms`. q1 scalar
analysis changed from `23.964, 23.678, 23.693 ms` to
`23.361, 23.360, 23.343 ms`; q8 scalar analysis changed from
`7.463, 7.477, 7.217 ms` to `7.191, 7.053, 7.113 ms`. The q8 spin-1
regressions make the profile unsuitable for the shared library workload. With both one codegen unit and thin LTO, the three-process T511 screen gave
q8 spin-1 synthesis `19.084 -> 20.400 ms` and q8 spin-1 analysis
`12.291 -> 12.135 ms`. q1 scalar analysis moved `23.865 -> 22.799 ms`;
q8 scalar synthesis was `12.088 -> 12.134 ms`. This combination also did
profile experiments changed compiler settings only; they did not modify Rust
source, runtime dispatch, public APIs or numerical tolerances. Keep the
portable default build settings until a profile change shows a repeatable
whole-transform improvement across scalar and spin-1 workloads.
