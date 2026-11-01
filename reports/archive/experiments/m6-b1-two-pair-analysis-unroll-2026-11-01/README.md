# B1 scalar analysis: ordinary f64 two-pair traversal

## Result

The f64 ordinary typed path previously scanned the latitude vectors once per coefficient pair. The candidate accumulates two adjacent pairs during one latitude pass, reusing each same/opposite spectrum vector load for both outputs and advancing the recurrence twice. The static precision specialization enables this only for f64 and only when the caller proves the complete range safe. Scaled and exceptional work stays on the existing route.

A focused GL T720 typed-block benchmark improved from 10.988 to 7.587 microseconds median across nine pinned batches of 100 calls (30.9%). A 15-pair whole-transform screen improved the exact parent by 3.6–9.7% in all four f64 cells, with confidence intervals below 1 and byte-identical output in all 60 pairs. The matching DUCC deficit remains 1.24–1.29x. This is an accepted bounded optimization, not parity.

The improvement supports the dataflow hypothesis that repeat traversal and spectrum loads are avoidable f64 cost. Hardware performance counters are blocked on this host (`perf_event_paranoid=4`, no perf capabilities), so this report does not claim a cycle-level attribution. Existing phase probes place recurrence plus accumulation at 82–91% of f64 B1 time; this is the largest measured stage, while FFT accounts for 8–18%.

## Source crosswalk

| RustSHT | Pinned DUCC 0.42.0 | Work |
|---|---|---|
| `ShtPlan::analysis_into`, `src/transform/scalar.rs:1103` | `alm2leg`, `src/ducc0/sht/sht.cc:576` | Transform setup and Fourier-order analysis |
| `analyze_map_to_coefficients` and `analyze_order_group`, `src/transform/scalar.rs:1271,2061` | `calc_map2alm`, `src/ducc0/sht/sht_inner_loop.h:636` | Ring/order work and output scheduling |
| `fused_scalar_analysis_pair_block_preweighted_typed`, `src/legendre/scalar_paired/analysis.rs:1001` | `map2alm_kernel`, `src/ducc0/sht/sht_inner_loop.h:592–616` | SIMD recurrence and degree accumulation |
| `FusedScalarAnalysisPairTyped::with_simd`, `analysis.rs:1237` | `iter_to_ieee` then `map2alm_kernel` | Exceptional prefix and ordinary range |

The production dispatch from the order group reaches the typed kernel at `src/legendre/mod.rs:1890`. On Pulp V3/AVX2, f64 uses four latitude lanes and f32 uses eight. DUCC's public f32 path uses double recurrence/accumulation; this package's f32 experiments are native Rust f32 and are kept separate from the f64 result.

The kernel evaluates the same recurrence in both precisions. In the ordinary vector path it forms the leading factor with FMA, advances the state with a second FMA, and accumulates four reflected real/imaginary channels. The two-pair f64 helper performs that recurrence and accumulation for the next two outputs before reducing. Coefficient order, reflected signs and the checked/scaled route are unchanged.

## Measured profile and generated code

The 2026-10-31 matched phase probes used parent `410c9ec2594007a1e081c949767fc5f7c28d2654`, nonzero GL T720 fixtures, V3 AVX2/FMA and q1/q8. Recurrence plus accumulation was 82–91% of f64 time; FFT was 8–18%. For f32, recurrence plus accumulation was 94–97%, and 52.9% of full-order coefficient-pair accumulations took the scaled route. Full-q1 f32 Callgrind reported 1.239B total Ir; targeted nonoverlapping region counts total 1.149B Ir. The factor-preparation and normalization counts listed in the profile package are subsets of larger regions. These are dynamic instruction counts, not cycles, and do not isolate the f64 ordinary-loop cost.

In the f64 V3 disassembly, the typed body is 7,236 bytes (`0x1c44`), compared with 5,318 bytes (`0x14c6`) in the parent. The hot loop holds eight accumulation vectors in YMM registers and has no accumulator spills. It does reload pointers/counters from stack slots, and writes the next recurrence vector to its state buffer between degrees. See [avx2-loop.txt](avx2-loop.txt) for the loop excerpt and binary/source hashes. The larger body is real, but the isolated kernel and whole-call measurements improved; code size alone does not identify the residual DUCC gap.

The benchmark runner recorded candidate/parent/DUCC worker hashes in every row of [paired-15.json](paired-15.json). A separate source-equivalent candidate build was used for disassembly; its executable and `.text` hashes are identified separately in [provenance.json](provenance.json). The parent and candidate sources are B1-identical between `410c9ec` and base `8fef71d5ce2b0921fd84f2feb1ecac213362bf0a`. The earlier pre-tail-correction screen and patch remain as explicitly named audit records; they are superseded by the corrected `paired-15.json` and `candidate-final-8fef.patch.gz`.

