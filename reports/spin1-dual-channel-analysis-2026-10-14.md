# Fused Q/U reduction for spin-1 analysis

## Result

Spin-1 STANDARD analysis now accumulates its Q and U scalar-conversion channels
in one f64 Pulp SIMD pass when both channels and the recurrence block are in
the normal range. The shared recurrence basis and cosine vectors are loaded
once for both channels. When either channel is outside the safe range, each
channel uses the existing safe SIMD or binary-scaled fallback independently. The f32 path retains its previous implementation through the trait default. Five alternating, process-isolated native comparisons improved T511 analysis
by 4.79% at one thread and 3.60% at eight threads. At T720, the gains were
3.67% at one thread and 0.70% at eight threads; the T720 q8 paired range
crosses zero, so that result is inconclusive. The change adds no plan or
workspace memory. The AVX2 candidate has eight live YMM accumulator vectors
and no vector spills. It retains a large remaining gap to DUCC0, particularly
with eight threads.

## Change and numerical fallback

The baseline callback in `accumulate_spin_scalar_analysis_pair_block` invoked
`accumulate_scalar_analysis_pair` once for Q and once for U over the same
basis and cosine slices. The f64 fast path now dispatches one reducer holding
both channel accumulations. This is a two-channel simultaneous reduction,
not the earlier rejected single-dispatch prototype that performed Q and U as
separate passes. The latter did not produce a repeatable q8 gain. The new dispatch is selected only when `products_safe == [true, true]` and
the recurrence is normal. Otherwise it invokes the original channel reducer
for each channel, preserving separate product-safety decisions, mixed
normal/scaled lanes, exponent restoration, and extreme finite input handling. The f64 SIMD kernel preserves each channel's existing vector accumulation
order and scalar tail behavior. The f32 implementation is unchanged.

## Native paired measurements

The baseline was commit `612a90e`; the candidate is the source change
accompanying this report. Both used `examples/m1b_bench.rs`, f64 spin-1
STANDARD analysis, GL, `phi0=0.37`, full `mmax=lmax`, reused plan/workspace,
one warm-up, and five timed transforms per fresh process. Five alternating
baseline/candidate process pairs were run for each case. q1 was pinned to one CPU core; q8 used 8 pinned CPU cores. Values are medians of five process medians. Paired
changes are medians across corresponding pairs.

| Band (grid) | Threads | Baseline | Candidate | Paired change | Paired range |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 (`512x1025`) | 1 | 37.868 ms | 36.048 ms | -4.79% | -5.04% to -4.22% |
| T511 (`512x1025`) | 8 | 12.322 ms | 11.810 ms | -3.60% | -4.97% to -0.85% |
| T720 (`721x1443`) | 1 | 100.818 ms | 97.351 ms | -3.67% | -3.97% to -2.26% |
| T720 (`721x1443`) | 8 | 33.842 ms | 33.278 ms | -0.70% | -2.22% to +0.23% |

The q1 gains were faster in all five pairs. T511 q8 was faster in all five
pairs; T720 q8 was faster in four of five, but its median gain is comparable
to process variation and is not treated as established. Raw samples are in
[`spin1-dual-analysis-paired`](data/current-kernel-baseline-2026-10-14/spin1-dual-analysis-paired/).

## DUCC0 comparison and current gap

Installed `ducc0==0.41.0` was used as the numerical and Python timing reference;
the corresponding source revision is
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The candidate extension was
rebuilt and compared with matching GL shape, coefficients, `phi0`, and thread
count. These Python plan timings provide end-to-end context; the acceptance
measurements above are native Rust timings and isolate the changed kernel.

| Band | Threads | RustSHT plan | DUCC0 | Ratio | Max abs / RMS / relative L2 error |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | 35.623 ms | 12.784 ms | 2.79x | `1.401e-14 / 1.910e-16 / 3.740e-14` |
| T511 | 8 | 12.210 ms | 2.004 ms | 6.09x | `1.401e-14 / 1.910e-16 / 3.740e-14` |
| T720 | 1 | 98.581 ms | 30.360 ms | 3.25x | `1.391e-14 / 2.200e-16 / 6.061e-14` |
| T720 | 8 | 34.680 ms | 4.355 ms | 7.96x | `1.391e-14 / 2.201e-16 / 6.064e-14` |

