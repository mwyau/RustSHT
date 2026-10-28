# Native-f32 GL T720 scalar-analysis SIMD accumulator screen

**Decision: retain the bounded f32 accumulation leaf.** It uses the existing shared paired scalar-analysis traversal and dispatches only ordinary, bounded, finite callback contributions to native-f32 SIMD. Scaled, subnormal-input, nonfinite, oversized and otherwise declined contributions keep the existing scalar scaled path. The f64 analysis override is unchanged.

This follows the already documented native-f32 scalar-synthesis transition blocker. Current production synthesis source at parent `4770e1c7504d084a979a62ca0d8ef57e9fd293f8` is identical to the exact suffix-screen source `2b531d73049fdbc6e1b94b2132bfb9cb3476d319` for `src/`. That screen found no safe nonzero-exponent handoff; prior zero-exponent suffixes began only after all exponents reached zero and did not provide a repeatable material whole-transform gain. The f64-internal diagnostic found reflected-ring reuse to be a larger remaining difference than f32 exponent range alone. The current f32 synthesis cells remain **GAP**; this candidate does not change synthesis.

## Source and method

- Exact parent: `4770e1c7504d084a979a62ca0d8ef57e9fd293f8`, fetched from `origin/dev` before editing.
- Candidate patch SHA-256: see [candidate-patch.sha256](candidate-patch.sha256); the candidate source is committed in the same revision as this report.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X. Rust 1.99.1 / LLVM 23.1.1, GCC 15.2.0, x86-64-v3 with AVX2/FMA. Rust flags were `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC numeric translation units used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with `-fno-fast-math` on the adapter.
- Timings use the same deterministic fixtures, worker warm-call boundaries, and fixed q1/q8 CPU-affinity sets for parent/candidate/DUCC. Fifteen interleaved process pairs were run for each scalar-analysis cell. Paired 95% percentile-bootstrap intervals use 100,000 resamples of the paired ratios (seed 77).
- f32 spin-1 adjacent paths used three interleaved pairs per q setting. The protected f64 T1023 spin-1 synthesis used three q1 pairs and fifteen q8 pairs. q8 runqueue wait was variable; the paired ratios and telemetry are retained in the JSON records.
- Callgrind recorded Ir for one first whole-transform call after plan/workspace setup, with the same fixture and q1 affinity for Rust parent, candidate, and pinned DUCC. A temporary wrapper-only early exit stopped each worker after that first call. Cache and branch simulation were not enabled. Ir is instruction work, not elapsed time.

Binary hashes, fixture hashes, compiler settings and affinities are in [build provenance](build-provenance.json). The paired timing records are in [scalar-analysis-15pairs.json](scalar-analysis-15pairs.json), with adjacent f32 spin controls in [spin1-controls-3pairs.json](spin1-controls-3pairs.json) and protected f64 results in [f64-t1023-spin1-controls.json](f64-t1023-spin1-controls.json).

## GL T720 f32 scalar-analysis whole-transform results

Medians are milliseconds. Candidate/parent intervals are paired; candidate/DUCC ratios are the medians of paired ratios.

| mmax | q | Parent | Candidate | DUCC | Candidate/parent [95% CI] | Candidate/DUCC | Status |
|---:|---:|---:|---:|---:|---:|---:|---|
| 720 | 1 | 542.702 | 455.413 | 14.013 | 0.839 [0.811, 0.841] | 32.44× | **GAP** |
| 720 | 8 | 71.905 | 60.078 | 2.246 | 0.834 [0.831, 0.840] | 26.73× | **GAP** |
| 180 | 1 | 188.596 | 140.455 | 5.752 | 0.743 [0.742, 0.748] | 24.37× | **GAP** |
| 180 | 8 | 25.572 | 19.145 | 1.042 | 0.750 [0.746, 0.765] | 19.06× | **GAP** |

The candidate reduces full-mmax latency by about 16% and reduced-mmax latency by 25–26%. All four improvements repeat across the 15 pairs. The remaining DUCC ratios are still large; this is an intermediate scalar-analysis gain, not parity.

## Safe callback coverage and executed work

A test-only census was run in an isolated source worktree; its atomic counters and worker times are not in production binaries. Route counts were the same for q1 and q8 because the useful transform work is identical.

| mmax | Useful paired degree–latitude terms | Normal terms eligible for SIMD | Scaled terms retained on fallback | Eligible share |
|---:|---:|---:|---:|---:|
| 720 | 80,729,837 | 58,828,653 | 21,901,184 | 72.9% |
| 180 | 40,662,194 | 32,554,162 | 8,108,032 | 80.0% |

There were zero unsafe normal-range terms in this fixture. The census counted 693,027 callbacks full and 342,906 reduced; the remaining scaled callbacks were 171,279 and 63,385. In the contribution-level transition check, 1,459 full and 395 reduced order/ring groups had scaled work before their first safe callback. The trailing safe runs covered 31.84M full and 13.87M reduced terms, with zero later invalidation. These counts justify applying the leaf per checked callback while retaining the existing exceptional route.

At q8, all eight workers were active and the per-worker job/work totals were balanced. Full used 721 order jobs (89–91 per worker, about 19.9–20.3M estimated updates each); reduced used 181 jobs (20–24 each, about 8.9–10.7M updates). The instrumented leaf times are diagnostic only. They do not indicate a scheduling bottleneck.

## Callgrind and AVX2 code

| mmax | Useful updates | Parent Ir | Candidate Ir | DUCC Ir | Parent → candidate Ir/update | DUCC Ir/update |
|---:|---:|---:|---:|---:|---:|---:|
| 720 | 161,199,393 | 6,174,114,752 | 4,847,357,931 | 221,749,440 | 38.30 → 30.07 | 1.38 |
| 180 | 81,210,177 | 2,379,430,515 | 1,639,654,952 | 91,744,987 | 29.30 → 20.19 | 1.13 |

On the same first-call boundary, candidate Ir falls 21.5% full and 31.1% reduced. The work-normalized candidate gap is still about 21.9× full and 17.9× reduced DUCC. These counts confirm less executed instruction work; they do not predict cycles.

The emitted AVX2 path in [f32-analysis-accumulator.disasm.txt](f32-analysis-accumulator.disasm.txt) processes eight f32 lanes per YMM vector, unrolls two vector iterations, uses separate `vmulps` and `vaddps` operations, then reduces the vectors. There are no YMM stack spills in that hot loop. The scalar tail handles incomplete vectors. DUCC's f32 interface uses double-precision Legendre arithmetic and its AVX2 `map2alm_kernel` performs fused double-FMA accumulation. Raw parent, candidate and DUCC Callgrind files are compressed under [callgrind/](callgrind/); normalized counts are in [work-normalized-instructions.json](work-normalized-instructions.json).

## Numerical and exceptional-value validation

The reduction tree changes, so scalar-analysis output is not bitwise identical to parent. Across the 15 timing pairs, candidate-vs-parent relative L2 was at most `8.554e-7`; maximum absolute difference was `4.471e-8`. Candidate-vs-DUCC relative L2 was at most `2.205e-5`, with maximum absolute difference `9.058e-7`.

The new T720 independent analytic-mode test measured relative L2 `4.321e-5` and max absolute error `4.649e-6` reduced, and `4.323e-5` / `4.590e-6` full, at both q1 and q8. Its bounds are `1e-4` relative L2 and `2e-5` maximum absolute error; the adjoint check uses the existing T720 f32 synthesis tolerance `2e-4`. No existing tolerance was weakened. The test poisons the destination before a repeated call and checks deterministic reset; it also checks the scalar adjoint identity.

The f32 leaf declines subnormal, nonfinite, oversized and out-of-range cosine inputs during preflight. Recurrence states outside the normal range or with nonzero exponents continue through the existing scaled callback. The direct SIMD test includes a 19-lane tail. The existing extreme-value tests exercise finite-extreme monopoles and operator-range boundaries.

Validation on the root candidate passed:

- `cargo fmt --all -- --check` and `git diff --check`.
- Focused SIMD leaf unit tests: 2 passed; focused T720 analytic/adjoint/reset test passed at the final `1e-4` relative-L2 bound.
- Full f32 integration suite: 13 passed, 1 intentionally ignored.
- f32 extreme-value tests: 2 passed.
- Paired scalar-analysis library suite: 34 passed, 8 intentionally ignored.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`.

