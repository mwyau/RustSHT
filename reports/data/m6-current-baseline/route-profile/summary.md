# Scalar synthesis route profile

The six release test logs in this directory profile the current GL scalar synthesis routes at T511, T1023, and T2047 with q1/q8. Each profile warms the same plan, measures one uninstrumented synthesis, enables per-worker counters, measures a second synthesis, and asserts the complete output map is bitwise identical. The counters are compiled only for tests. Route counts and timers are emitted by the enabled synthesis; the disabled/enabled elapsed comparison calibrates the profiling overhead.

## q1 route totals

| T | Disabled / enabled wall time (ms) | Direct fused pairs | Visitor normal / scaled lanes | Transition prefix lanes / fused pairs | Completed / declined transitions | Fused / visitor / transition timer sums (ms) |
|---:|---:|---:|---:|---:|---:|---:|
| 511 | 39.886 / 39.025 | 26,659,796 | 2,639,324 / 541,870 | 0 / 0 | 0 / 0 | 7.421 / 26.408 / 0 |
| 1023 | 281.237 / 282.932 | 199,808,786 | 18,174,506 / 4,881,166 | 500,992 / 1,639,808 | 108 / 0 | 55.352 / 191.910 / 13.420 |
| 2047 | 3182.514 / 3181.198 | 1,322,505,640 | 113,966,434 / 43,441,670 | 36,809,152 / 217,655,168 | 6,734 / 1,054 | 362.784 / 1505.354 / 1181.839 |

At q1, the visitor timer accounts for 78% of the three measured route timers at T511, 74% at T1023, and 49% at T2047. The transition timer accounts for another 39% at T2047. These routes are strong scalar synthesis targets: the existing visitor processes roughly 9–10% of committed route units but dominates measured time at T511/T1023, while the two-channel transition consumes a large share at T2047.

## q8 worker distribution

| T | Enabled wall time (ms) | Lowest / highest per-worker fused+visitor+transition timer sum (ms) | Max/min | Notable route split |
|---:|---:|---:|---:|---|
| 511 | 9.299 | 1.551 / 6.142 | 3.96x | Worker 5 idle; workers 3/6 carry most visitor work. |
| 1023 | 79.954 | 9.949 / 68.946 | 6.93x | Workers 0/5 are fused-only; workers 6/7 carry 58.666/62.695 ms of visitor work. |
| 2047 | 790.333 | 77.256 / 736.302 | 9.53x | Workers 0/2 are fused-only at ~77 ms; workers 1/6 carry ~690/703 ms of visitor plus transition work. |

Per-worker timer sums are CPU time across workers, not wall time. The q8 T1023 profile changed measured wall time by +13.8% with instrumentation enabled; treat those q8 timer magnitudes as noisy. The route counts are stable across q1/q8 and clearly show load imbalance. The T511 q8 profile had one idle worker and enabled wall time 7.5% below disabled; timer interpretation is likewise noisy.

## Interpretation and P01 target

The route accounting supports a bounded scalar-only P01 experiment. It should first replace the general paired visitor on eligible active lanes, while preserving the scaled visitor for unsafe coefficients, polar lanes, and declined transitions. It should also use a one-channel transition for scalar synthesis instead of calculating an unused second scalar channel. The candidate must keep the current recurrence scaling and transaction-on-success behavior; a declined candidate route must leave spectra unchanged before the established visitor fallback runs. `summary.csv` contains the reproducible per-case aggregate table. `workers.csv` records the per-worker counts and timer sums. The structured route totals and per-worker summaries are retained; raw test output remains local and untracked. The direct-test source is `src/legendre/route_profile.rs`; its changes are test-only.
