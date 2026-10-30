# M6 — live f32 performance and accuracy ledger

**Active direction (2026-10-30):** [breadth-first native-f32 port](../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md). Exact-source 3-pair GL T720 screens exist for isolated scalar-analysis, spin-1 synthesis and spin-1 analysis candidates. B2 spin-1 synthesis meets the interim **f32/f64 <=2.0x** gate in all four cells (1.55–1.74x); B1 and B3 remain GAPs (10.7–23.5x). No candidate is production code, and independent high-degree accuracy is UNMEASURED for every candidate. See the [current follow-up matrix and source patches](archive/experiments/native-f32-breadth-port-2026-10-30/README.md). Next port the shared fused B1 analysis block. Final acceptance still requires independent numerical validation and native-f32 DUCC <=1.0x per cell.

**Source baseline:** production transform code unchanged since `c63cde440d91e3ca2810226163a191199b4ca29d`; diagnostics through `3bb97c157632bbc2dfd5e8d2daf2a19d7e8c3ec7`. Reference is pinned native-f32 DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Timings are historical warm *whole transforms* on Ryzen 5950X, GL T720, 721 latitudes; full M720 / nphi1458, reduced M180 / nphi378; q1/q8 threads. All 16 Stage B native-f32 timings come from [the saved matrix](data/ducc-parity-m6-2026-10-29/f32-io-f64-internal-stage-b/results-16-cell.csv); the four current production scalar-synthesis sidecar measurements come from [its production checkpoint](data/ducc-parity-m6-2026-10-29/f32-internal-scalar-synthesis-production/README.md).

## Focused 16-cell GL T720 f32 whole-call matrix

Ratio is Rust/DUCC (lower is faster). "Production f32 route" states what is actually in the committed production implementation, **not** an isolated candidate. Intervals are paired 95% bootstrap CIs where provided; other ratios are rounded quotients of saved median times.

| Family / geometry | Production f32 ms | DUCC f32 ms | Production Rust/DUCC | Route / status |
|---|---:|---:|---:|---|
| s0 synthesis full q1 | 15.836 | 19.131 | 0.829 [0.823,0.834] | **f64 conversion sidecar**; timing PASS, architecture debt |
| s0 synthesis full q8 | 2.790 | 2.884 | 0.961 [0.934,0.974] | **f64 conversion sidecar**; timing PASS, architecture debt |
| s0 synthesis reduced q1 | 6.486 | 8.385 | 0.775 [0.770,0.779] | **f64 conversion sidecar**; timing PASS, architecture debt |
| s0 synthesis reduced q8 | 1.184 | 1.298 | 0.885 [0.793,0.918] | **f64 conversion sidecar**; timing PASS, architecture debt |
| s0 analysis full q1 | 453.143 | 14.064 | ~32.22 | Legacy native f32; GAP |
| s0 analysis full q8 | 60.005 | 2.274 | ~26.39 | Legacy native f32; GAP |
| s0 analysis reduced q1 | 139.936 | 5.774 | ~24.24 | Legacy native f32; GAP |
| s0 analysis reduced q8 | 20.132 | 1.001 | ~20.11 | Legacy native f32; GAP |
| s1 synthesis full q1 | 1487.599 | 41.068 | ~36.22 | Legacy native f32; GAP |
| s1 synthesis full q8 | 273.640 | 5.962 | ~45.90 | Legacy native f32; GAP |
| s1 synthesis reduced q1 | 625.249 | 18.059 | ~34.62 | Legacy native f32; GAP |
| s1 synthesis reduced q8 | 125.539 | 2.673 | ~46.97 | Legacy native f32; GAP |
| s1 analysis full q1 | 1040.405 | 28.807 | ~36.12 | Legacy native f32; GAP |
| s1 analysis full q8 | 138.878 | 4.879 | ~28.46 | Legacy native f32; GAP |
| s1 analysis reduced q1 | 375.930 | 11.759 | ~31.97 | Legacy native f32; GAP |
| s1 analysis reduced q8 | 58.159 | 2.258 | ~25.76 | Legacy native f32; GAP |

