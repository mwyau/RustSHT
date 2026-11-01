# B1 scalar-analysis hotspot profile — 2026-11-01

## Scope and provenance

This diagnostic pass profiles only B1 scalar analysis and screens three isolated f32 changes. The source parent is `5483faa8f3d2035ba505a171885ad23b051d1b8c`; `analysis.rs` is blob `baa5cee6c64ac43aa54828ab4c15055e8d23532e`, identical to the B1 file at `8560bcae`. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. No production code changed.

The host is an AMD Ryzen 9 5950X. Rust is 1.99.1 / LLVM 23.1.1, release `-C target-cpu=x86-64-v3`, with runtime Pulp V3 AVX2/FMA (eight f32 or four f64 lanes). DUCC uses `-O3 -march=x86-64-v3 -ffp-contract=fast`. The 15-pair f32 screen pins q1 to CPU 2 and q8 to CPUs 2, 8, 10, 12, 13, 16, 25 and 30. Per-pair fixture hashes, dispatch, affinity, binary hashes and original capture checksums are in the sanitized screen JSON files.

## Measured hotspot distribution

The production route is `ShtPlan<T>::analysis_into` → `analyze_map_to_coefficients` and the forward longitude FFT (`src/transform/scalar.rs:1271–1367`) → symmetric order jobs and `fused_scalar_analysis_pair_block_preweighted_typed` (`src/transform/scalar.rs:2061–2321`) → `FusedScalarAnalysisPairTyped<T>::with_simd` (`src/legendre/scalar_paired/analysis.rs:1478–1860`). The f32 scaled route is `accumulate_scaled_f32_impl` and `fill_b1_f32_analysis_factors` (`analysis.rs:147–551`); f64 uses its own precision leaf in the same shared block. The pinned DUCC route is `map2alm<float>`/`inner_loop_m2a` through `calc_map2alm`, `map2alm_kernel` and `extract_a2m`; DUCC f32 keeps the Legendre work in double precision.

For one nonzero GL T720 full-order q1 f32 transform, test-only Callgrind regions counted 719,316,173 Ir total. The main regions below are disjoint; the indented rows are subsets and must not be added again.

| Region | Ir | Share of total |
|---|---:|---:|
| Normal accumulation | 8,621,991 | 1.20% |
| Scaled accumulation, including state/factor work | 263,718,125 | 36.66% |
| Recurrence advancement | 22,870,504 | 3.18% |
| Range check plus normalization | 146,632,736 | 20.38% |
| **Disjoint subtotal** | **441,843,356** | **61.42%** |
| ↳ Factor materialization (inside scaled accumulation) | 106,855,383 | 14.86% |
| ↳ State setup (inside scaled accumulation) | 140,497,434 | 19.54% |
| ↳ Hard scalar fallback (inside scaled accumulation) | 21,915,387 | 3.05% |
| ↳ Scaled SIMD accumulation (inside scaled accumulation) | 41,396,359 | 5.76% |

The dominant f32 *instruction work* is scaled accumulation plus range/normalization; hard scalar fallback is small by comparison. A previous warm phase probe of the identical B1 source blob places 91–95% of f32 and 76–86% of f64 elapsed call time in combined Legendre work; the separately measured FFT phase was 5–9% f32 and 13–22% f64. Those ranges span q/thread cells and are not additive estimates. The probe does not split elapsed time within Legendre work. `perf stat`/`perf record` fail with `perf_event_paranoid=4` and no perf capability, so this pass cannot identify a hardware-cycle share. Callgrind Ir is not cycles.

The same-source f32 parent refresh in the lazy-clear 15-pair screen measured these matching DUCC ratios (paired geometric mean, 95% log-t interval):

| Cell | Rust parent ms | DUCC f32 ms | Rust / DUCC f32 |
|---|---:|---:|---:|
| Full q1 | 38.628 | 14.151 | 2.729 [2.702, 2.756] |
| Full q8 | 6.384 | 2.309 | 2.425 [2.198, 2.675] |
| Reduced q1 | 12.961 | 5.839 | 2.225 [2.198, 2.252] |
| Reduced q8 | 1.981 | 1.116 | 1.889 [1.775, 2.012] |

These matched current-source results do not reproduce the earlier stated 3.59–4.96× f32 range; the current ledger's f64 pairs are 1.032–1.131×, rather than the earlier 1.18–1.37× range. The source-identical runs used their own fixtures, affinity and host-load observations, so the datasets are not spliced and the difference is not attributed to a code change.

For f64, the existing exact-source 15-pair ledger remains 1.032–1.131× DUCC. A current f64 full-q1 Callgrind run counted 265,420,548 Ir; the checked scaled fallback accounts for 99,636,996 Ir (37.54%), with 94.4% of that region inside the V3 fused kernel. f64 remains much closer to DUCC in whole-call time, so the present measured opportunity is the f32 scaled path.

