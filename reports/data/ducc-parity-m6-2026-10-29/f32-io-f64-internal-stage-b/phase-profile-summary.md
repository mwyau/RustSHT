# Instrumented phase observations (diagnostic only)

These profiles explain the measured f64 adapter results; their timing is not used for acceptance. The candidate/DUCC comparisons in `results-16-cell.csv` are whole-call timings. Sources needed to rerun the profiles are saved under `sources/`.

| Operation and cell | Promotion | f64 transform | Narrowing/writeback | Complete adapter call |
|---|---:|---:|---:|---:|
| Scalar synthesis, GL T720 full q8 | 0.194 ms | 2.767 ms | 0.280 ms | 3.246 ms |
| Scalar analysis, GL T720 full q1 | 0.310 ms | 14.991 ms | 0.140 ms | 15.463 ms |
| Scalar analysis, GL T720 full q8 | 0.144 ms | 2.405 ms | 0.114 ms | 2.696 ms |
| Spin-1 synthesis, GL T720 full q8 | 0.295 ms | 8.386 ms | 0.509 ms | 9.157 ms |
| Spin-1 analysis, GL T720 full q8 | 0.650 ms | 6.225 ms | 0.275 ms | 7.155 ms |

The scalar-synthesis profile attributes most time to the f64 transform. Separate f64 analysis profiles attributed about 2.206 ms to scalar recurrence/accumulation and 0.750 ms to FFT at q8; spin-1 analysis attributed about 5.727 ms and 1.781 ms respectively. Instrumentation adds overhead, so these figures locate stages and do not replace whole-call measurements. The f64 spin-1 synthesis route census found 118 general fallback blocks and zero paired-hemisphere successes at T720.
