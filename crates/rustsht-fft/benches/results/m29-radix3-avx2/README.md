# M29: AVX2 radix-three complex pass

## Result

The f64 AVX2 radix-three pass is retained for passes with `ido >= 2` and a complete two-complex vector prefix. In the six-pair acceptance capture, the N=2904 C2C speedups were 1.298× forward and 1.212× inverse. The candidate does not change plan or workspace storage. Unsafe inverse inputs use the scalar path.

The benchmark ratio is reference latency divided by candidate latency, so values above one indicate a faster candidate. The full 34-cell capture and per-cell summaries are in `m29-radix3-acceptance-six-pairs.jsonl` and `m29-radix3-acceptance-summary.csv`.

## Method

The N=2904 factorization is `[8, 3, 11, 11]`. Its radix-three stage has `l1=8` and `ido=121`; the candidate processes 120 inner values in AVX2 and leaves the final inner value to the scalar loop. The vector kernel evaluates the radix-three butterfly, applies the stage twiddles, and scales inverse output when required. Runtime AVX2 detection and the existing inverse range guard select the kernel. Other precisions and unsupported targets retain their existing paths.

Callgrind counted 34,004,323 forward and 43,837,574 inverse instruction references for 100 M28-baseline transforms. The M29 candidate counted 25,204,012 forward references (25.9% fewer) and 34,836,098 inverse references (20.5% fewer). These profiles include process startup, plan and workspace construction, and input refill. Instruction references are not elapsed time. Baseline and candidate profiles are included here.

## Measurements

The acceptance capture used six alternating process pairs and seven warmed samples per operation. It covered all 34 current C2C and real SHT operation/length cells at the configured scalar and spin-1 row counts, with one transform thread. The compiler was Rust 1.99.0 stable with the optimized bench profile and portable x86_64 code. Processes were pinned to logical CPU 30 of an AMD Ryzen 9 5950X; AVX2 was selected at runtime. Source and binary hashes are recorded in `m29-radix3-acceptance-config.json`.

| Operation | Rows | Candidate/reference median | Reference/candidate | Paired 95% interval |
| --- | ---: | ---: | ---: | ---: |
| C2C forward, N=300 | 73 | 1,448 / 1,761 ns | 1.217× | 1.214–1.286× |
| C2C inverse, N=300 | 73 | 1,946 / 2,221 ns | 1.145× | 1.129–1.209× |
| C2C forward, N=1452 | 721 | 4,305 / 5,686 ns | 1.323× | 1.310–1.365× |
| C2C inverse, N=1452 | 721 | 6,414 / 7,808 ns | 1.219× | 1.215–1.257× |
| C2C forward, N=2904 | 721 | 9,398 / 12,154 ns | 1.298× | 1.288–1.342× |
| C2C inverse, N=2904 | 721 | 13,666 / 16,501 ns | 1.212× | 1.202–1.241× |

Seven of 34 cells had slower candidate medians. The largest median slowdown was 2.3% for C2C forward N=256 with 128 rows (reference/candidate 0.977×; paired interval 0.960–1.001×). Two cells had paired intervals wholly below parity: R2C N=1443 with 721 rows (0.986×; 0.982–0.991×) and C2C forward N=722 with 361 rows (0.992×; 0.986–0.998×). The other five slower medians had intervals that crossed parity.

A three-pair focus capture at N=2904 measured one-transform speedups of 1.312× forward and 1.211× inverse; the 721-row speedups were 1.310× and 1.209×. Full-matrix results for other affected lengths are in the summary CSV.

## Numerical checks

The radix-three AVX2 pass matched independent direct DFT evaluations for forward and inverse directions at `(l1, ido)` values `(1, 2)`, `(2, 3)`, and `(8, 121)`. The test checks the odd-inner tail and guard values around the output. The N=2904 dispatch test confirms the vector route for safe forward and inverse inputs and the scalar fallback for unsafe inverse inputs.

The end-to-end N=2904 transform was compared with DUCC 0.41.0 `ducc0.fft.c2c` on the same deterministic complex input. Relative L2 errors were `2.960e-16` forward and `3.015e-16` for the normalized inverse; maximum absolute errors were `2.542e-13` and `8.757e-17`. The saved outputs and comparison drivers are `m29-ducc-forward.txt`, `m29-ducc-inverse.txt`, `m29-ducc-crosscheck.rs`, and `m29-ducc-crosscheck.py`.

`cargo fmt --all -- --check`, `cargo test --workspace --all-features` (108 passed), and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed. The source patch is `m29-radix3-source.patch` (SHA-256 `74950ec0d849302e465f6a6517ae7ba18a47837f9f79c7e63443c1033047dc7c`).
