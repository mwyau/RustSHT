# Scaled-prefix accumulator T2047 q1 matrix result

## Decision

All six fresh-process pairs passed the predeclared timing, numerical, PSI, placement, and interference gates. The T2047 candidate/baseline median paired ratio was `0.407892` with a 95% bootstrap interval of `0.407274–0.408108` (50,000 resamples, seed `20261017`). Candidate RustSHT latency was 1370.613 ms at the median of process medians, versus 3360.075 ms for the clean baseline (59.21% lower).

Together with the T1023 matrix, both high-resolution q1 criteria pass. The T511 control later stopped at baseline pair 03 because the DUCC series failed stability; the full q1 gate is unresolved. The kernel remains experimental; these results do not route it in production.

At T2047 the candidate measured 3.666x DUCC and 5.040x SHTns using the median of candidate process medians; RustSHT remains materially slower. RustSHT output relative-L2 error against DUCC was `3.015e-13` and maximum absolute error was `1.110e-12` in every process. SHTns/DUCC relative-L2 error was `2.425e-13`.

## Frozen configuration

The source, extension, harness, reference revisions, software versions, host specifications, and workload are fixed by [q1 protocol v1](scaled-prefix-accumulator-q1-v1-protocol.md). Candidate source SHA-256: `edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`; candidate release extension SHA-256: `48db7ad3a6ba4c7fb0a8c50a303d9fbe948a7cac989c6f8a6e39b2f5e1b40184`; clean baseline extension SHA-256: `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`. Workload: GL f64 scalar `STANDARD` `synthesis_into`, T2047 (`2048x4096`), full `mmax`, `phi0=0.37`, q1. Each fresh process used one warm-up and five timed full transformations for RustSHT, DUCC 0.42.0, and SHTns.

## Backend timings

Each row gives all five samples, median, population CV, and first-three/final-two median drift. The fixed CV and drift limits are 5%.

| Pair | Variant | Backend | Samples (ms) | Median (ms) | CV | Drift |
|---:|---|---|---|---:|---:|---:|
| 01 | Baseline | RustSHT | 3361.805, 3366.924, 3354.777, 3359.366, 3362.696 | 3361.805 | 0.1190% | 0.0230% |
| 01 | Baseline | DUCC | 375.458, 378.531, 376.537, 377.822, 375.231 | 376.537 | 0.3428% | 0.0028% |
| 01 | Baseline | SHTns | 271.750, 272.179, 271.289, 271.461, 272.569 | 271.750 | 0.1726% | 0.0975% |
| 01 | Candidate | RustSHT | 1374.883, 1371.847, 1367.853, 1368.749, 1370.919 | 1370.919 | 0.1806% | 0.1468% |
| 01 | Candidate | DUCC | 443.661, 440.256, 440.159, 438.036, 439.035 | 440.159 | 0.4312% | 0.3909% |
| 01 | Candidate | SHTns | 272.861, 273.310, 273.077, 273.081, 272.500 | 273.077 | 0.0999% | 0.1049% |
| 02 | Baseline | RustSHT | 3372.713, 3369.394, 3369.812, 3366.145, 3363.938 | 3369.394 | 0.0906% | 0.1416% |
| 02 | Baseline | DUCC | 376.591, 375.362, 376.083, 374.629, 375.058 | 375.362 | 0.1880% | 0.3302% |
| 02 | Baseline | SHTns | 274.617, 276.438, 272.573, 276.072, 275.022 | 275.022 | 0.4944% | 0.3382% |
| 02 | Candidate | RustSHT | 1372.140, 1369.719, 1371.287, 1371.724, 1374.226 | 1371.724 | 0.1061% | 0.1231% |
| 02 | Candidate | DUCC | 375.174, 375.993, 376.685, 377.280, 377.139 | 376.685 | 0.2077% | 0.3229% |
| 02 | Candidate | SHTns | 271.332, 272.106, 272.960, 271.941, 270.832 | 271.941 | 0.2658% | 0.2646% |
| 03 | Baseline | RustSHT | 3365.786, 3367.739, 3364.568, 3360.640, 3365.493 | 3365.493 | 0.0696% | 0.0808% |
| 03 | Baseline | DUCC | 374.633, 375.070, 377.568, 376.402, 374.043 | 375.070 | 0.3398% | 0.0407% |
| 03 | Baseline | SHTns | 273.786, 272.730, 271.966, 272.293, 272.578 | 272.578 | 0.2258% | 0.1080% |
| 03 | Candidate | RustSHT | 1372.401, 1370.039, 1377.074, 1373.091, 1374.641 | 1373.091 | 0.1705% | 0.1067% |
| 03 | Candidate | DUCC | 375.144, 373.119, 373.686, 373.675, 373.938 | 373.686 | 0.1796% | 0.0322% |
| 03 | Candidate | SHTns | 271.720, 272.035, 272.557, 272.316, 271.482 | 272.035 | 0.1429% | 0.0500% |
| 04 | Baseline | RustSHT | 3363.700, 3345.236, 3358.791, 3358.345, 3354.362 | 3358.345 | 0.1842% | 0.0726% |
| 04 | Baseline | DUCC | 373.621, 373.753, 377.968, 374.973, 373.014 | 373.753 | 0.4722% | 0.0643% |
| 04 | Baseline | SHTns | 271.103, 269.978, 271.249, 271.022, 271.484 | 271.103 | 0.1915% | 0.0553% |
| 04 | Candidate | RustSHT | 1367.892, 1369.642, 1370.306, 1371.223, 1373.276 | 1370.306 | 0.1297% | 0.1903% |
| 04 | Candidate | DUCC | 372.024, 371.955, 372.341, 372.263, 372.179 | 372.179 | 0.0387% | 0.0529% |
| 04 | Candidate | SHTns | 271.137, 271.633, 271.852, 271.720, 271.467 | 271.633 | 0.0908% | 0.0145% |
| 05 | Baseline | RustSHT | 3359.463, 3357.191, 3356.400, 3346.254, 3346.102 | 3356.400 | 0.1708% | 0.3281% |
| 05 | Baseline | DUCC | 374.315, 375.949, 375.230, 375.513, 377.682 | 375.513 | 0.2953% | 0.3642% |
| 05 | Baseline | SHTns | 270.532, 271.552, 271.335, 271.458, 271.363 | 271.363 | 0.1350% | 0.0278% |
| 05 | Candidate | RustSHT | 1366.921, 1367.515, 1366.203, 1374.636, 1371.109 | 1367.515 | 0.2314% | 0.4352% |
| 05 | Candidate | DUCC | 373.147, 371.012, 371.751, 372.432, 373.337 | 372.432 | 0.2330% | 0.3044% |
| 05 | Candidate | SHTns | 271.963, 271.044, 270.674, 272.733, 272.648 | 271.963 | 0.3058% | 0.6054% |
| 06 | Baseline | RustSHT | 3356.665, 3351.200, 3353.879, 3346.739, 3352.510 | 3352.510 | 0.0977% | 0.1269% |
| 06 | Baseline | DUCC | 372.222, 372.562, 375.094, 373.334, 375.361 | 373.334 | 0.3451% | 0.4783% |
| 06 | Baseline | SHTns | 271.177, 271.089, 271.890, 271.282, 270.559 | 271.177 | 0.1570% | 0.0946% |
| 06 | Candidate | RustSHT | 1366.923, 1368.447, 1369.048, 1369.153, 1368.218 | 1368.447 | 0.0584% | 0.0174% |
| 06 | Candidate | DUCC | 373.803, 374.036, 373.412, 374.179, 379.088 | 374.036 | 0.5623% | 0.7567% |
| 06 | Candidate | SHTns | 270.824, 270.566, 269.376, 269.792, 271.672 | 270.566 | 0.2973% | 0.0614% |