**Isolated native-f32 scalar-synthesis prototype (NOT production):** the uniform-exponent eight-lane f32 scaler variant, measured in 15 matched pairs against native-f32 DUCC, gave **1.249 [1.233,1.258]** full q1, **1.256 [1.135,1.320]** full q8, **0.995 [0.992,1.009]** reduced q1 (**inconclusive**) and **1.082 [1.032,1.148]** reduced q8. Its full-grid gaps are ~25%; high-degree numerical accuracy is NOT accepted. The original generic native-f32 patch had 1.355/1.338/1.054/1.134; all were slower. Exact source patches, source/binary hashes, paired records and rejected scaling variants: [native generic synthesis archive](archive/experiments/native-generic-scalar-synthesis-2026-10-29/README.md).

**Matched Rust f64 controls:** The saved f64 full GL T1023 spin-1 synthesis parent/candidate screen found bitwise-identical outputs, with q1 about 0.24% slower and q8 inconclusive. That control predates generic scalar-order integration and does not protect the current B2 source; rerun it before promotion. The original production checkpoint's Rust/DUCC ~0.739 q1 and ~0.714 q8 remains historical and is not substituted for current candidate measurements.

## Isolated native-f32 B1–B3 candidates — current 3-pair screens

Each row gives whole-call Rust f32 and matched same-source f64 milliseconds, their ratio, and a separate f32/DUCC ratio. DUCC uses native f32. All rows are **screening evidence**, not 15-pair performance claims. `PASS` applies only to the interim <=2.0x f64 timing gate. Random-fixture distance to DUCC is not independent accuracy validation. Exact source, binary and fixture hashes, per-pair timings, test notes, and patches are in the [follow-up evidence package](archive/experiments/native-f32-breadth-port-2026-10-30/README.md), [CSV](archive/experiments/native-f32-breadth-port-2026-10-30/followup-matrix.csv), and [JSON](archive/experiments/native-f32-breadth-port-2026-10-30/followup-screen.json).

| Candidate family | Cell | f32 ms | f64 ms | f32/f64 | DUCC ms | f32/DUCC | Interim / accuracy |
|---|---|---:|---:|---:|---:|---:|---|
| Scalar analysis | Full q1 | 461.610 | 19.612 | 23.54x | 14.550 | 31.77x | GAP / UNMEASURED |
| Scalar analysis | Full q8 | 63.461 | 3.023 | 20.99x | 2.232 | 28.21x | GAP / UNMEASURED |
| Scalar analysis | Reduced q1 | 138.536 | 8.210 | 16.87x | 5.927 | 23.41x | GAP / UNMEASURED |
| Scalar analysis | Reduced q8 | 19.842 | 1.220 | 16.26x | 0.975 | 20.32x | GAP / UNMEASURED |
| Spin-1 synthesis | Full q1 | 67.687 | 38.882 | 1.74x | 42.160 | 1.61x | PASS / UNMEASURED |
| Spin-1 synthesis | Full q8 | 10.524 | 6.371 | 1.65x | 5.942 | 1.77x | PASS / UNMEASURED |
| Spin-1 synthesis | Reduced q1 | 27.745 | 16.638 | 1.65x | 18.119 | 1.53x | PASS / UNMEASURED |
| Spin-1 synthesis | Reduced q8 | 4.448 | 2.855 | 1.55x | 2.673 | 1.71x | PASS / UNMEASURED |
| Spin-1 analysis | Full q1 | 506.474 | 37.495 | 13.51x | 32.333 | 16.07x | GAP / UNMEASURED |
| Spin-1 analysis | Full q8 | 72.530 | 6.161 | 11.98x | 5.091 | 15.90x | GAP / UNMEASURED |
| Spin-1 analysis | Reduced q1 | 174.224 | 15.230 | 11.19x | 13.122 | 13.10x | GAP / UNMEASURED |
| Spin-1 analysis | Reduced q8 | 25.837 | 2.414 | 10.70x | 2.204 | 13.43x | GAP / UNMEASURED |

**Execution blockers:** f32 scalar analysis does not override `ShtScalar`'s fused paired-analysis hooks, so it declines the optimized fused block and uses per-degree recurrence callbacks. The best spin-1 analysis candidate fuses native-f32 two-channel recurrence, but lane-wise exponent restoration remains expensive and the f64 production route does not yet use that new kernel. B2 uses the shared native-f32/f64 order-major reflected kernel on GL and meets the interim timing target; its f64 T1023 protection control and independent accuracy are still open. All candidates use native f32 recurrence, SIMD, accumulation, spectra, FFT and output; widening exists only in matched diagnostic f64 workers.

