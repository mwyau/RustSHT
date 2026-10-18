# Scaled-prefix transition event profile v1

## Purpose

The isolated accumulator's T2047 profile measured 715.931 ms in the
pre-suffix transition. Its sampled subcomponents combined factor checks,
exponent rebasing and accumulation, and combined recurrence updates with
normalization. This diagnostic adds test-only event counters to separate
those mechanisms. It does not change production routing or measure candidate
performance.

## Frozen source reconstruction

- Base source commit: `739275d20987fec35506916cb682f961e8a7ffaf`.
- Tile-128 integration patch SHA-256:
  `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`.
- Frozen tile-128 `synthesis_fast.rs` SHA-256:
  `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`.
- Per-order safety-cache patch SHA-256:
  `2ee6fc08997a3f373412b6407164b16a81b36402df75931a5348682ca47f03cd`.
- Scaled-prefix accumulator patch SHA-256:
  `3a75e88d6c6d0640005caaa486e29d8fcbb8ecdd08c5419da9d015f39d736873`.
- Existing test-only route-timing patch SHA-256:
  `9c172821fd93710c9c54f738f60d9366648d1f96ec3a20ed6da749fe3a62d067`.
- Source hash after applying those patches, before event counters:
  `a068ce30e67ff96349af25f13893a488c5b63c0dd35984022ea4dcbbdea1cff8`.

The source is reconstructed in a disposable temporary tree. Do not modify the
clean baseline, production routing, or the existing synthesis-fast worktree.

## Fixed diagnostic

Run the ignored `m6_fast_route_counts` release test at T2047, q1, GL f64
scalar, full `mmax`, `phi0=0.37`, `2048x4096`. It performs one uninstrumented
warm-up synthesis and one instrumented synthesis, then asserts bitwise equality
between their outputs. Use one allowed CPU and no concurrent compilation or
benchmark workload. Timing values from this call are not performance evidence.

Add only test-gated counters for:

- Initial and prefix `normalize_pair_lanes` calls and lane visits.
- Prefix exponent-changing lanes and the sum of absolute exponent shifts.
- `pair_lanes_need_scaling` probes and probes that trigger normalization.
- Accumulator rebase events and the sum of absolute rebase exponent shifts.

Retain the existing route counters and sampled profile for context. Do not add
per-lane heap allocation, change recurrence arithmetic, skip coefficients or
alter fallback behavior.

## Validity and interpretation

Preflight and postflight PSI `some avg10` above 2.00% invalidates the
diagnostic; 0.50% through 2.00% is a warning. Review worker placement,
frequency, context switches, competing CPU-intensive work and cgroup pressure
locally. If the host shows clear interference, stop and preserve the counters
as inconclusive. Raw host, process and timing captures remain local and
untracked.

Event counts describe the frozen T2047 coefficient workload and are not a
universal route ratio. Use them to select at most one follow-up source
hypothesis. No timing acceptance decision or threshold change is part of this
profile.