## Pair ratios and process resources

| Pair | Baseline Rust median (ms) | Candidate Rust median (ms) | Candidate/baseline | Baseline RSS (KiB) | Candidate RSS (KiB) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 01 | 3361.805 | 1370.919 | 0.407793 | 955,412 | 955,796 |
| 02 | 3369.394 | 1371.724 | 0.407113 | 955,532 | 955,552 |
| 03 | 3365.493 | 1373.091 | 0.407991 | 955,552 | 955,672 |
| 04 | 3358.345 | 1370.306 | 0.408030 | 955,720 | 955,500 |
| 05 | 3356.400 | 1367.515 | 0.407435 | 955,648 | 955,732 |
| 06 | 3352.510 | 1368.447 | 0.408186 | 955,728 | 955,688 |

Median paired ratio: `0.407892`; 95% bootstrap interval: `0.407274–0.408108`. Baseline/candidate median-of-process-medians: `3360.075/1370.613 ms`. Candidate median ratios to DUCC and SHTns are `3.666x` and `5.040x`.

Peak RSS across all processes was 955,412–955,796 KiB. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Each process used one worker, and sampled benchmark threads remained within the configured placement. The local interference review found no disqualifying contention or concurrent benchmark/build workload. The CPU-pressure gate passed under the declared protocol. Exact host-state readings remain local.

## CPU-pressure validity

The declared CPU-pressure protocol was applied. Exact host readings remain local; the benchmark validity decision is recorded in the result summary.

## Interpretation

The candidate's pair 01 DUCC process median was 440.159 ms; the other five
candidate DUCC process medians ranged from 372.179 to 376.685 ms. Pair 01's
DUCC series passed the fixed CV and drift limits, so the measurement is
preserved. The RustSHT paired comparison uses the Rust medians and includes no
DUCC timing adjustment.

The T2047 cell passes its fixed q1 timing and numerical criteria. Both T1023 and T2047 candidate/baseline intervals meet the high-resolution portion of protocol v1. The six-pair T511 scalar control stopped at baseline pair 03 because the DUCC series exceeded the fixed CV and drift limits; see the [stopped T511 result](scaled-prefix-accumulator-t511-q1-stopped.md). These whole-transform data do not break execution into route timings; no route-time percentage is inferred from the earlier, different-source diagnostic. No q8, spin-1, or broader regression measurements are included.
