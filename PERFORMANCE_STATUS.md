# RustSHT — current f64 / pinned DUCC performance

**OPEN (2026-10-25):** Full per-cell parity is **not** established. Reference: native DUCC 0.42.0, commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; host AMD Ryzen 9 5950X, x86-64-v3/AVX2/FMA. Comparisons below are selected matched, warm whole-transforms in milliseconds; **q1/q8 are thread counts**, not an average across configurations. Lower is better. Do not treat historical rows as a current full matrix.

| f64 spin-1 workload | Rust ms | DUCC ms | Rust/DUCC | Standing and evidence |
|---|---:|---:|---:|---|
| GL T1023 full synthesis q1 | **90.402** | 124.899 | **0.724×** | **PASS, matched**: reflected-component dispatch `7ef6d6e1`, exact parent 173.557 ms; rel-L2 2.60e-13. [Root cause](reports/T1023_Q1_ROOT_CAUSE.md). |
| GL T720 full analysis q1 | 45.416 | 29.522 | 1.538× | **GAP, matched**: accepted SIMD safety scan `0961586a`; exact parent 49.615 ms, bit-identical to parent. |
| GL T720 full analysis q8 | 6.589 | 4.259 | 1.547× | **GAP, matched**: same accepted scan; exact parent 7.390 ms, bit-identical to parent. |
| CC T720 reduced m180 synthesis q1 | 27.547 | 18.684 | 1.474× | **GAP, preliminary**: exact parent 34.688 ms; candidate improved 20.6%, but Rust workers saw ~10% runqueue wait. |
| CC T720 reduced m180 synthesis q8 | **2.552** | 2.666 | **0.957×** | **PASS, matched**: safe exact-pole skip `80f36d21`, exact parent 3.461 ms; three clean interleaved pairs; candidate/DUCC rel-L2 3.38e-13. |
| CC T720 full m720 synthesis q8 | **5.582** | 6.082 | **0.918×** | **PASS, matched**: same candidate, exact parent 6.548 ms; three interleaved pairs; candidate/DUCC rel-L2 2.07e-13. |

**Accepted T720 scan:** q1 Callgrind 7.521B→6.892B instructions over eight transforms (−8.4%); exclusive safety-scan 655.8M→119.3M (−81.8%). Focused release correctness/extreme/adjoint/poison tests passed. Source: [rolling checkpoints](reports/PROGRESS.md) and commit `0961586a`. The later shared-scaled-check and cosine-preweight variants **failed and were reverted**, so are not reflected in this ledger.

**Next performance priority:** GL T720 spin-1 analysis recurrence/accumulation. Matched Rust phase helper: q1 ~38.781 ms recurrence + 6.310 ms FFT in 45.454 ms total; q8 ~5.286 ms recurrence + 0.890 ms FFT in 6.456 ms total. These phase timings are instrumented Rust-only evidence, **not a DUCC stage comparison**. CC T720 synthesis q8 full and reduced now pass on the measured cells; reduced q1 remains a 1.47× provisional gap. Refresh remaining older gaps after the analysis investigation.

**Historical snapshots — NOT certified at current HEAD:** F1 T720 / GL T720 / GL T1023 full spin-1 synthesis q8 were ~0.94/0.95/0.97× DUCC on older dated source; CC T1023 full q8 was 24.634/21.724 ms (1.13× gap), GL T511 full q8 3.175/2.384 ms (1.33× gap). Earlier CC T720 1.24×/1.45× ratios are **superseded** by the current rebaseline above. Spin-0 F1 wins are preserved in [dated benchmark evidence](reports/data/ducc-scalar-f64-f1-performance-2026-10-23/summary.csv), **not a latest-HEAD parity result**.

**Validation gates:** f64 spin-0/1 GL/CC/F1, full/reduced mmax, q1/q2/q4/q8, S/A/S*/A*, numerical accuracy including extremes/poles and scratch reset, and any distinct kernels. Current comprehensive PASS/GAP/UNMEASURED/INCONCLUSIVE matrix is still missing. Two pre-existing reduced-order CC/F1 q1-vs-q8 tolerance failures remain open; do not weaken tolerances. Follow [active plan](plans/f64-ducc-parity-all-cases-2026-10-24.md).
