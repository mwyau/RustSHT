# RustSHT — performance snapshot

**Branch:** `opt/sht-performance-parity`. **Reference:** pinned DUCC 0.42.0 (`42ce8ab5`). **Plan:** [f64 spin-1 parity](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md).

| f64 spin-1 synthesis | Rust / DUCC median (ms) | Ratio | Exact parent → candidate (ms) |
|---|---:|---:|---:|
| CC T720 full q8 | 7.534 / 6.091 | 1.24× | 7.845 → 7.534 |
| CC T720 reduced m180 q8 | 3.850 / 2.662 | 1.45× | 4.238 → 3.850 |
| F1 T720 full q8 | 5.528 / 5.861 | 0.94× | 6.285 → 5.528 |
| GL T720 full q8 | 5.555 / 5.859 | 0.95× | 6.250 → 5.555 |
| CC T1023 full q8 | 24.634 / 21.724 | 1.13× | 47.095 → 24.634 |
| GL T1023 full q8 | 22.468 / 23.097 | 0.97× | 23.442 → 22.468 |
| GL T1023 full q1 | 219.472 / 123.725 | 1.77× | 244.750 → 219.472 |
| GL T511 full q8 | 3.175 / 2.384 | 1.33× | 3.182 → 3.175 |

**Accepted changes:** the prepared scalar-ordered component adapter (M1), worker-local conversion for large order-major caches (M2), and checked pair iteration. GL T1023 q1 retains the measured checked-prefix vector-promotion gain; its remaining gap needs a distinct kernel organization.

**CC T720 reduced m180 q8 follow-up:** worker-local E/B conversion now streams each emitted degree into a three-degree pair-preparation window, removing the 4.18 MB global conversion cache and separate preparation scans. Three interleaved parent/candidate/DUCC process triples with 30 calibrated batches/backend produced 3.850 ms candidate, 4.238 ms parent (`0.906×` paired median), and 2.662 ms DUCC. Candidate output was byte-identical to parent and had `3.37e-13` relative L2 error to DUCC; reset/repeat checks passed.

**GL/F1 T720 full q8 suffix follow-up:** the ordinary two-vector suffix shares each coefficient broadcast across its SIMD vectors while preserving per-vector FMA order. Three interleaved process triples with 30 calibrated batches/backend measured GL at 5.555 ms candidate, 6.250 ms exact parent `f3b4b0a2` (`0.893×` paired median), and 5.859 ms DUCC (`0.956×` paired); F1 at 5.528 ms, 6.285 ms parent (`0.882×`), and 5.861 ms DUCC (`0.944×`). Both candidate outputs were byte-identical to parent; relative L2 error to DUCC was `3.51e-13` (GL) and `4.96e-13` (F1). Repeat/reset checks passed.

GL T720 q8 analysis remains 10.090 ms versus 4.553 ms DUCC (2.14×). The reduced-order CC and F1 q1-vs-q8 tolerance tests remain known failures on exact parent `d73c9217`; tolerance is unchanged. The latest focused local-conversion safety/workspace tests, strict parallel Clippy, formatting, and no-default-feature release check pass. The GL T511 q8 dispatch improvement remains intact.
