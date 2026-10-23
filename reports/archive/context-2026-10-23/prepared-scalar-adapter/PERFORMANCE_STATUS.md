# RustSHT — concise current performance

**Analysis closeout:** tested checkout `bab010ce` (production change `8f99a274`); sequential q8 spin-1 analysis reuses the first channel's checked recurrence-range proof for the second. **Active:** [f64 spin-1 DUCC parity](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md). [Full original status preserved](reports/archive/context-2026-10-23/PERFORMANCE_STATUS.md).

| Native f64 transform (RustSHT / DUCC, lower better) | Latest selected ratio | Latency Rust/DUCC |
|---|---:|---:|
| **Spin-1 CC T720 full synthesis q8** | **2.48×** | 15.103 / 6.094 ms |
| Spin-1 CC T720 quarter synthesis q8 | 2.62× | 6.962 / 2.660 ms |
| Spin-1 F1 T720 full synthesis q8 | 2.41× | 14.131 / 5.868 ms |
| Spin-1 GL T720 full synthesis q8 | 2.40× | 14.109 / 5.885 ms |
| **Spin-1 GL T511 full synthesis q8, order-major dispatch** | **2.95×** | **7.686 / 2.606 ms** |
| Spin-1 CC T720 full analysis q8 | 1.04× | 21.887 / 21.044 ms |
| Spin-1 F1 T720 full analysis q8 | 1.15× | 24.481 / 21.277 ms |
| Spin-1 GL T720 full analysis q8 | 2.08× | 8.947 / 4.287 ms |
| Spin-1 CC T1023 full analysis q8 | 1.36× | 48.322 / 35.508 ms |
| Scalar CC T720 full synthesis q1 / q8 | ~1.043× / ~1.014× | Separate historical focused runs |
| Scalar CC T1023 full synthesis q8 | ~1.018× | Focused prior source |
| Scalar F1 T720 full synthesis q1 / q8 | ~1.011× / ~1.03× | Separate runs |
| Scalar CC T720 quarter analysis q1 | 0.732× | Checked prepared-grid prefix |
| Scalar F1 T255/T720 full analysis q1/q8 | 0.57–0.75× | Earlier focused comparisons |

**Current-source analysis check (`bab010ce`):** exact T720/full-order q8, three interleaved process pairs and 30 ~25 ms batches per backend against pinned DUCC `42ce8ab5`. RustSHT/DUCC medians are shown above. Outputs were finite, repeat/reset checks passed, relative L2 error was at most `6.06e-14`, and maximum absolute error at most `1.33e-15`. Raw batches remain local under `reports/data/spin1-analysis-closeout-2026-10-23/`; synthesis rows are selected earlier runs, not a current-source matrix.

**Analysis closeout checks rerun on `4cbc1838`:** focused parallel release analysis/extreme/poison/fallback and T720 adjoint tests passed (6 tests); `spin1_` no-default release tests passed (26 Rust + 2 f32 tests); formatting and strict parallel release Clippy passed. The analysis implementation is unchanged since the measured `bab010ce` checkout.

**Source A/B for range-proof reuse (`8f99a274` vs `c31ef94c`):** GL T720 q8 paired median ratio `0.974` over 30 batches/backend and three process pairs; two low-wait pairs improved 1.8–2.6%, one parent process was contended. The three-grid check above is a current-source comparison to DUCC, not an A/B against the parent.

**Current-source GL T511 q8 synthesis A/B (`68ec59b5` vs q8 order-major dispatch candidate):** three interleaved process pairs, 30 calibrated batches/backend. The specialized hemisphere route measured `14.140 ms`; order-major dispatch measured `7.686 ms`; pinned DUCC measured `2.606 ms`. Relative L2 error versus DUCC was `2.82e-13` / `2.46e-13` for the two map components, and candidate versus parent was at most `1.54e-15`. Scheduler wait ranged from `2.8%` to `13.5%`; the q8 route change is material despite host contention. q4 keeps the hemisphere route.

**Rejected analysis follow-up:** 256-pair sequential q8 tiles passed focused reference, adjoint, poisoned-scratch and extreme-value checks, with T720 GL/CC/F1 output relative L2 error at most `6.06e-14` vs DUCC and repeat/reset checks passing. Three interleaved 10-batch process pairs against parent `0e7aa3c9` gave candidate/parent paired medians of `1.140×` GL, `1.114×` CC and `1.007×` F1; the candidate was retired. Host scheduler wait reached 15% during the unrelated pytest run.

**Rejected T720 mappings:** checked-prefix measured 21.283/16.111/6.157 ms for candidate/parent/DUCC over 30 batches; its coefficient-stream correction measured 20.225/15.174/7.070 ms over 10 calibrated batches. C64 category measured 35.884 vs 15.539 ms, and full-width parity stream 41.633 vs 25.873 ms. The earlier block-local fallback regressed 18.0% on CC and 19.4% on F1 q8. None improved whole-transform latency, and no source variant is retained. AVX2 codegen still spills accumulator vectors; see the [register-pressure diagnosis](reports/spin1-reflected-pair-stream-register-negative-2026-10-23.md) and [checked-prefix results](reports/spin1-reflected-pair-register-pressure-negative-2026-10-23.md).

**Accepted:** f64 scalar CC/F1 ordered synthesis and parallel analysis; spin-1 paired-coefficient preparation/reuse; GL-only T319–511 degree-tiled hemisphere kernel (small directional benefit, not T720 CC route). **Rejected:** separate reflected scalar transforms at CC T720 q8 `15.056→24.029 ms`; [negative report](reports/spin1-reflected-scalar-row-adapter-negative-2026-10-23.md).

**Latest rejected analysis candidate:** enabling the two-channel fused recurrence at q8 GL T720 and serializing its channel accumulators measured `10.805 ms`, effectively tied with the sequential-channel parent at `10.802 ms`; neither version changed outputs beyond `1.83e-16` relative L2. The fused variant is retired.

**Next code step:** retain the measured GL T511 q8 order-major dispatch. Continue on the broader CC/F1/GL T720 synthesis gap with a distinct order-major kernel mechanism; do not revisit the rejected reflected-pair or fused-analysis variants.

**Earlier next-code hypothesis:** continue reducing the GL T720 spin-1 analysis recurrence cost; checked range-proof reuse across channels yielded a small whole-transform gain, but the case remains 1.75× DUCC. CC/F1 analysis remain near DUCC. CC T720 q8 synthesis remains 2.48× and still needs a distinct kernel organization.

**Deferred:** dense CC sweep completion/figures, GL table cutoff, deep cleanup, f32. Evidence retained in `reports/data/`, [Sol review](reports/f64-ducc-independent-review-2026-10-22.md), [Astra review](reports/repo-state-review-summary-2026-10-22.md) and [archive](reports/archive/context-2026-10-23/README.md). Do not auto-read.
