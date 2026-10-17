# Matched RustSHT, DUCC and SHTns baseline

Medians pool the 15 retained raw samples (three groups of five); p10-p90 captures spread. Rust and DUCC measurements use reusable plans and caller-provided output buffers. SHTns is the documented adapter scope, including input preservation and E/B potential conversion.

| T | spin | threads | Rust S | DUCC S | Rust/DUCC | SHTns S | Rust A | DUCC A | Rust/DUCC | SHTns A | Rust peak RSS |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 511 | 0 | 1 | 40.267 | 7.995 | 5.04x | 4.562 | 16.058 | 5.943 | 2.70x | 11.182 | 104.4 MiB |
| 511 | 0 | 8 | 9.879 | 1.114 | 8.87x | 6.003 | 3.598 | 0.922 | 3.90x | 5.997 | 105.4 MiB |
| 511 | 1 | 1 | 42.151 | 17.046 | 2.47x | 11.235 | 27.233 | 11.512 | 2.37x | 22.948 | 147.8 MiB |
| 511 | 1 | 8 | 13.607 | 2.378 | 5.72x | 4.124 | 5.549 | 1.723 | 3.22x | 11.970 | 156.7 MiB |
| 1023 | 0 | 1 | 296.372 | 49.555 | 5.98x | 34.670 | 111.188 | 34.204 | 3.25x | 81.297 | 302.9 MiB |
| 1023 | 0 | 8 | 74.218 | 6.678 | 11.11x | 11.798 | 19.785 | 4.788 | 4.13x | 15.689 | 303.9 MiB |
| 1023 | 1 | 1 | 276.295 | 121.291 | 2.28x | 81.186 | 184.402 | 83.478 | 2.21x | 168.268 | 443.4 MiB |
| 1023 | 1 | 8 | 73.649 | 20.269 | 3.63x | 28.568 | 38.713 | 15.208 | 2.55x | 44.055 | 444.3 MiB |
| 2047 | 0 | 1 | 3355.273 | 370.438 | 9.06x | 269.622 | 1093.244 | 243.378 | 4.49x | 605.659 | 1013.5 MiB |
| 2047 | 1 | 1 | 4124.493 | 776.432 | 5.31x | 498.233 | 1767.881 | 496.897 | 3.56x | 1158.457 | 1638.0 MiB |

Relative L2 errors were checked in every process before timing. The T2047 scalar and spin-1 results are three-group, 15-sample q1 measurements; their exact source, package, host, error, and memory details are in the corresponding `t2047-*-q1-audit-provenance.txt` files. Per-process errors, all raw samples, RSS, wall time, and CPU utilization are in the neighboring `*.stdout` and `*.time` files. The missing T2047 q8 reference cells were measured in a separate, source-matched extension. The scalar and spin-1 results, error checks, p10-p90 values, host snapshots and raw samples are in [`t2047-q8-extension/summary.md`](t2047-q8-extension/summary.md). These runs reuse the same RustSHT extension and DUCC/SHTns libraries recorded above.
