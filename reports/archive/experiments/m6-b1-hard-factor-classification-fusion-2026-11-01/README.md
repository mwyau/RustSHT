# B1 f32 factor fill and hard-lane classification fusion

## Result

The isolated B1 f32 change classifies hard lanes while it materializes per-lane scale factors. Ordinary vectors no longer make a second pass over the finished factor array to discover that no factor underflowed to zero. Vectors with zero factors still use the same exceptional grouping and scalar fallback rules. f64 code and recurrence arithmetic are unchanged.

Fifteen interleaved warm whole-transform pairs improve every GL T720 f32 cell against exact parent `acd7fe5b2f68f4423c6904caed99c10545764bf3`. Candidate output is bitwise equal to the parent. The f32/DUCC f32 gap narrows to 2.91–4.14x and remains a GAP.

| GL T720 B1 | Parent f32 ms | Candidate f32 ms | DUCC f32 ms | Candidate/parent (95% CI) | Candidate/DUCC (95% CI) |
|---|---:|---:|---:|---:|---:|
| Full q1 | 64.665 | 57.942 | 14.039 | 0.8990 [0.8933, 0.9047] | 4.1426 [4.1113, 4.1742] GAP |
| Full q8 | 8.708 | 7.798 | 2.045 | 0.8959 [0.8925, 0.8994] | 3.7821 [3.7140, 3.8514] GAP |
| Reduced q1 | 20.573 | 18.581 | 5.737 | 0.9018 [0.8977, 0.9059] | 3.2417 [3.2295, 3.2540] GAP |
| Reduced q8 | 2.869 | 2.588 | 0.891 | 0.9047 [0.9010, 0.9083] | 2.9110 [2.8960, 2.9261] GAP |

The 95% intervals are Student-t intervals on paired log ratios. Each pair uses ten warm whole-call samples and includes plan execution, scheduling, Legendre work, FFT and output handling. q1 is pinned to one physical core; q8 to eight distinct physical cores. Logical CPU IDs are omitted from the archived report. The host is an AMD Ryzen 9 5950X. Rust release flags were `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; Pulp dispatched V3 AVX2/FMA with eight f32 lanes. DUCC is pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` and used its native public f32 path.

The exact candidate/parent/DUCC binary SHA-256 values, deterministic fixture hashes, all pair records, dispatch and output metrics are in [`results-15pair.json`](results-15pair.json). The three-pair screen that preceded the 15-pair run is in [`screen-3pair.json`](screen-3pair.json). The candidate's relative L2 error against DUCC is `1.8391e-5` full and `2.2203e-5` reduced; this change preserves the Rust parent's output bit-for-bit. No tolerance changed.

## Profile and diagnosis

The matched full-q1 Callgrind captures count 320.34M Rust f64 Ir, 1,178.62M Rust f32 Ir, 219.96M DUCC f64 Ir and 214.19M DUCC f32 Ir. Inclusive Rust attribution puts `analyze_symmetric_order_group` at 83.8% of f64 Ir and 96.55% of f32 Ir. Its fused pair block is 65.05% of f64 Ir and 92.03% of f32 Ir. DUCC's `inner_loop_m2a<double>` accounts for 62.70% of DUCC f64 Ir. These profiles include Callgrind instrumentation and are instruction counts, not elapsed cycles. Raw compact profiles and the test-only [`callgrind-instrumentation.patch.gz`](callgrind-instrumentation.patch.gz) are retained in this package.

Separate phase probes at parent `410c9ec2594007a1e081c949767fc5f7c28d2654` found recurrence plus accumulation taking 94–97% of Rust f32 whole-call time and 82–91% of Rust f64 time; FFT was 3–6% and 8–18%, respectively. A source annotation at `410c9ec` attributed 71.8M Ir (5.79% of full-q1 f32 total) to the second zero-factor scan. The f32-specific avoidable traversal was therefore measurable, but it is only one part of the larger scaled recurrence/accumulation workload. Hardware `perf stat` and `perf record` are unavailable here (`perf_event_paranoid=4`, no perf capabilities); no cycle attribution is claimed. See [`profile-summary.json`](profile-summary.json) and the earlier [B1 phase profile](../m6-b1-profile-410c-2026-10-31/README.md).

