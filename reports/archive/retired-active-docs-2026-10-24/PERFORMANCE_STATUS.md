# RustSHT — performance snapshot

**Completion: open.** The 2026-10-24 owner rule requires native DUCC 0.42.0 parity across supported f64 spin-0/1 GL/CC/F1 analysis and synthesis, full/reduced mmax, and 1/2/4/8 threads. These are selected spin-1 synthesis cells; each ratio above 1.00 remains a gap. See the [all-cases ledger](plans/f64-ducc-parity-all-cases-2026-10-24.md).

**Branch:** `opt/sht-performance-parity`. **Reference:** pinned DUCC 0.42.0 (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`).

| f64 spin-1 synthesis | Rust / DUCC median (ms) | Ratio | Exact parent → candidate (ms) |
|---|---:|---:|---:|
| CC T720 full q8 | 7.534 / 6.091 | 1.24× | 7.845 → 7.534 |
| CC T720 reduced m180 q8 | 3.850 / 2.662 | 1.45× | 4.238 → 3.850 |
| F1 T720 full q8 | 5.528 / 5.861 | 0.94× | 6.285 → 5.528 |
| GL T720 full q8 | 5.555 / 5.859 | 0.95× | 6.250 → 5.555 |
| CC T1023 full q8 | 24.634 / 21.724 | 1.13× | 47.095 → 24.634 |
| GL T1023 full q8 | 22.468 / 23.097 | 0.97× | 23.442 → 22.468 |
| GL T1023 full q1 | 169.779 / 120.825 | 1.41× | 174.473 → 169.779 |
| GL T511 full q8 | 3.175 / 2.384 | 1.33× | 3.182 → 3.175 |

**Latest accepted experiment — GL T1023 full q1 (`78eca228`):** three interleaved parent/candidate/DUCC process triples pinned to CPU 0, 500 ms target per worker, 15 timed calls per backend. Medians and p10–p90 were parent 174.473 ms (172.105–185.959), candidate 169.779 ms (168.670–181.862), and DUCC 120.825 ms (119.627–122.032). Paired candidate/parent ratios were 0.973, 0.966, and 0.982. Candidate output was bitwise identical to parent, with `3.795e-13` relative L2 error to DUCC; repeat/reset checks passed. Full evidence: `reports/data/spin1-polar-inverse-sine-hoist-2026-10-24/results.json`.

**Open analysis gaps:** GL T720 full q1 shared checked-recurrence trial was rejected after one targeted correction (candidate 56.571 ms, parent 52.004 ms, DUCC 29.330 ms). GL T720 full q8 remains recurrence-bound at 7.317 ms versus 4.267 ms DUCC. Continue with a distinct q1 mechanism, then address q8 and the remaining parity gaps.
