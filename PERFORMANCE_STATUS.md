# RustSHT — performance snapshot

**Source:** `opt/sht-performance-parity`, prepared-component checkpoint based on `36e11f51`. DUCC reference: pinned 0.42.0 (`42ce8ab5`). **Active:** [spin-1 parity plan](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md).

| f64 spin-1 synthesis | RustSHT / DUCC | Rust / DUCC median (ms) | Parent → candidate (ms) |
|---|---:|---:|---:|
| CC T720 full q8 | 1.57× | 10.834 / 6.890 | 17.493 → 10.834 |
| CC T720 reduced (m180) q8 | 1.82× | 5.322 / 2.925 | 8.675 → 5.322 |
| F1 T720 full q8 | 1.62× | 10.142 / 6.260 | 16.074 → 10.142 |
| GL T720 full q8 | 1.63× | 11.131 / 6.830 | 16.888 → 11.131 |
| CC T1023 full q8 | 1.13× | 24.634 / 21.724 | 47.095 → 24.634 |
| GL T1023 full q8 | 0.97× | 22.468 / 23.097 | 23.442 → 22.468 |
| GL T511 full q8 | 2.95× | 7.686 / 2.606 | 14.140 → 7.686 (accepted dispatch) |

**M1 accepted:** reuse the scalar ordered prepared SIMD component inside the spin-1 order-major queue. The T720 q8 cases use 30 timed batches/backend across three interleaved processes; T1023 is near DUCC parity. Candidate-to-DUCC relative L2 error was ≤`7.6e-13`; outputs were finite and repeat/reset consistent. Full-order q1 showed no measurable gain.

**M2 accepted:** skip the dense converted-coefficient cache for order-major q8 jobs when it exceeds 16 MB, converting each E/B order into the worker's existing scratch before the prepared-component recurrence. GL T1023 q8 used 30 timed batches/backend over three interleaved process pairs: 22.468 ms versus 23.442 ms on exact parent `08c1ba0e`, and 23.097 ms for pinned DUCC. It was bit-identical to parent, relative L2 to DUCC was `2.61e-13`, and reset/repeat checks passed. Smaller and non-order-major cache users remain enabled.

The attempted serial prepared-component route for GL T1023 q1 was rejected and reverted: across 15 long calls/backend it measured 443.850 ms versus 254.683 ms on parent and 124.158 ms for DUCC (1.74× parent, 3.57× DUCC).

Focused cache-path tests, formatting, strict parallel Clippy, and all 229 no-default-feature library tests passed. The parallel release suite passed 281 tests; two T720 reduced-order q1/q8 tolerance tests also fail on exact parent `08c1ba0e`. The GL T511 q8 selector remains intact.

**Next:** use one focused CC T720 q8 phase profile to attribute order-output clearing/scattering overhead and prepared-route fallback frequency before selecting another kernel change.
