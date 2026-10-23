# RustSHT — concise current performance

**Source baseline:** `6ce879013fffdc4ee6e4a43bf25409dfe1dc23b0` (October 23, 2026). Verify HEAD and measurements before reporting as current. **Active:** [f64 spin-1 reflected two-channel parity](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md). [Full original status preserved](reports/archive/context-2026-10-23/PERFORMANCE_STATUS.md).

| Native f64 transform (RustSHT / DUCC, lower better) | Latest selected ratio | Latency Rust/DUCC |
|---|---:|---:|
| **Spin-1 CC T720 full synthesis q8** | **2.48×** | 15.103 / 6.094 ms |
| Spin-1 CC T720 quarter synthesis q8 | 2.62× | 6.962 / 2.660 ms |
| Spin-1 F1 T720 full synthesis q8 | 2.41× | 14.131 / 5.868 ms |
| Spin-1 GL T720 full synthesis q8 | 2.40× | 14.109 / 5.885 ms |
| Scalar CC T720 full synthesis q1 / q8 | ~1.043× / ~1.014× | Separate historical focused runs |
| Scalar CC T1023 full synthesis q8 | ~1.018× | Focused prior source |
| Scalar F1 T720 full synthesis q1 / q8 | ~1.011× / ~1.03× | Separate runs |
| Scalar CC T720 quarter analysis q1 | 0.732× | Checked prepared-grid prefix |
| Scalar F1 T255/T720 full analysis q1/q8 | 0.57–0.75× | Earlier focused comparisons |

**Qualification:** selected data have different source revisions/protocols; not one comprehensive current-source sweep. A previous committed mirrored-geometry q8 change showed a 2–3% *directional* improvement with overlapping p10–p90 distributions, not a parity breakthrough. See [source report](reports/spin1-reflected-latitude-normalization-2026-10-23.md).

**Latest rejected candidate:** the T720+ reflected order-major CC/F1 extension passed DUCC correctness checks, but the targeted block-local fallback/reuse revision was 18.0% slower for CC q8 and 19.4% slower for F1 q8 over 30 paired batches. It was discarded; no production source change was pushed. The failed blocks restart through the scaled path after any SIMD lane leaves normal range. See [first schedule result](reports/spin1-order-major-reflected-pair-negative-2026-10-23.md) and [local fallback result](reports/spin1-reflected-order-major-local-fallback-negative-2026-10-23.md).

**Accepted:** f64 scalar CC/F1 ordered synthesis and parallel analysis; spin-1 paired-coefficient preparation/reuse; GL-only T319–511 degree-tiled hemisphere kernel (small directional benefit, not T720 CC route). **Rejected:** separate reflected scalar transforms at CC T720 q8 `15.056→24.029 ms`; [negative report](reports/spin1-reflected-scalar-row-adapter-negative-2026-10-23.md).

**Next implementation:** carry a checked scaled prefix into the ordinary SIMD suffix inside the reflected recurrence, preserving one recurrence across both channels and mirrored hemispheres. Do not restart each declined block through separate hemisphere fallbacks or revisit the older GL-only path. After a demonstrated synthesis gain, tackle spin-1 analysis recurrence (earlier T1023 CC recurrence ~51.7 ms, prepared resampling ~16.2 ms; DUCC resampling ~17.1 ms).

**Deferred:** dense CC sweep completion/figures, GL table cutoff, deep cleanup, f32. Evidence retained in `reports/data/`, [Sol review](reports/f64-ducc-independent-review-2026-10-22.md), [Astra review](reports/repo-state-review-summary-2026-10-22.md) and [archive](reports/archive/context-2026-10-23/README.md). Do not auto-read.
