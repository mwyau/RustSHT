# Spin-1 reflected checked-prefix mapping — rejected

**Scope:** a no-degree-tile CC T720 order-major synthesis candidate on `opt/sht-performance-parity`, parent `61ae7717c83ba86fe89f1c49e4e109e406fd721b`, compared with pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. It carried a checked scaled prefix into an ordinary SIMD suffix, with one reflected recurrence feeding both converted channels and both hemispheres. Polar pairs and declined SIMD groups used the existing checked fallback. The candidate passed the existing order-major workspace reuse test and matched the parent and DUCC numerically.

## Bounded CC T720 q8 result

The full `mmax=720`, `ntheta=722`, `nphi=1458` synthesis ran in three interleaved parent/candidate/DUCC process triplets. Each backend collected 30 calibrated 25 ms batches. Candidate and parent outputs were repeat/reset consistent.

| Backend | Median ms | p10–p90 ms | Batches |
|---|---:|---:|---:|
| Parent | 16.111 | 14.922–21.236 | 30 |
| Candidate | 21.283 | 13.302–26.827 | 30 |
| Pinned DUCC | 6.157 | 6.100–7.699 | 30 |

The three paired candidate/parent ratios were `0.892`, `1.619`, and `1.018` (median `1.018`). Numerical relative L2 errors were `8.99e-16` candidate-to-parent and `2.23e-13` candidate-to-DUCC. Timing spread was substantial across the interleaved runs. This does not demonstrate a gain, so reduced-order, F1, q1, and T1023 cases were skipped. An earlier screen was also flat at 15.215 ms candidate versus 15.186 ms parent.

## Source diagnosis

The most specific cost is accumulator register pressure. The ordinary suffix in `synthesize_reflected_pair_normal_suffix` holds eight SIMD sums live: two channels × even/odd parity × real/imaginary. The recurrence itself remains shared, and T720's central reflected run leaves only a small SIMD tail for scalar fallback, so neither duplicated recurrence work nor lane underfill explains the flat result. A bounded disassembly of the executed AVX2 V3 helper shows three accumulator vectors loaded from and stored to stack slots inside each degree-pair loop iteration. The checked scalar prefix does not reduce this live set once the suffix begins.

The eight parity accumulations still occur for every degree pair; fusion saves recurrence duplication but does not remove that work. Reconstruction then forms north/south outputs with parity sums and inverse sine, a single post-recurrence step per ring. This source and assembly evidence points to live accumulator pressure and spill traffic as the remaining mapping cost; it does not attribute the whole-transform gap to reconstruction. The candidate was discarded, and no production change was pushed. The raw worker record remains local and was not pushed; the 30-batch numerical and timing summary is recorded above.
