# M6 — live f32 performance and accuracy ledger

**Active direction (2026-10-30):** [breadth-first native-f32 port](../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md). B1 scalar analysis and B3 spin-1 analysis remain timing GAPs at 7.00–10.55x and 3.97–5.13x matched f64, respectively. B2 spin-1 synthesis passes the interim **f32/f64 <=2.0x** gate in all four cells in 15 pairs (1.59–1.75x); its protected f64 T1023 output is bitwise unchanged, q1 is 1.4% slower, and q8 is inconclusive. B3's shared SIMD scaled accumulator improves the previous candidate 2.5–3x; its T511 DUCC reference passes, while independent T720 high-degree accuracy remains open. Current source candidates exist for all three families; timing screens do not establish final parity. See the [B1/B2 package](archive/experiments/native-f32-breadth-port-2026-10-30/current-dev-followup.json) and [B3 screen, tests and hashes](archive/experiments/native-f32-breadth-port-2026-10-30/b3-spin1-analysis-current.md). Final acceptance still requires independent numerical validation and native-f32 DUCC <=1.0x per cell.

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

**Matched Rust f64 controls:** The current B2 source was compared with parent on GL T1023 full spin-1 synthesis in 15 pairs per q. Outputs are bitwise identical. q1 candidate/parent is **1.014 [1.012,1.029]** (about 1.4% slower); q8 is **1.001 [0.990,1.020]** (INCONCLUSIVE). The original production checkpoint's Rust/DUCC ~0.739 q1 and ~0.714 q8 remains historical; a same-run current-candidate f64/DUCC measurement is not yet available. Full pair records and binary hashes: [current-dev evidence](archive/experiments/native-f32-breadth-port-2026-10-30/current-dev-followup.json).

## Isolated native-f32 B1–B3 candidates — latest T720 screens

Each row gives whole-call Rust f32 and matched same-source f64 milliseconds, their ratio, and a separate f32/DUCC ratio. DUCC uses native f32. B2 has 15 interleaved process pairs and paired-ratio confidence intervals; B1 and the final B3 candidate use three-pair screens. B3 pair ranges below are not confidence intervals. `PASS` applies only to the interim <=2.0x f64 timing gate. Random-fixture distance to DUCC is not independent accuracy validation. Exact source, binary and fixture hashes, pair data, tests, and recoverable patches are in the [current-dev evidence package](archive/experiments/native-f32-breadth-port-2026-10-30/current-dev-followup.json), [earlier follow-up evidence](archive/experiments/native-f32-breadth-port-2026-10-30/README.md), and [patch archive](archive/experiments/native-f32-breadth-port-2026-10-30/patches/).

| Candidate family | Cell | f32 ms | f64 ms | f32/f64 | DUCC ms | f32/DUCC | Interim / accuracy |
|---|---|---:|---:|---:|---:|---:|---|
| Scalar analysis | Full q1 | 208.845 | 19.802 | 10.55x | 14.619 | 14.21x | GAP / high-degree UNMEASURED |
| Scalar analysis | Full q8 | 28.965 | 3.193 | 9.07x | 2.246 | 12.90x | GAP / high-degree UNMEASURED |
| Scalar analysis | Reduced q1 | 63.415 | 8.423 | 7.53x | 5.939 | 10.70x | GAP / high-degree UNMEASURED |
| Scalar analysis | Reduced q8 | 9.520 | 1.361 | 7.00x | 0.972 | 9.89x | GAP / high-degree UNMEASURED |
| Spin-1 synthesis | Full q1 | 67.199 | 38.608 | 1.747x [1.731,1.771] | 41.894 | 1.607x [1.604,1.630] | PASS / partial reference |
| Spin-1 synthesis | Full q8 | 10.398 | 6.336 | 1.650x [1.587,1.678] | 6.038 | 1.718x [1.710,1.782] | PASS / partial reference |
| Spin-1 synthesis | Reduced q1 | 27.434 | 16.402 | 1.672x [1.669,1.675] | 18.142 | 1.513x [1.484,1.527] | PASS / partial reference |
| Spin-1 synthesis | Reduced q8 | 4.540 | 2.827 | 1.591x [1.557,1.616] | 2.702 | 1.661x [1.592,1.679] | PASS / partial reference |
| Spin-1 analysis (current) | Full q1 | 170.261 | 38.391 | 4.571x (4.421–4.596) | 30.920 | 5.543x (5.489–5.625) | GAP / partial reference |
| Spin-1 analysis (current) | Full q8 | 25.051 | 6.274 | 3.969x (3.944–3.994) | 4.549 | 5.507x (5.493–5.525) | GAP / partial reference |
| Spin-1 analysis (current) | Reduced q1 | 79.439 | 15.603 | 5.126x (5.066–5.139) | 12.297 | 6.484x (6.460–6.501) | GAP / partial reference |
| Spin-1 analysis (current) | Reduced q8 | 11.567 | 2.429 | 4.762x (4.512–4.899) | 1.937 | 6.034x (5.844–6.161) | GAP / partial reference |

