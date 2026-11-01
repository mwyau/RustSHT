# B1 f32 range-predicate fast path

## Result

Commit `40241a5c1e0953968a3d1d2e80e774c7d9de5deb` adds a B1-only f32 range predicate. It avoids the exact lower-bound reduction for ordinary SIMD vectors and keeps the prior exact predicate for vectors flagged as exceptional. The existing scalar tail rule remains in place. Correctness tests compare the new predicate with the previous helper at range boundaries, NaNs/infinities, varied vector lengths and tails.

The 15-pair T720 screen shows a small reduced-grid improvement, while both full-grid cells are statistically unresolved. The change does not close the large f32/DUCC gap. It does not alter f64 B1 code.

## Source and execution

- Exact parent: `56561116fb9b78afadd20132b8f90f98b7e30d0c`.
- Candidate commit: `40241a5c1e0953968a3d1d2e80e774c7d9de5deb`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Source: [`analysis.rs`](../../../../src/legendre/scalar_paired/analysis.rs#L80) provides the typed trait hook; the f32 override is at line 479, the predicate at line 819, and the B1 recurrence calls it at line 1518.
- CPU: AMD Ryzen 9 5950X; q1 pinned to CPU 0 and q8 to CPUs 0–7.
- Rust: `1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1; flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; actual dispatch was V3/AVX2/FMA, eight f32 lanes.
- DUCC used its native f32 path. The whole-call worker hash is recorded in the machine-readable result file.

The benchmark JSON records `candidate_diff_sha256=fff61778824ffd8a91992f1b473006d2cbb3c36b41b9ace1f4e6e3a2c083a383` from the live candidate worktree. The committed source patch is archived as [`candidate.patch.gz`](candidate.patch.gz); its uncompressed SHA-256 is `f1063e75ce712f4ed196e16a1c7224ea51800e7e9b1366819e4ba150b6fc1493`. The final patch includes a test-only precedence clarification made during validation. The screened release worker itself is identified by its binary SHA-256 in `results-15pair.json`.

The committed screen runners are portable templates and take local candidate tree, worker, fixture, output and scratch paths through `RUSTSHT_*` environment variables. Raw runner scripts and captures with the original local paths remain in experiment scratch; the JSON committed here keeps numeric results, hashes and dispatch but redacts binary locations.

## Whole-transform screen

Fifteen interleaved parent/candidate/DUCC pairs, warm whole-call times in milliseconds. Candidate/parent uses geometric paired mean and a 95% t interval on log ratios. Rust f32/DUCC uses the matching native-f32 DUCC worker. All candidate/parent output difference metrics were zero; reset and repeat checks passed.

| GL T720 cell | Parent f32 | Candidate f32 | DUCC f32 | Candidate/parent (95% CI) | Candidate/DUCC f32 (95% CI) | Status |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 67.654 | 67.402 | 14.074 | 0.9902 [0.9753, 1.0054] | 4.7917 [4.7775, 4.8059] | parent INCONCLUSIVE; DUCC GAP |
| Full q8 | 9.122 | 9.065 | 2.066 | 0.9963 [0.9923, 1.0002] | 4.3946 [4.3666, 4.4229] | parent INCONCLUSIVE; DUCC GAP |
| Reduced q1 | 21.827 | 21.209 | 5.762 | 0.9761 [0.9699, 0.9824] | 3.6999 [3.6778, 3.7221] | parent PASS; DUCC GAP |
| Reduced q8 | 3.034 | 2.961 | 0.900 | 0.9735 [0.9664, 0.9806] | 3.2574 [3.2078, 3.3079] | parent PASS; DUCC GAP |

Relative L2 against DUCC was `1.8391e-5` full and `2.2203e-5` reduced. Existing numerical and exceptional guards passed; no tolerance changed. Exact pair records, fixture hashes, dispatch, output checks and worker hashes are in [`results-15pair.json`](results-15pair.json). The screen runner is [`screen-15pair.py`](screen-15pair.py).

The f64 source path is unchanged and the f64 B1 V3 symbol is byte-for-byte identical to parent. Its matched T720 baseline remains 1.226–1.310x DUCC f64 (GAP). The f32/f64 timing ratio is diagnostic only. The protected f64 T1023 spin-1 synthesis control produced exact outputs; q1 was 1.0049 [1.0011, 1.0086] against the parent and q8 was 0.9952 [0.9739, 1.0169]. Parent and candidate spin-1 binaries had identical `.text`, `.rodata`, `.data` and `.eh_frame` hashes, so the q1 wall-time shift is not an altered f64 code path. See [`t1023-protected-15.json`](t1023-protected-15.json).

## Predicate microbenchmark and generated code

A direct eight-vector probe used seven samples of 100,000 scans, comparing `numeric::f32_pair_lanes_need_scaling::<S, true>` with `f32_scalar_analysis_pair_needs_scaling`:

| Input pattern | Previous helper | New predicate | Ratio |
|---|---:|---:|---:|
| Ordinary values in all 64 lanes | 1.372 ms | 0.655 ms | 0.477 |
| Both state values below `SMALL` in lane 0 | 0.217 ms | 0.246 ms | 1.131 |
| Both state values below `SMALL` in lane 56 | 1.251 ms | 0.766 ms | 0.612 |

The microbenchmark source and machine-readable values are in [`predicate-microbenchmark.rs`](predicate-microbenchmark.rs) and [`predicate-microbenchmark.json`](predicate-microbenchmark.json). The first-exception case is slower because the prior helper returns immediately, while the new path performs a SIMD classification before its exact exceptional check. The late-exception case benefits because ordinary vectors avoid the horizontal lower-bound reduction. This probe isolates predicate scanning; it does not establish exception frequency in whole transforms.

The generated V3 kernel uses eight-lane `vandps`/`vmaxps`, ordered comparisons, `vptest` and conditional control flow; the exceptional path retains blends, lower-bound comparisons and reduction work. The saved excerpt is [`candidate-avx2-excerpt.txt`](candidate-avx2-excerpt.txt). Candidate code is 489 bytes and 107 static instructions larger in this kernel (22,879 vs 22,390 bytes; 4,578 vs 4,471 instructions). Thus the faster ordinary scan adds code and data-dependent control flow; assembly alone does not predict the whole-call result.

`perf stat` and `perf record` were unavailable on this host (`perf_event_paranoid=4`, no hardware events). A matched Callgrind profile at parent `410c9ec` counted 1,239,125,687 instructions for f32 full q1: normal accumulation 20,921,758 (1.7%); scaled accumulation plus factor preparation 539,089,068 (43.5%); recurrence advancement 49,750,530 (4.0%); range check plus normalization 538,792,655 (43.5%); state setup 4,585,065 (0.37%). These are annotated instruction counts, not cycles. Factor preparation (409,590,879) is included in the scaled-accumulation bucket, and normalization (232,239,654) is included in range-check/normalization; do not add these nested figures again. The earlier profile package is [`m6-b1-profile-410c-2026-10-31`](../m6-b1-profile-410c-2026-10-31/README.md).

## Validation and next

Focused B1 tests passed. The final isolated candidate also passed all-feature Rust tests (327 unit, 4 extreme-scaling, 14 f32 integration, 1 workspace compatibility; 39 ignored), strict Clippy with warnings denied, formatting, and 20 Python tests. The f64 T1023 control was exact. CI is checked after push.

**NEXT:** isolate f32 scaled accumulation/factor preparation from normalization and range classification with a bounded, source-matched probe. The range predicate improves only two reduced cells; full cells remain inconclusive and all f32/DUCC cells remain GAP. Use the profile to select one materially different candidate. Do not change B0/B2/B3 or f64 to improve the diagnostic f32/f64 ratio.
