# RustSHT — performance snapshot

**Checked branch:** `8074844c4d359ee84294a94c87ffd14f94c2798d` (2026-10-23 23:17 UTC). Re-fetch before claiming newer results. **Active:** [spin-1 prepared scalar-ordered adapter plan](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md). [Previous full status, preserved exactly](reports/archive/context-2026-10-23/prepared-scalar-adapter/PERFORMANCE_STATUS.md).

| f64 spin-1 workload | RustSHT / DUCC | Rust / DUCC median | Scope |
|---|---:|---:|---|
| **CC T720 full synthesis q8** | **2.48×** | 15.103 / 6.094 ms | Earlier focused source |
| **CC T720 quarter synthesis q8** | **2.62×** | 6.962 / 2.660 ms | Earlier focused source |
| **F1 T720 full synthesis q8** | **2.41×** | 14.131 / 5.868 ms | Earlier focused source |
| **GL T720 full synthesis q8** | **2.40×** | 14.109 / 5.885 ms | Earlier focused source |
| **GL T511 full synthesis q8** | **2.95×** | 7.686 / 2.606 ms | New measured order-major dispatch `4cbc1838` |
| CC T720 full analysis q8 | 1.04× | 21.887 / 21.044 ms | Analysis closeout checkout `bab010ce` |
| F1 T720 full analysis q8 | 1.15× | 24.481 / 21.277 ms | Same closeout |
| GL T720 full analysis q8 | 2.08× | 8.947 / 4.287 ms | Same closeout |
| CC T1023 full analysis q8 | 1.36× | 48.322 / 35.508 ms | Earlier focused source |

**Scalar CC/F1 status:** selected full synthesis still ~1.01–1.04× DUCC; selected scalar analyses faster, but no exhaustive/current uniform parity matrix. Historical ratios are not current measurements.

**Completed and accepted:** `8f99a274` analysis range-proof reuse; GL/CC/F1 analysis closeout passes focused reference/extreme/poison/adjoint, no-default, fmt and strict Clippy at `4cbc1838` (source unchanged from closeout). **New major win:** `4cbc1838` chooses existing **order-major** synthesis for GL T511 q8, `14.140→7.686 ms` Rust (−45.6%, candidate vs old relative L2 ≤1.54e-15); DUCC `2.606 ms`. q4 retains old hemisphere selector; no general T720 recurrence improvement yet.

**Rejected, do not repeat unchanged:** two high-level scalar ordered passes (`15.056→24.029 ms`); reflected C64 half-lane packing (`35.884 vs 15.539 ms`); eight-accumulator parity/register spills; reflected scaled-prefix stream (`20.225 vs 15.174 ms`); block-local fallback ~18–19% worse; GL/CC/F1 256-pair analysis tiles (14%/11.4%/0.7% slower) and q8 fused analysis (neutral). Detailed source reports remain at `reports/*.md`; raw benchmark captures for some closeouts remain local, not in Git.

**NEXT production step:** adapt the **existing low-level scalar ordered prepared SIMD recurrence** from `src/legendre/scalar_ordered.rs` into `src/spin_transform.rs`'s T720+ order-major spin-1 synthesis. Keep one E/B conversion + reflected ring packing, sequential Q/U **four-accumulator** SIMD scalar passes, checked scaled-prefix/polar fallback, and transactional order output. **One CC T720 full q8 matched A/B first**, broader benchmark only after a useful source change. This is a source-grounded **hypothesis**, not measured acceleration. GL T720 analysis 2.08× is a later >=2× target; f32 and benchmark completeness paused.
