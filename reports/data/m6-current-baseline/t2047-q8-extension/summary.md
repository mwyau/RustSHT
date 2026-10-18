# Matched T2047 q8 RustSHT, DUCC and SHTns results

Each backend/operation has 15 samples (three groups of five) from one fresh process per spin. Times are medians of the retained timing samples; p10-p90 values and individual process samples are in `summary.csv` and structured `timing-samples.jsonl`; raw process output remains local. Rust and DUCC used reusable plans and caller-provided outputs. The SHTns adapter timing includes input preservation and E/B conversion.

| spin | Rust S | DUCC S | Rust/DUCC | SHTns S | Rust/SHTns | Rust A | DUCC A | Rust/DUCC | SHTns A | Rust/SHTns | Rust peak RSS |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 818.883 | 58.599 | 13.97x | 71.983 | 11.38x | 221.369 | 40.820 | 5.42x | 124.631 | 1.78x | 1016.0 MiB |
| 1 | 1027.308 | 126.510 | 8.12x | 138.667 | 7.41x | 358.881 | 81.650 | 4.40x | 242.638 | 1.48x | 1640.2 MiB |

Full-array errors were checked before timing in each process. This is an M1 reference baseline, not a P01 candidate measurement. Structured timing and memory summaries are retained; raw host and process captures remain local.
