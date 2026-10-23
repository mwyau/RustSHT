# Spin-1 reflected order-major local fallback — rejected

**Status:** no production change retained. Tested against parent `807d9c62b54e6cb5c724761edd07da1227a96170` on October 23, 2026. This experiment extended one checked recurrence over mirrored CC/F1 latitude pairs and both converted scalar channels in the actual T720 order-major synthesis route. It used the existing bounded degree tile and deferred odd-cosine reconstruction.

The single targeted correction changed a declined reflected block to fall back only that block, reusing the paired coefficients already prepared for the order. It preserved successful reflected blocks and passed the T720 q1/q8 reuse test. It did not recover the performance regression.

## Correctness

Pinned DUCC was 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The whole-map candidate outputs were repeat/reset consistent. Relative L2 error against DUCC was `2.23e-13` for CC T720 and `5.21e-13` for F1 T720. The candidate also passed `cargo check --release --all-features` and `spin1_order_major_synthesis_reuses_workspace_across_changed_coefficients`; the test verifies that the T720 q8 CC/F1 reflected route executes.

## Matched whole-transform timing

Rust parent and candidate were native release builds with `-C target-cpu=x86-64-v3`. Each table entry aggregates 30 timed batches across three interleaved parent/candidate/DUCC process pairs, targeting 25 ms batches. Lower is better.

| Grid, T720 full synthesis | Threads | Parent median [p10, p90] ms | Candidate median [p10, p90] ms | DUCC median ms | Paired candidate / parent |
|---|---:|---:|---:|---:|---:|
| CC | 1 | `93.444 [92.830, 94.515]` | `94.093 [93.330, 97.583]` | `44.009` | `1.015×` |
| CC | 8 | `15.470 [15.208, 16.092]` | `18.262 [17.989, 19.161]` | `6.163` | `1.180×` |
| F1 | 1 | `87.707 [86.650, 89.671]` | `88.013 [87.313, 90.332]` | `42.583` | `1.001×` |
| F1 | 8 | `14.627 [14.358, 15.691]` | `17.349 [17.082, 17.686]` | `5.931` | `1.194×` |

The q1 route is unchanged, so its small differences are not attributable to this kernel. The q8 regression is clear in both grids. The candidate remains roughly 2.9× slower than DUCC at q8.

## Diagnosis and next source change

`FusedHemispherePairBlock` declines a complete SIMD latitude block when any lane's recurrence leaves the normal range. The order-major caller then restarts that block in the existing scaled path for each hemisphere. Reusing already prepared coefficients removes duplicate coefficient setup, but it does not remove the restarted recurrence work; the decline/replay boundary is the remaining cost.

Retire this block-local fallback variant. The next reflected-pair attempt must carry a checked scaled prefix into its ordinary SIMD suffix while retaining one recurrence for both channels and both hemispheres. Do not retry the local fallback schedule or spend time on the older GL-only path.