The protected GL T1023 f64 spin-1 synthesis output was bitwise identical to parent in every control pair. q1 candidate/parent median was 1.000× across three pairs. q8 used fifteen pairs: median was 1.002× with a 95% paired bootstrap interval `[0.980, 1.047]`; observed q8 variability does not establish a meaningful regression. Candidate/DUCC relative L2 was `2.602e-13`, maximum absolute error `5.411e-9`. Adjacent f32 spin-1 analysis was bitwise identical and about 4% faster in three-pair q1/q8 screens. Spin-1 synthesis was neutral in the q1 screen and noisy at q8; those small effects are not claimed as gains.

## Decision and next step

Retain the safe f32 callback accumulator. It materially reduces whole-transform time and Callgrind Ir, covers most ordinary contributions, preserves the scaled exceptional route, passes the T720 analytic/adjoint/reset and extreme-value checks, and shows no meaningful protected f64 regression. The exact f32 scalar-analysis DUCC gap remains **19–32×**.

The f32 scalar-synthesis gap remains **22.75–40.42×** at the exact code-equivalent parent; this change does not address it. The prior transition blocker is documented in the single [M6 ledger](../../../DUCC_PARITY_M6.md). Continue to profile native-f32 spin-1 synthesis/analysis next, then return to unresolved f64 cells. M6 remains active.