The q8 Python Rust timing at T511 had a 4.324 ms range across three
processes, so its ratio is context rather than a controlled performance
claim. The native paired run is the q8 acceptance result. Full Python records
are in [`python-candidate-spin1-analysis`](data/current-kernel-baseline-2026-10-14/python-candidate-spin1-analysis/). The broader pre-change baseline shows the next optimization is still
substantial. At T720, native Rust scalar synthesis/analysis take
`103.395/61.274 ms` at q1 and `27.905/19.521 ms` at q8. Spin-1 synthesis and
analysis take `125.544/101.040 ms` at q1 and `41.646/33.532 ms` at q8. The
Python plan matrix against DUCC and all within-process samples is in
[`python-process-samples`](data/current-kernel-baseline-2026-10-14/python-process-samples/).

## Profile and code generation

`4,263,278,408` instruction references across plan/workspace setup and six
warm transforms. Source attribution placed the analysis callback at about
39.27% inclusive references, with Q and U calls to
`accumulate_scalar_analysis_pair` each at about 16.17% inclusive. Inclusive
figures overlap. This supported testing reuse of basis/cosine reads while
not a native cycle or time measurement. The host disallows perf hardware

The candidate AVX2 assembly was inspected in the generated release binary. The loop keeps eight vector accumulator chains live and has no YMM spills. The dispatch shares basis and cosine vector loads, at the cost of maintaining
both channel accumulators in the same loop. No raw intrinsics or CPU-specific
dispatch were added. The host was an x86-64 CPU with AVX2/FMA; AVX-512
was unavailable to this runtime path. `5,506,566,784` instruction references across setup and six warm transforms. A separate experiment split the final recurrence pair to remove vector-loop
bounds checks. Although generated code changed as expected, five whole-
transform paired runs did not establish a benefit; that prototype was

## Validation

The new `dual_analysis_accumulation_matches_independent_channels_and_scaled_fallback`
test checks vector tails from one through 33 lanes, both normal channels, and
an unsafe extreme finite channel that must use the established fallback. `paired_analysis_extreme_finite_products_use_scaled_fallback` separately
checks finite inputs whose direct intermediate product would overflow. The
existing comparison verifies that the fallback result remains finite and
matches the scaled multiplication reference. Checks passed on the candidate:

- Focused dual-channel reducer, spin-1 STANDARD analysis/general-kernel and
  adjoint tests, and q1/q8 analysis equivalence.
- Release spin-1/2 T511 DUCC 0.41.0 and round-trip checks on GL, CC, and F1.
- Release exact spin-1/2 T1023 DUCC checks on GL, CC, and F1.
- Spin-3/4 analytic and adjoint regression.
- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `cargo test --workspace --all-features` (152 passed, 9 ignored; 10 f32
  tests passed).
- `cargo test -p rustsht --no-default-features` (130 passed, 7 ignored; 10
  f32 tests passed).
- `uv run pytest python/tests` (20 passed). No tolerance, numerical convention, public API, FFT path, or workspace size
changed.

## Reproduction

Machine: x86-64 CPU, Linux. Rust `1.99.0`, LLVM
`23.1.1`. Release `opt-level=3`, 16 codegen units, LTO disabled, debug
symbols enabled for profiles, portable Pulp runtime dispatch (AVX2/FMA on
this CPU), and no `target-cpu=native`. Python was 3.11.15, NumPy 2.4.6. Build and native benchmark commands:

```text
cargo build --release --example m1b_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 511 1 analysis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 511 8 analysis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 720 1 analysis GL STANDARD 721 1443 720 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 720 8 analysis GL STANDARD 721 1443 720 requested-only
```

Each native command was run in five alternating baseline/candidate process
pairs. Each process performs one warm-up and five timed transforms. The
baseline executable was built from `612a90e`; the candidate was built from
the source change recorded here. Python plan comparisons used:

```text
RAYON_NUM_THREADS=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 511 1 1 GL 2 plan analysis both STANDARD
RAYON_NUM_THREADS=8 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 511 1 8 GL 2 plan analysis both STANDARD
RAYON_NUM_THREADS=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 720 1 1 GL 2 plan analysis both STANDARD
RAYON_NUM_THREADS=8 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 720 1 8 GL 2 plan analysis both STANDARD
```

The initial baseline matrix used three fresh processes for each T511/T720,
scalar/spin-1, synthesis/analysis, and q1/q8 case. Native process samples are
in [`native-process-samples.txt`](data/current-kernel-baseline-2026-10-14/native-process-samples.txt).