## Matched f64 whole-call screen

Fifteen interleaved parent/candidate/DUCC pairs per cell, GL T720, full M720 or reduced M180, q1/q8. Rust used rustc 1.99.1 with `-C target-cpu=x86-64-v3`; Pulp selected V3 AVX2/FMA. q1 was pinned to one physical core and q8 to eight physical cores; specific core IDs are omitted from the committed summary. Warm whole-call batches targeted 25 ms. The candidate binary hash recorded by the runner is `b1808a60e5a4a7461972fbaf3349886e8994c4d1e16d8b8cbad27b03001cd649`; the per-pair fixture and executable hashes and bootstrap intervals are preserved in `paired-15.json`. DUCC is pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

| Cell | Parent ms | Candidate ms | DUCC f64 ms | Candidate / parent (95% paired CI) | Candidate / DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 19.974 | 19.218 | 14.887 | 0.9606 [0.9533, 0.9821] | 1.2890 GAP |
| Full q8 | 2.787 | 2.691 | 2.121 | 0.9638 [0.9597, 0.9701] | 1.2663 GAP |
| Reduced q1 | 8.472 | 7.665 | 6.037 | 0.9034 [0.8938, 0.9201] | 1.2716 GAP |
| Reduced q8 | 1.193 | 1.138 | 0.912 | 0.9526 [0.9462, 0.9610] | 1.2416 GAP |

## Typed-block microbenchmark

The test-only harness directly calls the typed f64 block for m=1, lmax=720, and 64 equatorial GL T720 lanes with fixed nonzero channels, production recurrence factors, sectoral normalization and a proven-normal-range block. Nine batches of 100 calls were pinned to one physical core.

| Source | Median per block | Batch range |
|---|---:|---:|
| Parent `410c9ec` | 10.988 us | 10.931–11.171 us |
| Candidate | 7.587 us | 7.518–8.785 us |

Checksums match. Logs and the test-only harness patch are in [microbench-parent.log.gz](microbench-parent.log.gz), [microbench-candidate.log.gz](microbench-candidate.log.gz) and [microbench-harness.patch.gz](microbench-harness.patch.gz). No harness instrumentation is part of production code.

## f32 unroll diagnostic and tail correction

The original f32-unroll failure was traced to an incorrect scalar-tail state handoff in the new helper: after the second degree, the recurrence state must be written to `previous_tail`, not `next_tail`. The new typed-path assertion in `analysis/tests.rs` calls the f64 unroll with 21 lanes, forcing a scalar SIMD tail; the incorrect handoff fails this reference comparison. With the handoff corrected, all 14 non-ignored f32 integration tests pass while the f32 unroll is temporarily enabled, and the outputs remain finite and repeat/reset-consistent.

A three-pair f32 screen was then run on all four T720 cells with the corrected handoff and unroll enabled only in the diagnostic candidate. Candidate and parent outputs were byte-identical in all 12 pairs, but candidate/parent median ratios were 1.0252 full q1, 1.0273 full q8, 1.0146 reduced q1 and 1.0161 reduced q8. This is a small consistent slowdown, so production f32 remains unrolled=false. The screen's source, binary, fixture hashes and individual pairs are in [f32-unroll-3pair-screen.json](f32-unroll-3pair-screen.json); the one-line toggle is preserved, while the absolute-path measurement runner remains in local task scratch. The V3 f32 typed-kernel symbol remains byte-identical to parent while the production flag is false; see [f32-codegen.json](f32-codegen.json). The patch and microbenchmark logs are stored losslessly as gzip files. `SHA256SUMS.txt` hashes stored artifacts; hashes in provenance identify decoded patch payloads where stated.

## Correctness and validation

- The direct typed-tail reference test passes; its tolerance is unchanged.
- In the isolated candidate worktree, all workspace Rust tests with all features passed (325 unit tests; 39 ignored; 4 extreme-range, 14 f32 and 1 compatibility integration tests), strict all-target Clippy passed, and Python tests passed (20).
- The full f32 integration file also passed with the f32 unroll temporarily enabled (14 passed, 1 ignored).
- All 60 f64 parent/candidate whole-call outputs were byte-identical. T1023 f64 spin-1 synthesis q1/q8 had a three-pair parent/candidate timing screen with bitwise-equal output and no regression indicated; this is not a statistical PASS.
- No tolerance, API, f32 production route or f64 spin path changed.

## Next

Keep the f64-only traversal. Profile the native f32 scaled recurrence and factor/state preparation separately from normalization, then screen one distinct f32-safe change. Do not report Callgrind Ir as cycles. B0, B2 and B3 remain parked.
