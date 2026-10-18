# Scaled-prefix SIMD precheck T2047 baseline stability control v1

## Result

The fixed T2047 baseline-versus-baseline control passed. It used the same
clean RustSHT extension in two fresh processes, one warm-up and five timed
transformations per backend in each process. The result validates the host
and timing protocol for the next one-pair screen; it does not measure or
accept the SIMD precheck candidate.

## Frozen configuration

- Workload: GL f64 scalar `STANDARD` synthesis, plan scope, q1, full `mmax`,
  `phi0=0.37`, `ntheta=2048`, `nphi=4096`.
- RustSHT baseline source: `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`;
  release extension SHA-256:
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- DUCC: version 0.42.0, source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`; extension SHA-256:
  `07fe9333f8edc80dab0285d51debb5e45979d3754b4ac9033712712668e28468`.
- Harness `examples/python_perf_baseline.py` SHA-256:
  `e7ffbe889922cc4da1b1ece63bc24bf264603c7373adb195abde5d705bb51c07`.
  Fresh-process runner SHA-256:
  `430fe2ebb6cac4a6be30decf68687fb1fddf2f5d88dcaba00d79d266da940d7c`.
- Rust/Cargo 1.99.0, Python 3.11.15, NumPy 2.4.6. Host: AMD Ryzen 9
  5950X, x86-64 AVX2/FMA, 16 physical and 32 logical cores, 96 GiB RAM,
  Ubuntu 26.04.1 LTS.
- One worker was pinned to one allowed logical CPU. Both benchmark processes
  and their worker threads followed that placement. No global ISA flag or
  `target-cpu=native` was used.

## Timing samples

All values are milliseconds. The fixed order was baseline A, then baseline B.

| Process | Backend | Samples | Median | CV | First-three/final-two drift |
|---|---|---|---:|---:|---:|
| A | RustSHT | 3291.940, 3298.677, 3298.694, 3296.015, 3287.094 | 3296.015 | 0.135% | 0.216% |
| A | DUCC | 372.927, 372.198, 372.070, 375.016, 377.115 | 372.927 | 0.518% | 1.037% |
| B | RustSHT | 3293.676, 3297.618, 3301.946, 3301.540, 3292.887 | 3297.618 | 0.115% | 0.012% |
| B | DUCC | 372.704, 373.926, 372.307, 374.139, 373.517 | 373.517 | 0.189% | 0.301% |

Each series passed CV `<=5%` and drift `<=5%`. The RustSHT A/B median
ratio was 1.0005; the DUCC A/B median ratio was 1.0016.

## Correctness, memory, and run validity

Both processes reported relative-L2 error `3.015e-13`, RMS error
`1.426e-14`, and maximum absolute error `1.110e-12` against DUCC. Peak RSS
was 618.6 MiB for A and 610.7 MiB for B.

The prescribed CPU-pressure gate passed before and after the pair. One
during-run monitor sample entered the protocol's warning band.
The process and worker placement checks passed. Local process and thread
placement, frequency, context-switch, PSI, and cgroup records were reviewed;
no competing build or benchmark, cgroup throttling, or clear interference
correlated with the timings was observed. Dynamic frequency changes were
present, while all four timing series remained stationary. Raw telemetry and
GNU time captures remain local and untracked.

## Decision

The baseline stability diagnostic passes, so the predeclared single-pair
baseline/candidate cells at T1023 and T2047 may proceed. No candidate timing
has been collected in this screen. A favorable one-pair result remains
screening evidence only; the six-pair q1 matrix, T511 controls, q8 controls,
and complete scalar/spin-1 regression matrix are still required before any
acceptance decision. See the [fixed protocol](scaled-prefix-simd-precheck-q1-screen-v1-protocol.md).
