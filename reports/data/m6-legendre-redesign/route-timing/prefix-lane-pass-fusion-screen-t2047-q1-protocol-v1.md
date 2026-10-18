# Prefix normalization and accumulation fusion T2047 q1 screen protocol v1

## Purpose and stop rule

This is a one-pair screen of a source-level fusion in the isolated tile-128
scaled-prefix accumulator. It tests whether normalizing each recurrence lane
immediately before accumulating that lane can remove a second active-lane
walk. It changes no recurrence coefficients, coefficient thresholds, prefix
contributions, or fallback semantics. The implementation can delay the
ordinary SIMD transition by one pair on some tiles.

Run one frozen accumulator process followed by one fused process. Each process
performs one warm-up and five timed complete scalar synthesis transforms. Stop
if either process fails a validity check, if the map comparison fails, or if
the fused median is not at least 3% lower. Do not retry or replace a process.
Passing this screen authorizes a separate six-pair q1 matrix; it does not
accept the kernel or authorize q8, spin-1, or production routing.

## Frozen source and workload

- The accumulator base is the tile-128 integration patch
  `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`,
  coefficient-safety cache patch
  `2ee6fc08997a3f373412b6407164b16a81b36402df75931a5348682ca47f03cd`,
  and scaled-prefix accumulator patch
  `3a75e88d6c6d0640005caaa486e29d8fcbb8ecdd08c5419da9d015f39d736873`.
  The resulting `synthesis_fast.rs` SHA-256 is
  `edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`.
- The fused variant applies
  [`prefix-normalization-accumulation-fusion.patch`](prefix-normalization-accumulation-fusion.patch),
  SHA-256 `fe8c9cc2d049e2398cd731183f3b9a388f3db1061317f1766bbd3fbd30c4e0bb`.
  Its `synthesis_fast.rs` SHA-256 is
  `eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407`;
  its `scalar.rs` SHA-256 is
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
- The shared Rust harness is
  [`prefix-lane-pass-fusion-bench.rs`](prefix-lane-pass-fusion-bench.rs),
  SHA-256 `4d76eeed1909adf665e8ec260544030183eec5bc42bf7cef53e836f40fdda364`.
  Copy it unchanged to `examples/m6_prefix_fusion_bench.rs` in both source
  trees. Compile each source with the same Rust 1.99.0 release configuration.
- Workload: GL f64 scalar synthesis, T2047, full `mmax=2047`, grid
  `2048x4096`, `phi0=0.37`, one Rayon worker. The harness measures five full
  `ShtPlan::synthesis_into` calls after one warm-up and requires each timed
  output to match that process's warm-up map bit for bit. Plan construction,
  workspace construction, coefficient creation, and map serialization are
  outside the timed intervals.
- Run the two processes sequentially on the same single-CPU affinity mask,
  accumulator first, fused candidate second. Do not compile while either
  process is running. Save raw maps and raw host/process captures locally;
  commit only the timing samples, numerical comparison, aggregate memory,
  gate outcomes, and conclusions.

## Validity and correctness gates

- Reject preflight or postflight CPU PSI `some avg10` above 2.00%; 0.50%
  through 2.00% is a warning. Keep exact host readings local.
- Each five-call series must have population CV at most 5% and first-three /
  final-two median drift at most 5%. Review actual CPU affinity, worker
  placement, frequency, involuntary context switches, cgroup pressure, and
  competing CPU-intensive work. Clear interference makes the process
  inconclusive below the PSI rejection threshold.
- Compare candidate and accumulator output maps using maximum absolute error
  at most `2e-12` and relative-L2 error at most `2e-11`. The accumulator
  baseline previously matched DUCC 0.42.0 with relative-L2 `3.015e-13` and
  maximum absolute error `1.110e-12` at T2047. This screen does not time DUCC
  or claim a new direct DUCC error measurement; a qualifying candidate must
  pass direct DUCC comparisons in the next q1 matrix.
- Advance only if the fused / accumulator Rust median ratio is at most `0.97`
  and both timing series pass every fixed gate. No q8 or regression claim is
  allowed from this screen.

## Prior route evidence

The uninstrumented accumulator profile timed 715.931 ms in the pre-suffix
transition and 543.517 ms in the ordinary SIMD suffix. Its event profile
recorded 1,034,964 prefix normalization calls over 109,490,856 lane visits,
with 706,324 exponent changes. The fused route-count diagnostic passed its
bitwise warm-up check and measured 111,390,956 prefix lane-pairs, compared
with 110,444,000 for the accumulator. That 0.86% increase reflects the
transition delay; route counts alone do not establish a speed result.
