# GL T720 f32 task grain screen — global form rejected

## Decision

Setting GL scalar transform tasks to one latitude group materially improves q8 scalar synthesis, but the global change is not safe to promote. It changed f32 q8 scalar analysis too and produced a repeatable 3.4% slowdown in protected f64 GL T1023 spin-1 synthesis q8 (all three pairs). Keep the exact parent in production. The next test will scope one-group scheduling to native-f32 GL scalar synthesis only, leaving analysis and f64 routing unchanged.

This is scheduling evidence. It does not reduce the scalar fallback's arithmetic or establish DUCC parity.

## Provenance and method

- Exact parent and candidate base: RustSHT `0677b5bfe98117c1d1a5d5204b0a710828fcb9c8`.
- Candidate: GL and CC scalar scheduling changed to `min_groups_per_task=1`; F1 unchanged. The candidate patch is `candidate.patch.gz`; uncompressed SHA-256 `5cf0bbaafbd132dbc2943dda12decebcce44831ae68e905f639ea7bb80f7b12a`.
- DUCC: pinned native 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust release flags: `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC numerical sources: `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, benchmark driver with `-fno-fast-math`.
- Same deterministic fixtures and equivalent transform geometry; three initial interleaved parent/candidate/DUCC triplets per f32 synthesis cell. The q8 full and reduced cells were extended to 15 independent pairs. Timings cover warm whole transforms.
- Rust parent/candidate outputs were bitwise identical in all recorded pairs. The candidate binary had SHA-256 `5ed89947e700c80c3a6d8e45cddcffec06c85abf899d90773f4b938ad7eb3f4b`; parent `99b108efcb6eb9362535a4e2ec2532f8946d5cfb6cdff7b329c81da71dd56fc0`; DUCC f32 `9ff1e51c906f17f31f6e80c3928276b41acb576d5aef4681bebaacf4949ca3b8`.

Raw process captures and build trees remain local scratch. Machine-readable pair data, group rollups and rerunnable sanitized drivers are stored beside this report.

## f32 scalar synthesis

`q1` and `q8` are thread counts. Ratios are paired candidate/parent medians; the q8 confidence intervals are percentile bootstrap intervals over paired ratios.

| GL T720 cell | Candidate/parent median | Candidate/parent range or 95% CI | Candidate/DUCC median paired ratio | Decision |
|---|---:|---:|---:|---|
| Full M720 q1 | 0.9989 | 0.9840–1.0187, 3 pairs | 37.21× | Neutral; no q1 gain |
| Reduced M180 q1 | 0.9997 | 0.9966–1.0010, 3 pairs | 25.87× | Neutral; no q1 gain |
| Full M720 q8 | 0.4518 | 0.4408–0.4641, 95% CI; 15 pairs | 51.71× | Strong scheduling gain; still far from parity |
| Reduced M180 q8 | 0.8510 | 0.8433–0.8629, 95% CI; 15 pairs | 69.39× | Repeatable gain; still far from parity |

All 15 q8 pairs favored the candidate in both cells. The q8 medians were 330.555 ms parent / 149.531 ms candidate / 3.100 ms DUCC for full M720, and 114.670 / 97.588 / 1.404 ms for reduced M180. Candidate/DUCC values above are medians of paired ratios; ratios of the separate timing medians differ slightly.

The initial q1/q8 triplets also include candidate/DUCC comparisons and error measures. Relative L2 versus DUCC was `1.72343e-4` full and `2.72546e-4` reduced; maximum absolute errors were `9.64403e-4` and `1.54787e-3`. These describe existing f32 behavior; this screen does not establish a new T720 tolerance.

## q8 worker distribution

The exact-parent group profile had 12 groups, `min_groups_per_task=2`, and used four of eight workers. Each worker received three groups. Across three runs, max/min summed worker group time was 7.35–7.48×.

With one-group tasks, all eight workers received jobs in every instrumented run. For each run, worker job counts were distributed as 1–3 jobs per worker and max/min summed group-time ratios were 2.33–2.99×. The corresponding group-time instrumentation substantially inflated total transform wall time (523.5–700.1 ms), so only within-run job distribution and skew are used here. Uninstrumented q8 whole-transform timings are reported above.

The candidate schedule profile was:

| Run | Jobs by worker IDs 0–7 | Summed group ms by worker IDs 0–7 | Max/min |
|---:|---|---|---:|
| 0 | 2, 1, 1, 1, 2, 2, 1, 2 | 224.4, 670.6, 468.8, 416.6, 240.0, 402.5, 651.3, 316.4 | 2.99× |
| 1 | 2, 3, 2, 1, 1, 1, 1, 1 | 271.9, 417.7, 230.7, 248.0, 683.4, 657.7, 626.9, 384.1 | 2.96× |
| 2 | 1, 2, 3, 1, 1, 1, 2, 1 | 490.4, 361.4, 294.9, 334.0, 506.0, 217.6, 367.8, 479.5 | 2.33× |

