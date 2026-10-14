# Spin-1 staged synthesis factorization

The staged spin-1 scalar-conversion synthesis path now combines each paired
degree coefficient before multiplying by its scaled Legendre value. For each
channel and latitude lane, it evaluates

\[
  (c_{\rm even} + c_{\rm odd}\cos\theta)\,Y_{lm}(\theta)
\]

with one scaled complex multiply. The previous path used separate scaled
multiplies for the even and odd terms. The ordinary register-resident fused
kernel already used the combined factor; this change brings the scaled-to-normal
transition path to the same arithmetic structure. The finite-range checks
still reject a nonzero coefficient sum that rounds to zero or a subnormal
value, and the caller reruns the general scaled recurrence when the staged
kernel declines the block.

## Measurement

The baseline was commit `d19af76` on `main`; the candidate changed only
`src/legendre/scalar_paired.rs` and its focused regression in
`src/legendre/scalar.rs`. Native measurements used `examples/m1b_bench`, release
builds with Rust 1.99.0 / LLVM 23.1.1, `opt-level=3`, 16 codegen units, LTO off,
and portable Pulp runtime dispatch. The host was an x86-64 CPU running
Linux T511, T720 and T1023 used exact GL dimensions
`512x1025`, `721x1443` and `1024x2049`, full `mmax`, `phi0=0.37`, deterministic
f64 E/B coefficients, and one reused plan/workspace. Each fresh process warmed
the plan and recorded five calls. Five baseline/candidate process pairs were
alternated per band and thread count; q1 was pinned to one CPU core and q8 to 8 pinned CPU cores.

| Band | Threads | Baseline median (ms) | Candidate median (ms) | Median paired change |
| --- | ---: | ---: | ---: | ---: |
| T511 | 1 | 54.011 | 46.799 | −13.57% |
| T511 | 8 | 18.185 | 18.050 | −0.74% |
| T720 | 1 | 125.406 | 109.864 | −12.43% |
| T720 | 8 | 41.695 | 39.834 | −5.38% |
| T1023 | 1 | 402.017 | 346.801 | −13.07% |
| T1023 | 8 | 153.282 | 137.557 | −10.19% |

Every q1 pair improved. The paired q8 ranges crossed zero at T511 and T720;
the T1023 q8 range was −12.75% to −9.25%. The whole-transform q1 gains are
therefore repeatable through T1023. Raw five-call samples are in
[`the T511/T720 CSV`](data/current-kernel-baseline-2026-10-14/stage-factor-process-samples-t511-t720.csv)
and [`the T1023 CSV`](data/current-kernel-baseline-2026-10-14/stage-factor-process-samples-t1023.csv). Representative commands were `cargo build --release --example m1b_bench`,
`taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 511 1 synthesis GL STANDARD 512 1025 511 requested-only`, and the same command with `taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}"` and the thread argument set to `8`. The paired harness alternated archived baseline and candidate binaries and wrote the retained CSV samples. Python comparisons used
`RUSTSHT_PERF_RAW=1 RAYON_NUM_THREADS=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 511 1 1 GL 1 plan synthesis both STANDARD`; q8 used `RAYON_NUM_THREADS=8`, worker-sized CPU affinity, and the thread argument `8`. Each process used five timed method calls after one warm-up; three fresh processes were recorded per case.

spin-1 synthesis operations before the change and 4.708 billion after it
(−11.4%). The profile does not report CPU cycles, and its timing is not used as
the speedup estimate. Hardware performance counters were unavailable with
`ducc0==0.41.0`, matching grid, coefficients, phase and thread count. RustSHT
and DUCC relative-L2 differences were `6.697e-14` at T511 q1,
`4.715e-14` at T511 q8, and `9.123e-14` at T720 q1/q8. Candidate Python-plan
medians were 48.413/18.105 ms at T511 q1/q8 and 111.604/42.764 ms at T720;
DUCC medians were 22.276/2.557 ms and 52.166/6.357 ms, respectively. These
Python method timings include binding work and describe the remaining gap;
they are not used to attribute the native kernel improvement. Per-process
samples and error summaries are in
[`stage-factor-python`](data/current-kernel-baseline-2026-10-14/stage-factor-python/).

## Numerical checks and review

The scalar and runtime-dispatched paths pass a focused exact-cancellation test:
the staged path declines the block, and the separate scaled products cancel to
zero. The ordinary transition comparison, mixed-lane and tail checks, and
extreme-coefficient fallback passed. Release DUCC comparisons passed for spin-1
T511 across GL/CC/F1, sparse spin synthesis at T1023, and sparse spin synthesis
and sectoral formulas at T2047. No tolerances changed. The read-only Sol High review found no blocker in arithmetic, fallback,
workspace ownership, SIMD tails or measured speedups. It requested a
near-cancellation regression; the exact-cancellation test above covers that
fallback boundary. Full checks on the candidate passed: formatting, Clippy,
`cargo test --workspace --all-features` (153 Rust tests and 10 f32 integration
tests passed; 9 ignored), `cargo test -p rustsht --no-default-features` (131
Rust tests and 10 f32 tests passed; 7 ignored), and `uv run pytest python/tests`
(20 passed). The T511/T1023/T2047 release regressions also passed. A separate table-threshold-only trial temporarily lowered the scalar
scaled-transition threshold from T1023 to T511. It did not improve steady-state
scalar synthesis: the T511 medians changed by +0.8% at q1 and +2.7% at q8; T720
was flat at q1 and +0.9% at q8. The threshold was restored. No table policy
change follows from that trial.
