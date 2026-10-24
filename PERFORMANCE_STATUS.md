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
| GL T1023 full q1 | 176.605 / 125.492 | 1.41× | 193.242 → 176.605 |
| GL T511 full q8 | 3.175 / 2.384 | 1.33× | 3.182 → 3.175 |

**Accepted changes:** the prepared scalar-ordered component adapter (M1), worker-local conversion for large order-major caches (M2), checked pair iteration, two-vector suffix broadcasts, prepared-pair reuse in the checked transition, removal of a redundant prior-basis range check, and AVX2 checked scaled-prefix accumulation.

**CC T720 reduced m180 q8 follow-up:** worker-local E/B conversion now streams each emitted degree into a three-degree pair-preparation window, removing the 4.18 MB global conversion cache and separate preparation scans. Three interleaved parent/candidate/DUCC process triples with 30 calibrated batches/backend produced 3.850 ms candidate, 4.238 ms parent (`0.906×` paired median), and 2.662 ms DUCC. Candidate output was byte-identical to parent and had `3.37e-13` relative L2 error to DUCC; reset/repeat checks passed.

**GL/F1 T720 full q8 suffix follow-up:** the ordinary two-vector suffix shares each coefficient broadcast across its SIMD vectors while preserving per-vector FMA order. Three interleaved process triples with 30 calibrated batches/backend measured GL at 5.555 ms candidate, 6.250 ms exact parent `f3b4b0a2` (`0.893×` paired median), and 5.859 ms DUCC (`0.956×` paired); F1 at 5.528 ms, 6.285 ms parent (`0.882×`), and 5.861 ms DUCC (`0.944×`). Both candidate outputs were byte-identical to parent; relative L2 error to DUCC was `3.51e-13` (GL) and `4.96e-13` (F1). Repeat/reset checks passed.

**GL T1023 full q1 checked-prefix update:** AVX2 now adjusts and validates scaled-product exponents in registers, retaining checked fallback for exceptional values. Three interleaved parent/candidate/DUCC process triples (15 timed calls/backend) measured candidate 176.605 ms (p10–p90 174.013–197.713), exact parent `c554e705` 193.242 ms (191.178–197.391), and DUCC 125.492 ms (123.000–129.234). Paired candidate/parent ratios were 0.913–0.917; candidate/DUCC was 1.41×. Maps were bitwise identical to parent and had `3.795e-13` relative L2 error to DUCC; repeat/reset checks passed. This is an 8.6% median gain over parent. The earlier active-ring trim reduced fallback blocks from 3,040 to 2,413 and remains part of the path. The reduced T720 m180 CC grouped and F1 dynamic synthesis test failures also reproduce on exact parent `4d7499c2`; tolerances remain unchanged.

A reflected checked-prefix follow-up was rejected: corrected candidate 265.776 ms versus 203.656 ms on exact parent and 119.651 ms DUCC (three interleaved triples, 15 calls/backend). Output remained bitwise identical to parent and within `3.85e-13`/`3.74e-13` relative L2 of DUCC. Review traced the residual to per-degree normal-range guards on the ordinary suffix; the active plan records the next distinct strategy.

**T720 analysis:** sequential-channel q1 measured GL at 51.536 ms (1.75× DUCC), CC at 124.317 ms (1.07×), and F1 at 125.496 ms (1.06×); outputs remain within `1.79e-16` of parent and `6.06e-14` of DUCC. The GL T720 full q8 shared-recurrence candidate measured 7.381 ms versus 7.519 ms on exact parent `3ceb5641` (paired median `0.978×`) and 4.279 ms for DUCC 0.42.0 (`1.73×`) across 30 calibrated batches/backend and three process triples; E/B rel L2 errors are `2.94e-17`/`2.04e-17` to parent and `4.76e-14` to DUCC. Repeat/reset, extreme-workspace fallback, and adjoint tests pass. GL T511 q8 dispatch improvement remains intact.
