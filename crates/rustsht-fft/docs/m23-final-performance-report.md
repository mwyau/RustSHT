# M23 final performance and validation report

Date: 2026-10-16. Branch: `opt/fft-kernel-performance`.

## Revisions and accepted changes

The session started at M18 production source revision
`3c027942535233bc1d1fe7b889409d3573202da8`. The final production source is
M21 commit `c48e653978079da8d126757a9362c27b4b64e2c7`; later session commits
record benchmarks and documentation only.

- M20 commit `343e5adc08e44cc5472032138dfb263bb34b4786` adds an f64 AVX2
  radix-eleven mixed pass. It handles eligible even `ido` lanes in pairs and
  sends an odd tail to the scalar pass. It improves complete C2C execution at
  N=726, 1,452 and 2,904 while retaining the N=682 specialization and the
  portable fallback. f32 keeps the scalar path.
- M21 commit `c48e653978079da8d126757a9362c27b4b64e2c7` reuses the forward
  general-prime pass twiddle and caches inverse Hermitian inputs per
  inner-frequency lane. The pass equations, normalization, dispatch and
  fallbacks remain unchanged. Plan and workspace layouts do not change.
- M22 commit `3dc4b54` adds f32 results and atmospheric length coverage; it
  changes no production code.
- Commit `d3b7413` corrects the M21 N=2,049 C2R slowdown figures after the
  independent reviewer identified an understated percentage. The corrected
  report is [M21 real-prime pass reuse](m21-real-prime-pass-report.md). The M20 experiments rejected from production remain documented in the
[M20 report](m20-radix11-mixed-pass-report.md). Generalizing the N=682 kernel
reduced its row-batched forward result by about 1–3%; an inline radix-eleven
dispatcher slowed N=1,442 inverse by 8–11%. The accepted out-of-line kernel
keeps the N=682 route intact and avoids the N=1,442 regression.

## Final comparison method

The M21-source executable SHA-256 is
`b88e1fa3b418ea1fdba8343dd53101cd2a09dc4b228ba066fa282bae52f89ec8`. The session-start M18 executable SHA-256 is
`a97674f383732a07ee860887f5407c12659874ee70eb8d9ab0aa058d968dca17`. Both used Rust 1.99.0 stable, optimized bench builds, portable target flags,
runtime SIMD dispatch and one transform thread on an x86-64 CPU, with
each process pinned to logical one CPU core. Each comparison used six alternating
process pairs and seven warmed samples per operation. The workload covers the
complete current C2C and real FFT target matrix, one transform and row batches
at the scalar and spin-1 SHT row counts. Real timings include public packing,
reconstruction and scaling. DUCC is pinned to `c1ed01b8fda69d2a378190f0915f1a19762dddbf` and was built with
g++ 15.2.0. The native build uses `-O3 -DNDEBUG -march=native` with planner
vectorization enabled; the portable build disables planner vectorization. FFTW is system version 3.3.10, using `FFTW_ESTIMATE` and
`FFTW_PRESERVE_INPUT`. Admiral is revision `ce67bc00`, built in Release mode
for the native architecture with threading disabled. RustFFT is version
6.4.1 in the same Rust benchmark executable. Load-average telemetry was retained only in local diagnostics. The clean native DUCC retry recorded 1.01–2.26. The portable
DUCC, FFTW and Admiral captures overlapped a separate f32 SHT benchmark pinned
to eight workers; the FFT comparisons used one worker. Host-pressure observations
were 2.05–4.52, 3.59–4.66 and 3.38–4.15, respectively. Treat those three
cross-library captures as diagnostic; their paired intervals are reported
without claiming quiet-host repeatability. A first native DUCC capture was
stopped after three complete pairs when concurrent SHT activity affected host pressure. Those samples remain diagnostic; the six-pair retry is the reported native result. Ratios below are reference time divided by M21 time, so values above 1 favor
M21. Intervals are bootstrap 95% intervals over six paired process ratios. The M21 absolute time column is the median per transform from the M18 paired
capture. Full per-process medians, plan/workspace construction medians, bytes,
checksums, hashes and structured timing results are in the linked CSV and JSONL files. Raw host-state records remain local.

