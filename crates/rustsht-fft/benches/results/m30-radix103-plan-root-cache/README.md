# M30: plan-level radix-103 root-pair cache

## Result

The f64 complex N=1442 plan stores the radix-103 frequency-pair coefficients once and reuses them across transforms. Six alternating process pairs measured 1.151× forward and 1.140× inverse speedups at 721 rows against M29 commit `d7c4bb3`. The paired 95% intervals were 1.144–1.162 and 1.134–1.144. The transforms saved about 3.94 and 3.97 μs per row, respectively. The cache contains 1,275 packed frequency pairs and 51 tail roots, for 41,616 bytes of vector payload. The 48-byte inline field in `ComplexPlanData` also increases each `Option<BluesteinPlan>` slot by 48 bytes. The N=1442 plan has three slots, so the M30 plan adds 41,760 bytes of retained payload relative to M29 and 48 bytes to its inline plan object. These counts exclude allocator metadata. The public f64 N=1442 plan uses the cache. f32, internal plans, non-AVX2 execution, and inverse inputs outside the safe range use their existing routes. Workspaces use the same 26,368 bytes as M29. N=1442 plan construction took a median 28.70 μs, compared with 26.54 μs for M29.

## Method

The radix-103 pass has `l1=14` and `ido=1`. Plan construction packs the roots for frequency pairs `(1,2)` through `(49,50)` and the frequency-51 tail root. The AVX2 kernel loads those values from immutable plan storage. It uses the existing range guard for inverse execution and keeps the scalar path for unsupported execution routes. The root table is held once on `ComplexPlanData`, so `ComplexPass` descriptors do not grow. The full acceptance capture compared all 34 current complex and real SHT operation/length cells. It used six alternating process pairs, seven warmed samples per operation, one transform thread, Rust 1.99.0 stable, the optimized bench profile, and a portable x86_64 build with runtime AVX2 dispatch. Processes ran on logical one CPU core of an x86-64 CPU. The candidate and M29 binaries have SHA-256 hashes `1170d4ff79fe99557cc9e3c04c586bf6d76dd2a108ad4793f23038419112c00d` and `744f83c3140aa87614aed132f3d2d6c90c1e58ddf954d4396f1376cfbdda3c24`. Ratios are reference latency divided by candidate latency; values above one indicate a faster candidate.

| Operation | Rows | Candidate / reference | Speedup | Paired 95% interval |
| --- | ---: | ---: | ---: | ---: |
| C2C forward, N=1442 | 721 | 26,251 / 30,190 ns | 1.151× | 1.144–1.162× |
| C2C inverse, N=1442 | 721 | 28,255 / 32,224 ns | 1.140× | 1.134–1.144× |

Twenty of the 34 cells had slower candidate medians. Five paired intervals were wholly below parity; the other 15 crossed parity. The largest median regression was 2.8% for C2R N=4097 with 2,048 rows. Its ratio was 0.973× with a paired interval of 0.968–0.981×, corresponding to 1.9–3.3% higher candidate latency. The five intervals below parity were:

| Operation | Rows | Reference / candidate | Paired 95% interval |
| --- | ---: | ---: | ---: |
| C2R, N=4097 | 2,048 | 0.973× | 0.968–0.981× |
| C2C inverse, N=512 | 256 | 0.979× | 0.971–0.986× |
| R2C, N=723 | 361 | 0.982× | 0.976–0.990× |
| C2R, N=1443 | 721 | 0.989× | 0.985–0.995× |
| C2C inverse, N=150 | 73 | 0.991× | 0.989–0.994× |

Each measured control regression was below the 5% limit used in earlier FFT acceptance reports. Full cell values and process records are in `m30-plan-data-root-cache-acceptance-summary.csv` and `m30-plan-data-root-cache-acceptance-six-pairs.jsonl`. The original pass-level cache pilot and its captures remain under their original filenames; the files above record the final plan-level cache layout.

## Numerical checks

The radix-103 AVX2 kernel matches independent direct DFT values for `l1=1`, `14`, and `15` in both directions, including guard values. The N=1442 route test confirms AVX2 dispatch for ordinary inputs, scalar fallback for unsafe inverse inputs, and the f32 scalar path. The final plan-level cache output was compared with DUCC 0.41.0 `ducc0.fft.c2c` on the same deterministic complex input. Relative L2 errors were `7.856e-16` forward and `4.446e-16` for the normalized inverse. Maximum absolute errors were `9.097e-13` and `2.289e-16`. The driver, comparison script, and outputs are `m30-ducc-crosscheck.rs`, `m30-plan-data-ducc-crosscheck.py`, `m30-plan-data-ducc-forward.txt`, and `m30-plan-data-ducc-inverse.txt`. `cargo fmt --all -- --check`, all-target/all-feature Clippy with warnings denied, debug and release `cargo test --workspace --all-features` (108 tests per suite), and `cargo bench --bench rustfft --no-run` passed. The final source patch is `m30-plan-data-root-cache-source.patch` (SHA-256 `9c00c19810d24eb6be1cec86a0109d0b89c1a36ff25eb482c996655266effe3e`).
