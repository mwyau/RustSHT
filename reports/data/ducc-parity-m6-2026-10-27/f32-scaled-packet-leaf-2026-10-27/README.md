# Native-f32 scaled-contribution packet screen — 2026-10-27

## Decision

Promoted source `ec5d56ca29ec6ad4e34fe80443d934677b0515f8` adds a small shared-traversal visitor hook and a native-f32 SIMD leaf for full packets of scaled synthesis contributions. It keeps the existing paired recurrence, skips exponent adjustment only when every lane passes the normal-value checks, and sends unsafe packets and tails through the scalar helper. Exact parent is `3816adce3226a92669a5807241b03f80cb6d8cfd`; pinned DUCC 0.42.0 is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

The candidate improved three of the four primary cells beyond paired run variation. Reduced-mmax q8 is near-neutral and its bootstrap interval crosses parity, so that cell is **inconclusive**. All four Rust/DUCC cells remain **GAP**. This is a measured small step on the scaled arithmetic path, not DUCC parity.

| GL T720 native-f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent (95% bootstrap interval) | Candidate/DUCC | Result |
|---|---:|---:|---:|---:|---:|---|
| Full M720 q1 | 711.759 | 696.580 | 19.129 | 0.9783 [0.9772, 0.9809] | 36.37× | Improvement |
| Full M720 q8 | 111.768 | 107.858 | 2.684 | 0.9754 [0.9545, 0.9847] | 40.07× | Improvement |
| Reduced M180 q1 | 216.946 | 210.643 | 8.339 | 0.9685 [0.9614, 0.9738] | 25.11× | Improvement |
| Reduced M180 q8 | 55.788 | 55.329 | 1.219 | 0.9901 [0.9864, 1.0024] | 45.32× | Inconclusive |

These are 15 interleaved, fresh-process parent/candidate/DUCC pairs per cell, with the order cycled through all six permutations. Rust used release + `parallel`, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, Pulp V3/AVX2/FMA and eight f32 lanes; q1 was pinned to CPU 0 and q8 to CPUs 0–7. The benchmark records warm transform batches; setup and thread-pool setup are separate. Binary hashes, fixture hashes, every pair's timings and checks are in [`results.json`](results.json).

The fine-grain task route runs only when `use_parallel` is true, which requires multiple execution threads. The q1 timing movement in the earlier 32-ring schedule screen therefore was not assigned to task scheduling. That screen's q8 worker counts, ring-indexed costs, job overhead, workspace allocation and utilization are recorded in the [32-ring scheduling report](../f32-32-ring-schedule-2026-10-27/README.md). It showed 12→23 jobs, eight active workers, lower worker-time skew, and small eligibility scans. The present q1 packet-leaf gain is separate from task scheduling.

## Correctness and protected paths

All 60 primary candidate/parent outputs were bitwise equal. Candidate, parent and DUCC workers reported finite outputs and passed repeated-call and workspace-reset checks. Maximum candidate/DUCC relative L2 and maximum absolute differences were `1.72343e-4` / `9.64404e-4` for full mmax and `2.72546e-4` / `1.54788e-3` for reduced mmax; these equal the exact parent's DUCC differences.

The isolated packet tests passed (2 tests), and the f32 integration suite passed (11 passed, 1 ignored). The packet tests compare output bits against native scalar arithmetic and verify fallback for subnormal basis/results, exponent `-300`, overflow and nonfinite coefficients. The existing T720 independent acceptance criteria remain unchanged: low-order analytic modes, the `l=m=720` sectoral mode, coefficient recovery, scalar adjoint identity, polar behavior, smallest normal/subnormal values, `1e30`, `f32::MAX`, repeat calls and workspace reset. The detailed measured errors are in the [32-ring report](../f32-32-ring-schedule-2026-10-27/README.md); the candidate is bitwise equal to that report's exact parent on all four primary workloads.

The adjacent GL T720 f32 scalar-analysis screen was neutral: candidate/parent was `1.0006` q1 across 3 pairs and `0.9985` q8 across 15 pairs; outputs were bitwise equal. The protected f64 GL T1023 spin-1 synthesis screen was `1.0047` q1 and `0.9751` q8 across 3 pairs, bitwise equal to parent, with candidate/DUCC `0.7338` and `0.7142`. The f32 spin-1 routes do not use the changed paired-scalar visitor; their current q1/q8 controls remain recorded in the [32-ring report](../f32-32-ring-schedule-2026-10-27/README.md). No meaningful protected-cell regression was measured.

## What the packet path changes

