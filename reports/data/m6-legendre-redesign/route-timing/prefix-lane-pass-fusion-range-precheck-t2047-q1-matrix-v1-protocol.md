# Range-precheck T2047 q1 six-pair matrix protocol v1

## Purpose and scope

This matrix tests whether the T2047 q1 improvement from the valid one-pair
[screen](prefix-lane-pass-fusion-range-precheck-t2047-q1-screen-v1.md)
repeats across independent processes. It uses six fresh baseline/candidate
pairs. The screen pair is excluded from every matrix statistic. The matrix
does not accept the candidate or authorize q8, spin-1, or production routing.

Run pair 1 baseline then candidate, pair 2 candidate then baseline, and
alternate that order through pair 6. Review each process before starting the
next. Stop at the first invalid process, retain every acquired sample, and do
not retry, replace, or omit a process. A process invalidates its pair if any
RustSHT, DUCC, or SHTns five-sample series fails the fixed timing gates, a
numerical comparison fails, preflight or postflight PSI exceeds the threshold,
or monitoring shows clear interference.

## Frozen source, binaries, references, and workload

- Clean production baseline source revision:
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; extension SHA-256
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Isolated candidate `synthesis_fast.rs` SHA-256:
  `07c41fe4ad285e59d7ed180931d145f72841d7adc9fe889a20a755837bf48b06`;
  `scalar.rs` SHA-256:
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`;
  extension SHA-256:
  `58c029273cb3aedd0bd1350fe2dde22fe703559c819ace8e90ce52d06e25d562`.
- Use the harness base SHA-256
  `1f1bdfa8f2cf886db3874b2ae9160d9c68f89f0a4026a19bc9ada47948606fb8`,
  patched by
  [`prefix-fusion-explicit-warmup-v2.patch`](prefix-fusion-explicit-warmup-v2.patch)
  (SHA-256
  `dba74d632d2abdcc20c73e90a8b2538efbd0f298d797488dde6d8bf427e6dc45`).
  The patched script SHA-256 is
  `61b640657990bebf80a1d19a1cd4cd0e99e2adcb7234161ea9b984b6971669ab`.
- DUCC 0.42.0 source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`, extension SHA-256
  `1218502fa03116cd14578fe73f716f5a1d88db74d296359cc9e8c27e04047d1e`.
  SHTns 3.7.5 source revision
  `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`, library SHA-256
  `641195ac574ba02078f3238b357caecf0e268b3f4ef7eba1d87898197fcc888f`.
- Build settings: Rust/Cargo 1.99.0, release profile, all features. Runtime:
  Python 3.11.15 and NumPy 2.4.6. Hardware: AMD Ryzen 9 5950X, x86-64, AVX2
  and FMA, 16 physical and 32 logical cores, 96 GiB RAM; Ubuntu 26.04.1.
- Workload: GL f64 scalar `STANDARD` synthesis, `lmax=mmax=2047`, `2048x4096`,
  `phi0=0.37`, q1, `scope=into`, `operation=synthesis`, `backend=all`,
  `runs=1`. Pin both builds to the same single allowed CPU and set
  `RAYON_NUM_THREADS=1`, with DUCC, SHTns, and BLAS limited to one thread.
  Set `RUSTSHT_PERF_RAW=1` and `RUSTSHT_BENCH_EXTRA_WARMUPS=3`. Each backend
  gets five fixed untimed calls and five timed complete transformations.

Build and verify both release extensions before starting acquisition. Do not
compile or run another benchmark during the matrix. Retain raw stdout, GNU-time
output, PSI, affinity, frequency, process/thread placement, context-switch,
and cgroup captures locally and untracked. Record structured timing samples,
aggregate peak RSS, numerical errors, validity outcomes, and conclusions in
the result report. The unusually slow untimed NumPy copy diagnostics in the
screen result are out-of-interval observations; record them again and use
them only as contextual evidence when reviewing possible interference.

## Validity, numerical, and decision rules

- CPU PSI `some avg10` above 2.00% in preflight or postflight rejects the
  process; readings from 0.50% through 2.00% are warnings. Clear evidence of
  interference invalidates a process even below the PSI limit.
- Each RustSHT, DUCC, and SHTns series must have population CV at most 5% and
  first-three/final-two median drift at most 5%.
- RustSHT and SHTns must agree with DUCC within relative-L2 `2e-11` and
  maximum absolute error `2e-12` in every process.
- Retain all six paired ratios. If all six pairs pass, calculate the median
  candidate/baseline RustSHT ratio and a 95% bootstrap interval using 50,000
  resamples, seed `20261017`, and linearly interpolated quantiles. This T2047
  matrix demonstrates repeatable q1 benefit only if the median ratio is at
  most `0.90` and the interval upper bound is below `1.00`.
- Passing authorizes only subsequent bounded scalar control measurements.
  T1023/T511, q8, spin-1, complete regression gates, and Clippy cleanliness
  remain required before production acceptance.