**Execution blockers:** B1's typed fused block executes, but one bounded direct-basis SIMD change improved whole-call timing only 6–10%; 45.9–52.9% of pair accumulations still use scaled accumulation, leaving a 7.00–10.55x gap. B3's cached-factor SIMD FMA accumulator removed lane-wise per-component restoration and improved its prior full-q1 result about 3x, yet the final gap remains 3.97–5.13x. The scaled B3 recurrence still runs a range test after each degree pair and may normalize lane mantissas/exponents; its time share is unmeasured. Perf-event profiling is blocked by host policy (`perf_event_paranoid=4`). Stop these bounded passes and retain their GAP status. B2 uses the shared native-f32/f64 order-major reflected kernel and passes the interim gate in 15 pairs; its f64 T1023 output is unchanged, with q1 1.4% slower and q8 inconclusive. Candidate f32 routes use native recurrence, SIMD, accumulation, spectra, FFT and output; widening occurs only in matched diagnostic f64 workers.

## Accuracy status — do not confuse speed with correctness

- **Production scalar synthesis ordinary inputs via sidecar:** last random T720 fixture maximum relative L2 versus DUCC f32 about **3.88–3.89e-8**. Exceptional values decline to the legacy native route; neither makes the sidecar compliant with desired native arithmetic.
- **Isolated generic native-f32 scalar synthesis:** random full/reduced T720 relative L2 versus DUCC about **1.86–1.90e-4**. An isolated corrected m=0 leaf improves selected zonal accuracy but not random aggregate accuracy. This route is not numerically accepted.
- **Independent source-level trace** ([diagnosis](archive/experiments/numerical-causality-ducc-0.42.0-2026-10-29/REPORT.md)): 357 selected single modes, 256 dense-band pre-FFT components, 385 public single-mode cells. DUCC's f32 API uses a double Legendre recurrence. In the selected public map, generic f32 paths reach **3.2883e-3** max per-ring scaled error, while DUCC reaches **1.1906e-7** against a binary128/long-double reference. The dense high band has **4.50e-5** maximum pre-FFT Rust error vs **1.71e-9** DUCC; controlled recurrence-state substitution reduces Rust error to **1.24e-7**. The formulas agree; f32 seed/subnormal loss, cos² rounding and accumulated recurrence errors are confirmed. The FFT is secondary on these fixtures. **No science tolerance or full/random-field post-fix accuracy pass exists.**
- **Plan-time cos² control, isolated native-f32 scalar synthesis:** at `m=5`, ring 5, the f32 `cos²` operand changes one ULP toward binary128; the first leading-factor and recurrence-state results change one ULP, and the degree-719 state error falls about 74%. On 357 direct outputs RMS relative error falls 24.2%, with 102 improved, 255 unchanged and none worsened. Across 13,090 public pixels relative L2 falls 59.0%; 1,992 improve, 16 worsen, and the correctly-rounded count is unchanged. This is a selected-fixture causal improvement, not a numerical acceptance pass. Full result and recurrence table: [cos² control](archive/experiments/cos2-plan-time-control-2026-10-30/README.md).
- **Dense and random validation of plan-time cos²:** on 256 pre-FFT high-band outputs, 62 absolute errors improve, 2 worsen and 192 stay unchanged. RMS absolute error falls `9.94269e-6 → 6.40809e-6`; cancellation-scaled RMS falls `1.01733e-3 → 7.15017e-4`. Correctly rounded output count remains zero; maximum absolute error is `4.32209e-5`. With recurrence-factor preparation fixed, random full/reduced relative L2 to pinned DUCC changes `1.83217e-4 → 1.70306e-4` and `1.81741e-4 → 1.60901e-4`. Many map pixels move both closer and farther from DUCC. DUCC's double recurrence is the random comparison reference; no high-precision random-map oracle exists. The f32 candidate remains isolated. Machine-readable data, hashes and patches: [cos² dense/random package](archive/experiments/cos2-plan-time-control-2026-10-30/README.md).
- **Scalar analysis and spin-1 families:** B1 passes its T720 analytic/adjoint/workspace-reset test (relative L2 about 4.28e-5, maximum absolute error about 5.6e-6). B3 passes the existing T511 pinned-DUCC analysis test and the all-grid f32 spin operation/adjoint test; its T720 random fixture is 1.85e-5 / 2.17e-5 relative L2 versus DUCC (full/reduced), but that is not an independent accuracy PASS. B2 has selected T1023 DUCC-reference coverage; its T720 high-degree matrix remains open. Do not infer full-field accuracy from these checks.

## Current decision and reporting contract

**NEXT:** all three missing families now have native-f32 candidates and T720 screens. Resume the deferred scalar-synthesis high-degree accuracy work using the saved `3bb97c1` trace; after that, return to B1/B3 timing GAPs and complete the independent accuracy matrix. B2 passes only the interim f32/f64 timing gate and remains short of DUCC parity. Every screen reports per-cell `f32 ms / f64 ms / f32:f64 / DUCC ms / f32:DUCC`, actual dispatch, numerical status, source/compiler/fixture hashes and uncertainty.

Do not claim conversion-sidecar PASS as native-f32 parity; do not promote known inaccurate routes to production. Keep long diagnostic detail in immutable archives, not here. Preserved full previous [long-form ledger](archive/DUCC_PARITY_M6-2026-10-30-pre-breadth-port.md) and original [pre-generic ledger](archive/DUCC_PARITY_M6-2026-10-29-pre-native-generic.md).
