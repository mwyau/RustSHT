# Timing audit: spin0-t511-q8

The selected full-matrix cell was scalar (spin 0), GL STANDARD f64, T511, q8, synthesis. The runner kept all five measured calls per process. Before those five calls, the harness makes one unmeasured warm-up call (`time_runs`, in the matched benchmark script). All 12 pairs use the declared alternating order; selected runs all have PSI `some avg10` <= 0.50, with no compiler or benchmark process listed in the preflight active-process section.

- Selected pairs: 12; paired median-ratio range: 0.628–1.489.
- Within-process sample coefficient of variation: median 40.1%, range 21.9–107.4%.
- Across all 24 process runs, median of the first three measured samples: 19.699 ms; median of the last two: 9.539 ms (0.484×).
CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local.
- The large within-process spread and consistent decline between early and late samples are not explained by any recorded active compiler/benchmark process. They make the per-process medians and paired ratios unstable under this protocol. No timed sample was discarded.
- The bootstrap threshold calculation alone had median 0.891984 and 95% interval [0.776221, 0.998472], but this is not a usable control pass given the raw timing inconsistency. Cell status: inconclusive.

## All selected paired samples

| Pair | Attempt | Order | Baseline samples (ms) | Candidate samples (ms) | C/B |
| 1 | 2 | baseline,candidate | 21.948,17.882,15.990,13.217,9.555 | 14.098,20.593,19.237,12.319,9.855 | 0.881676 |
| 2 | 1 | candidate,baseline | 15.414,15.885,18.023,16.926,9.129 | 27.860,21.977,12.869,9.390,9.755 | 0.810135 |
| 3 | 2 | baseline,candidate | 23.928,16.455,15.163,9.535,9.599 | 26.922,22.026,9.447,9.521,9.737 | 0.642155 |
| 4 | 1 | candidate,baseline | 25.839,19.125,14.814,9.534,9.569 | 22.959,22.987,16.926,9.595,9.461 | 1.142568 |
| 5 | 1 | baseline,candidate | 22.759,19.986,18.695,9.463,9.509 | 20.973,23.995,14.411,9.573,9.547 | 0.770848 |
| 6 | 2 | candidate,baseline | 26.953,28.988,9.543,9.320,9.080 | 16.363,22.833,14.208,9.693,9.406 | 1.488840 |
| 7 | 2 | baseline,candidate | 15.487,15.914,21.546,9.466,9.670 | 20.957,20.993,14.214,9.285,9.545 | 0.917802 |
| 8 | 1 | candidate,baseline | 21.910,23.223,17.010,9.547,9.249 | 15.348,21.551,18.993,9.274,9.541 | 0.902293 |
| 9 | 2 | baseline,candidate | 22.904,21.040,15.328,9.513,9.515 | 21.940,21.898,9.308,9.630,9.346 | 0.628262 |
| 10 | 1 | candidate,baseline | 15.263,21.821,18.271,9.112,9.660 | 25.508,19.412,16.275,9.335,9.347 | 1.066304 |
| 11 | 1 | baseline,candidate | 18.296,19.995,18.803,9.328,9.572 | 27.707,17.997,17.027,9.682,9.628 | 0.930641 |
| 12 | 2 | candidate,baseline | 23.948,21.017,16.387,9.339,9.506 | 22.935,22.982,12.808,9.775,9.824 | 0.781595 |

The paired numerical timing samples are retained in [spin0-t511-q8-pairs.csv](spin0-t511-q8-pairs.csv).
