# SIMD prefix accumulation T2047 q1 screen

## Decision

The v1.2 screen is inconclusive. The scalar-prefix process passed its timing
checks. The SIMD-prefix process's RustSHT series passed, but its DUCC series
failed the fixed stability limits (CV `19.812%`, first-three/final-two median
drift `25.775%`). The protocol therefore rejects the process. No retry or
six-pair comparison ran, and the kernel remains isolated.

The single RustSHT process medians were 1367.195 ms for scalar prefix and
1143.564 ms for SIMD prefix, a descriptive ratio of `0.836431` (16.36% lower
latency). The RustSHT samples passed their fixed CV and drift limits in both
processes, but the failed DUCC control prevents this ratio from authorizing
further performance measurements or acceptance.

## Frozen inputs

The [v1.2 screen protocol](simd-prefix-screen-v1-protocol.md) specifies the
source and extension hashes, reference revision, build configuration, workload,
and validity gates. The scalar-prefix source SHA-256 is
`edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`; its rebuilt
extension SHA-256 is
`7e41cdc2804e11a9c49a64394ec305104e758e9d0b24525765e40cf09cd00fcf`. The
SIMD-prefix source SHA-256 is
`b4a849f0fbac0537cf13a203462d75f233cdb5d0a11ef4ce9ac0d956b0521d10`; its
extension SHA-256 is
`2e161316c276140c04ce46928569a483fb3d2f9f8391100c7086f5d9b150fa16`. The
source change vectorizes coefficient-factor formation and prefix accumulation
across ordinary latitude lanes. Exponent rebasing and scaled fallback remain
per lane.

The source patch is [archived here](simd-prefix-accumulator.patch), SHA-256
`1234ed3f5f268baaf42e1edbadf69278500b52c1419b4fac468c5976cebbbe81`. It applies
with zero fuzz to the frozen scalar-prefix source.

## Timing samples

Each process used one warm-up and five timed complete `synthesis_into`
transformations for RustSHT and DUCC. The workload was GL f64 scalar T2047,
full `mmax=2047`, `2048x4096`, `phi0=0.37`, and one worker. The scalar-prefix
process ran first.

| Variant | Backend | Samples (ms) | Median (ms) | CV | Drift |
|---|---|---|---:|---:|---:|
| Scalar prefix | RustSHT | 1364.233, 1367.195, 1367.333, 1366.631, 1368.797 | 1367.195 | 0.109% | 0.038% |
| Scalar prefix | DUCC | 483.286, 484.280, 483.788, 486.706, 486.867 | 484.280 | 0.310% | 0.619% |
| SIMD prefix | RustSHT | 1141.275, 1147.672, 1154.067, 1143.564, 1130.421 | 1143.564 | 0.683% | 0.934% |
| SIMD prefix | DUCC | 485.091, 500.713, 549.511, 484.245, 775.296 | 500.713 | 19.812% | 25.775% |

The SIMD-prefix RustSHT map had max absolute error `7.976e-13`, RMS error
`1.345e-14`, and relative-L2 error `2.842e-13` against DUCC. The scalar-prefix
map had the same errors in this screen. The RustSHT errors meet the v1.2 limits.

Peak RSS was 632,184 KiB for scalar prefix and 623,988 KiB for SIMD prefix.
Context-switch diagnostics were reviewed locally; exact counts remain untracked. These are descriptive
process measurements; the screen is not valid for performance acceptance.

## Host checks

The local interference review did not establish the cause of timing variation. Raw placement, frequency, pressure, and process diagnostics remain untracked.

## Numerical checks

The SIMD-prefix implementation passed `cargo fmt --all -- --check`, workspace
Clippy with all targets/features and warnings denied, and
`cargo test --workspace --all-features` (202 unit tests, 4 extreme-FFT
integration tests, 11 f32 tests, and workspace compatibility; 14 unit and one
f32 test were ignored). The serial release high-bandlimit scalar comparison
passed for T511, T1023, and T2047 on GL, CC, and F1 against analytic fields and
DUCC 0.41.0. At T2047, the GL analytic-map max error was `9.953e-12` and the
selected DUCC middle-row max error was `2.092e-13`. The targeted T1023 extreme
coefficient transition fallback and SIMD-kernel unsafe-coefficient fallback
tests passed.

## Interpretation

The SIMD accumulation is numerically safe in the tested cases and produced a
lower RustSHT median in the single screen process. The paired screen failed its
DUCC stability gate, so the result cannot establish a speedup. The measured
T2047 pre-suffix transition remains the next route target; its previous
scalar-prefix profile was 715.931 ms versus 543.517 ms for the SIMD suffix.
The source patch is preserved, but this screen does not authorize a repeat,
q8, spin-1, or production routing.
