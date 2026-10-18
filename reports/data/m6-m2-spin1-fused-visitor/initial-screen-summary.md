# Initial six-pair spin-1 fused visitor screen

The candidate and baseline are the exact isolated binaries identified in `source-and-binary-sha256.txt` and `build-binary-sha256.txt`. Each process used GL STANDARD f64, full mmax, `phi0=0.37`, synthesis_into, and five retained Rust timing samples. q8 used 8 pinned CPU cores; q1 used one CPU core. The fixed preflight gate was CPU PSI `some avg10` <= 0.50. Bootstrap resampling used the six paired ratios, 200,000 draws, seed 20261017, and the median statistic.

## T2047 spin1 q8 synthesis primary

| Pair | Baseline median ms | Candidate median ms | C/B | RSS change kB |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 1025.805 | 968.383 | 0.944022 | 1848 |
| 2 | 1011.378 | 969.315 | 0.958410 | 1324 |
| 3 | 1014.741 | 963.088 | 0.949097 | 1628 |
| 4 | 1013.088 | 976.155 | 0.963544 | 2252 |
| 5 | 1019.231 | 965.714 | 0.947493 | 2208 |
| 6 | 1015.916 | 962.072 | 0.947000 | 1156 |

- Median C/B: 0.948295; paired bootstrap 95% interval: [0.945511, 0.960977].
- Pair ratio range: [0.944022, 0.963544]; pairs above 1.02: 0/6.
- Gate: **pass** (primary median <= 0.98 and upper < 1.00).
- Initial refused attempts and retained orphans are excluded from these selected pairs and remain in the raw directory with their original preflights.

## T511 spin1 q1 synthesis control

| Pair | Baseline median ms | Candidate median ms | C/B | RSS change kB |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 41.678 | 42.040 | 1.008686 | 1412 |
| 2 | 41.656 | 41.841 | 1.004441 | 1384 |
| 3 | 41.643 | 41.890 | 1.005931 | 1552 |
| 4 | 41.635 | 42.855 | 1.029302 | 1080 |
| 5 | 41.567 | 41.834 | 1.006423 | 1620 |
| 6 | 41.914 | 41.940 | 1.000620 | 1168 |

- Median C/B: 1.006177; paired bootstrap 95% interval: [1.002531, 1.018994].
- Pair ratio range: [1.000620, 1.029302]; pairs above 1.02: 1/6.
- Gate: **pass** (control upper <= 1.02).
- Initial refused attempts and retained orphans are excluded from these selected pairs and remain in the raw directory with their original preflights.