## Accuracy status — do not confuse speed with correctness

- **Production scalar synthesis ordinary inputs via sidecar:** last random T720 fixture maximum relative L2 versus DUCC f32 about **3.88–3.89e-8**. Exceptional values decline to the legacy native route; neither makes the sidecar compliant with desired native arithmetic.
- **Isolated generic native-f32 scalar synthesis:** random full/reduced T720 relative L2 versus DUCC about **1.86–1.90e-4**. An isolated corrected m=0 leaf improves selected zonal accuracy but not random aggregate accuracy. This route is not numerically accepted.
- **Independent source-level trace** ([diagnosis](archive/experiments/numerical-causality-ducc-0.42.0-2026-10-29/REPORT.md)): 357 selected single modes, 256 dense-band pre-FFT components, 385 public single-mode cells. DUCC's f32 API uses a double Legendre recurrence. In the selected public map, generic f32 paths reach **3.2883e-3** max per-ring scaled error, while DUCC reaches **1.1906e-7** against a binary128/long-double reference. The dense high band has **4.50e-5** maximum pre-FFT Rust error vs **1.71e-9** DUCC; controlled recurrence-state substitution reduces Rust error to **1.24e-7**. The formulas agree; f32 seed/subnormal loss, cos² rounding and accumulated recurrence errors are confirmed. The FFT is secondary on these fixtures. **No science tolerance or full/random-field post-fix accuracy pass exists.**
- **Plan-time cos² control, isolated native-f32 scalar synthesis:** at `m=5`, ring 5, the f32 `cos²` operand changes one ULP toward binary128; the first leading-factor and recurrence-state results change one ULP, and the degree-719 state error falls about 74%. On 357 direct outputs RMS relative error falls 24.2%, with 102 improved, 255 unchanged and none worsened. Across 13,090 public pixels relative L2 falls 59.0%; 1,992 improve, 16 worsen, and the correctly-rounded count is unchanged. This is a selected-fixture causal improvement, not a numerical acceptance pass. Full result and recurrence table: [cos² control](archive/experiments/cos2-plan-time-control-2026-10-30/README.md).
- **Dense and random validation of plan-time cos²:** on 256 pre-FFT high-band outputs, 62 absolute errors improve, 2 worsen and 192 stay unchanged. RMS absolute error falls `9.94269e-6 → 6.40809e-6`; cancellation-scaled RMS falls `1.01733e-3 → 7.15017e-4`. Correctly rounded output count remains zero; maximum absolute error is `4.32209e-5`. With recurrence-factor preparation fixed, random full/reduced relative L2 to pinned DUCC changes `1.83217e-4 → 1.70306e-4` and `1.81741e-4 → 1.60901e-4`. Many map pixels move both closer and farther from DUCC. DUCC's double recurrence is the random comparison reference; no high-precision random-map oracle exists. The f32 candidate remains isolated. Machine-readable data, hashes and patches: [cos² dense/random package](archive/experiments/cos2-plan-time-control-2026-10-30/README.md).
- **Scalar analysis and spin-1 families:** the prior native-f32 suite has analytic, adjoint, finite and repeat/reset tests, but no matching independent high-degree accuracy matrix for the proposed shared-native implementations. Label their new numerical status **UNMEASURED** until each port is exercised.

## Current decision and reporting contract

**NEXT:** port the f64 fused scalar-analysis order block into shared f32/f64 control flow with typed SIMD/scaling leaves, then confirm f32 actually takes that route. B1/B3 remain timing GAPs; B2's interim screen passes, with accuracy and the protected f64 T1023 rerun still open. Independent high-degree accuracy is UNMEASURED for all candidates. Keep scalar-synthesis compensation deferred. Every new screen reports per-cell `f32 ms / f64 ms / f32:f64 / DUCC ms / f32:DUCC`, actual dispatch, independent numerical status, source/compiler/fixture hashes and uncertainty.

Do not claim conversion-sidecar PASS as native-f32 parity; do not promote known inaccurate routes to production. Keep long diagnostic detail in immutable archives, not here. Preserved full previous [long-form ledger](archive/DUCC_PARITY_M6-2026-10-30-pre-breadth-port.md) and original [pre-generic ledger](archive/DUCC_PARITY_M6-2026-10-29-pre-native-generic.md).