| Operation | N | Rows | M21 µs | M18/M21 | RustFFT/M21 | DUCC native/M21 | DUCC portable/M21 | FFTW/M21 | Admiral/M21 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| C2C fwd | 726 | 361 | 4.825 | 1.325 (1.318–1.337) | 0.322 (0.320–0.324) | 0.521 (0.501–0.526) | 0.727 (0.704–0.734) | 0.505 (0.493–0.514) | 0.458 (0.450–0.467) |
| C2C inv | 726 | 361 | 5.798 | 1.262 (1.259–1.270) | 0.297 (0.294–0.299) | 0.453 (0.433–0.457) | 0.668 (0.647–0.675) | 0.448 (0.439–0.454) | 0.382 (0.375–0.388) |
| C2C fwd | 1,442 | 721 | 54.976 | 0.992 (0.970–1.025) | 0.235 (0.233–0.262) | 0.317 (0.315–0.326) | 0.441 (0.441–0.460) | 0.443 (0.437–0.452) | 0.365 (0.364–0.368) |
| C2C inv | 1,442 | 721 | 60.451 | 1.003 (0.992–1.008) | 0.220 (0.218–0.222) | 0.271 (0.266–0.282) | 0.411 (0.403–0.416) | 0.413 (0.408–0.420) | 0.335 (0.332–0.340) |
| C2C fwd | 1,452 | 721 | 9.897 | 1.317 (1.296–1.328) | 0.337 (0.336–0.342) | 0.338 (0.332–0.344) | 0.764 (0.736–0.778) | 0.586 (0.567–0.598) | 0.360 (0.356–0.363) |
| C2C inv | 1,452 | 721 | 11.844 | 1.260 (1.247–1.263) | 0.312 (0.308–0.318) | 0.309 (0.302–0.315) | 0.703 (0.681–0.715) | 0.536 (0.509–0.568) | 0.304 (0.301–0.306) |
| C2C fwd | 2,904 | 721 | 20.532 | 1.313 (1.302–1.321) | 0.354 (0.346–0.365) | 0.384 (0.381–0.395) | 0.805 (0.789–0.821) | 0.731 (0.697–0.748) | 0.402 (0.398–0.403) |
| C2C inv | 2,904 | 721 | 24.512 | 1.243 (1.237–1.250) | 0.320 (0.317–0.327) | 0.342 (0.337–0.352) | 0.707 (0.695–0.715) | 0.636 (0.630–0.655) | 0.334 (0.331–0.339) |
| R2C | 723 | 361 | 10.921 | 1.007 (1.005–1.017) | 0.426 (0.422–0.432) | 1.071 (1.052–1.098) | 1.542 (1.497–1.553) | 0.791 (0.780–0.818) | 0.951 (0.946–0.965) |
| C2R | 723 | 361 | 11.683 | 1.004 (0.999–1.007) | 0.408 (0.401–0.410) | 1.000 (0.983–1.018) | 1.462 (1.417–1.482) | 0.756 (0.628–0.768) | 0.900 (0.891–0.910) |
| R2C | 1,443 | 721 | 14.857 | 1.221 (1.212–1.244) | 0.938 (0.928–0.939) | 0.369 (0.363–0.400) | 0.437 (0.428–0.444) | 0.602 (0.592–0.606) | 0.827 (0.821–0.848) |
| C2R | 1,443 | 721 | 17.918 | 1.228 (1.222–1.255) | 0.779 (0.772–0.782) | 0.288 (0.280–0.307) | 0.392 (0.384–0.399) | 0.493 (0.485–0.497) | 0.719 (0.712–0.772) |
| R2C | 2,049 | 1,024 | 31.839 | 0.993 (0.974–1.017) | 1.141 (1.120–1.154) | 1.204 (1.185–1.217) | 1.576 (1.553–1.609) | 1.336 (1.325–1.356) | 0.878 (0.869–0.902) |
| C2R | 2,049 | 1,024 | 35.578 | 0.979 (0.961–0.995) | 1.024 (1.020–1.033) | 1.091 (1.080–1.114) | 1.461 (1.431–1.494) | 1.207 (1.178–1.214) | 0.810 (0.797–0.827) |
| R2C | 4,097 | 2,048 | 89.727 | 1.143 (1.140–1.152) | 0.679 (0.677–0.683) | 0.835 (0.823–0.843) | 1.177 (1.084–1.196) | 0.706 (0.700–0.724) | 0.708 (0.700–0.716) |
| C2R | 4,097 | 2,048 | 100.802 | 1.174 (1.166–1.188) | 0.600 (0.596–0.601) | 0.729 (0.711–0.737) | 1.048 (1.025–1.053) | 0.599 (0.559–0.612) | 0.642 (0.633–0.651) |

