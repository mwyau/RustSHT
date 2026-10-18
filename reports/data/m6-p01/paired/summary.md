# P01 paired whole-transform screen

Six alternating fresh-process pairs were completed for six cells. Each process completed five synthesis and five analysis samples and exited successfully. Preflight validity checks reported no competing build, FFT, or benchmark. All timing samples, including outliers, are retained in the structured `process-pairs.csv`; raw process and GNU-time captures remain local.

| Cell | Operation | Baseline median (ms) | Candidate median (ms) | Median paired C/B ratio | Paired bootstrap 95% interval |
|---|---:|---:|---:|---:|---:|
| T511 scalar q1 | Synthesis | 40.405 | 39.905 | 0.9880 | 0.9867–0.9898 |
| T511 scalar q1 | Analysis | 16.035 | 15.702 | 0.9785 | 0.9426–0.9833 |
| T511 spin1 q1 | Synthesis | 42.086 | 43.042 | 1.0196 | 1.0179–1.0444 |
| T511 spin1 q1 | Analysis | 28.073 | 27.994 | 0.9952 | 0.9876–1.0054 |
| T1023 scalar q1 | Synthesis | 296.153 | 197.335 | 0.6647 | 0.6565–0.6765 |
| T1023 scalar q1 | Analysis | 109.850 | 108.410 | 0.9877 | 0.9785–0.9895 |
| T1023 scalar q8 | Synthesis | 75.295 | 47.461 | 0.6320 | 0.6244–0.6361 |
| T1023 scalar q8 | Analysis | 20.889 | 19.604 | 0.9443 | 0.9328–1.0814 |
| T2047 scalar q1 | Synthesis | 3264.641 | 2391.943 | 0.7330 | 0.7324–0.7332 |
| T2047 scalar q1 | Analysis | 1073.908 | 1065.817 | 0.9913 | 0.9891–0.9970 |
| T2047 scalar q8 | Synthesis | 797.656 | 544.914 | 0.6838 | 0.6822–0.6854 |
| T2047 scalar q8 | Analysis | 218.674 | 217.785 | 0.9944 | 0.9898–1.0021 |

Candidate scalar synthesis is substantially faster at T1023 and T2047 at both thread counts. The T511 scalar synthesis control is 1.2% faster. The spin1 T511 synthesis control is 1.96% slower at the median; three of six paired ratios are above 1.02, and the bootstrap interval extends to 1.0444. This is close to the predeclared 2% control threshold and is evidence to review before deciding the gate. Spin1 analysis is effectively unchanged. The q8 T1023 analysis result is variable: paired ratios range from 0.9237 to 1.2103 and its interval crosses 1. The high candidate sample cluster in pair 5 is retained and has no recorded competing process. T2047 q8 analysis is much steadier, although the candidate's paired interval narrowly crosses 1. The q8 synthesis improvements are consistent across all six pairs at both bandlimits. Median paired RSS deltas range from −144 KiB to +270 KiB across cells. Exact operation sample ranges, pair ratios, and RSS values are in `summary.csv` and `process-pairs.csv`; structured timing and RSS samples are retained alongside them; raw process and host captures remain local. This is measurement evidence only; it does not decide candidate acceptance.