A direct helper microbenchmark compares the exact previous two-pass behavior with the candidate fused helper: nine interleaved pairs, 20,000 fills per sample, eight-lane V3, 64 synthetic exponent entries with six ordinary vectors and two hard vectors, pinned to one physical core (logical ID omitted). The paired geometric ratio is `0.625` [0.602, 0.648] and outputs/classification are equal. This isolates helper overhead; it is not a whole-transform estimate. See [`microbenchmark.json`](microbenchmark.json) and the temporary test-only [`microbenchmark-snippet.rs`](microbenchmark-snippet.rs). The probe was removed after measurement.

The V3 AVX2 excerpt in [`candidate-avx2-excerpt.txt`](candidate-avx2-excerpt.txt) confirms eight-lane `vbroadcastss`, YMM loads and fused `vfmadd213ps` accumulation remain in the ordinary kernel. The monomorphized f32 typed method is 9,554 bytes versus 10,632 bytes in the parent binary; static instruction count is 1,953 versus 2,139, with 64 FMA instructions in both. Static counts and code size are not cycle counts. The code change removes a second factor-array pass and the associated work; it does not address the larger recurrence/accumulation cost or the f64 gap.

## Source crosswalk

| Stage | Rust source | Pinned DUCC source | Notes |
|---|---|---|---|
| Public analysis entry and FFT/order preparation | `src/transform/scalar.rs:1103` `analysis_into`; `:1271` `analyze_map_to_coefficients` | `src/ducc0/sht/sht.cc:576` `alm2leg` | Whole-call timing includes FFT, Legendre stage and scheduling. |
| Reflected order/ring jobs | `src/transform/scalar.rs:2119` `analyze_symmetric_order_group` | `sht_inner_loop.h:1752` `inner_loop_m2a` | Rust shares reflected work across northern/southern ring jobs. |
| f32 factor creation and hard-lane grouping | `src/legendre/scalar_paired/analysis.rs:406` `fill_b1_f32_analysis_factors`; `:525` f32 precision impl | `sht_inner_loop.h:636` `calc_map2alm` | Rust native f32 retains exponent state and eight-lane factors. |
| Degree recurrence and same/opposite accumulation | `src/legendre/scalar_paired/analysis.rs:1165` `fused_scalar_analysis_pair_block_preweighted_typed` | `sht_inner_loop.h:592` `map2alm_kernel`, `:636` `calc_map2alm` | DUCC kernel advances by two same-parity degrees (`l += 4`) and reduces accumulated channels; DUCC f32 analysis uses double work arrays in `sht.cc`. |

## Validation and provenance

- Candidate base: `acd7fe5b2f68f4423c6904caed99c10545764bf3`.
- Candidate source patch SHA-256: `be17b7a0e8d02e8108d1416bdb4c3f58967d6a8dc6f8277ca30fe14031d3bc3e` (stored as [`candidate.patch.gz`](candidate.patch.gz); checksum is for decompressed patch bytes).
- Candidate and parent whole-call binary hashes: `344be45b4c2388c5d21bab1606fe3e79390b211fdcb05a39ab54905eceb92a5d` and `6fcb31167af51511d63593e5f8ddc871c6ea1a52655cd930ef549333bb27cd06`.
- DUCC f32 worker binary hash: `6c70f3b27a594d2f3e37d12bc5be650183072e439435640aedc397d566dc4100`.
- Focused B1 release module tests: 38 passed, 8 ignored; f32 integration tests: 14 passed, 1 ignored; extreme scaling tests: 4 passed. The candidate preserves parent bits and workspace reset behavior.
- `cargo fmt --all -- --check`, strict all-target/all-feature Clippy, and all-target/all-feature workspace checks passed. Workspace Rust tests passed (328 unit, 4 extreme scaling, 14 f32 integration, 1 workspace compatibility; existing ignored tests unchanged). Python tests passed: 20.
- Protected f64 T1023 spin-1 synthesis 3-pair control: q1 candidate/parent median `1.00015`, q8 `0.99649`; outputs were bitwise equal. Raw control data is in [`t1023-protected-3pair.json`](t1023-protected-3pair.json).
- [`SHA256SUMS`](SHA256SUMS) covers every stored artifact in this package.

## NEXT

Keep this f32 classification fusion. B1 f64 remains 1.226–1.310x matching DUCC from its existing 15-pair baseline and was not changed; B1 f32 remains 2.91–4.14x DUCC. Profile the ordinary/scaled recurrence and accumulation dataflow in the current B1 kernels to find a larger shared or precision-specific cost. Do not repeat factor-fill, range-threshold, tile, sparse-index or rejected recurrence-fusion variants without a new measured hypothesis. Continue B1 only.