M20 cuts full C2C latency by roughly 20–27% at N=726, 1,452 and 2,904. M21
then reaches 1.22–1.23× speedup for real N=1,443 and 1.13–1.15× at N=4,097
over M20. N=1,442 C2C remains at M18 parity. The largest remaining C2C gap is
N=1,442: about 4.3–4.5 times slower than RustFFT. The other listed complex
lengths are about 2.8–3.4 times slower than RustFFT. Real N=723 remains about
2.3–2.5 times slower than RustFFT. N=1,443 is near RustFFT parity in R2C and
about 1.28 times slower in C2R. N=4,097 is 1.47–1.67 times slower than
RustFFT, while the clean DUCC-native comparison places M21 1.20–1.37 times
slower. N=2,049 remains competitive with RustFFT, DUCC and FFTW; its comparison
with Admiral still favors Admiral. The M21 N=2,049 C2R slowdown versus M20 is 2.9% in the full matrix (paired
M21/M20 ratio 1.029, 95% interval 1.018–1.035). A separate six-pair control
measured 4.0% slower, with a 2.1–5.6% slowdown interval. The M23 M18 comparison
also shows a 2.1% slowdown versus the session-start source. This control result
is disclosed because the M21 change targets other prime lengths.

## Setup, memory and workspaces

All 34 M23 target cells have identical workspace byte counts between M18 and
M21. The general-prime reuse change adds no plan payload, retained allocation
or caller-owned scratch; cached values are stack-local. Across the 34 cells,
M18-to-M21 median plan-construction time and workspace-construction time show
no systematic change. At N=4,097, M20 and M21 plan construction was 93.63 and
93.67 microseconds, respectively; workspace construction was 1.82
microseconds in both builds. At N=2,049, M21 workspace use is 159,856 bytes,
unchanged from M20. The complete per-cell measurements are retained in the
paired CSV summaries.

## Numerical and precision validation

The final f64 executable was compared against native and portable DUCC,
DUCC's native scalar-planner route, FFTW 3.3.10 and Admiral at N=1,443 and
N=4,097 for C2C forward/inverse and R2C/C2R. Each comparison covered 19,391
complex output records (38,782 scalar components) at `rtol=2e-11` and
`atol=2e-11`. The largest absolute differences were `4.547e-13` for native
DUCC, portable DUCC and the DUCC scalar-planner route; `4.547e-13` for FFTW;
and `1.137e-12` for Admiral. All comparisons passed. Exact commands, paths and
reference hashes are in the
[numerical comparison record](../benches/results/m23-final-comparisons/m23-ducc-numerical-comparison.txt)
and [configuration](../benches/results/m23-final-comparisons/m23-ducc-numerical-comparison-config.json). M22's six-pair f32 matrix measures N=723, 1,025, 1,443, 2,049 and 4,097 with
one row and 32 rows. Relative to M20, f32 M21 improves N=1,443 by 11–13% and
N=4,097 by 6–8%, remains near parity at N=723 and N=2,049, and slows N=1,025
R2C by about 5%. Workspace byte counts match M20 at every tested length. RustFFT remains faster at all these f32 lengths. See the [M22 f32 results and
atmospheric coverage](m22-atmospheric-coverage-report.md) and its paired raw
records. The M22 atmospheric screen covers 50 deduplicated longitude lengths from T42
through T4,095: minimum odd, minimum even, existing GL sampling and the
FFT-friendly count. T4,095 was screened at one row only. A separate SHT process
overlapped this one-row screen, so its per-length timings are route diagnostics,
not paired performance claims. M22 also derives CC/F1 meridional profile counts
and resampling lengths from RustSHT call sites; it does not claim a separate
meridional timing matrix.

## Checks and independent review

