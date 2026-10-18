# Prefix range precheck T2047 q1 route screen protocol v1

## Hypothesis

The fused tile-128 kernel calls `normalize_pair_lane` for every active lane
while any lane remains in the scaled recurrence. Mantissas inside the
normalizer range with a nonzero shared exponent pass through `normalize_pair`
without changing either mantissa or exponent. The candidate runs the existing
SIMD range predicate on each prefix pair and skips the lane loop when no lane
needs scaling.

The change does not skip a degree contribution or alter accumulation order. If
any lane crosses the range, the existing lane normalizer and accumulator
rebasing run. The kernel enters the ordinary suffix only when the existing
all-lane ordinary-state checks pass.

## Frozen sources

- Fused candidate baseline `synthesis_fast.rs` SHA-256:
  `eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407`.
- Baseline `scalar.rs` SHA-256:
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
- Range-precheck variant `synthesis_fast.rs` SHA-256:
  `07c41fe4ad285e59d7ed180931d145f72841d7adc9fe889a20a755837bf48b06`.
- Variant `scalar.rs` is unchanged.
- Source delta:
  [`prefix-lane-pass-fusion-range-precheck-v1.patch`](prefix-lane-pass-fusion-range-precheck-v1.patch), SHA-256
  `479c8ab94c0e90e83e00ba762b6abf2cf41d7308ff91ed11fde0f4b09b9e262b`.
- Test-only route instrumentation:
  [`prefix-lane-pass-fusion-route-profile-v1.patch`](prefix-lane-pass-fusion-route-profile-v1.patch), SHA-256
  `4f14fee0baf4bdc7ba8a322d6f01efbbc4a2a480bb01952edc8e256402f2e704`.

The tests use Rust and Cargo 1.99.0 with release settings and all Cargo
features. Before profiling, the range-precheck variant passed six
`legendre::synthesis_fast::tests`, the scalar SIMD range-guard test, and the
large/small lane-rescaling test. One fast-route test is ignored unless enabled
explicitly.

## Fixed acquisition

Profile the frozen fused source first, then the range-precheck source. Build
both test binaries before acquisition. Use one allowed CPU, one Rayon worker,
GL f64 scalar synthesis at T2047 (`2048x4096`, full `mmax`, `phi0=0.37`), and
the existing ignored `m6_fast_route_counts` test. Each process performs one
unprofiled synthesis followed by one instrumented synthesis and checks their
maps bitwise. The route instrumentation and workload match
[route profile v1](prefix-lane-pass-fusion-route-profile-v1-protocol.md).

Collect preflight and postflight CPU PSI, affinity, worker placement, CPU
frequency, cgroup throttling, involuntary context switches, and competing
CPU-intensive process observations locally. Reject a process if preflight or
postflight `some avg10` exceeds 2.00%, or if monitoring shows clear
interference. Readings from 0.50% through 2.00% are warnings. Do not retry a
failed process or replace its result.

The route test has one instrumented synthesis per process, so it does not
produce a timing CV or drift statistic. Route times and peak RSS are diagnostic
measurements, not whole-transform acceptance results. Keep raw host captures
and test output local; record only aggregate route times, route counts, peak
RSS, source hashes, and concise validity findings.

A favorable route profile can justify a separate complete-transform screen.
It does not accept the fused kernel or this range-precheck variant.