The visitor receives the already-selected Pulp SIMD implementation from the generic paired recurrence traversal. At each degree it checks full-width f32 packets. If all exponents are in `[-126,127]`, it builds exact power-of-two factors, computes the coefficient FMA and products in native f32 SIMD, and commits the sum only if the coefficients, basis, products, scaled contributions and next sums satisfy the checks. A failed packet is evaluated by the existing scalar scaled helper; a partial SIMD tail is scalar. Polar exclusions, mixed-block compaction, recurrence staging and transactional order commit remain in their existing routes. The change adds no recurrence restart, per-degree allocation, f32 array widening or f64 numerical implementation.

The final benchmark executable SHA-256 is `c59179272f3597b7e3f63a8f503baa8276b579408e4c8ceb11421785716d8941`. Its executed AVX2 packet sequence is preserved in [`avx2-f32-packet-path.txt`](avx2-f32-packet-path.txt). The YMM path uses vector FMA, products, exponent-field range tests and branches to the scalar packet loop at `0x20be90`. Those instructions establish code generation; the whole-transform timings establish elapsed-time effects.

## Remaining arithmetic gap and next measurement

Instrumentation at exact parent `3816adce` counted 45,163,996 total fallback degree–latitude contributions for full-mmax q8 and 18,740,156 for reduced-mmax q8. Of those, 10,195,254 and 3,213,008 had nonzero exponents. A separate test-only counter pass reproduced the candidate packet's safety predicate in the exact-parent callback. It found 4,991,073 full-width packets eligible for the candidate leaf and 5,000,602 nonzero-exponent contributions inside them for full mmax; reduced mmax had 2,130,773 eligible packets and 1,559,888 eligible nonzero-exponent contributions. That leaves 5,194,652 full and 1,653,120 reduced nonzero-exponent contributions on the scalar path, before counting exponent-zero visits in unsafe packets. Three counter runs per cell agreed exactly. This eligibility attribution does not execute the candidate leaf, and it excludes the candidate's next-sum finite check; all profiled fixture outputs were finite and the timed candidate also passed finite-output checks. Exponent-only eligibility alone would admit 5,206,201 / 1,626,235 nonzero-exponent contributions, so normal-value checks decline relatively few additional lanes; the main remaining ceiling is packet-wide exponent range. The isolated [`packet-route-instrumentation.patch.gz`](packet-route-instrumentation.patch.gz) preserves the test-only counter source. Repeated numeric counts and limitations are in [`results.json`](results.json).

DUCC 0.42.0 uses a scaled paired-recurrence prefix in `iter_to_ieee`, corrects the remaining scale state once in `calc_alm2map`, and sends its normal suffix to the unit-stride SIMD `alm2map_kernel`. Its f32 adapter promotes this recurrence/accumulation work to f64. RustSHT keeps native f32 but currently invokes a contribution visitor at each degree; the packet leaf reduces per-lane scaled arithmetic only for safe packets and does not remove that repeated callback/transition work. Historical Callgrind totals are instruction counts for their recorded revisions, not elapsed-time ratios or current-candidate instruction counts.

The next bounded experiment should first count, per active eight-ring packet, the first degree at which both recurrence states reach exponent zero and the accepted normal range, plus suffix contribution coverage and all later range declines. About half the nonzero-exponent fallback contributions remain outside the current packet leaf, so a new handoff must reduce repeated scaling there rather than re-test the already accepted packets. Only if measured suffixes cover enough work should a f32-synthesis-only handoff be tested: fold the exponent into the paired recurrence state once at the proven transition, then reuse the existing native-f32 recurrence/accumulation loop for that suffix while leaving polar, subnormal, overflow and nonfinite packets on the current exact fallback. Keep the shared f32/f64 recurrence policy generic and keep the f64 analysis visitor path unchanged. This directly addresses repeated per-degree scaling while bounding the previously rejected global early-stop visitor's f64 cost; it is a hypothesis pending route counts, not a claimed implementation or speedup. M6 remains open.

## Provenance

- Rust parent: `3816adce3226a92669a5807241b03f80cb6d8cfd`.
- Rust candidate: `ec5d56ca29ec6ad4e34fe80443d934677b0515f8`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust benchmark binary SHA-256: parent `110e67f6b59bcacf8a1f7c14520fa1301c113b7645fce27e6def8f34f935ac5b`, candidate `c59179272f3597b7e3f63a8f503baa8276b579408e4c8ceb11421785716d8941`.
- DUCC benchmark binary SHA-256: `0333b19a8d5bae948fc489d5a95e238af9838b5af267a6302e92b33d5928d160`.
- DUCC numerical SHT sources: `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; benchmark driver: `-fno-fast-math`.
- Artifact checksums are listed in [`artifact-checksums.json`](artifact-checksums.json). The [single M6 ledger](../../../DUCC_PARITY_M6.md) and [machine-readable per-cell ledger](../../ducc-parity-m6-2026-10-27.csv) hold current status; this report contains this experiment's paired evidence.
