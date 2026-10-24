# RustSHT — performance snapshot

**Source:** `opt/sht-performance-parity`, prepared-component checkpoint based on `36e11f51`. DUCC reference: pinned 0.42.0 (`42ce8ab5`). **Active:** [spin-1 parity plan](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md).

| f64 spin-1 synthesis | RustSHT / DUCC | Rust / DUCC median (ms) | Parent → candidate (ms) |
|---|---:|---:|---:|
| CC T720 full q8 | 1.57× | 10.834 / 6.890 | 17.493 → 10.834 |
| CC T720 reduced (m180) q8 | 1.82× | 5.322 / 2.925 | 8.675 → 5.322 |
| F1 T720 full q8 | 1.62× | 10.142 / 6.260 | 16.074 → 10.142 |
| GL T720 full q8 | 1.63× | 11.131 / 6.830 | 16.888 → 11.131 |
| CC T1023 full q8 | 1.13× | 24.634 / 21.724 | 47.095 → 24.634 |
| GL T511 full q8 | 2.95× | 7.686 / 2.606 | 14.140 → 7.686 (accepted dispatch) |

**M1 accepted:** reuse the scalar ordered prepared SIMD component inside the spin-1 order-major queue. The T720 q8 cases use 30 timed batches/backend across three interleaved processes; T1023 is near DUCC parity. Candidate-to-DUCC relative L2 error was ≤`7.6e-13`; outputs were finite and repeat/reset consistent. Full-order q1 showed no measurable gain.

Focused validation passed: full CC T720 prepared-route test, exceptional-value/workspace-reuse test, release checks with and without `parallel`, `cargo fmt --check`, and strict parallel Clippy. The GL T511 q8 selector remains intact.

**Next:** reduce the remaining ~1.6× T720 q8 gap with a distinct source-level mechanism; keep the accepted scalar-component path as the baseline. Do not revisit spin-1 analysis or broaden benchmark coverage without a candidate.
