# Scaled-prefix SIMD precheck T1023 baseline stop v1

## Result

The first T1023 screen process was the clean baseline. RustSHT met the fixed
within-process stability limits, but DUCC failed the CV limit. The process is
inconclusive, so the screen stopped before launching the candidate. The
T2047 baseline/candidate cell did not run, and no sample was retried or
removed.

## Frozen configuration

- Workload: GL f64 scalar `STANDARD` synthesis, plan scope, q1, full `mmax`,
  `phi0=0.37`, `ntheta=1024`, `nphi=2048`.
- RustSHT baseline source: `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`;
  release extension SHA-256:
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- DUCC: version 0.42.0, source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`; extension SHA-256:
  `07fe9333f8edc80dab0285d51debb5e45979d3754b4ac9033712712668e28468`.
- Harness SHA-256:
  `e7ffbe889922cc4da1b1ece63bc24bf264603c7373adb195abde5d705bb51c07`.
- Rust/Cargo 1.99.0, Python 3.11.15, NumPy 2.4.6. Hardware and protocol
  details are in the [screen protocol](scaled-prefix-simd-precheck-q1-screen-v1-protocol.md).

## Timing samples

All values are milliseconds. One warm-up preceded each five-call series.

| Backend | Samples | Median | CV | First-three/final-two drift | Result |
|---|---|---:|---:|---:|---|
| RustSHT | 296.072, 300.656, 299.620, 300.424, 299.923 | 299.923 | 0.559% | 0.185% | Pass |
| DUCC | 67.555, 57.312, 53.996, 64.789, 53.057 | 57.312 | 9.807% | 2.811% | Fail: CV |

The timing series do not establish a baseline/candidate speed comparison.

## Correctness, memory, and run validity

The RustSHT output had relative-L2 error `1.182e-13`, RMS error
`2.795e-15`, and maximum absolute error `1.556e-13` against DUCC. Peak RSS
was 178.1 MiB.

The prescribed CPU-pressure checks passed before and after the process. A
monitor sample entered the warning band during the run. Process and worker
placement were pinned consistently to one allowed logical CPU; cgroup
throttling did not increase. No build, benchmark, or clear competing CPU
workload was observed. Frequency and context-switch telemetry was reviewed
locally but did not isolate the cause of DUCC's variation. Raw PSI, process,
thread, frequency, context-switch, and GNU time captures remain local and
untracked.

## Decision

Stop the frozen screen. Do not launch the T1023 candidate or T2047 screen
process, and do not retry this baseline cell. The SIMD precheck source remains
isolated and unaccepted. The T2047 baseline-versus-baseline control passed,
but this T1023 baseline failure prevents using that control to authorize the
candidate matrix. Any later performance screen needs a separately declared
protocol before new measurements. The existing T511 control and q8 and
scalar/spin-1 regression gates also remain outstanding.
