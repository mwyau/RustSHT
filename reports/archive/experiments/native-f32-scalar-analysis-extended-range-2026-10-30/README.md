# Native-f32 scalar-analysis extended ordinary-range screen

## Scope and provenance

This bounded candidate layers an analysis-only f32 recurrence range cutoff (`2^-64`) onto the adaptive seed-prefix schedule and SIMD input preflight candidate. When both recurrence states fit the wider normal range, it materializes the scaled pair and continues on the ordinary vector path; otherwise it retains the existing exponent-aware path. The candidate changes neither f64 execution nor other transform families.

The source patch is recoverable on `f122428c24709c53233d0922deeffd6b46dce048` from [candidate.patch.gz](candidate.patch.gz), SHA-256 `f72138a31f8c17b88a66ef7bdda6679306c46f6f67ef1e372362147412038331`. It includes the adaptive/preflight parent implementation as well as this range change. The benchmark runner is [benchmark.py](benchmark.py). Full paired data and provenance are in [t720-3pair.json](t720-3pair.json); no benchmark binaries or duplicate fixtures are stored.

The exact native-f32 Rust worker SHA-256 is `5a171d36e9f590d28252cee54b0ca9b189a1d8eac0b64438b831b06998b119ed`. Pinned DUCC 0.42.0 revision `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` worker SHA-256 is `1e8412966f7bf306a1fea8141969208f661d5a705f7737e7a8f0dc1cc74cc36d`. The matched f64 denominator is the unchanged adaptive/preflight parent worker `35b67e3e2909bd2eaad86c76fff2e7697a04010eb4d9312bb2739682cfd21535`, with parent patch SHA-256 `52d3557bd004ff53a4da32bdbd3d4d8bacae0eac61fd5b6ae896296b06f82efa`; this candidate changes only the native-f32 range leaf. The original clean-source f64 baseline remains in the [adaptive/preflight package](../native-f32-scalar-analysis-adaptive-preflight-2026-10-30/fresh-current-source-f64-baseline.json) and is not used for the paired ratios below.

## T720 screen

Three interleaved fresh-process pairs, AMD Ryzen 9 5950X, `-C target-cpu=x86-64-v3`, AVX2/FMA, q1 pinned to CPU 0 and q8 pinned to CPUs 0–7. The same deterministic f32 fixtures were used for each candidate and DUCC case; their SHA-256 hashes are in the JSON. The f64 worker receives those fixture values widened component-wise. Native f32 executes eight-lane SIMD, recurrence, accumulation, spectra, FFT and output. Each entry below is the median of the paired-screen process medians.

| Geometry | Rust f32 ms | matched Rust f64 ms | f32/f64 | DUCC f32 ms | f32/DUCC | Gate |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 56.160 | 20.411 | 2.751x | 14.239 | 3.912x | GAP |
| Full q8 | 7.436 | 3.562 | 2.088x | 2.077 | 3.582x | GAP |
| Reduced q1 | 20.091 | 8.559 | 2.347x | 5.856 | 3.436x | GAP |
| Reduced q8 | 2.751 | 1.341 | 2.051x | 0.946 | 3.002x | GAP |

The wider range does not meet the interim `<=2.0x` matched-f64 target in any cell, and it remains far from DUCC. The three-pair screen is a directional result, not a consequential performance claim. The adaptive/preflight parent f64 call is the exact denominator because the f64 path is unchanged and the input fixtures and workload match.

## Work-count and numerical status

The route counter observed full geometry move from 629,891 normal / 266,685 scaled pair-visits on the adaptive/preflight parent to 631,357 / 265,219 with the wider range. Reduced geometry moved from 284,278 / 89,429 to 284,820 / 88,887. The scaled share therefore changed only from about 29.75% to 29.58% full and 23.93% to 23.78% reduced. The cutoff changes too few visits to explain or close the whole-call gap; per-operation time remains unmeasured. Do not repeat this cutoff adjustment unchanged.

The candidate's random-fixture output relative L2 against DUCC is `1.8390064e-5` full and `2.2203729e-5` reduced, unchanged at the reported precision from the parent. This comparison is not an independent accuracy check or a science-tolerance PASS. The existing analytic, adjoint, workspace-reset and typed-route tests passed; see [test-results.txt](test-results.txt). Selected high-band independent measurements for the adaptive/preflight parent are documented in that parent's package and are not a complete-array accuracy acceptance for this candidate.

## Reproduction and checksums

From a checkout at the patch base, decompress and apply `candidate.patch.gz`, then build the `ducc_f32_full_transform_parity` example in release mode with the recorded target flags. Run `benchmark.py` with `RUSTSHT_B1_EXTENDED_RUST`, `RUSTSHT_B1_EXTENDED_DUCC`, and `RUSTSHT_B1_EXTENDED_OUT` set to the corresponding worker paths and output JSON path. The script records fixture, worker and timing details. `SHA256SUMS` records the package checksums.

## Decision

Keep this candidate isolated. The B1 native-f32 implementation remains functional but misses every interim timing cell, while independent accuracy acceptance remains open. Stop B1 range micro-tuning here and continue breadth work on spin-1 analysis.
