# RustSHT — performance snapshot

**PARITY COMPLETION RULE (owner 2026-10-24):** The table below is a historical selected-sample snapshot, **not** evidence that all f64 spin-0/1 cells are at DUCC parity. Any ratio above 1.00 is a remaining gap; unmeasured supported GL/CC/F1 synthesis/analysis, full/reduced, q1/q2/q4/q8 cases also remain open. [Coverage ledger and execution plan](plans/f64-ducc-parity-all-cases-2026-10-24.md). The previous ≥2× threshold is retired. Final acceptance requires a quiet-host matched matrix with numerical gates intact.

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

**GL T720 full q8 analysis:** for `GL-T720-m720-n721-p1458`, three interleaved parent/candidate/DUCC process triples (30 calibrated batches/backend) measured parent 7.317 ms (p10–p90 7.135–7.542) and DUCC 4.267 ms (4.205–4.343), a 1.71× ratio. The 256-pair block candidate regressed to 8.919 ms (8.861–9.182; 1.22× parent); outputs were within `1.19e-15` relative L2 of parent and `4.76e-14` of DUCC, and repeat/reset passed. A focused profile with `nphi=1442` measured recurrence/accumulation rising from 6.34 to 8.10 ms, while FFT stayed near 1.86 ms. Reject larger blocks. T720 q1 analysis remains GL 51.536 ms (1.75× DUCC), CC 124.317 ms (1.07×), and F1 125.496 ms (1.06×); outputs remain within `1.79e-16` of parent and `6.06e-14` of DUCC. GL T511 q8 dispatch improvement remains intact.

**GL T720 q8 analysis follow-ups:** scan fusion measured 7.274 ms versus 7.326 ms parent (<1%; bitwise identical), and separate-channel replay was 7.329 ms versus 7.268 ms (1.005× paired). A focused route profile found 599/3,579 blocks with mixed/scaled recurrence across 475/721 order leaves, covering about 80% of estimated recurrence work. Caching checked-route decisions measured 7.182 ms versus 7.232 ms parent but was 1.011× by paired median; output was bitwise identical and repeat/reset passed. SIMD-first checked-lane normalization was bitwise identical but reached only a 0.991× paired median (7.172 ms candidate vs 7.241 ms parent; 30 batches/backend), so it is rejected. The next distinct target is the mixed/scaled Q/U accumulation fallback.
