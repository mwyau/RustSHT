# Native scalar visitor accumulator decline diagnostic

## Decision

The one-channel accumulator remains an isolated experiment and is rejected as
implemented. It committed only 244,818 of 1,734,378,064 active lane-pairs
(0.014%). The established scaled visitor still handled almost all of the
rejected work. The profile did not compare this source against a clean
baseline, DUCC or SHTns, and does not establish a whole-transform speedup.

## Source and workload

The measured release test binary SHA-256 is
`2f8a46137ca233529923bfaacc8c49bfa9c2de3d5197f65cfad2e4cc53a35668`. The
source file hashes below identify the complete local test snapshot; the
candidate source remains in the preserved isolated experimental worktree and
was not copied into production:

| Source file | SHA-256 |
|---|---|
| `src/legendre/mod.rs` | `e4328fc12b511dacb4a6354766c6419100d8107acd23a7ca75e50b5e439a4106` |
| `src/legendre/scalar_paired.rs` | `ae565d232dcbb43c8a28c8d8ac7d3e1fed55763cd5e6c76aa6139574c738ebb6` |
| `src/transform.rs` | `f3b3d8771b9d9a73c233a613dc481f74f64c45d83f591b58e2e8a01f8022a448` |
| test-only `src/legendre/route_profile.rs` | `10bb3d078152916108359cdc51408524880dfb3f50001d9f6cfefadaab8c7d38` |

The profile used GL f64 scalar synthesis, `lmax=mmax=2047`,
`ntheta=2048`, `nphi=4096`, `phi0=0.37`, and one Rayon worker. Rust/Cargo
1.99.0, Rayon 1.12.0 and `pulp` 0.22.3 were used on an AMD Ryzen 9 5950X
with AVX2/FMA, 16 physical and 32 logical CPUs, and 96 GiB RAM. The diagnostic
ran one warm-up, one transform with counters inactive and one with counters
active. The two measured maps were bitwise equal. The prospectively revised
host-pressure gate passed without observed interference; exact transient
telemetry remains local. One call per state does not establish CV or drift.
Peak RSS was not measured.

## Route results

| Quantity | Result |
|---|---:|
| Active lane-pairs | 1,734,378,064 |
| Accumulator calls | 3,008 |
| Committed calls / lane-pairs | 4 / 244,818 |
| Declined calls | 3,004 |
| Declines: scaled range | 2,976 |
| Declines: product range | 17 |
| Declines: accumulator range | 11 |
| Other decline reasons | 0 |

The scaled-range decline occurs in the lane-wise scaled-prefix path when its
intermediate contribution cannot pass the existing normal-or-zero check. No
threshold or fallback semantics were changed. The kernel stages its result;
a declined route continues through the established visitor.

The route timers were worker-summed diagnostics: fused path 369.376 ms,
paired visitor 1,363.127 ms, two-channel transition 510.148 ms, and scalar
accumulator 41.967 ms. Within the visitor, compact-order work was 521.675 ms,
regular-order work 427.531 ms, and scaled-lane fallback work 414.304 ms. These
values can exceed wall time and are not independent latency measurements. The
single wall-time calls were 2,396.576 ms with counters inactive and 2,409.859
ms with counters active; this difference is instrumentation evidence only.

Two focused release tests passed: agreement with the paired scaled visitor
for the ordinary fixture and transactional decline for unsafe coefficients.
The route profile also preserved the map bitwise between counter states. No
full extreme-input matrix, T511 control, clean-baseline pair, q8 cell, or
memory measurement was run for this candidate. It is therefore not accepted
for correctness or performance.

## Next measured bottleneck

The compact-order visitor remains the largest measured visitor subpath, with
521.675 ms of worker-summed time in this q1 profile. The accumulator decline
counts show that whole-block eligibility is dominated by scaled-range work,
not coefficient safety: 2,976 of 3,004 declines had that cause. The lane-state
and per-lane safe-prefix measurements below do not justify lane packing or a
continuation kernel. Preserve the existing scaled path for every lane that
cannot be shown safe.

## Lane-state occupancy follow-up

A second q1 profile added test-only lane-state counts without changing
production arithmetic. The release test binary SHA-256 was
`1ef0eaa27a76944963a5631d04a98290d615bd90aa040eed7a532ec8062c0656`. The
updated `src/legendre/scalar_paired.rs` and test-only `route_profile.rs` file
hashes were `a7b92a0784aab97f0340f726a14b294a96bbb16ac6ee3856800292c4ed893a38`
and `6ad51b56b90b1a50f9b7eea9c99617c7b7c2d043fb62ed2898824a4b310b86c7`.
The same T2047 GL f64 scalar route profile passed its map-equality assertion
and the revised host-pressure gate; exact transient telemetry remains local.
This was one diagnostic process, not a timing acceptance sample, and peak RSS
was not measured.

| Scaled-branch state check | Lane-pair samples | Fraction |
|---|---:|---:|
| Exponent zero; both recurrence values in ordinary range | 1,105,506 | 76.48% |
| Nonzero exponent or recurrence value outside ordinary range | 340,042 | 23.52% |
| **Total state samples** | **1,445,548** | **100%** |

At the first paired degree of scaled blocks, 109,374 of 133,176 lane-pairs
(82.13%) passed the existing contribution and accumulation checks; 23,802
failed. These first-degree checks reproduce the current safety conditions.
They do not show that a lane remains safe at subsequent degrees, and the
profile does not prove that packing these lanes would improve a full
transform. The corrected per-lane safe-prefix replay below extends these
counts across the degree range. The scaled path remains required for every lane
that fails those checks.

## Corrected per-lane safe-prefix replay

An initial replay hook was mistakenly attached to the two-channel transition
kernel. That run did not measure the scalar accumulator and is excluded; its
raw local diagnostic files remain unchanged. The corrected test-only replay
runs once per scalar accumulator attempt and independently advances each lane
through the existing scaled recurrence and ordinary-range checks. The release
test binary SHA-256 was
`4da15bb1633afbae0a1e9d5bdb7730cf3b68e2ae79a791a0c11b70c53079373e`. Updated
source hashes were `1b139eda6cc327dd19d3e86cb42b26bb4abf51c126ce0de5b0b0bba7ffe45f0b`
for `src/legendre/scalar_paired.rs` and
`962c35453cbc194340815ad74f1a80640828e8e12e08091e8cc8d08c906e4ce4` for the
test-only `src/legendre/route_profile.rs`. The replay changed no production
arithmetic. The q1 profile preserved map equality and passed the revised
host-pressure gate without observed interference. One process is not an
acceptance sample; the replay itself added substantial timing overhead, so
its wall time and route timers are not used for performance conclusions.

| Per-lane replay result | Value |
|---|---:|
| Lanes profiled | 133,176 |
| Lanes safe through all pairs | 206 (0.1547%) |
| Pair budget | 84,363,880 |
| Safe prefix pairs before first unsafe contribution | 10,786,624 (12.786%) |
| Safe ordinary-range pairs | 196,116 (0.232%) |
| Safe scaled pairs | 10,590,508 (12.553%) |

The pair budget matches the source-resolved compact- and regular-order visitor
counts. The replay applies the current product, contribution, factor and
accumulator checks. Its ordinary scalar lane predicate is conservative for
SIMD vector lanes. The result shows limited complete-lane eligibility and
very little ordinary SIMD work. It does not show that committing partial
prefix sums and resuming the robust visitor would be faster or numerically
safe across the full supported input range. No lane-packing or continuation
kernel is justified by this profile alone.
