# Prefix lane-pass fusion q1 matrix protocol v1

## Scope and stop rule

This protocol advances the fused normalization/accumulation candidate only
after its valid one-pair T2047 screen against the scaled-prefix accumulator.
It compares the fused source with the clean production baseline at T1023 and
T2047, then runs the T511 scalar control. It does not accept the candidate or
authorize q8 or spin-1 measurements by itself.

Run six fresh-process pairs per cell. Alternate process order: baseline then
candidate in pair 1, candidate then baseline in pair 2, continuing through
pair 6. Start with T1023. Run T2047 only if every T1023 process passes its
validity and numerical gates. Run the T511 control after both high-resolution
cells pass. Stop a cell at its first invalid process. Do not retry, replace,
or omit a sample. Review each process before launching the next.

## Frozen source and benchmark configuration

- Clean baseline: scalar worktree source at original M6 baseline revision
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; Python extension SHA-256
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Candidate: tile-128 integration, coefficient-safety cache, scaled-prefix
  accumulator, and lane-pass fusion. Its integration patch SHA-256 is
  `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`,
  coefficient-safety cache patch SHA-256 is
  `2ee6fc08997a3f373412b6407164b16a81b36402df75931a5348682ca47f03cd`, and
  scaled-prefix accumulator patch SHA-256 is
  `3a75e88d6c6d0640005caaa486e29d8fcbb8ecdd08c5419da9d015f39d736873`. The
  fusion patch is
  [lane-pass fusion patch](prefix-normalization-accumulation-fusion.patch).
  Candidate `synthesis_fast.rs` SHA-256 is
  `eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407`; its
  `scalar.rs` SHA-256 is
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
  Build and freeze the release extension before acquisition, and record its
  SHA-256 in the result report. Use identical Rust 1.99.0 release settings for
  the variants.
- Harness: [`python_perf_baseline.py`](../../m6-m1-scheduling/python_perf_baseline.py),
  SHA-256 `1f1bdfa8f2cf886db3874b2ae9160d9c68f89f0a4026a19bc9ada47948606fb8`.
  This harness times reusable Rust and DUCC plans with caller-provided output
  buffers and the established SHTns 3.7.5 adapter.
- References: DUCC 0.42.0 at source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`, extension SHA-256
  `1218502fa03116cd14578fe73f716f5a1d88db74d296359cc9e8c27e04047d1e`;
  SHTns 3.7.5 at source revision
  `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`, library SHA-256
  `641195ac574ba02078f3238b357caecf0e268b3f4ef7eba1d87898197fcc888f`.
- Workload: GL f64 scalar `STANDARD` synthesis, full `mmax`, `phi0=0.37`,
  q1. T511 is `512x1024`, T1023 is `1024x2048`, and T2047 is `2048x4096`.
  Each fresh process uses one warm-up and five timed samples per backend.
  Select `scope=into`, `operation=synthesis`, `backend=all`, and `runs=1`.
  RustSHT, DUCC, and SHTns setup and output allocation follow the harness
  definitions; retain that scope for every variant.
- Use one fixed allowed CPU for all processes, `RAYON_NUM_THREADS=1`, one
  DUCC thread, one SHTns thread, and `OPENBLAS_NUM_THREADS=1`. Build all
  binaries before acquisition. Do not compile or run another benchmark while
  a process is active.
- Hardware and software: AMD Ryzen 9 5950X, x86-64 with AVX2/FMA, 16 physical
  and 32 logical cores, 96 GiB RAM; Ubuntu 26.04.1, Rust 1.99.0, Python
  3.11.15, NumPy 2.4.6. Record the selected affinity policy and relevant
  library versions; keep host-specific identifiers local.

## Validity and numerical gates

- Reject preflight or postflight CPU PSI `some avg10` above 2.00%. Values from
  0.50% through 2.00% are warnings. Review affinity, worker placement,
  frequency, involuntary context switches, cgroup pressure, and concurrent
  CPU-intensive work. Clear interference makes a process inconclusive even
  below the PSI rejection threshold.
- Every RustSHT, DUCC, and SHTns five-call series must have population CV at
  most 5% and first-three/final-two median drift at most 5%. Do not remove
  samples or change these limits.
- Before timing, the harness compares RustSHT and SHTns outputs against DUCC.
  Candidate synthesis must remain within relative-L2 `2e-11` and maximum
  absolute error `2e-12` for every process. SHTns output must meet the same
  reference limits.
- Save raw stdout, GNU-time, PSI, process, frequency, affinity, and cgroup
  captures in a local directory outside the repository. Commit only
  structured timing samples, error metrics, aggregate peak RSS, gate outcomes,
  and conclusions.

## q1 decision rule

Compute six paired Rust candidate/baseline median ratios per bandlimit. Use
50,000 bootstrap resamples with seed `20261017`, median ratio per resample,
and linearly interpolated 2.5th and 97.5th percentiles. The high-resolution
screen passes only if one of T1023 or T2047 has median ratio at most `0.90`
and interval upper bound below `1.00`, while the other interval upper bound
is at most `1.02`. The T511 control interval upper bound must also be at most
`1.02` before q8 or spin-1 regression measurements.

The previous accumulator matrices and the new fusion screen belong to
different source variants; do not combine their samples or ratios. The
candidate remains experimental until this protocol and the subsequent q8 and
regression controls pass.
