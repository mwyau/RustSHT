# Spin-1 T720+ reflected order-major candidate — rejected

**Status:** no production change retained or pushed. Source baseline: `6ce879013fffdc4ee6e4a43bf25409dfe1dc23b0`. Candidate worktree changes were discarded after the matched q8 runs showed no gain.

The candidate routed CC/F1 T720+ order-major synthesis through one reflected north/south Legendre recurrence shared by both converted scalar channels. It used bounded 8-degree tiles, accumulated even/odd parity, and applied odd cosine factors during reconstruction. A targeted revision accumulated the first channel while generating each tile to avoid rereading that tile. The GL T319–511 route remained in its direct-cosine specialization; no GL-specific optimization was attempted. The candidate preserved the checked polar and scale fallbacks.

## Validation

Pinned DUCC was `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Candidate q1/q8 outputs were finite, repeat/reset consistent, and matched DUCC with these whole-map metrics:

| Grid, T720 full | Relative L2 | Maximum absolute error |
|---|---:|---:|
| CC | `2.23e-13` | `2.49e-9` |
| F1 | `5.21e-13` | `5.24e-9` |

`cargo check --release --all-features`, `spin1_order_major_synthesis_reuses_workspace_across_changed_coefficients`, and `spin1_hemisphere_pair_synthesis_matches_general_recurrence` passed. The first test asserts the CC/F1 order-major reflected route executes.

## Matched whole-transform timing

Native Rust release builds used `-C target-cpu=x86-64-v3`; DUCC 0.42.0 used the pinned source. Each q8 value aggregates 30 timed batches over three interleaved process pairs, with 25 ms target batches. Lower is better.

| Grid, T720 full synthesis q8 | Parent median [p10, p90] ms | Candidate median [p10, p90] ms | DUCC median ms | Candidate / parent |
|---|---:|---:|---:|---:|
| CC | `15.456 [15.091, 16.709]` | `15.845 [15.554, 16.878]` | `6.210` | `1.025×` |
| F1 | `14.413 [14.245, 15.193]` | `14.852 [14.591, 15.833]` | `5.895` | `1.030×` |

The q1 control used the unchanged route: CC parent/candidate medians were `92.816/92.170 ms`, and F1 `86.677/85.876 ms` over ten batches. These small differences are not attributable to the candidate. The initial single-process q8 screen suggested a CC gain, but the repeated paired runs did not confirm it; the p10–p90 intervals overlap. The remaining q8 gap is about `2.5×` DUCC for both grids.

**Decision:** retire this degree-tiled/deferred-cosine implementation. It is numerically sound, but the T720+ order-major whole-transform runs do not demonstrate a performance gain. Do not repeat the same kernel schedule as the next candidate.
