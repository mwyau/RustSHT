# Spin-1 reflected category-lane mapping — rejected

**Scope:** f64 spin-1 CC T720 order-major synthesis, full `mmax=720`, `ntheta=722`, `nphi=1458`, q8. The candidate kept one checked recurrence for both scalar channels and reflected hemispheres, then packed Q/U even/odd accumulators into four complex SIMD vectors. Polar rows and declined groups used the existing scalar fallback. Parent was `61ae7717c83ba86fe89f1c49e4e109e406fd721b`; DUCC was pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Bounded result

Three interleaved process triplets collected 30 calibrated batches per backend at a 25 ms target. The whole-transform boundary reused the Rust plan/workspace within each worker. Candidate and parent outputs were reset/repeat consistent.

| Backend | Median ms | p10–p90 ms | Batches |
|---|---:|---:|---:|
| Parent | 15.539 | 15.326–16.370 | 30 |
| Category-lane candidate | 35.884 | 34.898–43.108 | 30 |
| Pinned DUCC | 6.212 | 6.143–6.481 | 30 |

Candidate/parent paired ratios were `2.736`, `2.306`, and `2.249`. Relative L2 error was `9.90e-16` versus the parent and `2.23e-13` versus DUCC; maximum absolute error versus DUCC was `2.48e-9`. q8 runqueue wait stayed below 0.9% in each process. The result is a clear regression, so reduced order, F1, q1, and T1023 cases were skipped. Raw worker records are in [the bounded result data](data/spin1-reflected-pair-category-2026-10-23/cc-t720-full-q8/results.json).

## Source diagnosis

The recurrence work is shared as intended: one scalar basis state advances per reflected latitude pair, not once per channel or hemisphere. The specific mapping cost is SIMD lane utilization. The candidate batches by `S::C64_LANES` and requires `F64_LANES == 2 * C64_LANES`, assigning one complex lane to each reflected pair so its real and imaginary accumulator slots can travel together. On the benchmark host (Ryzen 9 5950X, Pulp V3/AVX2), that is two latitude pairs per vector instead of four f64 lanes in the current order-major route. The mapping halves the accumulator-vector count from eight real vectors to four complex vectors, but it also halves pair throughput while retaining four channel/parity FMA streams per degree pair and adding pack/unpack work. The measured 2.31× median regression is consistent with that lane-width loss outweighing the saved recurrence work.

The rejected no-tile suffix candidate had a different pressure point: eight live real accumulator vectors, three of which spilled in the inspected AVX2 loop. The category-lane mapping reduced that live set, but its fixed lane underfill made it slower. Final north/south reconstruction is constant work per pair after the degree loop, so it is not a plausible primary cost. No source change from this candidate is retained or pushed.
