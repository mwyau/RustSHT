# Prefix range precheck T2047 q1 route profile v1

## Scope

This profile compares the frozen fused tile-128 source with one isolated
range-precheck change. It measures route timing in a test-instrumented release
binary; it is not a complete-transform benchmark against the production
baseline, DUCC, or SHTns.

- Frozen `synthesis_fast.rs` SHA-256:
  `eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407`.
- Range-precheck `synthesis_fast.rs` SHA-256:
  `07c41fe4ad285e59d7ed180931d145f72841d7adc9fe889a20a755837bf48b06`.
- Both variants use `scalar.rs` SHA-256
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
- Test-only instrumented source hashes: baseline
  `3d8beae28d28a4f8fb83681f1953cf7e5c486648b92e7183b4c8888134ae4c18`;
  range-precheck `e97a893c0a0cb510f10efb8a9682753a1c22ffc2d568d8f41e9340138b8d1a8f`.
- Release test binary hashes: baseline
  `8eef140b4724a7690d0dc5ce9ffa3808dc0893dcdbeb1d8418ef7aee34b0bcaf`;
  range-precheck
  `9b0da7d3b5de990cb11c04373f954bd269efafa10598a833a53e92afa1b9d59b`.
- Build: Rust/Cargo 1.99.0, release settings, all features. The CPU and OS
  configuration are recorded in
  [protocol v1](prefix-lane-pass-fusion-route-profile-v1-protocol.md).

The fixed order was frozen source, then range-precheck. Each profile ran the
same ignored T2047 GL f64 scalar route test with `2048x4096`, full `mmax`,
`phi0=0.37`, and one worker. The test ran one unprofiled synthesis followed by
one instrumented synthesis and checked the maps bitwise. Both processes passed
the 2.00% PSI gate. Neither preflight nor postflight entered the warning band.
Worker placement stayed within the one-CPU affinity. The cgroup recorded no
throttling, and the process checks showed no clear competing CPU-intensive
workload. Exact transient telemetry and GNU-time output remain local.

## Route timing

| Component | Frozen fused (ms) | Range precheck (ms) | Change (%) |
|---|---:|---:|---:|
| Instrumented synthesis call | 1,336.402 | 1,301.405 | -2.619 |
| `process_tile` sum | 1,288.763 | 1,254.001 | -2.697 |
| Fast transition sum | 1,225.231 | 1,185.544 | -3.239 |
| Prefix plus transition setup estimate | 687.458 | 643.386 | -6.411 |
| Ordinary SIMD suffix | 537.773 | 542.158 | +0.815 |
| Scaled fallback | 41.783 | 46.284 | +10.772 |
| Coefficient-safety scan | 5.542 | 5.584 | +0.758 |

The timer scopes are nested. `process_tile` contains the transition,
fallback, and scatter; the transition contains the suffix. The prefix estimate
is the transition time minus the suffix time. Test instrumentation was active,
so these values do not establish a whole-transform speedup. Each source had one
profile process; the 2.619% lower instrumented call time has no uncertainty
interval.

Both sources used 23,201 tiles, completed 23,165 transitions, declined 36
tiles, and recorded the same prefix, suffix, fallback, and polar lane-pair
counts. Average active occupancy was 122.80 of 128 slots. Peak RSS was
300,544 KiB for the frozen source and 300,336 KiB for the range-precheck
source. The time and memory differences are diagnostic measurements.

## Correctness and status

Before profiling, the range-precheck source passed six
`legendre::synthesis_fast::tests`, the scalar SIMD range-guard test, and the
large/small lane-rescaling test. The paired-reference tests use relative-L2
tolerance `2e-11` and maximum absolute tolerance `2e-12`. The route test passed
for both binaries and verified bitwise equality between each unprofiled and
instrumented map. The patch changes only when the existing lane normalizer
runs. It skips that loop when the SIMD predicate finds no lane that needs
scaling; it does not skip a degree contribution or change the accumulation
order. The baseline and candidate maps were not compared directly.

The route profile supports one complete-transform T2047 q1 screen against the
clean RustSHT baseline. It does not accept the fused candidate or the
range-precheck change. The T1023 q1 paired matrix remains incomplete because
the baseline SHTns timing series failed its fixed stability limit.
