# M28: N=1442 radix-103 root-pair precomputation

## Result

The candidate was rejected and removed. Six alternating process pairs improved the 721-row N=1442 C2C target by 9.8% forward and 9.1% inverse. Across the 34-cell workload matrix, 10 cells had slower candidate medians. Seven paired 95% bootstrap intervals were below parity; the other three slower medians had intervals that crossed parity. The largest regression was 8.2% for C2R N=723 with 361 rows. The M26 kernel remains the production path.

The benchmark ratio is reference latency divided by candidate latency, so values below one indicate a slower candidate. The complete cell summaries and paired samples are in `m28-rootpairs-acceptance-summary.csv` and `m28-rootpairs-acceptance-six-pairs.jsonl`.

## Method

M26 routes complex N=1442 through factors `[2, 7, 103]`. Its f64 AVX2 radix-103 `ido=1` pass accounted for about 50.5% of forward instruction references in the N=1442 Callgrind profile. The M28 candidate computed 25 adjacent-frequency root pairs and the singleton tail roots once per transform, then reused those vectors across the 14 outer groups. It used 41,616 bytes of local stack arrays and added no plan or workspace storage.

Callgrind counted 80,592,671 forward and 85,280,866 inverse instruction references for 100 M26 transforms. The candidate counted 46,477,305 forward references (42.3% fewer) and 51,018,358 inverse references (40.2% fewer). These profiles include process startup and plan, workspace, and input setup. Instruction references are not elapsed time and did not predict the regressions in the whole-workload capture. Compressed profiles are included here.

## Target measurements

The full capture used six alternating process pairs, seven warmed samples per operation, Rust 1.99.0 stable, an optimized bench profile, portable x86_64 code with runtime AVX2 dispatch, and a Ryzen 9 5950X pinned to logical CPU 30. The source and binary hashes and runner details are in `m28-rootpairs-acceptance-config.json`.

| Operation | Rows | Candidate/reference median | Reference/candidate | Paired 95% interval |
| --- | ---: | ---: | ---: | ---: |
| C2C forward, N=1442 | 721 | 27,547 / 30,281 ns | 1.098× | 1.091–1.103× |
| C2C inverse, N=1442 | 721 | 29,627 / 32,323 ns | 1.091× | 1.082–1.095× |

A three-pair focus capture also measured one-transform N=1442 C2C forward and inverse speedups of 1.355× and 1.085×. Its 721-row results were 1.102× forward and 1.092× inverse. These pilot results did not include the full 34-cell control matrix.

The 10 cells with candidate medians below parity were:

| Operation | N | Rows | Reference/candidate | Paired 95% interval |
| --- | ---: | ---: | ---: | ---: |
| C2R | 723 | 361 | 0.918× | 0.899–0.986× |
| C2C inverse | 146 | 73 | 0.922× | 0.916–0.928× |
| C2C inverse | 722 | 361 | 0.958× | 0.946–0.980× |
| R2C | 2049 | 1024 | 0.978× | 0.968–0.982× |
| R2C | 257 | 128 | 0.979× | 0.970–0.994× |
| C2R | 257 | 128 | 0.986× | 0.975–1.042× |
| C2R | 2049 | 1024 | 0.988× | 0.970–1.001× |
| R2C | 1443 | 721 | 0.991× | 0.985–0.997× |
| C2C forward | 722 | 361 | 0.992× | 0.985–0.998× |
| C2R | 4097 | 2048 | 0.998× | 0.985–1.010× |

## Numerical checks

The candidate passed `f64_complex_general_prime_radix103_ido1_matches_direct_dfts_and_guards`, the N=1442 AVX2 route and unsafe-inverse guard test, the meridional complex-length independent-DFT test, and `cargo fmt --all -- --check`. A deterministic N=1442 complex input was compared with DUCC 0.41.0 `ducc0.fft.c2c`: relative L2 errors were `7.856e-16` forward and `4.446e-16` for the normalized inverse; maximum absolute errors were `9.097e-13` and `2.289e-16`. The saved outputs and comparison driver are `m28-ducc-forward.txt`, `m28-ducc-inverse.txt`, `m28-ducc-crosscheck.rs`, and `m28-ducc-crosscheck.py`.

The full workspace suite and Clippy were not run on this temporary candidate. The saved source change is `m28-rootpairs-source.patch` (SHA-256 `3448916d40950667cb424e8c009d5df7fbfc73c52b4ba59d0e38fb854cff4c14`).
