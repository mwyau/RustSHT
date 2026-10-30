# M6 — live f32 performance and accuracy ledger

**Active direction (2026-10-30):** [breadth-first native-f32 port](../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md). Isolated native-f32 candidates now exist for **scalar analysis, spin-1 synthesis and spin-1 analysis**. Their 12-cell GL T720 screens are complete; every cell misses the provisional **f32/f64 <=2.0x** target (stretch <=1.5x). None is production code. Phase C scalar-synthesis accuracy work resumes now; final acceptance still requires independent numerical validation and native-f32 DUCC <=1.0x per cell.

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

**Matched Rust f64 controls:** The f64 full GL T1023 spin-1 synthesis route remains protected. A current B2 parent/candidate screen found bitwise-identical outputs; q1 was a small ~0.24% slowdown and q8 was inconclusive. See the candidate evidence package for pair data. The original production checkpoint's Rust/DUCC ~0.739 q1 and ~0.714 q8 remains historical and is not substituted for current candidate measurements.

## Isolated native-f32 B1–B3 candidates — screen only

Each cell lists matched native-f32 / same-source Rust f64 whole-call milliseconds, f32/f64 ratio, and the separate f32/DUCC ratio. All cells used three pairs. All twelve are **GAP** against the 2.0x interim limit; f32/DUCC is not an accuracy pass. Exact source, binary and fixture hashes, candidate patches, raw pairs and protected T1023 f64 control are in the [evidence package](archive/experiments/native-f32-breadth-port-2026-10-30/README.md) and its [machine-readable matrix](archive/experiments/native-f32-breadth-port-2026-10-30/matrix.json).

| Candidate family | Full q1: f32/f64 ms (ratio); f32/DUCC | Full q8 | Reduced q1 | Reduced q8 | Independent accuracy |
|---|---:|---:|---:|---:|---|
| Scalar analysis | 461.610 / 19.612 (23.54x); 31.77x | 63.461 / 3.023 (20.99x); 28.21x | 138.536 / 8.210 (16.87x); 23.41x | 19.842 / 1.220 (16.26x); 20.32x | UNMEASURED |
| Spin-1 synthesis | 1160.539 / 39.305 (29.53x); 26.11x | 221.618 / 8.112 (27.32x); 35.37x | 544.511 / 39.909 (13.64x); 29.25x | 85.722 / 3.059 (28.02x); 28.22x | UNMEASURED |
| Spin-1 analysis | 907.290 / 37.528 (24.18x); 30.42x | 123.190 / 5.832 (21.12x); 27.33x | 268.704 / 15.057 (17.85x); 22.09x | 38.507 / 2.535 (15.19x); 20.39x | UNMEASURED |

Random-fixture relative-L2 versus DUCC is 1.83e-5 / 2.21e-5 for scalar analysis full/reduced, 2.14e-4 / 1.95e-4 for spin-1 synthesis, and 1.85e-5 / 2.17e-5 for spin-1 analysis. These comparisons do not independently validate high-degree accuracy. Candidate execution uses native f32 recurrence, SIMD, accumulation, spectra and FFT; f64 widening occurs only inside matched diagnostic reference workers.

## Accuracy status — do not confuse speed with correctness

- **Production scalar synthesis ordinary inputs via sidecar:** last random T720 fixture maximum relative L2 versus DUCC f32 about **3.88–3.89e-8**. Exceptional values decline to the legacy native route; neither makes the sidecar compliant with desired native arithmetic.
- **Isolated generic native-f32 scalar synthesis:** random full/reduced T720 relative L2 versus DUCC about **1.86–1.90e-4**. An isolated corrected m=0 leaf improves selected zonal accuracy but not random aggregate accuracy. This route is not numerically accepted.
- **Independent source-level trace** ([diagnosis](archive/experiments/numerical-causality-ducc-0.42.0-2026-10-29/REPORT.md)): 357 selected single modes, 256 dense-band pre-FFT components, 385 public single-mode cells. DUCC's f32 API uses a double Legendre recurrence. In the selected public map, generic f32 paths reach **3.2883e-3** max per-ring scaled error, while DUCC reaches **1.1906e-7** against a binary128/long-double reference. The dense high band has **4.50e-5** maximum pre-FFT Rust error vs **1.71e-9** DUCC; controlled recurrence-state substitution reduces Rust error to **1.24e-7**. The formulas agree; f32 seed/subnormal loss, cos² rounding and accumulated recurrence errors are confirmed. The FFT is secondary on these fixtures. **No science tolerance or full/random-field post-fix accuracy pass exists.**
- **Scalar analysis and spin-1 families:** the prior native-f32 suite has analytic, adjoint, finite and repeat/reset tests, but no matching independent high-degree accuracy matrix for the proposed shared-native implementations. Label their new numerical status **UNMEASURED** until each port is exercised.

## Current decision and reporting contract

**NEXT:** Phase C scalar-synthesis numerical convergence from the saved `3bb97c1` trace, beginning with seed normalization and then plan-time cos² as separate substitutions. B1–B3 candidates remain performance GAPs and accuracy UNMEASURED. For new revisions, report per-cell `matched f32 ms / matched f64 ms / f32:f64 / DUCC ms / f32:DUCC`, actual dispatch, independent numerical status, source/compiler/fixture hashes and uncertainty. Revisit family performance after measuring their scaled fallback mechanisms; do not promote known inaccurate routes.

Do not claim conversion-sidecar PASS as native-f32 parity; do not promote known inaccurate routes to production. Keep long diagnostic detail in immutable archives, not here. Preserved full previous [long-form ledger](archive/DUCC_PARITY_M6-2026-10-30-pre-breadth-port.md) and original [pre-generic ledger](archive/DUCC_PARITY_M6-2026-10-29-pre-native-generic.md).
