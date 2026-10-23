# RustSHT — concise current performance

**Latest tested source:** `8f99a274` on `opt/sht-performance-parity`; sequential q8 spin-1 analysis reuses the first channel's checked recurrence-range proof for the second. Historical results retain their recorded source/protocols. **Active:** [f64 spin-1 DUCC parity](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md). [Full original status preserved](reports/archive/context-2026-10-23/PERFORMANCE_STATUS.md).

| Native f64 transform (RustSHT / DUCC, lower better) | Latest selected ratio | Latency Rust/DUCC |
|---|---:|---:|
| **Spin-1 CC T720 full synthesis q8** | **2.48×** | 15.103 / 6.094 ms |
| Spin-1 CC T720 quarter synthesis q8 | 2.62× | 6.962 / 2.660 ms |
| Spin-1 F1 T720 full synthesis q8 | 2.41× | 14.131 / 5.868 ms |
| Spin-1 GL T720 full synthesis q8 | 2.40× | 14.109 / 5.885 ms |
| Spin-1 CC T720 full analysis q8 | 1.04× | 22.389 / 21.432 ms |
| Spin-1 F1 T720 full analysis q8 | 1.10× | 23.480 / 21.399 ms |
| Spin-1 GL T720 full analysis q8 | 1.75× | 10.037 / 5.729 ms |
| Spin-1 CC T1023 full analysis q8 | 1.36× | 48.322 / 35.508 ms |
| Scalar CC T720 full synthesis q1 / q8 | ~1.043× / ~1.014× | Separate historical focused runs |
| Scalar CC T1023 full synthesis q8 | ~1.018× | Focused prior source |
| Scalar F1 T720 full synthesis q1 / q8 | ~1.011× / ~1.03× | Separate runs |
| Scalar CC T720 quarter analysis q1 | 0.732× | Checked prepared-grid prefix |
| Scalar F1 T255/T720 full analysis q1/q8 | 0.57–0.75× | Earlier focused comparisons |

**Qualification:** selected data have different source revisions/protocols; not one comprehensive current-source sweep. A previous committed mirrored-geometry q8 change showed a 2–3% *directional* improvement with overlapping p10–p90 distributions, not a parity breakthrough. See [source report](reports/spin1-reflected-latitude-normalization-2026-10-23.md).

**Historical q8 analysis result (`2507e53a`):** fused reflected analysis improved the exact `a5ed00b1` parent by 16–38% across those cases. CC/F1 T720 were within 4–10% of pinned DUCC 0.42.0; GL T720 measured 2.22× and CC T1023 1.36×.

**Focused GL T720 analysis q8 (`8f99a274` vs `c31ef94c`):** 30 25 ms batches/backend across three interleaved process pairs; paired candidate/parent median ratio 0.974. Two low-wait pairs improved 1.8–2.6%; one parent process was contended. Candidate/DUCC relative L2 was `4.75e-14`, maximum absolute error `1.18e-15`; repeat/reset checks passed. The candidate remains 1.75× DUCC.

**Rejected T720 mappings:** checked-prefix measured 21.283/16.111/6.157 ms for candidate/parent/DUCC over 30 batches; its coefficient-stream correction measured 20.225/15.174/7.070 ms over 10 calibrated batches. C64 category measured 35.884 vs 15.539 ms, and full-width parity stream 41.633 vs 25.873 ms. The earlier block-local fallback regressed 18.0% on CC and 19.4% on F1 q8. None improved whole-transform latency, and no source variant is retained. AVX2 codegen still spills accumulator vectors; see the [register-pressure diagnosis](reports/spin1-reflected-pair-stream-register-negative-2026-10-23.md) and [checked-prefix results](reports/spin1-reflected-pair-register-pressure-negative-2026-10-23.md).

**Accepted:** f64 scalar CC/F1 ordered synthesis and parallel analysis; spin-1 paired-coefficient preparation/reuse; GL-only T319–511 degree-tiled hemisphere kernel (small directional benefit, not T720 CC route). **Rejected:** separate reflected scalar transforms at CC T720 q8 `15.056→24.029 ms`; [negative report](reports/spin1-reflected-scalar-row-adapter-negative-2026-10-23.md).

**Next code step:** continue reducing the GL T720 spin-1 analysis recurrence cost; checked range-proof reuse across channels yielded a small whole-transform gain, but the case remains 1.75× DUCC. CC/F1 analysis remain near DUCC. CC T720 q8 synthesis remains 2.48× and still needs a distinct kernel organization.

**Deferred:** dense CC sweep completion/figures, GL table cutoff, deep cleanup, f32. Evidence retained in `reports/data/`, [Sol review](reports/f64-ducc-independent-review-2026-10-22.md), [Astra review](reports/repo-state-review-summary-2026-10-22.md) and [archive](reports/archive/context-2026-10-23/README.md). Do not auto-read.