These are grouped-work timers, not per-worker CPU cycles. The large q8 whole-call improvement and the change from four active workers to eight support scheduling imbalance as a major q8 constraint. The q1 result and remaining 51.7×–69.4× DUCC ratios show that it is not the only bottleneck.

## Route counts and fallback work

The exact-parent mixed-span profile at the same full-mmax q8 geometry counted 1,216 polar-mixed blocks, 40,170 compacted active lanes and 6,293,674 mixed degree–latitude visits. Those visits included 6,253,504 useful recurrence lane updates, 1,020,840 exponent-zero contributions and 5,272,834 nonzero-exponent contributions. Two complex channel helpers per visit produced 12,587,348 scaled-complex helper calls. There were zero all-lane-normal callbacks and zero transactional replays in this mixed route. The adapter's 1,216 declines happen before spectrum writes; it compacts active lanes and accumulates directly, without staging complex sums.

The exact-parent entry-route counters, collected in a separate instrumented safe-suffix experiment based on `0677b5bf`, recorded 8,652 blocks: 1,937 fully inactive polar blocks, 1,744 initially all-normal blocks entering the fused kernel, 3,755 all-active scaled-seed blocks, and 1,216 mixed blocks declined by the polar partition. The prepared route staged 347,095 lanes. The parent entry route has no suffix retries. In the *separate rejected safe-suffix candidate*, 3,205 all-active scaled-seed blocks later attempted fused suffixes; 3,158 succeeded and 47 range declines restored and replayed 862,036 pair-lane visits, with no nonfinite declines. Those 47 replays belong to that candidate only; do not attribute them to the parent or this task-grain candidate.

Compaction scans and copies took about 0.123 ms summed over all mixed blocks. Recurrence plus coefficient-callback timers summed to 388–390 ms across parallel blocks; this is accumulated thread work, not wall time, and the timer does not yet split recurrence updates from scaled multiplication within the callback. The exact-parent Callgrind whole-call totals were 8,366,310,958 Rust Ir and 252,802,007 DUCC Ir (33.09× instructions, not elapsed time). DUCC's equivalent native-f32 adapter promotes recurrence and accumulation to f64 before its AVX2 `alm2map_kernel`; matched earlier assembly inspection found four double lanes, 12 vector FMAs in its four-pair body, unit-stride memory operations and no hot-loop spills. Rust uses native f32 and eight lanes on this target, while polar/scaled routes retain per-contribution scaling. These differences explain distinct algorithm paths but do not make instruction ratios equivalent to elapsed-time ratios.

A separate earlier parent trace at `972bacdaef57388509738d1c2cdebbb7973b89be` counted 74,402,963 degree–latitude visits, of which 59,376,532 (79.8%) were normal-range, while initially eligible all-normal fused blocks covered 27,997,047 (37.6%). This explains why “normal range” does not imply block eligibility. It is route evidence from that exact source, not a current 0677 instruction or timing profile.

There is little justification for more scan or staging work: the mixed route has no replay or staged sums, the scan/copy timing is tiny, and the safe contiguous width-eight suffix upper bound is only 814,291 visits (12.9% of mixed visits) before setup and later safety declines. The q8 scheduler is a measured high-leverage change; the scaled recurrence and multiplication remain the next arithmetic target after scoped scheduling is validated.

## Adjacent and protected controls

Parent/candidate outputs were bitwise equal in all controls. Because the global patch applies to scalar transforms beyond synthesis, f32 GL T720 scalar analysis q1 was 2.95% slower by median (3 pairs); q8 was near neutral. Spin-1 analysis was near neutral. Spin-1 synthesis q8 was highly variable.

The protected f64 GL T1023 spin-1 synthesis q8 candidate/parent ratios were `1.03384`, `1.00891` and `1.04947` (median `1.03384`, 0/3 faster). This is a repeatable protected-cell slowdown, even though the candidate remained about `0.734×` DUCC. The global patch is rejected. The next candidate must affect only f32 scalar synthesis scheduling and must remeasure these protected q1/q8 cells.

Focused paired-scalar unit tests (34 passed, 8 ignored), f32 integration tests (11 passed, 1 ignored), formatting and diff checks passed on the global candidate. These include scaled fallback and overflow behavior, analytic fields, poles, adjoints and workspace reuse. The scoped candidate must rerun the relevant tests and explicit extreme/polar/workspace checks before promotion.

## Next action

Add a small `ShtScalar` policy for fine scalar-synthesis groups, default false and enabled for f32 only when GL has more groups than execution threads. Apply it only in scalar synthesis. Preserve CC's existing fine grain, GL f64 scheduling, analysis scheduling and all numerical fallback code. Compare the exact parent, this scoped candidate and pinned DUCC on all four f32 T720 full/reduced q1/q8 cells; include adjacent f32 analysis/spin controls and protected f64 T1023 synthesis q1/q8. If this candidate passes, investigate a more detailed timing/Callgrind split of fallback recurrence updates versus coefficient scaling; do not call the task-grain result DUCC parity.
