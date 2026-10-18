# Prefix lane-pass fusion route profile protocol v1

## Purpose

Measure route composition and active-lane occupancy in the frozen tile-128
fused scalar synthesis candidate at T2047 GL f64 q1. This is one diagnostic
profile, not a performance-acceptance benchmark. It does not compare against
the clean baseline, DUCC, or SHTns and does not authorize candidate
acceptance.

## Source and instrumentation

- Frozen candidate `synthesis_fast.rs` SHA-256:
  `eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407`.
- Frozen candidate `scalar.rs` SHA-256:
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
- Apply [`route-profile-v1.patch`](prefix-lane-pass-fusion-route-profile-v1.patch),
  SHA-256 `4f14fee0baf4bdc7ba8a322d6f01efbbc4a2a480bb01952edc8e256402f2e704`.
  It adds test-only timers and counters; it does not change production code
  or the kernel route.
- The existing ignored `m6_fast_route_counts` test first runs one unprofiled
  synthesis, then enables counters and runs one profiled synthesis. It
  requires bitwise equality between the two maps.

## Workload and acquisition

- GL f64 scalar, full `mmax=2047`, `ntheta=2048`, `nphi=4096`,
  `phi0=0.37`, q1, deterministic dense coefficients of magnitude `1e-5`.
- Build the release test binary before acquisition. Run one process, pin it to
  one allowed CPU, use `RAYON_NUM_THREADS=1`,
  `RUSTSHT_FAST_ROUTE_LMAX=2047`, and
  `RUSTSHT_FAST_ROUTE_THREADS=1`.
- Command template:

  ```sh
  taskset -c "${CPUSET:?set one allowed CPU}" env \
    RAYON_NUM_THREADS=1 \
    RUSTSHT_FAST_ROUTE_LMAX=2047 \
    RUSTSHT_FAST_ROUTE_THREADS=1 \
    cargo test --release --all-features --lib \
      m6_fast_route_counts -- --ignored --nocapture
  ```

- Collect preflight and postflight pressure checks, affinity, worker
  placement, active frequency, cgroup throttling, and competing-process
  observations locally. Do not compile or run another performance workload
  during the profile process.
- Stop if preflight or postflight CPU PSI exceeds 2.00% or monitoring shows
  clear interference. The 0.50–2.00% range is a warning. These conditions
  determine whether the mechanism profile is interpretable; they do not
  create a candidate-versus-baseline result.

## Timer scope

The profile reports the timed synthesis call, summed `process_tile` time,
summed fast-transition time, summed ordinary SIMD-suffix time, the
transition-minus-suffix estimate for prefix and transition setup, scaled
fallback time, coefficient-safety scan time, average active lanes per tile,
and the ordinary-lane share. `process_tile` includes nested transition,
fallback, and output scatter work. Transition includes the nested suffix.
Fallback and coefficient-scan times are also nested within tile time. Do not
add nested phase values together or interpret this instrumented run as a
whole-transform acceptance timing.

Retain raw test output, GNU-time output, CPU PSI, process listings,
thread-placement, and frequency captures locally. Commit route counts,
structured aggregate timings, active-lane statistics, peak RSS, and concise
validity conclusions only. Preserve exact source and patch hashes with the
result.
