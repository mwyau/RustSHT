# Range-precheck T2047 q1 complete-transform screen v1

## Scope

This was one baseline/candidate process pair under the
[frozen screen protocol](prefix-lane-pass-fusion-range-precheck-t2047-q1-pair-protocol-v1.md).
The test measured GL f64 scalar `STANDARD` synthesis into caller-owned output
at T2047 (`2048x4096`, full `mmax=2047`, `phi0=0.37`) with one worker. The
clean production baseline ran first, followed by the isolated range-precheck
candidate. Each process used the same DUCC 0.42.0 and SHTns 3.7.5 references,
patched harness, five fixed untimed calls per backend, and five measured calls.

The baseline and candidate extension hashes were, respectively,
`69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036` and
`58c029273cb3aedd0bd1350fe2dde22fe703559c819ace8e90ce52d06e25d562`.
Candidate source hashes were `07c41fe4ad285e59d7ed180931d145f72841d7adc9fe889a20a755837bf48b06`
for `synthesis_fast.rs` and
`961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559` for
`scalar.rs`. The harness and reference identities are recorded in the
protocol.

Hardware was an AMD Ryzen 9 5950X, x86-64 with AVX2 and FMA, 16 physical and
32 logical cores, and 96 GiB RAM. The system used Ubuntu 26.04.1, Rust 1.99.0,
Python 3.11.15, and NumPy 2.4.6. The benchmark process was restricted to one
logical CPU; observed worker placement stayed within that affinity.

## Complete-transform timings

All values are milliseconds. Each sample is retained in acquisition order.

| Build | Backend | Five samples | Median | CV | First-three/final-two drift |
|---|---|---|---:|---:|---:|
| Clean baseline | RustSHT | 3272.188, 3261.540, 3269.807, 3272.137, 3270.589 | 3270.589 | 0.1212% | 0.0476% |
| Clean baseline | DUCC | 367.097, 369.241, 366.696, 367.142, 368.282 | 367.142 | 0.2550% | 0.1675% |
| Clean baseline | SHTns adapter | 265.278, 265.835, 265.861, 265.908, 265.795 | 265.835 | 0.0872% | 0.0062% |
| Range-precheck candidate | RustSHT | 1237.260, 1239.364, 1235.409, 1235.040, 1235.384 | 1235.409 | 0.1321% | 0.1658% |
| Range-precheck candidate | DUCC | 365.582, 373.156, 362.512, 373.903, 373.187 | 373.156 | 1.2711% | 2.1340% |
| Range-precheck candidate | SHTns adapter | 266.260, 266.703, 266.598, 266.240, 265.994 | 266.260 | 0.0968% | 0.1807% |

Every series passed the fixed 5% CV and drift limits. RustSHT and SHTns
matched DUCC within relative-L2 `2e-11` and maximum absolute error `2e-12`:

| Build | RustSHT vs DUCC relative-L2 | RustSHT max abs | SHTns vs DUCC relative-L2 | SHTns max abs |
|---|---:|---:|---:|---:|
| Clean baseline | `3.015e-13` | `1.110e-12` | `2.425e-13` | `6.488e-13` |
| Range-precheck candidate | `3.015e-13` | `1.110e-12` | `2.425e-13` | `6.487e-13` |

The candidate RustSHT median was 0.377733 times the baseline median, a 2.647x
speedup and 62.23% lower latency in this one pair. The revalidated baseline
was 8.91x the paired DUCC median; the candidate was 3.31x its paired DUCC
median. Relative to SHTns, the baseline and candidate were 12.30x and 4.64x.
These single-pair ratios do not establish a confidence interval or parity.

Peak RSS was 958,428 KiB for the baseline and 956,672 KiB for the candidate
(935.96 MiB and 934.25 MiB). The runner elapsed times were 96.25 s and 76.92 s.

## Validity and diagnostic observations

Both processes passed preflight and postflight CPU PSI gates; neither entered
the 0.50% warning band. Monitoring found no clear competing CPU-intensive
workload or cgroup CPU throttling. The fixed affinity was honored. CPU
frequency, worker placement, context switches, process observations, and exact
PSI captures were collected locally; raw captures remain untracked.

The harness also measured NumPy input and output copies outside the transform
interval. Their reported medians were 3.693 ms and 10.690 ms for the baseline,
and 60.706 ms and 125.877 ms for the candidate. These ancillary values are
unusually different, but the timed RustSHT, DUCC, and SHTns series were stable,
and the copy operations were complete before the timed transformations. Under
the frozen rules, this is a diagnostic caveat rather than evidence that the
transform samples were interfered with. It must be checked again in the
multi-pair acquisition.

## Correctness checks

The candidate's full Cargo workspace test suite passed: 203 library tests, 4
extreme-FFT tests, 11 f32 tests, and the workspace compatibility test passed;
14 library tests and 1 f32 test were ignored. The suite includes fast-kernel
tail and polar-lane checks, scaled-fallback checks, high-order recurrence
checks, and workspace reuse across execution capacities. The existing
T2047 route profile for this exact source also passed its one-worker route
test and bitwise comparison between unprofiled and instrumented output.

`cargo fmt --all -- --check` passed. Clippy with warnings denied did not pass:
it reports one `collapsible_if` warning in the experimental
`synthesis_fast.rs` at the scaled transition guard. This is not a numerical
failure, but it must be resolved before production acceptance.

## Decision and next measurement

This valid one-pair screen is favorable and authorizes the frozen six-pair
T2047 q1 matrix. It does not accept the candidate. The source remains isolated
and is not production-routed. T1023, T511, q8, spin-1 controls, and the full
regression matrix remain outstanding. The candidate still trails DUCC by
3.31x for T2047 q1 scalar synthesis.

The route profile places the next measured kernel work in the scaled prefix:
the range-precheck profile measured 643.386 ms in prefix plus transition setup,
542.158 ms in the ordinary SIMD suffix, 46.284 ms in scaled fallback, and
5.584 ms in coefficient-safety scans. These nested instrumented timings are
mechanism diagnostics, not complete-transform acceptance measurements. See
the [route profile](prefix-lane-pass-fusion-range-precheck-t2047-v1.md).