## Machine code and causal hypothesis

The parent V3 f32 `FusedScalarAnalysisPairTyped<f32>` body is 22,307 bytes with a 6,120-byte stack frame; the corresponding f64 body is 7,227 bytes with a 504-byte frame. The f32 body contains eight-lane YMM `vfmadd231ps` accumulation and scalar-recurrence FMAs; f64 uses four-lane YMM `vfmadd231pd`. The assembly excerpt is preserved in `raw/v3-kernel-disasm-summary.txt`; the matching pinned DUCC loop is in the [earlier B1 disassembly package](../m6-b1-profile-2026-11-01/ducc-f32-inner-loop.asm.gz). Code size and static stack references are structural evidence, not measured instruction-cache misses or cycles.

The source and profile support this hypothesis: native f32 reaches the scaled lane-state route frequently, and factor preparation plus range/normalization adds substantial work on top of the recurrence and accumulation. DUCC's public f32 path uses double Legendre state, so it avoids the same native-f32 range behavior. This explains a plausible mechanism for the f32 gap but is not a cycle-level proof. The f64 path's near-parity result and nontrivial fallback Ir show that instruction count alone does not predict the wall-time ratio.

## Bounded candidate screens

All candidates were based on the exact parent above, produced bitwise-equal output to it, and remain isolated. No candidate passed the whole-call screen.

| Candidate | Focused result | 15-pair whole-call candidate / parent | Decision |
|---|---|---|---|
| Reuse adjacent exponent factors | Factor-fill microbenchmark: 141.4 → 109.7 ns/fill (0.780); checksums equal | Full q1 0.937 [0.835, 1.051]; full q8 0.905 [0.720, 1.137]; reduced q1 1.100 [0.945, 1.280]; reduced q8 1.146 [0.927, 1.417] | Reject; all intervals cross 1 and reduced cells trend slower |
| Fuse recurrence range check | Focused exact-predicate/tail/late-failure tests passed; V3 body 22,307 → 21,789 bytes | Full q8 1.037 [0.888, 1.212]; point is slower | Reject; no whole-call gain |
| Clear hard-factor vectors lazily | 721-lane one-hard-vector microbenchmark: 256.1 → 245.1 ns/fill (0.943) | Full q1 1.001 [0.988, 1.013]; full q8 0.949 [0.886, 1.015]; reduced q1 1.003 [0.995, 1.011]; reduced q8 0.986 [0.933, 1.043] | Reject; full-q8 point gain is unresolved |

The parent f32 V3 body has 871 static frame-relative (`[rsp]`) memory instructions; adjacent reuse has 863 and lazy clear 846. All three have the same 86 static YMM references to frame memory (80 stores and four loads; two other references), so the edits did not remove vector stack traffic. These operations appear to include local scale-state initialization/access; this disassembly pass does not classify them as register spills. Lazy clear grew the body by 135 bytes and added two conditional branches; adjacent reuse grew it by 763 bytes and 56 branches. The local helper wins therefore do not establish a transform improvement. Focused tests also passed for lazy hard-factor overwrites from poisoned scratch and exact range predicate behavior. Sanitized 15-pair JSON, compressed Callgrind regions, test harnesses and rejected patches are in `raw/`; `raw/SHA256SUMS` covers the preserved files. Original raw JSON captures remain outside the repository; their SHA-256 values are embedded in the sanitized JSON. Decoded candidate patch SHA-256 values are adjacent reuse `da04e06c0a7a3cbb77947af7c92a2c3f405edbac8b5113f8fed428051e9b7778`, range fusion `65ecae69f799cff85b5e5242f5d1b661b72c90d7f1a6613bc6b21c141737ac59`, and lazy clear `cb01cf94f3a53fbe367ea5833aae867b88b45bb6002b5b06e063c2952057ec91`.

## Conclusion and NEXT

The data locate f32's largest avoidable work in scale-factor/state handling and range/normalization, rather than the recurrence update or hard-lane fallback. The selected factor and range micro-optimizations were too local or code-size-expensive to produce a resolved whole-call gain. f64 remains close to DUCC; do not slow it or optimize a cross-precision ratio. No production change is promoted.

**NEXT:** use a low-overhead sampled wall-time probe on the exact nonzero T720 stream to separate f32 factor/state rebuild, scaled SIMD accumulation and pair range/normalization time. Account for sampling overhead and keep these disjoint from FFT and scheduling. Use those elapsed-time shares to select one materially different f32 candidate; screen it in three interleaved whole-call pairs and stop if no stable signal. Keep the 15-pair f64 baseline as a guard, leave B0/B2/B3 untouched, and do not repeat adjacent-factor reuse, range fusion or lazy hard-factor clearing.
