# Spin-1 hemisphere pair degree-tile accumulation

**Status:** production change, narrow directional speedup; larger spin-1 parity gap remains open.

## Change

`FusedHemispherePairBlock` now advances its shared recurrence once and stages each 8-degree tile in bounded worker-owned scratch. It accumulates the first and second converted channels in separate passes, keeping four real/imaginary parity sums live at a time instead of eight. This addresses the AVX2 spills seen in the direct accumulation loop without running the recurrence a second time. The 8 × 64 `f64` scratch buffer is used by the existing GL T319–511 full-order q4–q8 hemisphere route.

The inspected x86-64-v3 Pulp specialization has four YMM accumulators in the degree accumulation loop and no repeated per-degree stack spill/reload sequence for the parity accumulators. This is code-generation evidence for the intended change; the full-transform timings below are the performance evidence.

## Matched measurement

- Case: GL T511, even smooth sampling, `nphi=512`; full order (`mmax=511`) and quarter order (`mmax=127`), synthesis.
- Rust parent: source `769cc109f015f3bee6261fa456f8f06afa289228`, binary SHA-256 `3a943136e5080a0d9730610cc06dbde7af40d83f7f97a6c7017cdbbe907336ef`.
- Rust candidate: degree-tile working tree, binary SHA-256 `693ffeb96b4730996b1efabc6ca97fd00343e19bf2d27987aba0f873809c42a6`.
- DUCC: 0.42.0, source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, binary SHA-256 `e3908ae1ce2502b1a7b9c9ede1442d85d4f977efbb62adc39542482d2352f61d`.
- Host/toolchain: AMD Ryzen 9 5950X; Rust 1.99.1 `-C target-cpu=x86-64-v3`; GCC 15.2 DUCC `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` with the benchmark driver's `-fno-fast-math` driver flag.
- Repetitions: 30 calibrated timed batches per backend and case, across three interleaved process pairs. q1 pinned to CPU 0; q8 pinned to CPUs 0–7; CPU 8 monitored. Raw diagnostics remain local and are not committed.

| Workload | q | Parent median (p10–p90), ms | Candidate median (p10–p90), ms | Candidate/parent process-pair ratios | DUCC median, ms |
|---|---:|---:|---:|---:|---:|
| Quarter order | 1 | 20.559 (20.476–21.028) | 20.438 (20.381–21.049) | 0.996, 0.994, 1.001 | 7.126 |
| Quarter order | 8 | 5.381 (5.320–5.530) | 5.227 (5.143–5.322) | 0.971, 0.970, 0.977 | 1.015 |
| Full order | 1 | 38.743 (38.246–39.653) | 39.410 (38.886–40.322) | 1.009, 1.027, 1.008 | 16.736 |
| Full order | 8 | 12.533 (12.391–12.903) | 12.346 (12.240–12.644) | 1.002, 0.990, 0.973 | 2.379 |

The specialized kernel is selected only for full-order GL at q4–q8. Thus the q1 and quarter-order rows are controls, not executions of the new tiled kernel. Their distributions overlap; the q1 full-order median moved upward and is recorded as an inconclusive control, not evidence that the kernel regressed. The q8 full-order median is 1.5% lower, with overlapping p10–p90 intervals and one near-neutral process pair. The q8 quarter-order row also moved lower but is outside this route. These are directional observations, not a broad speedup claim.

At q8, the Rust candidate remains about 5.19× DUCC for full-order synthesis and 5.15× for quarter order. The separate active target—order-major reflected CC T720 spin-1 synthesis—is untouched; its previously measured 2.44× gap remains open.

## Numerical validation

The candidate matched parent Rust outputs exactly. Reset/repeat and finite-output checks passed. Relative L2 error versus DUCC was `2.646e-13` at full order and `3.168e-13` at quarter order. The existing `spin1_hemisphere_pair_synthesis_matches_general_recurrence` test covers T319/T320/T321/T511, DUCC reference samples, the T319 adjoint and extreme finite fallback.

Validation on this source: all-feature library tests, 278 passed / 36 ignored; no-default-features library tests, 226 passed / 17 ignored; strict all-target/all-feature Clippy passed; formatting and `git diff --check` passed.

## Reproduction

The benchmark used `scripts/benchmark_spin1_f1_coefficient_cache.py` with prebuilt parent, candidate and DUCC workers, `--grid GL --lmax 511 --process-pairs 3 --target-ms 25`. Build/run flags, source IDs, binary hashes and affinity are recorded above and in the local raw JSON. The raw file contains host diagnostics and is intentionally not committed.
