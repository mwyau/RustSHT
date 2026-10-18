# Scaled-prefix accumulator T1023 q1 matrix result

## Decision

All six fresh-process pairs passed the declared timing, numerical, PSI, placement, and interference gates. The T1023 target passes: median paired candidate/baseline ratio is `0.568982` with a 95% bootstrap interval of `0.563779–0.571237` (50,000 resamples, seed `20261017`). Candidate RustSHT latency is 171.624 ms at the median of process medians, versus 301.582 ms for the clean baseline (43.10% lower). This is a T1023 q1 result only; the full q1 gate still requires T2047 and the T511 control. No q8 or spin-1 regression process has run.

Candidate-to-DUCC and candidate-to-SHTns ratios, using the median of candidate process medians, are 3.380x and 4.860x. RustSHT versus DUCC relative-L2 error was `1.182e-13` (maximum absolute error `1.556e-13`) in every process; SHTns versus DUCC relative-L2 error was `8.764e-14`.

## Frozen configuration

The matrix used the baseline and candidate source/binary hashes in [protocol v1](scaled-prefix-accumulator-q1-v1-protocol.md): clean baseline extension `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`; accumulator candidate source `edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0` and release extension `48db7ad3a6ba4c7fb0a8c50a303d9fbe948a7cac989c6f8a6e39b2f5e1b40184`. Workload was GL f64 scalar STANDARD synthesis, T1023, `1024x2048`, full `mmax`, `phi0=0.37`, q1, RustSHT/DUCC/SHTns. Each process used one warm-up and five timed full transformations. Reference revisions and host/software specifications are in the protocol. Raw machine/process captures remain local and untracked.

## Backend timings

Each row gives all five samples, population CV, and first-three/final-two median drift. The fixed limit for both CV and drift was 5%.

