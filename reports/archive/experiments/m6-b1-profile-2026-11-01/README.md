# B1 scalar-analysis profile and rejected candidates — 2026-11-01

## Scope and provenance

This is a diagnostic-only follow-up on exact parent source `255e34d4a078c42ffa5de3616c73f6eca173f472` (same as fetched `origin/dev`). DUCC was pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. No production dispatch change was retained. Rust's parent source SHA is shared by the f32 and f64 binaries. The exact parent and DUCC binary hashes, fixture hashes, all three paired rows, and errors are recorded in `f32-whole-call-screen.json` and `f64-whole-call-screen.json`; candidate binary hashes are recorded there as well. `f32-prior-three-pair-screen.json` preserves a separate screen using the same parent and DUCC binaries and identical fixtures.

Host: AMD Ryzen 9 5950X; Rust 1.99.1 / LLVM 23.1.1; `RUSTFLAGS="-C target-cpu=x86-64-v3 -Aunexpected_cfgs"`; AVX2/FMA Pulp V3 (8 f32, 4 f64 lanes); q1 pinned CPU 2, q8 CPUs 0–7. DUCC used `-O3 -march=x86-64-v3 -ffp-contract=fast`. `perf stat` and `perf record` are unavailable because `perf_event_paranoid=4` and the process lacks PMU capability.

## Dispatch and machine code

Rust B1 dispatch is `ShtPlan::analysis_into` → `analyze_map_to_coefficients` (`src/transform/scalar.rs:1271`) → `analyze_order_jobs_dynamic` (`:1944`) → `analyze_order_group` (`:2061`) → `fused_scalar_analysis_pair_block_preweighted_typed` → `FusedScalarAnalysisPairTyped<T>::with_simd` (`src/legendre/scalar_paired/analysis.rs`). Forward rows use `forward_analysis_rows` (`src/transform/fft_rows.rs:199`). Pinned DUCC uses `alm2leg<float>` (`sht.cc:576`) → `inner_loop_m2a` (`sht_inner_loop.h:1752`) → `calc_map2alm` / `map2alm_kernel` → `extract_a2m`. DUCC's public f32 path accepts f32 spectra but accumulates recurrence/work in double precision.

Rust's ordinary f32 loop emits eight-lane AVX2 FMA: four same/opposite real/imag accumulators plus the two FMA recurrence operations per two-degree step. f64 uses four lanes. The ordinary loops show no YMM spills, but the f32 loop reloads several pointer bases from stack slots. Rust's f32 typed kernel is 22,532 B with a 6,120 B local frame; f64 is 7,102 B with a 504 B local frame. DUCC's f32 inner loop is 3,509 B. These code-size and stack facts do not alone establish cycle cost.

## Current-source hotspot

A test-only phase probe put f32 Legendre recurrence plus accumulation at 42.053 of 44.676 ms for full q1 (94.1%) and 7.027 of 7.763 ms for full q8 (90.5%). f64 Legendre work was 13.682 of 17.352 ms for full q1 (78.9%) and 2.260 of 3.088 ms for full q8 (73.2%). Matching Rust/DUCC whole-call ratios from a three-pair source-specific screen were:

| Cell | f32 / DUCC f32 (Rust / DUCC ms) | f64 / DUCC f64 (Rust / DUCC ms) |
|---|---:|---:|
| Full q1 | 2.840 (42.026 / 14.723) | 1.068 (16.310 / 15.280) |
| Full q8 | 2.675 (5.993 / 2.224) | 1.261 (2.990 / 2.372) |
| Reduced q1 | 2.314 (14.161 / 6.255) | 1.065 (6.521 / 6.094) |
| Reduced q8 | 2.139 (2.244 / 1.036) | 1.073 (1.091 / 1.009) |

A separate three-pair f32 screen used the same parent and DUCC binaries and the same fixture hashes but measured ratios 2.848/2.830/2.424/2.075. The change between runs is unexplained timing variation, not a source change; the raw paired JSON for both screens is included, and neither is a promotion claim. The four phase-probe logs and a sanitized q8 queue summary are retained. A single nonzero full T720 q1 test-instrumented call counted 1,067,695 normal and 266,685 scaled f32 pair accumulations. Scaled work visited 8,533,920 active lanes. The state was prepared 3,883 times; factors were rebuilt in 159,441 epochs (5,102,112 lane visits) and invalidated 158,692 times. Thus a factor epoch occurs once per 1.67 scaled-pair calls on average. These counts localize frequent scale-state transitions but are not elapsed-time shares. Callgrind's dynamic instructions attribute 1.150B f32 instructions (22.18% of total) and 281.3M f64 instructions (13.54%) to the fused typed vectorized function; it is inclusive of recurrence and accumulation, not a decomposition between them. The instrumented program totals are 5.184B f32 and 2.078B f64 instructions, and test/Rayon work is included. Compressed Callgrind profiles and source annotations are retained alongside the Rust/DUCC disassembly.

## Candidate results

The scaled-accumulator microbenchmark holds exponents fixed across 360 pairs and measured 47.33 ns/pair at parent, 44.60 ns with inline preweights, and 40.89 ns with lazy boxed state plus preweights. The production trace changes scale state about every one to two scaled pairs, so the microbenchmark's reuse assumption does not hold.

| Candidate | Whole-call screen | Decision |
|---|---|---|
| Inline f32 channel preweight | Full q1: 1.151 candidate/parent geometric mean over 3 pairs; candidate was 14.6–15.7% slower | Reject |
| Box exceptional B1 state lazily | Candidate/parent geometric means: 1.003 full q1, 1.002 full q8, 0.992 reduced q1, 1.060 reduced q8 (3 pairs each) | Reject; no stable gain, reduced q8 regressed |
| Lazy box plus channel preweight | Full q1: 1.320 candidate/parent over 3 pairs | Reject |

All screened candidate outputs were bitwise identical to parent. The focused scaled-transition unit test passed. No 15-pair validation was warranted because no candidate screened as a winner. Candidate binaries are not committed; their hashes are recorded in the paired JSON. Compact paired results, route counts, benchmark driver scripts, the test probe, compressed Rust/DUCC disassembly, and the combined rejected patch are included here. `SHA256SUMS` covers every evidence file in this package.

## Conclusion and NEXT

The measured f32 cost is concentrated in native-f32 Legendre work and its frequent scale epochs. Channel preweighting loses because it rebuilds four weighted channels nearly every one to two scaled pairs. Moving the state off the common stack shrinks the frame but did not improve warm whole calls. Current evidence does not yet apportion the avoidable Legendre time among recurrence advancement, normalization/factor preparation, and hard scalar/vector fallback. For f64, the full-q1 gap is about 6.8%, while full-q8 is 26.1%; the queue-lock probe found only about 1.9% of its separate q8 call in queue locking, so the larger q8 difference needs another profile. FFT is not the dominant avoidable share in the measured whole calls.

**NEXT:** on B1 only, sample warmed current-source wall time around recurrence advancement, state normalization/factor preparation, and hard fallback; count hard scalar/vector lanes. Use that profile to select one materially different candidate. Keep f32 and f64 matching-DUCC comparisons separate. Do not promote any candidate until it wins three interleaved pairs; use 15+ pairs only for a consequential promotion.
