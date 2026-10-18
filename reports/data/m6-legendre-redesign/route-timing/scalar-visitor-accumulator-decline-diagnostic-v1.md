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
not coefficient safety: 2,976 of 3,004 declines had that cause. The next
bounded diagnostic should measure lane-level normal/scaled occupancy within
those compact-order blocks before selecting another kernel design. Preserve
the existing scaled path for every lane that cannot be shown safe.
