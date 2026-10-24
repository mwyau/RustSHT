# RustSHT — performance snapshot

**Source:** `opt/sht-performance-parity` through `c200958` plus the checked-prefix vector-promotion candidate below. DUCC reference: pinned 0.42.0 (`42ce8ab5`). **Active:** [spin-1 parity plan](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md).

| f64 spin-1 synthesis | RustSHT / DUCC | Rust / DUCC median (ms) | Parent → candidate (ms) |
|---|---:|---:|---:|
| CC T720 full q8 | 1.57× | 10.834 / 6.890 | 17.493 → 10.834 |
| CC T720 reduced (m180) q8 | 1.82× | 5.322 / 2.925 | 8.675 → 5.322 |
| F1 T720 full q8 | 1.62× | 10.142 / 6.260 | 16.074 → 10.142 |
| GL T720 full q8 | 1.63× | 11.131 / 6.830 | 16.888 → 11.131 |
| CC T1023 full q8 | 1.13× | 24.634 / 21.724 | 47.095 → 24.634 |
| GL T1023 full q8 | 0.97× | 22.468 / 23.097 | 23.442 → 22.468 |
| GL T1023 full q1 | 1.77× | 219.472 / 123.725 | 244.750 → 219.472 (0.899× paired) |
| GL T511 full q8 | 1.33× | 3.175 / 2.384 | 3.182 (`6633b3ec`) → 3.175 (`a889cf87`) |

**M1 accepted:** reuse the scalar ordered prepared SIMD component inside the spin-1 order-major queue. The T720 q8 cases use 30 timed batches/backend across three interleaved processes; T1023 is near DUCC parity. Candidate-to-DUCC relative L2 error was ≤`7.6e-13`; outputs were finite and repeat/reset consistent. Full-order q1 showed no measurable gain.

**M2 accepted:** skip the dense converted-coefficient cache for order-major q8 jobs when it exceeds 16 MB, converting each E/B order into the worker's existing scratch before the prepared-component recurrence. GL T1023 q8 used 30 timed batches/backend over three interleaved process pairs: 22.468 ms versus 23.442 ms on exact parent `08c1ba0e`, and 23.097 ms for pinned DUCC. It was bit-identical to parent, relative L2 to DUCC was `2.61e-13`, and reset/repeat checks passed. Smaller and non-order-major cache users remain enabled.

The latest GL T511 q8 recheck used 30 timed batches/backend over three pairs: `a889cf87` 3.175 ms, exact parent `6633b3ec` 3.182 ms, DUCC 2.384 ms (1.33×); outputs matched parent bit-for-bit and DUCC relative L2 was `2.22e-13`. The earlier 2.95× result is stale. For GL T1023 q1, checked-prefix promotion at each SIMD latitude vector measured 219.472 ms versus 244.750 ms on exact parent `c200958` and 123.725 ms for pinned DUCC. Across three interleaved process pairs (15 timed calls/backend), the paired candidate/parent median ratio was `0.899×`; output relative L2 was `1.01e-17` to parent and `4.41e-13` to DUCC, with repeat/reset checks passing. Keep this improvement; the remaining ratio is 1.77× DUCC. Earlier checked-prefix correction, degree-major, two-vector tile (AVX2 spills), two-pair-unroll and serial prepared-component candidates remain rejected.

**Bounded GL T720 q8 analysis follow-up:** preweight each opposite input once per lane before the existing checked per-channel recurrence. Across 30 batches/backend and three interleaved pairs, candidate was 10.090 ms versus 10.899 ms on exact parent `a889cf87` (paired median `0.934×`) and 4.553 ms for DUCC (paired `2.14×`). Output relative L2 was `1.91e-16` to parent and `4.76e-14` to DUCC; weighted/unweighted, reflected recurrence, fallback and no-default tests pass. This is a 6–7% gain, not parity.

The attempted serial prepared-component route for GL T1023 q1 was rejected and reverted: across 15 long calls/backend it measured 443.850 ms versus 254.683 ms on parent and 124.158 ms for DUCC (1.74× parent, 3.57× DUCC).

Formatting, strict parallel Clippy, focused transition/finite-value tests and 281 release tests passed; 36 release tests were ignored. The only release failures were the known T720 m180 q1-vs-q8 assertions in F1 `spin1_f1_dynamic_synthesis_groups_match_serial_and_reuse_workspace` and CC `spin1_cc_grouped_synthesis_reuses_checked_coefficients`; both also fail separately on exact parent `c200958` at the unchanged `2e-11` tolerance. No-default tests reported 219 passed and 10 failures in existing scalar-analysis tile assertions and a spin-1 analysis endpoint-count check; representative failures reproduce on `c200958`. The GL T511 q8 selector remains intact.

**Focused CC T720 q8 profile:** all 721 orders used the prepared route; 114 polar blocks used scalar fallback. Across workers, prepared passes summed to 42.7–47.8 ms, compared with 0.19–0.71 ms for row clearing, 2.36–5.00 ms for scattering, and 2.86–6.07 ms for fallback. These are test-instrumented worker sums, not wall latency; clear/scatter/fallback do not explain the remaining gap. A distinct organization of the prepared passes is the next useful mechanism to investigate.
