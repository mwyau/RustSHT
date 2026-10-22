# Plan 17 Milestone A — native f32 baseline

This report records the current-source f32 baseline before any Plan 17 kernel
change. It is a measurement checkpoint, not a parity claim.

## Sources and build

- RustSHT baseline: `dcb3836603aaee0133b544fa4510e1da597dca62`.
- DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1; Cargo release profile, no `RUSTFLAGS` or target CPU override. Runtime Pulp selected V3 (AVX2/FMA), with eight f32 lanes.
- Host: AMD Ryzen 9 5950X. q1 was pinned to CPU 0; q8 to CPUs 0–7 in one L3 domain. DUCC SHT sources used GCC 15.2 with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`. The standalone driver was built separately with `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3` so validation retains strict NaN/Inf checks.

The native Rust example uses public `ShtPlan<f32>` APIs and reused workspaces;
the DUCC driver uses the public native transform API. Both read the same
deterministic fixtures, reset outputs, and validate repeat/reset consistency.
Fresh Rust/DUCC processes were interleaved in three pairs. Calls below 100 ms
used ten calibrated batches per process (30 batches per library and case);
long calls used five individually timed repetitions per process (15 per
library and case). Setup, first call, warm call, and steady-state batches are
separate fields. CPU affinity is recorded in the JSON. The runner also sampled
frequency and scheduling pressure during each process; those transient host
snapshots are intentionally omitted from the published machine-readable data.
The run showed varying host pressure, so the distributions are observed host
timings and no samples were discarded.

## Coverage and results

The screened matrix contains 112/112 complete comparisons:

- GL: T127, T255, T511, T720 and T1023, with full order and selected quarter
  order; even-smooth longitude throughout and odd-minimum controls at T127 and
  T255.
- CC and F1: T255 full-order controls.
- Scalar spin 0 and vector spin 1; analysis and synthesis; q1 and q8.
- All cases use `phi0=0.23`. There were 56 q1 and 56 q8 cases. Every output was
  finite and every repeated/reset check passed.
- The saved per-process timing arrays contain 2,805 Rust and 3,345 DUCC timed
  observations across the matrix. Long Rust calls use 15 individually timed
  invocations; faster cases use 30 calibrated batches per implementation.

Median warmed latency and Rust/DUCC ratio for the full-order even-longitude GL
scalar cases:

| T | Operation | Threads | Rust ms | DUCC ms | Ratio | Cross-library relative L2 |
|---:|---|---:|---:|---:|---:|---:|
| 127 | Analysis | 1 | 4.111 | 0.246 | 16.62× | 2.04e-6 |
| 127 | Analysis | 8 | 0.574 | 0.072 | 8.04× | 2.04e-6 |
| 127 | Synthesis | 1 | 8.357 | 0.285 | 29.46× | 7.3e-6 |
| 127 | Synthesis | 8 | 2.544 | 0.072 | 35.36× | 7.3e-6 |
| 255 | Analysis | 1 | 34.313 | 1.156 | 29.71× | 5.0e-6 |
| 255 | Analysis | 8 | 4.571 | 0.218 | 20.91× | 5.0e-6 |
| 255 | Synthesis | 1 | 71.127 | 1.459 | 48.46× | 2.8e-5 |
| 255 | Synthesis | 8 | 18.054 | 0.231 | 78.55× | 2.8e-5 |
| 720 | Analysis | 1 | 667.197 | 14.001 | 47.63× | 1.2e-5 |
| 720 | Analysis | 8 | 87.849 | 2.049 | 42.74× | 1.2e-5 |
| 720 | Synthesis | 1 | 1403.086 | 18.881 | 74.44× | 1.7e-4 |
| 720 | Synthesis | 8 | 476.150 | 2.663 | 178.66× | 1.7e-4 |
| 1023 | Analysis | 1 | 1763.455 | 32.376 | 54.51× | 1.7e-5 |
| 1023 | Analysis | 8 | 231.511 | 4.630 | 50.11× | 1.7e-5 |
| 1023 | Synthesis | 1 | 3732.001 | 47.646 | 78.33× | 3.0e-4 |
| 1023 | Synthesis | 8 | 647.631 | 6.629 | 97.97× | 3.0e-4 |

Across the five full-order GL degrees, the median case ratio by operation and
thread count was:

| Spin | Operation | q1 median ratio (range) | q8 median ratio (range) |
|---:|---|---:|---:|
| 0 | Analysis | 44.00× (16.62–54.51×) | 37.42× (8.04–50.11×) |
| 0 | Synthesis | 67.37× (29.46–78.33×) | 92.56× (35.36–178.66×) |
| 1 | Analysis | 71.20× (30.71–93.14×) | 63.84× (18.47–87.86×) |
| 1 | Synthesis | 60.72× (24.17–75.06×) | 61.45× (24.53–126.33×) |

These unexpectedly large current-source gaps need source-level explanation;
the ratios are not evidence of an isolated f32 regression because no matched
earlier-source f32/DUCC baseline exists. At T1023 the largest observed
cross-library error was relative L2 `2.981e-4` and max absolute `2.264e-3`, in
spin-0 synthesis q8. This is a cross-library difference, not a failed
Rust-internal correctness check. The existing release test suites below passed
without tolerance changes. High-degree independent basis and adjoint coverage
was not exhaustive in this benchmark, so the cross-library errors should not
be generalized beyond these deterministic inputs.

## DUCC precision path and stage attribution

The pinned DUCC implementation accepts f32 maps and coefficients, but the
scalar SHT recurrence and accumulation use `double`/`complex<double>`
intermediates (`Tv` is a four-lane double vector on AVX2); conversion back to
the public scalar type occurs at output boundaries. This differs from Rust's
Pulp V3 f32 recurrence, which uses eight f32 lanes. It is an important numeric
and instruction-width distinction, though the measurements alone do not
quantify its performance contribution.

A test-only phase probe on GL T720, `nphi=1470`, `phi0=0.23` attributed
approximately 98.8–99.9% of full-transform latency to Legendre recurrence and
coefficient accumulation. Representative q8 results were:

| Spin | Operation | Wall ms | Recurrence + accumulation ms | Other measured phases |
|---:|---|---:|---:|---:|
| 0 | Analysis | 81.35 | 80.49 | FFT 0.58; preparation/packing about 0.20 |
| 0 | Synthesis | 473.36 | 472.90 | inverse FFT/writeback 0.40 |
| 1 | Analysis | 296.56 | 295.30 | FFT 0.99 |
| 1 | Synthesis | 650.83 | 649.67 | inverse FFT 0.80 |

The q1 probes had the same attribution: scalar analysis 608.92 ms total with
605.07 ms in recurrence/accumulation; scalar synthesis 1363.85 ms total with
1361.68 ms there; spin-1 analysis 2203.67 ms total with 2197.38 ms there; and
spin-1 synthesis 2463.96 ms total with 2459.52 ms there. Phase totals matched
wall measurements within roughly 0.01–0.04 ms. This points Plan 17 Milestone B
toward recurrence/dataflow and numeric representation, not FFT work. Linux
hardware performance counters were unavailable (`perf_event_paranoid=4`); the
host setting was left unchanged.

## Artifacts and reproduction

- [Consolidated raw results JSON](data/plan17-f32-native-baseline-2026-10-22/results/results.json)
- [Consolidated case CSV](data/plan17-f32-native-baseline-2026-10-22/results/results.csv)
- [Coverage and validation audit](data/plan17-f32-native-baseline-2026-10-22/results/coverage-audit.json)
- [Rust native worker](../examples/ducc_f32_full_transform_parity.rs)
- [DUCC CMake project](data/plan17-f32-native-baseline-2026-10-22/CMakeLists.txt)
- [DUCC native driver](data/plan17-f32-native-baseline-2026-10-22/ducc_f32_full_transform_parity.cpp)
- [Python orchestration](../scripts/benchmark_ducc_f32_native.py)

Verification on the baseline source passed `cargo fmt --all --check`, strict
Clippy for all targets with and without default features, and release tests:
all features passed 269 unit, 4 extreme-value, 11 f32 and 1 workspace test
(36 unit and 1 f32 test ignored); no-default-features passed 219 unit, 4
extreme-value, 11 f32 and 1 workspace test (17 unit and 1 f32 test ignored).
The dedicated ignored T720 phase-profile test also ran successfully. The
opt-in pinned-DUCC T1023 legacy f32 fixture test remains ignored; the native
DUCC cases above provide the current-source comparison instead. Python syntax
compilation and the benchmark help command passed. `perf` hardware counters were
unavailable under the host's existing `perf_event_paranoid=4` policy; it was
not changed.

The saved JSON contains the per-case, per-process batch distributions, first
call/warmup/setup durations, error metrics and backend/affinity details. The
runner captures CPU frequency and scheduling pressure locally; only qualitative
host observations are included in this report. Reproduction requires the pinned DUCC source checkout, then
build the driver with the included CMake project and invoke the Python runner
with the same q1/q8 CPU lists and `--process-pairs 3 --target-ms 25` options.
For example:

```sh
cmake -S reports/data/plan17-f32-native-baseline-2026-10-22 \
  -B build/ducc-f32-plan17 \
  -DDUCC_SRC="$DUCC_SRC"
cmake --build build/ducc-f32-plan17 --parallel
cargo build --release --example ducc_f32_full_transform_parity --features parallel
python3 scripts/benchmark_ducc_f32_native.py \
  --rust-binary target/release/examples/ducc_f32_full_transform_parity \
  --ducc-binary build/ducc-f32-plan17/ducc_f32_full_transform_parity \
  --outdir benchmark-output/plan17-f32 \
  --scratch-dir benchmark-scratch/plan17-f32 \
  --process-pairs 3 --target-ms 25 --q1-cpu 0 --q8-cpus 0,1,2,3,4,5,6,7
```

Plan 17 Milestone A is complete. The next Plan 17 task is Milestone B, beginning
with a source-grounded explanation of the recurrence/accumulation gap and a
f32-specific candidate only if that analysis identifies a concrete mechanism.
