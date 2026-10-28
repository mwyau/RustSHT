# Native-f32 spin-1 shared paired-scalar route census and screen

**Decision: reject the unconditional route.** The isolated adapter materially improved both q1 cells, but it regressed full q8 slightly and reduced q8 by 39–75% in all three initial pairs. It also changed output relative to the exact parent. The candidate was not promoted; no 15-pair expansion or broad numerical promotion suite was run after the q8 result.

## What changed in the isolated candidate

The earlier one-gate screen changed only the spin synthesis dispatch predicate. A test-only census on exact current parent `8472668b463e2fe06eae189800b83cf8d5620804` confirmed that gate did not enter the route: f32 spin plans had a zero-length fast-scalar buffer, and the f32 prepared-block hook inherited its default `Ok(false)`. The candidate patch from that screen therefore had no effective f32 paired-scalar dispatch in these cells.

This bounded follow-up provisioned the existing worker-owned scalar scratch for f32 spin 1 and added a native-f32 adapter that stages channel sums on the stack and invokes the existing shared `synthesize_paired_scalar_order_prepared_f32` kernel. It did not add a recurrence implementation. Polar or rejected outer blocks return to the general spin route; prepared blocks still use the existing scaled recurrence fallback when not all-normal. The test-only counters are `cfg(test)` only and were absent from timed binaries.

The candidate is based on parent `8472668b463e2fe06eae189800b83cf8d5620804`; patch SHA-256 is recorded in [`build-provenance.json`](build-provenance.json). The exact patch, runner and initial paired results are preserved alongside this report.

## Route census

GL T720 f32 spin-1 synthesis used deterministic synthetic E/B coefficients, warmed twice before counters were reset, then counted one transform. Each work count below is a paired recurrence lane update: one latitude lane per paired recurrence slot. A fused slot combines adjacent-degree coefficients. These counts cover only blocks entering the prepared paired-scalar adapter; declined outer blocks and their general-spin work are not expanded into lane updates.

| mmax | Threads | Prepared kernel calls | Fused blocks | Fallback blocks | Fused paired lane updates | Scaled/generic paired lane updates | Outer block declines |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 720 | 1 | 12,888 | 3,564 | 9,324 | 55,530,546 | 106,375,296 | 0 |
| 180 | 1 | 3,990 | 1,844 | 2,146 | 38,417,842 | 39,688,422 | 150 |
| 720 | 8 | 11,014 | 3,488 | 7,526 | 56,105,010 | 93,215,590 | 1,214 |
| 180 | 8 | 3,990 | 1,844 | 2,146 | 38,417,842 | 39,688,422 | 150 |

The counter definitions and full dispatch totals are in [`route-census.json`](route-census.json). These are route counts, not timing or complete whole-transform useful-work totals.

## Initial matched whole-transform screen

The benchmark used three interleaved fresh-process parent/candidate/DUCC pairs per cell, with the same fixture per triplet, warm reusable workspaces, native f32 input/output, `phi0=0.23`, `ntheta=721`, and AVX2/FMA. The even longitude sizes were `nphi=1458` full and `nphi=362` reduced. q1 used CPU 3 and q8 used CPUs 8–15. Rust used `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC 0.42.0 used GCC 15.2 with the recorded x86-64-v3 flags. Whole-transform timings include FFTs.

| Cell | Parent / candidate / DUCC median ms | Candidate/parent | Parent/DUCC | Candidate/DUCC | Candidate faster pairs |
|---|---:|---:|---:|---:|---:|
| Full q1 | 1528.307 / 786.247 / 40.824 | 0.515 | 37.50× | 19.25× | 3/3 |
| Reduced q1 | 613.044 / 389.330 / 17.695 | 0.633 | 34.70× | 21.96× | 3/3 |
| Full q8 | 270.121 / 274.927 / 5.571 | 1.018 | 48.62× | 49.06× | 1/3 |
| Reduced q8 | 125.068 / 177.267 / 2.498 | 1.417 | 49.49× | 70.80× | 0/3 |

All workers passed finite-output, repeat-consistency and workspace-reset checks. Candidate-parent relative-L2 / max-absolute differences were `1.700e-4 / 1.448e-3` full and `2.525e-4 / 1.587e-3` reduced. Candidate-DUCC relative-L2 / max-absolute errors were `1.962e-4 / 1.351e-3` full and `2.106e-4 / 1.281e-3` reduced. The changed operation order is not bitwise-identical. These random-input metrics do not replace independent analytic-mode, adjoint, polar, extreme-value, underflow/overflow or f64-control validation.

**Decision rationale:** the q1 speedups are large, but the q8 reduced regression is unambiguous in the initial three pairs, and full q8 shows no gain. No 15-pair promotion screen was justified for this unconditional candidate. A q1-only policy would be a separate candidate and is not evaluated here. The previous active M6 scalar-synthesis gap remains open.

## Reproduction and evidence

Pinned DUCC is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Exact flags, compiler, affinity, and binary hashes are in [`build-provenance.json`](build-provenance.json). Machine-readable paired timings and per-worker checks are in [`paired-results.json`](paired-results.json); the test-only route counter patch and paired runner are included for reproduction.
