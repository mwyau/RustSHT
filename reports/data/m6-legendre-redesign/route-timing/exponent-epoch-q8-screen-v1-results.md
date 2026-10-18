# Exponent-epoch scalar synthesis q8 screen, results

The T1023 q8 acquisition stopped after the baseline process for pair 5 failed
the fixed timing-stability limits. No candidate mate or T2047 process ran.
The four complete q8 pairs are descriptive only; this screen has no
acceptance interval and does not accept the candidate.

The workload, source and binary hashes, thread placement policy, pressure
thresholds and decision rules are in the [v1 protocol](exponent-epoch-q8-screen-v1-protocol.md).
The paired q1 results are in the [q1 result record](exponent-epoch-q1-screen-v1-results.md).

## T1023 q8 process results

| Pair | Run order | Baseline median (ms) | Candidate median (ms) | Paired ratio | Cell status |
|---:|---|---:|---:|---:|---|
| 1 | Baseline, candidate | 75.820 | 35.488 | 0.468056 | Both valid |
| 2 | Candidate, baseline | 74.672 | 35.617 | 0.476979 | Both valid |
| 3 | Baseline, candidate | 74.878 | 35.782 | 0.477871 | Both valid |
| 4 | Candidate, baseline | 74.674 | 35.231 | 0.471797 | Both valid |
| 5 | Baseline only | 76.267 | — | — | Baseline failed stability; candidate not run |
| 6 | Not run | — | — | — | Acquisition stopped |

The baseline and candidate RustSHT medians for pairs 1–4 were
`75.820, 74.672, 74.878, 74.674` ms and
`35.488, 35.617, 35.782, 35.231` ms. The four ratios are shown only to retain
the acquired pair evidence; no aggregate or confidence interval is reported.

Baseline pair 5 had RustSHT samples
`74.335, 76.339, 92.360, 76.267, 75.638` ms. Its RustSHT CV was 8.514%,
above the 5% limit. SHTns samples were
`12.677, 17.006, 16.376, 11.893, 12.025` ms; CV was 15.904% and
first-three/final-two median drift was 34.843%. DUCC passed its stability
limits. The 92.360 ms RustSHT sample and all SHTns samples remain in the
record. No retry or candidate mate ran.

All four valid candidate processes passed numerical comparison with DUCC;
maximum absolute error was `1.556e-13` and relative-L2 error was `1.182e-13`.
Every RustSHT, DUCC and SHTns series in pairs 1–4 passed the fixed stability
limits.

Peak RSS ranged from 273,756 to 274,732 KiB for the five baseline processes
and from 273,976 to 274,364 KiB for the four candidate processes. The shared
Python harness loaded NumPy, DUCC and SHTns in each process.

## Pressure and placement review

Preflight and postflight CPU PSI `some avg10` was `0.00%` for every process,
including the failed pair 5 baseline. No CPU PSI warning or rejection
occurred. Manual review found no concurrent CPU-intensive build or benchmark,
cgroup throttling, or worker-placement issue. Captured worker samples placed
eight threads across eight distinct physical cores. CPU-frequency values and
thread-level captures remain local.

The fixed protocol stops this screen after a failed process. The cause of the
RustSHT and SHTns timing variation is unresolved, so T2047 q8 and the remaining
T1023 pairs were not measured. The q1 high-resolution results remain valid;
the candidate remains isolated pending the T511 control and complete
regression measurements.
