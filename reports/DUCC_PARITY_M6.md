# M6 — live f32 performance and accuracy ledger

**Active direction (2026-10-30):** [breadth-first native-f32 port](../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md). Immediate work is native-f32 **scalar analysis → spin-1 synthesis → spin-1 analysis** using the optimized f64 algorithms, with provisional **f32/f64 whole-call <=2.0x per cell** (stretch <=1.5x). Then return to scalar-synthesis high-degree accuracy and final native-f32 DUCC <=1.0x. **The matched Rust f64 T720 baseline has not yet been freshly recorded; all f32/f64 ratios are UNMEASURED.** Avoid presenting DUCC ratios as f32/f64 ratios.

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

**Matched Rust f64 controls:** The f64 full GL T1023 spin-1 synthesis route was preserved against its parent and below DUCC, with f64 Rust/DUCC ~0.739 q1 and ~0.714 q8 in the production checkpoint. **This is not a matching f64 T720 baseline for the new 2.0x gate.** B1–B3 must measure f32 and f64 T720 with identical equivalent inputs and the same source/compiler/affinity before reporting success.

## Accuracy status — do not confuse speed with correctness

- **Production scalar synthesis ordinary inputs via sidecar:** last random T720 fixture maximum relative L2 versus DUCC f32 about **3.88–3.89e-8**. Exceptional values decline to the legacy native route; neither makes the sidecar compliant with desired native arithmetic.
- **Isolated generic native-f32 scalar synthesis:** random full/reduced T720 relative L2 versus DUCC about **1.86–1.90e-4**. An isolated corrected m=0 leaf improves selected zonal accuracy but not random aggregate accuracy. This route is not numerically accepted.
- **Independent source-level trace** ([diagnosis](archive/experiments/numerical-causality-ducc-0.42.0-2026-10-29/REPORT.md)): 357 selected single modes, 256 dense-band pre-FFT components, 385 public single-mode cells. DUCC's f32 API uses a double Legendre recurrence. In the selected public map, generic f32 paths reach **3.2883e-3** max per-ring scaled error, while DUCC reaches **1.1906e-7** against a binary128/long-double reference. The dense high band has **4.50e-5** maximum pre-FFT Rust error vs **1.71e-9** DUCC; controlled recurrence-state substitution reduces Rust error to **1.24e-7**. The formulas agree; f32 seed/subnormal loss, cos² rounding and accumulated recurrence errors are confirmed. The FFT is secondary on these fixtures. **No science tolerance or full/random-field post-fix accuracy pass exists.**
- **Scalar analysis and spin-1 families:** the prior native-f32 suite has analytic, adjoint, finite and repeat/reset tests, but no matching independent high-degree accuracy matrix for the proposed shared-native implementations. Label their new numerical status **UNMEASURED** until each port is exercised.

## Current decision and reporting contract

**NEXT:** Stage A matched f64/f32/DUCC whole-call benchmark; B1 scalar analysis, B2 spin-1 synthesis, B3 spin-1 analysis using shared native arithmetic; Phase C numerical convergence and final DUCC parity. See [active plan](../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md). For each new source revision update four cell rows and include `f32 ms / matched f64 ms / f32:f64 / DUCC ms / f32:DUCC`, q1/q8 and full/reduced, actual dispatch, numerical status, source/compiler hashes and uncertainty. The first gate is **<=2.0x f64** individually, not a DUCC PASS; retain independent numerical/exceptional safety and f64 regression controls.

Do not claim conversion-sidecar PASS as native-f32 parity; do not promote known inaccurate routes to production. Keep long diagnostic detail in immutable archives, not here. Preserved full previous [long-form ledger](archive/DUCC_PARITY_M6-2026-10-30-pre-breadth-port.md) and original [pre-generic ledger](archive/DUCC_PARITY_M6-2026-10-29-pre-native-generic.md).
