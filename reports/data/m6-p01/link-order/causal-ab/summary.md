# P01 default versus ordered link results

Six alternating q1 pairs compare the default and ordered final links of the
same P01 source. Each fresh process ran five synthesis samples on one CPU core. Ratios are ordered/default process medians; intervals resample the six paired
ratios 50,000 times with seed 20261017.

| Cell | Default median (ms) | Ordered median (ms) | Median ratio | 95% bootstrap interval | Pair range | Median RSS change (KiB) |
|---|---:|---:|---:|---:|---:|---:|
| T511 spin-1 synthesis | 43.0875 | 41.6010 | 0.96668 | 0.96024–0.97375 | 0.95980–0.97703 | +178 |
| T1023 scalar synthesis | 196.3960 | 196.9305 | 1.00025 | 0.99767–1.01304 | 0.99738–1.01926 | +98 |

All six spin-1 paired ratios favor the ordered binary. The T1023 scalar
comparison shows no measurable loss of the P01 gain. Several five-sample
process records contain high outliers; they remain in the raw data and pair
summaries. One initial T1023 ordered process was refused at CPU PSI
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
were rerun. The accepted process preflights show no competing compiler or
benchmark workload. The same-source comparison supports final text layout as the explanation for
the default-link spin-1 shift. The successful linker command was not retained,
so the exact link option and file cannot be verified. The ordering was applied
to the local benchmark executable at final link and is not encoded in the
RustSHT rlib. This result does not accept P01 for downstream builds.
