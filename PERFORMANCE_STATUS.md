# RustSHT — current performance status

**Current outcome (2026-11-02):** B1's bounded attempts and B3's kernel candidates produced no credible whole-call improvement. Exact-current B3 profiling confirms the fused AVX2 leaf is the dominant instruction-count hotspot but identifies no removable operation or cycle-level cause. Later phase-fold and code-outline screens did not produce a candidate; do not add another kernel without a profile that distinguishes front-end pressure from recurrence or accumulation throughput. B0/B2 are controls and f32 performance is deferred. See the [active plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md), [M6 ledger](reports/DUCC_PARITY_M6.md), and [progress](reports/PROGRESS.md).

## B3 f64 production baseline

At the accepted seed-handoff source (`f4344353`, carried at `dev` `4a558596`), 15-pair GL T720 ratios against DUCC f64 are:

| B3 spin-1 analysis | Full q1 | Full q8 | Reduced q1 | Reduced q8 |
|---|---:|---:|---:|---:|
| Rust / DUCC | 1.113 [1.105, 1.120] GAP | 1.124 [1.115, 1.132] GAP | 1.098 [1.091, 1.104] GAP | 1.088 [1.081, 1.096] GAP |

The accepted seed handoff improved its exact parent 10–14%; candidate/DUCC parity remains open. The isolated per-degree fusion screen on base `4a558596` was rejected: candidate/parent point estimates were 1.068–1.136× and candidate/DUCC was 1.238–1.252×. A 9-pair synthetic ordinary-leaf probe measured 1.425 [1.375, 1.477]× parent, so the candidate helper itself is slower. No production code changed. Full evidence is in the [screen package](reports/archive/experiments/m6-b3-per-degree-shared-channel-2026-11-02/README.md).

An exact-current `4a558596` three-pair screen gives a separate, small-sample control:

| B3 spin-1 analysis | Full q1 | Full q8 | Reduced q1 | Reduced q8 |
|---|---:|---:|---:|---:|
| Rust / DUCC, 95% paired interval | 1.077 [1.065, 1.090] GAP | 1.144 [1.070, 1.224] GAP | 1.098 [1.075, 1.122] GAP | 1.110 [1.077, 1.143] GAP |

All four intervals remain above parity; full q8 is wide. This screen does not replace the 15-pair seed-handoff results above. The exact-current production-path Callgrind rerun reproduces the prior q1 counts (577.66M/235.07M full/reduced Rust instructions); the 11,160-byte AVX2/FMA ordinary leaf has no YMM stack traffic and exceeds DUCC's kernel by 1.178×/1.207× in instruction count. Hardware PMU access is blocked, so this does not attribute cycles or justify another kernel change. [Full results, profiles and hashes](reports/archive/experiments/m6-b3-second-channel-attribution-2026-11-02/README.md).

Current profile evidence puts 45.5%/57.5% of full/reduced q1 Rust instructions in the ordinary fused leaf; that leaf has no YMM stack traffic, while hardware PMU access is blocked. A test-only second-channel skip shortened instrumented full/reduced T720 calls by 19–24%, but the skipped accumulation is required output math also present in DUCC; this does not identify removable work. Scaled-prefix sampled phase estimates rank exceptional recovery above recurrence/normalization and masked accumulation, but they are not cycle attribution. Native-leaf selection, duplicate scalar passes and per-degree fusion all lost whole-call screens. B3 has reached the bounded stop rule: no more kernel edits before a different source-matched profile identifies recoverable excess.

The follow-up ring-scale fold had a small reduced-q1 gain only (candidate/parent 0.9944 [0.9905, 0.9984] over 30 pairs); other cells were inconclusive and candidate/DUCC remained 1.088–1.141×. Two code-outline direct-leaf screens were neutral without a tail and 1.180–1.199× slower in the tested cases. These results do not change the stop decision; details are in the [ring-phase](reports/archive/experiments/m6-b3-ring-phase-fold-2026-11-02/README.md) and [tail-outline](reports/archive/experiments/m6-b3-tail-outline-2026-11-02/README.md) reports.

## B1 f64 status

The 15-pair GL T720 results in the B1 zip-loop package use parent `792c8f36` and its zipped-loop candidate; they are not a fresh DUCC refresh at `4a558596`. Those source-specific ratios are full q1 1.059 [1.002, 1.120] GAP, full q8 1.131 [1.097, 1.166] GAP, reduced q1 1.047 [0.988, 1.110] INCONCLUSIVE, and reduced q8 1.032 [0.998, 1.068] INCONCLUSIVE. On exact source `4a558596`, a five-call elapsed phase profile placed 68–81% of the four T720 cells in combined Legendre recurrence and accumulation. A const-specialized optional recurrence step removed an inner-loop branch and pointer reload, but grew the AVX2 leaf 16.7%; three-pair parent/candidate medians were 1.003/0.991/0.987/0.994 and did not establish a material gain. Candidate rejected; B1's bounded attempts are complete. The isolated patch, codegen excerpts, focused test results, hashes and per-call logs are in the [branch-hoist package](reports/archive/experiments/m6-b1-hoisted-pair-branch-2026-11-02/README.md).

Pinned DUCC revision: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Other grid, truncation, adjoint, and T1023 coverage is not certified at this source. The intermittent native-f32 nonfinite SIMD-tail issue is a separate correctness concern; do not resume f32 performance tuning or weaken tolerances.

## B1/B3 analysis source cleanup

On base `9399520`, removed the rejected compile-time fused group-range-check alternative and its benchmark route. Archived measurements show it was 7.7–15.3% slower than the production post-group scan. Also removed the optional AVX2 `mask_all` helper: its 1.0%/1.1%/0.3% changes were within measurement variation. The checked post-group range scan, SIMD mask reduction, and exceptional fallback remain the production behavior; no performance improvement is claimed. Scalar-analysis tests passed (38 passed, 9 ignored), spin tests passed (73 passed, 12 ignored), and strict all-feature library Clippy passed.
