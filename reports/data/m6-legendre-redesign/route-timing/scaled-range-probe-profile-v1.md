# Scaled-prefix whole-tile range probe profile

## Result

A bounded T2047 q1 route profile found that an existing SIMD range predicate
could have bypassed the scalar normalization walk on 535,953 of 1,034,964
scaled-prefix normalization calls (51.78%). The other 499,011 calls
(48.22%) had at least one out-of-range mantissa and would still require the
existing robust lane normalizer. This count supports a focused SIMD
precheck experiment, but says nothing about its timing benefit.

## Source and method

The profile extends the isolated no-op normalization source. Before the
test-only counter, `synthesis_fast.rs` had SHA-256
`04aadcae00ab04feb18704e3f2d50687141ed088d32c7ce932222c4d186c308b`.
The counter patch is preserved in
[`scaled-range-probe-profile-v1.patch`](scaled-range-probe-profile-v1.patch),
SHA-256
`9fa58ae4030e04f0d811a30b534fb3b8dde8141026a56caf385c01730410c0a7`; the
instrumented source has SHA-256
`3813fcdae3837ccc33992269e91b5086f350c8f11e5a39d7df722fd199ba700e`. The
release test executable SHA-256 is
`6467f0736fc76c8767c51e0f6484dced45bae97170bd920c06eb944d1b658dd5`.

The test used GL f64 scalar synthesis at T2047, full `mmax`, `phi0=0.37`,
`2048x4096`, and one worker. It ran one unprofiled warm-up and one profiled
transform, then required bitwise equality. The check passed. Rust/Cargo
1.99.0 release settings were used on the same AMD Ryzen 9 5950X x86-64 host
with AVX2/FMA, 16 physical and 32 logical cores, and 96 GiB RAM.

For each normalization reached after the route had entered scaled execution,
test-only code called the existing SIMD `pair_lanes_need_scaling` predicate
before the scalar normalizer. It counted the calls for which every mantissa
was already in the supported normal range. This additional probe changes the
profiled route cost, so no elapsed-time result from this run is used.

## Interpretation

The current no-op candidate still checks lanes individually on every scaled
normalization call. The profile shows that the existing vector predicate
could skip this scalar walk for about half of those calls while preserving
the current normalizer for calls with any out-of-range lane. A future
candidate can test that route with numerical controls and a separately frozen
complete-transform protocol. Its SIMD probe cost on the other half of calls
must be measured; this profile does not establish a speedup or change
production routing.

The no-op candidate screen remains inconclusive because its candidate DUCC
series failed the fixed drift limit. This profile is a separate route-count
diagnostic and does not authorize a retry of that screen. The T511 control
and full regression gate remain unresolved.