On the final production source, all-feature debug and release test suites each
passed 103 tests. `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --all-features -- -D warnings` passed. The tests include direct DFT references, analytic cases, f32/f64 paths,
exceptional-value guards, inverse normalization, fallback routes, and shared
plans with independent workspaces. The M23 DUCC/FFTW/Admiral comparisons add
independent full-output checks at the reported lengths; roundtrips are not the
only numerical evidence. The requested final Sol High review found no correctness or numerical blocker. It confirmed that the pass-local caches are initialized before use, the
Hermitian indices and arithmetic are bounded, and transform normalization,
dispatch and fallback behavior remain unchanged. The review found one P3
reporting issue in the N=2,049 C2R slowdown percentage; commit `d3b7413` corrected
the full-matrix and separate-control figures. No additional production change
was requested.

## Remaining work and integration readiness

The primary unresolved cost is C2C N=1,442, which still takes 55–60
microseconds per row transform and is about 4.3–4.5 times slower than RustFFT. The N=726, 1,452 and 2,904 mixed-radix paths improved substantially but remain
about 2.8–3.4 times slower. Real N=723 and N=4,097 also retain meaningful
gaps against RustFFT. These results justify more focused FFT work before using
the crate as a general replacement for the current RustFFT paths. Keep the crate standalone on this branch. It now has a meaningful complete
transform improvement at three complex mixed-radix lengths and two real
general-prime lengths, with numerical and workspace guarantees retained. Later RustSHT integration still needs end-to-end scalar and spin-1 SHT
validation, normalization and endpoint behavior checks, and full grid-level
performance measurements. The existing [standalone integration notes](standalone-readiness.md)
document those steps. This session adds no runtime FFT dependency, threading
dependency, public API or main RustSHT integration.

## Supporting M23 data

All M23 process records, exact configurations and paired CSV summaries are
under [`benches/results/m23-final-comparisons/`](../benches/results/m23-final-comparisons/):

- [M18 six-pair raw capture](../benches/results/m23-final-comparisons/m23-m18-six-pairs.jsonl),
  [summary](../benches/results/m23-final-comparisons/m23-m18-summary.csv) and
  [configuration](../benches/results/m23-final-comparisons/m23-m18-config.json).
- [RustFFT six-pair raw capture](../benches/results/m23-final-comparisons/m23-rustfft-six-pairs.jsonl),
  [summary](../benches/results/m23-final-comparisons/m23-rustfft-summary.csv) and
  [configuration](../benches/results/m23-final-comparisons/m23-rustfft-config.json).
- [Native DUCC retry](../benches/results/m23-final-comparisons/m23-ducc-native-retry-six-pairs.jsonl),
  [summary](../benches/results/m23-final-comparisons/m23-ducc-native-retry-summary.csv)
  and [configuration](../benches/results/m23-final-comparisons/m23-ducc-native-retry-config.json).
- [Portable DUCC six-pair raw capture](../benches/results/m23-final-comparisons/m23-ducc-portable-six-pairs.jsonl),
  [summary](../benches/results/m23-final-comparisons/m23-ducc-portable-summary.csv)
  and [configuration](../benches/results/m23-final-comparisons/m23-ducc-portable-config.json).
- [FFTW six-pair raw capture](../benches/results/m23-final-comparisons/m23-fftw-six-pairs.jsonl),
  [summary](../benches/results/m23-final-comparisons/m23-fftw-summary.csv) and
  [configuration](../benches/results/m23-final-comparisons/m23-fftw-config.json).
- [Admiral six-pair raw capture](../benches/results/m23-final-comparisons/m23-admiral-six-pairs.jsonl),
  [summary](../benches/results/m23-final-comparisons/m23-admiral-summary.csv) and
  [configuration](../benches/results/m23-final-comparisons/m23-admiral-config.json).
- [Interrupted three-pair native DUCC capture](../benches/results/m23-final-comparisons/m23-ducc-native-six-pairs.jsonl)
  with its [capture configuration](../benches/results/m23-final-comparisons/m23-ducc-native-config.json)
  and [preservation note](../benches/results/m23-final-comparisons/m23-ducc-native-partial-note.md). The [M19 report](m19-current-head-performance-report.md), [M20 report](m20-radix11-mixed-pass-report.md),
[M21 report](m21-real-prime-pass-report.md) and [M22 report](m22-atmospheric-coverage-report.md)
retain the intermediate evidence and rejected experiments.
