# Current-source f32 scalar-analysis recurrence profile

## Provenance

- Production source parent: `d507297c2c7cdc777557e0b1dcc08813aae92de5`.
- The production source files match `1bd4bd274cacda38f87cc741c2d4f20342107b8f`; the parent descendant contains report evidence only.
- Pinned DUCC reference: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- Host: AMD Ryzen 9 5950X; pinned to one logical CPU with taskset (the host-specific CPU index is omitted from committed evidence).
- Rust `1.99.1 (58cf8f9c3 2026-10-13)`, LLVM `23.1.1`, Callgrind `3.26.0`, objdump `2.46`.
- Release build: `--locked --release --features parallel --lib`, `RUSTFLAGS="-C target-cpu=x86-64-v3 -Aunexpected_cfgs"`.
- Cell: GL T720, full mmax, ntheta 721, nphi 1458, q1.
- Fixture SHA-256 values are in `fixture-hashes.json`.

## Profile route and call boundary

This isolated test-only route used the f32 longitude FFT, widened requested
spectra on demand, and entered the existing shared f64 scalar-analysis
recurrence. A prepared plan/workspace was reused. The test performed one warm
call and one poisoned-workspace reset call before seven timed calls. The
Callgrind run included process/test setup and all nine transforms; its event is
instruction references (`Ir`), not cycles or elapsed time. DUCC was not
profiled in this experiment.

The exact test-only direct-route and counter patch is
`profile-source-and-route-counters.patch.gz` (SHA-256
`8bb8d2e128ad3ff600207b21d3875b08b1298981619c2e39f3476b1f2426a14c`). It adds
no production behavior. The test binary SHA-256 is
`67baf38a6803ba3c803087718ea274ee37a1fc55dd2ee7958b254dc5f2b20042`.
The raw Callgrind file SHA-256 is
`2d5b139ee1b3866a9a3da3aa66b6baee600b824a2702e705d83b21bb4c154d51`.
Compact exclusive self-Ir attribution and route counts are in
`callgrind-self-ir-summary.json`; raw binary, profile, fixture and patch hashes
are listed in `binary-hashes.txt`.

## Findings

The largest exclusive self-Ir entries were:

| Function | Self Ir | Share of process Ir |
|---|---:|---:|
| `FusedScaledScalarAnalysisPair<true, false>` | 752,613,952 | 30.29% |
| `FusedScalarAnalysisPair<true, false>` | 411,406,168 | 16.56% |
| f32 `RealFftPlan::forward_modes` | 279,623,988 | 11.25% |
| `ShtPlan::analyze_symmetric_order_group` | 214,702,743 | 8.64% |
| `prepare_analysis_products_and_opposite` | 187,030,962 | 7.53% |

The exact AVX2/V3 symbols were executed. The ordinary fused kernel uses
four-double YMM recurrence/accumulation vectors and FMA instructions; its
symbol has no YMM stack spills. The scaled kernel has scalar range/correction
checks and substantially more control flow before it enters the fused suffix.
This is source and instruction attribution for one Rust route, not a Rust/DUCC
cycle comparison. See `assembly-review.txt`.

Across the seven counter-instrumented calls, counters recorded 3,318
prefix-enabled scaled blocks, zero counted two-pair scaled-prefix advances,
723,324 ordinary suffix pair steps, and zero mixed-suffix pair steps. The
prefix counter increments only after a two-pair advance; single-pair early
returns are not counted. Therefore zero counted prefix steps does not imply
that every scaled block reached the ordinary suffix. Prefix initialization,
recurrence, and checked-recurrence declines were all zero.

The counter-instrumented run's elapsed median was 14.611339 ms, but those
timings are excluded from acceptance because the counters add atomics in the
measured loop. Outputs were finite and poisoned-workspace reset passed. No
independent analytic, adjoint, exceptional-value, SIMD-tail, wider-bandlimit,
parent/candidate elapsed, or DUCC comparison was run in this profile.

## Status and next action

**Status: profile only; no optimization accepted or rejected here.** The
previous direct-spectrum matched measurements remain in the linked M6 ledger;
this Callgrind run only attributes the current recurrence route.

Next, in an isolated worktree at the then-current `dev` HEAD, inspect the
scaled-prefix early-return and ordinary-promotion cases around
`fused_scalar_analysis_pair_block_impl` and
`FusedScaledScalarAnalysisPair::with_simd` in
`src/legendre/scalar_paired/analysis.rs`. Extend the test-only counters to
distinguish one-pair early returns from ordinary promotions. Only if that
separates a meaningful common path, test a bounded fast-promotion variant
using the existing `prepare_analysis_scale_corrections` and
`promote_analysis_pair_to_ordinary` helpers, with the current checked scaled
route retained on every decline. Compare full/reduced GL T720 q1/q8 against
the exact parent and native-f32 DUCC before doing broader controls. Do not
change the recurrence or assume this profile alone establishes a speedup.
