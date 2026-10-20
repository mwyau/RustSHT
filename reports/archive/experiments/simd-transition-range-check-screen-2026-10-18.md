# SIMD transition range checks were slower in the measured workloads

The candidate replaced two scalar scans of recurrence lanes with Pulp SIMD
reductions at the scalar synthesis transition decisions. The per-pair current
value check, exponent rebasing, recurrence and scaled fallback were unchanged.
The candidate patch is in [`candidate.patch`](../../data/simd-transition-range-check-screen-2026-10-18/candidate.patch).
Apply it to `ff7c91d4` with `git apply --unidiff-zero`.

The comparison used baseline commit `ff7c91d4` and the same source with that
patch applied. Each process ran `examples/m1b_bench` in release mode with one
warm-up, five timed synthesis calls, and a reused plan and workspace. The
workloads used f64, GL, full `mmax`, and `phi0=0.37`. The host was an AMD Ryzen
9 5950X (x86_64, 16 cores/32 threads, AVX2/FMA, 96 GiB RAM); software was Rust
1.99.0, Pulp 0.22.3, Rayon 1.12.0, and RustFFT 6.4.1. No host-pressure protocol
ran.

| Workload | Baseline samples (ms) | Candidate samples (ms) | Median change |
|---|---|---|---:|
| T2047 GL 2048x4097 q1 | `1202.723, 1203.485, 1204.018, 1204.860, 1215.911` | `1267.062, 1273.278, 1275.184, 1262.410, 1265.448` | +5.2% |
| T1279 GL 1280x2561 q8 | `143.813, 142.437, 142.088, 143.201, 143.490` | `150.208, 161.318, 150.261, 151.444, 150.535` | +5.1% |

The candidate sample ranges did not overlap either baseline range. These
single-process screens do not establish a general performance result, but
both measured workloads were slower, so the SIMD reductions were rejected.
No DUCC timing was collected. The focused `synthesis_fast::tests` passed with
7 tests and 1 ignored; `cargo fmt --all -- --check` passed. The exact benchmark
commands are in [`run-screen.sh`](../../data/simd-transition-range-check-screen-2026-10-18/run-screen.sh).
