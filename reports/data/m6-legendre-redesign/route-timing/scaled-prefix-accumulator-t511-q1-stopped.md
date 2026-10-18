# Scaled-prefix accumulator T511 q1 control (stopped)

## Result

The predeclared six-pair T511 control stopped at baseline pair 03. RustSHT and SHTns timing series passed; the DUCC series failed both stability limits (CV `5.6218%`, first-three/final-two median drift `8.2650%`). The CPU-pressure check failed the declared gate; exact transient readings remain local. The runner sampled no external CPU load on the assigned core and no cgroup throttling. The failed DUCC samples remain included below.

The first two complete RustSHT pairs were stable and had candidate/baseline median ratios `0.683677` and `0.677842`. Those two pairs are descriptive only; the required six-pair interval was not computed. Pair 03 had no candidate mate. The control does not pass, so q8 and scalar/spin-1 regression matrices were not started.

This is a T511 reference-stability failure, not a RustSHT timing failure. It leaves the T511 guard unresolved and does not accept the isolated kernel. No process was retried and no sample was excluded.

## Frozen configuration

The run followed [q1 protocol v1](scaled-prefix-accumulator-q1-v1-protocol.md): GL f64 scalar `STANDARD` `synthesis_into`, T511 (`512x1024`), full `mmax`, `phi0=0.37`, one worker; each fresh process used one warm-up and five timed calls for RustSHT, DUCC 0.42.0, and SHTns. Candidate source and extension hashes are recorded in the protocol. The fixed stability limits were CV ≤5% and first-three/final-two median drift ≤5%.

## Samples and stability

| Pair | Variant | Backend | Samples (ms) | Median (ms) | CV | Drift | Status |
|---:|---|---|---|---:|---:|---:|---|
| 01 | Baseline | RustSHT | 41.017, 41.324, 41.383, 40.961, 41.309 | 41.309 | 0.4223% | 0.4575% | PASS |
| 01 | Baseline | DUCC | 8.172, 7.972, 8.053, 8.050, 8.093 | 8.053 | 0.8072% | 0.2297% | PASS |
| 01 | Baseline | SHTns | 4.725, 4.770, 4.710, 4.690, 4.678 | 4.710 | 0.6803% | 0.8705% | PASS |
| 01 | Candidate | RustSHT | 28.267, 28.264, 28.242, 28.227, 28.208 | 28.242 | 0.0790% | 0.1646% | PASS |
| 01 | Candidate | DUCC | 8.135, 8.285, 8.095, 8.083, 8.067 | 8.095 | 0.9745% | 0.7412% | PASS |
| 01 | Candidate | SHTns | 4.785, 4.779, 4.765, 4.777, 4.781 | 4.779 | 0.1411% | 0.0000% | PASS |
| 02 | Candidate | RustSHT | 27.997, 27.982, 27.980, 27.979, 28.223 | 27.982 | 0.3411% | 0.4253% | PASS |
| 02 | Candidate | DUCC | 8.116, 8.017, 8.037, 7.986, 7.993 | 8.017 | 0.5820% | 0.5925% | PASS |
| 02 | Candidate | SHTns | 4.852, 4.789, 4.768, 4.739, 4.744 | 4.768 | 0.8564% | 0.9962% | PASS |
| 02 | Baseline | RustSHT | 41.295, 41.281, 41.264, 41.183, 41.318 | 41.281 | 0.1118% | 0.0739% | PASS |
| 02 | Baseline | DUCC | 8.143, 8.056, 8.042, 8.054, 7.986 | 8.054 | 0.6248% | 0.4470% | PASS |
| 02 | Baseline | SHTns | 4.788, 4.750, 4.741, 4.777, 4.799 | 4.777 | 0.4640% | 0.7955% | PASS |
| 03 | Baseline | RustSHT | 41.388, 41.256, 41.230, 41.072, 41.397 | 41.256 | 0.2888% | 0.0521% | PASS |
| 03 | Baseline | DUCC | 8.153, 8.074, 8.644, 9.383, 8.294 | 8.294 | 5.6218% | 8.2650% | FAIL |
| 03 | Baseline | SHTns | 4.850, 4.797, 5.099, 5.123, 4.849 | 4.850 | 2.7961% | 2.8041% | PASS |

## Process resources and validity

| Pair | Variant | Peak RSS (KiB) |
| ---: | --- | ---: |
| 01 | Baseline | 98,240 |
| 01 | Candidate | 98,392 |
| 02 | Candidate | 98,592 |
| 02 | Baseline | 98,144 |
| 03 | Baseline | 98,380 |

Benchmark processes and worker threads stayed affinity-restricted to the assigned single logical CPU. No competing benchmark or build ran. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. CPU-pressure warnings were reviewed under the declared protocol; exact transient readings remain local. Brief the local interference review found no direct workload overlap.2%, and not on the assigned core.

## CPU-pressure validity

The declared CPU-pressure protocol was applied. Exact host readings remain local; the benchmark validity decision is recorded in the result summary.

## Next step

The high-resolution q1 matrices remain valid, but the full q1 acceptance gate is incomplete. A future T511 control needs a newly declared protocol if the benchmark harness or sampling rules change; this report does not authorize a retry under protocol v1. Until T511 passes, the accumulator kernel stays experimental and q8 and broader regression work remain gated.
