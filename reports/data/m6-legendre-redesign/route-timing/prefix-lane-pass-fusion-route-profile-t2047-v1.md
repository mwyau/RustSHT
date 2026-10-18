# Prefix lane-pass fusion T2047 route profile v1

## Scope and validity

This was one test-only route profile of the frozen tile-128 fused candidate at
GL f64 scalar T2047 q1. The ignored route-count test ran one unprofiled
synthesis and one instrumented synthesis, then verified bitwise equality of
their maps. The route-profile test passed. It is a mechanism measurement; it
does not compare candidate and baseline, DUCC, or SHTns and does not accept
the candidate.

- Candidate `synthesis_fast.rs` SHA-256 before and after profiling:
  `eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407`.
- Candidate `scalar.rs` SHA-256:
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
- Test-only instrumentation patch SHA-256:
  `4f14fee0baf4bdc7ba8a322d6f01efbbc4a2a480bb01952edc8e256402f2e704`.
- Release test binary SHA-256:
  `8eef140b4724a7690d0dc5ce9ffa3808dc0893dcdbeb1d8418ef7aee34b0bcaf`.
- Workload: GL f64 scalar, `2048x4096`, full `mmax=2047`, `phi0=0.37`,
  deterministic coefficients of magnitude `1e-5`, one pinned CPU and one
  Rayon worker.
- Rust 1.99.0 and the hardware/software configuration recorded in
  [protocol v1](prefix-lane-pass-fusion-q1-matrix-t2047-t1023-t511-protocol-v1.md).

The preflight and postflight pressure gates passed. Monitoring showed the
worker within its configured affinity, no cgroup throttling, and no clear
competing CPU-intensive workload. Exact PSI, frequency, process, affinity,
and context-switch captures remain local and untracked. Peak process RSS was
300,604 KiB (293.6 MiB).

## Route timing

| Component | Time (ms) | Fraction of profiled synthesis | Scope |
|---|---:|---:|---|
| Profiled `synthesis_into` call | 1,312.645 | 100% | One instrumented call |
| `process_tile` sum | 1,266.030 | 96.45% | Includes nested routes and scatter |
| Fast transition sum | 1,202.070 | 91.58% | Includes nested SIMD suffix |
| Prefix plus transition setup estimate | 674.421 | 51.39% | Transition minus suffix timer |
| Ordinary SIMD suffix | 527.649 | 40.22% | Nested in transition |
| Scaled fallback | 41.454 | 3.16% | Nested in tiles |
| Coefficient-safety scan | 5.441 | 0.41% | Nested in tiles |

The timer scopes are nested; phase times must not be added. Test counters were
active during the profiled call, so the whole-call time is diagnostic and is
not an acceptance timing.

## Route work and active lanes

| Counter | Result |
|---|---:|
| Tiles | 23,201 |
| Fast transitions completed | 23,165 |
| Tiles declined to scaled fallback | 36 |
| Coefficient-safety scans | 2,048 |
| Prefix lane-pairs | 111,390,956 |
| Ordinary suffix lane-pairs | 1,619,628,108 |
| Scaled-fallback lane-pairs | 5,439,548 |
| Polar lane-pairs, included in fallback | 1,331,458 |
| Average active lanes per tile | 122.80 of 128 |
| Ordinary lanes among active slots | 99.95% |

Packing left few inactive or polar lanes in the ordinary route. The scaled
prefix remains the largest measured route component even though it processes
far fewer lane-pairs than the SIMD suffix. Its per-lane normalization,
normal-range checks, complex factor construction, exponent rebasing, and
accumulation remain scalar in the frozen source. Scaled fallback and
coefficient scanning are comparatively small in this profile.

## Follow-up

The next source-level optimization target is the scaled-prefix lane loop,
while preserving the existing range checks, exponent rebasing, and robust
fallback. Any SIMD work must retain every scaled contribution and be measured
separately from changes to the suffix. The T1023 v2 q1 candidate matrix
remains incomplete because its baseline SHTns timing series failed the fixed
stability limit. The fused candidate remains isolated and unaccepted.
