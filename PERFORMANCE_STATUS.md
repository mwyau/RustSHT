# RustSHT — current performance snapshot

**OPEN:** All representative f64 spin-0/1 GL/CC/F1 SHT workloads must individually reach pinned native DUCC 0.42.0 parity; this is **not** a complete current-source matrix. [Active plan](plans/f64-ducc-parity-all-cases-2026-10-24.md) · [tried/negative mechanisms](reports/NEGATIVE_EXPERIMENTS.md).

| Selected spin-1 case | Rust / DUCC ms | Ratio | Evidence |
|---|---:|---:|---|
| GL T1023 synthesis q1 | **169.779 / 120.825** | **1.41×** | accepted polar inverse-sine hoist `78eca228`, exact-parent 174.473 ms |
| GL T720 analysis q1 | 49.878 / 29.723 | 1.68× | accepted shared-recurrence dispatch `bddb420e`; a **different** later checked variant was rejected |
| GL T720 analysis q8 | 6.968 / 4.251 | 1.64× | mixed Q/U checked accumulation `f1f4589b` |
| CC T720 synthesis full q8 | 7.534 / 6.091 | 1.24× | earlier matched case |
| CC T720 synthesis reduced m180 q8 | 3.850 / 2.662 | 1.45× | earlier matched case |
| CC T1023 synthesis full q8 | 24.634 / 21.724 | 1.13× | earlier matched case |
| GL T511 synthesis full q8 | 3.175 / 2.384 | 1.33× | earlier matched case |
| GL/F1 T720 synthesis full q8; GL T1023 synthesis full q8 | selected 0.94–0.97× | PASS for measured snapshots | not a complete matrix |
| CC T720 analysis full q8 | 20.590 / 20.942 | 0.98× | `f1f4589b`, measured case |

**Priority:** the ~49 ms GL T1023 q1 synthesis deficit, then GL T720 q1/q8 analysis, then every residual and UNMEASURED scalar/spin-1 cell. Optimize hot numerical kernels—not status documents. The two pre-existing reduced-order CC/F1 one-thread/eight-thread tolerance tests remain open correctness debt, not a new performance regression.

**Latest source improvement:** `78eca228` caches reciprocal sine per polar latitude instead of calculating it every Legendre degree. Exact-parent 174.473 → 169.779 ms (2.7% faster); 3 interleaved paired ratios 0.973/0.966/0.982, outputs bit-identical, rel-L2 vs DUCC ~3.8e-13. Historical raw evidence stays in `reports/data/spin1-polar-inverse-sine-hoist-2026-10-24/results.json`. All table rows are selected, differently dated runs and must be reconfirmed before final parity certification.
