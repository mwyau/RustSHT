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
| GL T1023 full q1 | 200.660 / 120.673 | 1.66× | 206.888 → 200.660 |
| GL T511 full q8 | 3.175 / 2.384 | 1.33× | 3.182 → 3.175 |

**Accepted changes:** the prepared scalar-ordered component adapter (M1), worker-local conversion for large order-major caches (M2), checked pair iteration, two-vector suffix broadcasts, prepared-pair reuse in the checked transition, and removing a redundant prior-basis range check from checked SIMD/scalar suffixes. The last change improved GL T1023 q1 by 3.0% against exact parent `83132761` across 15 calls/backend; outputs were bit-identical to parent and `3.79e-13` relative L2 from DUCC. GL T1023 q1 remains the largest measured synthesis gap.

**CC T720 reduced m180 q8 follow-up:** worker-local E/B conversion now streams each emitted degree into a three-degree pair-preparation window, removing the 4.18 MB global conversion cache and separate preparation scans. Three interleaved parent/candidate/DUCC process triples with 30 calibrated batches/backend produced 3.850 ms candidate, 4.238 ms parent (`0.906×` paired median), and 2.662 ms DUCC. Candidate output was byte-identical to parent and had `3.37e-13` relative L2 error to DUCC; reset/repeat checks passed.

**GL/F1 T720 full q8 suffix follow-up:** the ordinary two-vector suffix shares each coefficient broadcast across its SIMD vectors while preserving per-vector FMA order. Three interleaved process triples with 30 calibrated batches/backend measured GL at 5.555 ms candidate, 6.250 ms exact parent `f3b4b0a2` (`0.893×` paired median), and 5.859 ms DUCC (`0.956×` paired); F1 at 5.528 ms, 6.285 ms parent (`0.882×`), and 5.861 ms DUCC (`0.944×`). Both candidate outputs were byte-identical to parent; relative L2 error to DUCC was `3.51e-13` (GL) and `4.96e-13` (F1). Repeat/reset checks passed.

**GL T1023 full q1 prepared-transition follow-up:** the checked transition and ordinary suffix reuse the prepared even/odd channel pairs instead of rebuilding them per SIMD vector. Fifteen interleaved process triples (75 calls/backend) measured candidate 206.728 ms (p10–p90 204.695–210.714), exact parent `04095ef6` 223.290 ms (221.993–227.468), and DUCC 0.42.0 121.067 ms (119.662–122.605). Paired candidate/parent median was `0.929×`; candidate/DUCC was `1.719×`. Output was byte-identical to parent with `3.79e-13` relative L2 error to DUCC; repeat/reset checks passed. A focused scaled SIMD-prefix test verifies exact prepared/original sums.

**T720 analysis:** sequential-channel q1 measured GL at 51.536 ms (1.75× DUCC), CC at 124.317 ms (1.07×), and F1 at 125.496 ms (1.06×); outputs remain within `1.79e-16` of parent and `6.06e-14` of DUCC. The GL T720 full q8 shared-recurrence candidate measured 7.381 ms versus 7.519 ms on exact parent `3ceb5641` (paired median `0.978×`) and 4.279 ms for DUCC 0.42.0 (`1.73×`) across 30 calibrated batches/backend and three process triples; E/B rel L2 errors are `2.94e-17`/`2.04e-17` to parent and `4.76e-14` to DUCC. Repeat/reset, extreme-workspace fallback, and adjoint tests pass. Two reduced T720 m180 tests remain exact-parent failures on `83132761`; tolerances are unchanged. GL T511 q8 dispatch improvement remains intact.
