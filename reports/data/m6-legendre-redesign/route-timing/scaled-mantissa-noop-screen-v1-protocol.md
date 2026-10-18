# Scaled-mantissa no-op normalization screen v1

## Hypothesis

In the T2047 event profile, prefix normalization changed the lane exponent in
706,324 of 109,490,856 lane-visits. Most nonzero-exponent visits therefore
passed through `normalize_pair` without a scale change. The source currently
skips that helper only when the lane exponent is zero and both mantissas are
inside `[SMALL, BIG]`.

Test a synthesis-only helper that applies the same mantissa bounds for every
lane. If both mantissas are already in range, skip `normalize_pair` and retain
the current exponent. Keep the route in scaled execution whenever any exponent
is nonzero. Call the existing normalizer for every out-of-range pair. This
preserves each lane's binary scale while avoiding the no-op helper call.

## Frozen screen

- Base source: commit `739275d20987fec35506916cb682f961e8a7ffaf`, the tile-128
  integration and scaled-prefix accumulator patches, plus test-only event and
  occupancy counters. The `synthesis_fast.rs` source before this optimization
  is SHA-256
  `e877d0bfab25fe3c643337526eb1d918f00e5374748192ce32408bcc7c6d3613`.
  Record the scalar-helper patch, modified source hashes, and release extension
  hashes in the result.
- Compare the unmodified accumulator source with the no-op normalization
  candidate in one fresh-process pair at T2047. Run the frozen baseline process
  first and the candidate process second. Do not retry or replace either
  process.
- Use the existing `examples/python_perf_baseline.py` harness, SHA-256
  `e7ffbe889922cc4da1b1ece63bc24bf264603c7373adb195abde5d705bb51c07`, with
  `GL`, f64, scalar, `STANDARD` synthesis, `mmax=2047`, `phi0=0.37`,
  `ntheta=2048`, `nphi=4096`, one worker, plan scope, and one run group. Each
  fresh process performs one warm-up and five timed complete transformations
  for RustSHT and DUCC. Compare numerical output with DUCC in both processes.
- Use DUCC 0.42.0, source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`, Python 3.11, and the same NumPy,
  compiler, and Rust release configuration for both variants. Record CPU
  specifications and software versions as in the accumulator q1 protocol.

## Correctness and screen rules

First compare the new lane helper with `normalize_pair_lanes` over normal,
zero, extreme finite, and subnormal mantissas with positive, negative, and
zero exponents. Run the Rust workspace suite with all features. The T2047
profile test must also pass its bitwise warm-up comparison. Candidate and
baseline output must each satisfy the existing DUCC tolerances: relative-L2 at
most `2e-11` and maximum absolute error at most `2e-12`.

Apply the current CPU-pressure protocol: reject preflight or postflight PSI
`some avg10` above 2.00%; treat 0.50% through 2.00% as a warning. Keep the
within-process CV and first-three/final-two drift limits at 5%. Review CPU
placement, frequency, context switches, cgroup pressure, and concurrent
CPU-intensive work locally. Clear interference makes the screen inconclusive.
Keep exact host-state and process captures untracked.

The screen supports further q1 validation only if both processes pass all
validity and numerical checks and the candidate RustSHT median is at most
0.98 times the accumulator baseline median. Otherwise stop this variant. This
one-pair screen cannot accept a kernel, clear the failed T511 control, authorize
q8/regression tests, or change production routing. Do not alter the threshold
after results.