| Pair | Variant | Backend | Samples (ms) | Median (ms) | CV | Drift |
|---:|---|---|---|---:|---:|---:|
| 01 | Baseline | RustSHT | 306.168, 305.978, 305.345, 305.447, 304.937 | 305.447 | 0.1456% | 0.2573% |
| 01 | Baseline | DUCC | 51.055, 50.779, 50.713, 50.753, 50.770 | 50.770 | 0.2413% | 0.0345% |
| 01 | Baseline | SHTns | 36.294, 35.812, 35.872, 35.755, 35.898 | 35.872 | 0.5301% | 0.1268% |
| 01 | Candidate | RustSHT | 171.173, 171.052, 171.564, 171.196, 172.268 | 171.196 | 0.2585% | 0.3265% |
| 01 | Candidate | DUCC | 50.593, 50.745, 50.296, 50.819, 50.336 | 50.593 | 0.4170% | 0.0306% |
| 01 | Candidate | SHTns | 35.719, 35.741, 35.604, 35.573, 36.103 | 35.719 | 0.5282% | 0.3332% |
| 02 | Baseline | RustSHT | 301.523, 301.353, 300.049, 301.113, 301.404 | 301.353 | 0.1782% | 0.0314% |
| 02 | Baseline | DUCC | 51.855, 51.398, 50.948, 50.571, 51.180 | 51.180 | 0.8413% | 1.0209% |
| 02 | Baseline | SHTns | 35.461, 35.234, 35.076, 36.949, 35.643 | 35.461 | 1.8692% | 2.9948% |
| 02 | Candidate | RustSHT | 171.625, 172.240, 171.662, 171.510, 171.473 | 171.625 | 0.1619% | 0.0993% |
| 02 | Candidate | DUCC | 50.809, 51.044, 50.801, 50.638, 50.625 | 50.801 | 0.2988% | 0.3494% |
| 02 | Candidate | SHTns | 35.438, 35.431, 36.101, 35.380, 35.211 | 35.431 | 0.8606% | 0.4022% |
| 03 | Baseline | RustSHT | 301.110, 301.857, 301.114, 301.331, 301.207 | 301.207 | 0.0924% | 0.0515% |
| 03 | Baseline | DUCC | 50.284, 50.254, 51.352, 50.636, 50.756 | 50.636 | 0.7871% | 0.8137% |
| 03 | Baseline | SHTns | 35.215, 35.209, 35.667, 35.985, 35.895 | 35.667 | 0.9239% | 2.0327% |
| 03 | Candidate | RustSHT | 171.735, 172.244, 171.355, 173.051, 172.828 | 172.244 | 0.3710% | 0.6993% |
| 03 | Candidate | DUCC | 50.929, 50.591, 51.025, 50.617, 50.606 | 50.617 | 0.3647% | 0.6273% |
| 03 | Candidate | SHTns | 35.682, 35.195, 35.223, 34.986, 35.103 | 35.195 | 0.6727% | 0.5072% |
| 04 | Baseline | RustSHT | 301.821, 301.273, 301.812, 301.743, 303.330 | 301.812 | 0.2309% | 0.2401% |
| 04 | Baseline | DUCC | 50.954, 51.252, 49.864, 50.991, 51.241 | 50.991 | 1.0090% | 0.3177% |
| 04 | Baseline | SHTns | 35.436, 35.078, 35.328, 35.096, 35.086 | 35.096 | 0.4226% | 0.6753% |
| 04 | Candidate | RustSHT | 171.018, 170.404, 171.530, 171.152, 171.163 | 171.152 | 0.2143% | 0.0815% |
| 04 | Candidate | DUCC | 51.207, 51.044, 50.863, 50.315, 50.776 | 50.863 | 0.5944% | 0.9801% |
| 04 | Candidate | SHTns | 35.023, 35.039, 35.175, 35.298, 35.006 | 35.039 | 0.3197% | 0.3225% |
| 05 | Baseline | RustSHT | 302.001, 302.668, 302.712, 300.517, 300.984 | 302.001 | 0.2940% | 0.6349% |
| 05 | Baseline | DUCC | 51.200, 51.006, 51.663, 50.926, 50.890 | 51.006 | 0.5554% | 0.5725% |
| 05 | Baseline | SHTns | 35.935, 35.707, 36.159, 35.974, 35.388 | 35.935 | 0.7389% | 0.7068% |
| 05 | Candidate | RustSHT | 171.595, 171.936, 172.052, 171.558, 171.672 | 171.672 | 0.1141% | 0.1870% |
| 05 | Candidate | DUCC | 50.740, 49.838, 51.020, 50.013, 50.810 | 50.740 | 0.9284% | 0.6474% |
| 05 | Candidate | SHTns | 34.969, 35.028, 35.068, 35.846, 35.282 | 35.068 | 0.9125% | 1.5285% |
| 06 | Baseline | RustSHT | 301.475, 301.211, 300.760, 300.030, 300.494 | 300.760 | 0.1703% | 0.3155% |
| 06 | Baseline | DUCC | 51.035, 51.149, 51.821, 51.325, 51.022 | 51.149 | 0.5773% | 0.0479% |
| 06 | Baseline | SHTns | 35.848, 35.598, 35.648, 35.374, 35.842 | 35.648 | 0.4925% | 0.1122% |
| 06 | Candidate | RustSHT | 172.033, 171.622, 172.271, 171.527, 171.075 | 171.622 | 0.2420% | 0.4265% |
| 06 | Candidate | DUCC | 50.764, 50.946, 50.946, 50.534, 51.006 | 50.946 | 0.3401% | 0.3455% |
| 06 | Candidate | SHTns | 37.354, 36.506, 37.202, 37.069, 35.333 | 37.069 | 2.0108% | 2.7004% |

## Pair ratios and process resources

| Pair | Candidate/baseline Rust median | Baseline RSS (KiB) | Candidate RSS (KiB) |
| ---: | ---: | ---: | ---: |
| 01 | 0.560477 | 278,544 | 278,828 |
| 02 | 0.569515 | 278,612 | 274,900 |
| 03 | 0.571846 | 275,036 | 274,996 |
| 04 | 0.567081 | 278,516 | 274,832 |
| 05 | 0.568448 | 274,676 | 274,812 |
| 06 | 0.570628 | 274,636 | 274,708 |

Peak RSS ranged from 274,636 to 278,828 KiB across the 12 processes. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. All observed benchmark worker affinity and placement samples stayed within the assigned single-CPU set; no competing build or benchmark was detected, and no cgroup-related interference was identified in the local review.

## CPU-pressure validity

The declared CPU-pressure protocol was applied. Exact host readings remain local; the benchmark validity decision is recorded in the result summary.

## Interpretation

The T1023 q1 result passes the protocol’s required high-resolution target criterion: its median ratio is below 0.90 and its confidence-interval upper bound is below 1.00. The accumulator source remains experimental. T2047 q1 runs only because every T1023 process passed; T511 remains a required control before q8 or broader scalar/spin-1 regression work.
