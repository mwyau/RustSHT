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

**Latest rejected candidates:** the checked-prefix CC T720 q8 run measured 21.283/16.111/6.157 ms for candidate/parent/DUCC across 30 batches; its coefficient-stream correction measured 20.225/15.174/7.070 ms across 10 calibrated batches. Neither showed a reliable gain, and neither source change is retained. The earlier block-local fallback candidate regressed 18.0% on CC and 19.4% on F1 q8; see [its report](reports/spin1-reflected-order-major-local-fallback-negative-2026-10-23.md).

**Other rejected T720 mappings:** C64 category (`35.884` vs `15.539 ms`) and full-width parity stream (`41.633` vs `25.873 ms`) also failed CC q8. AVX2 codegen still spills accumulator vectors; see the [latest diagnosis](reports/spin1-reflected-pair-stream-register-negative-2026-10-23.md) and [checked-prefix results](reports/spin1-reflected-pair-register-pressure-negative-2026-10-23.md).

**Accepted:** f64 scalar CC/F1 ordered synthesis and parallel analysis; spin-1 paired-coefficient preparation/reuse; GL-only T319–511 degree-tiled hemisphere kernel (small directional benefit, not T720 CC route). **Rejected:** separate reflected scalar transforms at CC T720 q8 `15.056→24.029 ms`; [negative report](reports/spin1-reflected-scalar-row-adapter-negative-2026-10-23.md).

**Next implementation — corrected technical criterion (2026-10-23):** The **shared-two-component recurrence constraint is lifted**. DUCC runs separate optimized scalar recurrences on the *same reflected ring pairs*. Try a specialized per-order DUCC-style mirrored-pair kernel with one shared coefficient conversion/setup, active-lane packing and output scatter, but **two register-bounded full-width f64 scalar-component recurrence passes**—not two independent calls through Rust's existing high-level scalar ordered engine (that measured 24.029 ms). Preserve checked scale/polar fallback. One small T720 CC q8 A/B decides whether to pursue. If no credible improvement after a new mechanism and one targeted correction, shift to **spin-1 analysis recurrence** (T1023 CC recurrence ~51.7 ms vs prepared resampling ~16.2 ms; DUCC resampling ~17.1 ms) rather than continuing failed synthesis mappings. This is a hypothesis, not an accepted kernel.

**Deferred:** dense CC sweep completion/figures, GL table cutoff, deep cleanup, f32. Evidence retained in `reports/data/`, [Sol review](reports/f64-ducc-independent-review-2026-10-22.md), [Astra review](reports/repo-state-review-summary-2026-10-22.md) and [archive](reports/archive/context-2026-10-23/README.md). Do not auto-read.
