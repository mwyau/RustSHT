# GL T720 f32 mixed fallback and q8 schedule profile — 2026-10-27

## Decision

This is a route profile, not a production change. It confirms that the mixed polar fallback contains normal lane islands, but only a small portion can be handed to the existing width-eight fused kernel as disjoint contiguous suffix spans. Do not implement another mixed safe-span route from these counts. The stronger measured lead is q8 scheduling: the current 12-group schedule uses four of eight Rayon workers, with a repeatable 7.35–7.48× max/min summed group-time skew. Test a smaller f32 GL group grain against the exact parent before changing the inner recurrence.

The active M6 performance ledger is [reports/DUCC_PARITY_M6.md](../../../DUCC_PARITY_M6.md). The previous matched parent/candidate/DUCC screen, including DUCC algorithm and assembly observations, is in the [safe-suffix report](../f32-safe-suffix-handoff-2026-10-27/README.md).

## Provenance and method

- Exact requested parent: 0677b5bfe98117c1d1a5d5204b0a710828fcb9c8.
- Profile checkout: 4fc7b34dd0bc5e3c5b4913235aa03b053aacc618; its production src/ tree is byte-identical to the exact parent. That commit only checkpoints the preceding report.
- Pinned DUCC: 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b (0.42.0).
- Workload: GL, lmax=mmax=720, ntheta=721, nphi=1458, spin 0, phi0=0.23, q8.
- Build: release + parallel, Rust 1.99.1, -C target-cpu=x86-64-v3, with temporary m6_polar_profile instrumentation. The instrumentation was in an isolated worktree and was not applied to a timed production binary.
- Three warm transform runs took 318.037, 318.873 and 318.077 ms. Outputs were finite and bitwise identical across repeats. These are profile-workload timings, not a parent/candidate/DUCC comparison.
- The input coefficients are deterministic finite f32 values (seed 0x6d3620261027); the imaginary part of the m=0 row is zero.

The reusable temporary instrumentation is in [instrumentation.patch.gz](instrumentation.patch.gz) (gzip-compressed unified diff). Per-order route counts are in [route-by-order.csv](route-by-order.csv); per-group timings are in [group-timings.csv](group-timings.csv). The compact machine-readable summary is [summary.json](summary.json).

## Mixed polar fallback work

All three runs produced identical route counts:

| Measure | Count |
|---|---:|
| Mixed polar blocks | 1,216 |
| Active lanes compacted | 40,170 |
| Active contiguous spans | 1 per block |
| Recurrence callbacks | 188,465 |
| Degree–latitude contribution visits | 6,293,674 |
| Useful paired recurrence lane updates | 6,253,504 |
| Exponent-zero lane visits | 1,020,840 |
| Nonzero-exponent lane visits | 5,272,834 |
| Scaled complex helper calls executed | 12,587,348 |
| All-lane-normal callbacks | 0 |
| Per-lane states with zero exponent and normal previous/current | 999,875 (15.9%) |
| Width-eight contiguous normal runs by degree | 42,724 runs; 952,862 lane–degree states (15.1%) |
| Disjoint safe suffix spans selected by a best-case partition DP | 2,063 spans; 814,291 lane–degree visits (12.9%) |

Every one of the 40,170 active seed lanes had a nonzero scale exponent. No mixed block became all-lane normal. The suffix partition bound assumes each selected span can execute through the remaining degrees without a later range or finite-sum decline; it omits setup and dispatch cost. It is therefore an optimistic ceiling, not a predicted speedup.

The callback executes two scaled-complex helper calls per lane visit. In 20,069 visits the highest paired degree is lmax, so its odd coefficient is structurally zero: 6,273,605 of the 6,293,674 odd-channel calls correspond to a nonzero odd degree term, while the even channel has one term per visit. The recurrence executes 6,253,504 lane updates; the remaining 40,170 visits are the final state in their order/block and do not step again.

The mixed adapter declines once per block before touching the spectrum because each block includes polar-excluded lanes. The mixed fallback then compacts active sine, cosine and ring-index values and writes contributions directly into their spectrum slots: it does not stage complex sums and has no transactional replay. The scan/copy timer summed to about 0.123 ms across the 1,216 blocks per run. The recurrence-plus-callback timers summed to 388–390 ms across parallel blocks; this is accumulated thread work, not transform wall time. Counters separate recurrence steps, coefficient callbacks, exponent-zero/nonzero visits, and helper calls, but no fine-grained elapsed split inside the recurrence callback was collected.

The earlier route census recorded 122,339 inactive-scan predicates, 367,793 partition predicates, 20,698 mixed-detection predicates, and 75,474 compact-scan lane visits. Those scans are not the cost center indicated by this profile; compact-plus-copy time is about 0.1 ms summed over the run.

## q8 scheduling and comparison with DUCC

Rust created 12 64-ring groups with min_groups_per_task=2. Each run used four of eight Rayon workers. The groups were consistently assigned in three-group runs with sharply different work:

| Run | Worker group-time totals (ms) | Max/min |
|---:|---|---:|
| 0 | 317.47, 88.03, 258.69, 42.44 | 7.48× |
| 1 | 318.14, 88.60, 43.26, 259.10 | 7.35× |
| 2 | 317.49, 89.26, 42.87, 261.89 | 7.41× |

Summed group time was 707–712 ms while transform wall time was about 318 ms. Group timers include each group's transform work; their sums overlap in time. This profile indicates both underused workers and a highly uneven static grouping. It does not yet test a different grain.

The matched screen at exact parent 0677b5bf measured f32 GL T720 scalar synthesis at 38.08× DUCC q1 / 119.21× q8 for full mmax, and 26.53× / 90.89× for reduced mmax. Its q8 full-mmax parent and DUCC medians were 319.383 and 2.679 ms. The corresponding source Callgrind totals were 8,366,310,958 Rust Ir and 252,802,007 DUCC Ir (33.1×). Instruction counts are not elapsed time.

DUCC's f32 adapter promotes the recurrence and accumulation to f64. Its paired recurrence uses a scaled prefix and normal suffix, then feeds dynamically scheduled latitude batches to the AVX2 kernel. The inspected four-double-lane loop has 12 vector FMAs in the four-pair body, unit-stride loads/stores and no hot-loop spills. RustSHT keeps native f32 arithmetic and eight AVX2 lanes; its fused loop contains recurrence and complex-accumulation FMAs but also range checks and branches. The outer Rust schedule statically groups latitude rings, then the mixed polar route compacts active lanes and performs each degree's scaled contributions through the callback. The parent assembly and codegen limitations are detailed in the [safe-suffix report](../f32-safe-suffix-handoff-2026-10-27/README.md); no candidate-specific assembly is claimed here.

## Next bounded experiment

First compare min_groups_per_task=1 for the affected f32 GL scalar-synthesis schedule against exact parent 0677b5bf, with the same fixtures, compiler/ISA, output checks and pinned DUCC. Start with q8 triage; expand to the required full/reduced q1/q8 matrix and protected f64/adjacent controls only if it shows a repeatable material gain. This changes scheduling only and leaves the shared recurrence, f32 scaled fallback, polar cutoff, and f64 routing untouched. Keep M6 active; the overall per-cell target remains at or below 1.00× DUCC.
