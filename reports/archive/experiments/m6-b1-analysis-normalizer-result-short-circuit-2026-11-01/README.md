# B1 f32 normalizer result short-circuit

## Decision

Keep this bounded f32-only change. Once any SIMD group makes the block-wide `normal_range` predicate false, later groups cannot make it true again. The candidate skips only the remaining range classifications: every lane still receives the same state normalization, exponent update, exceptional handling, recurrence and accumulation. This removes redundant range-mask reductions without changing the f64 algorithm.

The candidate is based on parent `ecb66f10ecb11146fb1c1a49ef3fdfd5bd1c7505`, tree `00ebc654130e2f4cd153be07759c9c517de6c6ca`. Decompressing [`candidate.patch.gz`](candidate.patch.gz) yields patch SHA-256 `aaffc4602e6d1fd218cad39ff335fab226e8f8344e547d6d5af8add63f70fe26` and reproduces the exact measured source. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Hotspot evidence

The matched phase probe at parent `410c9ec` put Rust f32 Legendre recurrence plus accumulation at 94–97% of the whole B1 call, with FFT at 3–6%. Its f64 path spent 82–91% in recurrence plus accumulation, with FFT at 8–18%. These are disjoint phase timings, but the recurrence and accumulation components were not split from each other. The f32 path at `ecb66f10` was unchanged from that source. The older profile details and limitations are in the [410c profile report](../m6-b1-profile-410c-2026-10-31/README.md).

Hardware counters were unavailable: `perf stat` reports `perf_event_paranoid=4` and no supported events. No cycle attribution is claimed. The direct release microbenchmark ran 30,000 64-lane normalizations per sample after one warm-up. Across eight timed samples, the parent median was 1,068.090 µs and the candidate was 1,043.713 µs (0.9772, 2.3% faster); the test also compares the output values, epochs and result flag. This synthetic mixed-scale probe measures the target kernel only, not a production transform.

## Executed AVX2 path

Rust B1 dispatch reaches `fused_scalar_analysis_pair_block_preweighted_typed<T>` and `FusedScalarAnalysisPairTyped<T>::with_simd` in `src/legendre/scalar_paired/analysis.rs`. Its normalizer calls `T::normalize_analysis_pair_state` at lines 1288 and 1449. The f32 implementation at line 395 selects `normalize_pair_lanes_with_state_f32_simd_analysis`; f64 uses the unchanged trait default.

The V3 f32 vectorizer compiled in `ducc_f32_full_transform_parity` is 22,390 bytes (`0x5776`) in the candidate versus 22,750 bytes (`0x58de`) in the parent. The candidate uses eight AVX2 lanes. See [`avx2-range-gate.txt`](avx2-range-gate.txt) for actual `objdump` excerpts: after storing the normalized state, the candidate tests the persistent range flag and branches around the epoch scan and SIMD `vcmple`/`vcmpge`/`vptest` reduction after the first failed group. The parent executes those checks for every group. Normalization remains outside the skipped region.

## Matched GL T720 f32 whole calls

Fifteen interleaved parent/candidate/Rust-f64/DUCC pairs per cell; full M720 and reduced M180; q1/q8. Rust used rustc 1.99.1 with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; runtime dispatch was V3 AVX2/FMA with eight f32 lanes. The host was an AMD Ryzen 9 5950X. q1 was pinned to one physical core and q8 to eight physical cores. DUCC used the pinned native f32/f64 workers built with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`. Ratios and 95% intervals are geometric paired means with t intervals on log ratios. Full per-pair times, fixture hashes, binary hashes, dispatch and reset results are in [`paired-15.json`](paired-15.json).

| Cell | Parent f32 ms | Candidate f32 ms | DUCC f32 ms | Candidate / parent (95% CI) | Rust f32 / DUCC f32 (95% CI) | Rust f64 / DUCC f64 (95% CI) |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 70.101 | 67.815 | 14.113 | 0.9626 [0.9565, 0.9688] PASS | 4.7958 [4.7579, 4.8340] GAP | 1.3100 [1.2748, 1.3462] GAP |
| Full q8 | 9.406 | 9.102 | 2.058 | 0.9659 [0.9589, 0.9730] PASS | 4.4186 [4.3967, 4.4407] GAP | 1.2733 [1.2436, 1.3038] GAP |
| Reduced q1 | 22.531 | 21.816 | 5.770 | 0.9668 [0.9604, 0.9732] PASS | 3.7793 [3.7543, 3.8045] GAP | 1.2784 [1.2516, 1.3058] GAP |
| Reduced q8 | 3.132 | 3.034 | 0.899 | 0.9692 [0.9603, 0.9783] PASS | 3.3734 [3.3455, 3.4014] GAP | 1.2255 [1.1941, 1.2577] GAP |

The f32 candidate was faster than its exact parent in all 60 paired calls; candidate/parent relative L2 was 0.0 in all 60 comparisons, and repeat/reset checks passed. This is a 3.1–3.7% whole-call reduction, not a DUCC parity result. The f32/DUCC gap remains 3.37–4.80x.

## f64 and protected T1023 controls

Because the whole f64 executable hash also changed, a separate 15-pair parent/candidate f64 B1 screen was run. Parent/candidate outputs were bitwise equal in all 60 calls. Every paired interval includes 1.00; the f32 specialization has no measured f64 timing effect.

| Cell | Candidate / parent f64 (95% CI) | Status |
|---|---:|---|
| Full q1 | 1.0009 [0.9911, 1.0109] | INCONCLUSIVE |
| Full q8 | 1.0071 [0.9997, 1.0145] | INCONCLUSIVE |
| Reduced q1 | 1.0002 [0.9437, 1.0602] | INCONCLUSIVE |
| Reduced q8 | 1.0110 [0.9872, 1.0354] | INCONCLUSIVE |

A separate three-pair T1023 f64 spin-1 synthesis control compared the whole binaries. Outputs were bitwise equal. Candidate/parent ratios were 0.9917, 1.0040, 1.0011 at q1 and 1.0025, 1.0035, 1.0023 at q8. This small control shows no regression; it is not a statistical performance claim. Fixture and executable hashes are in [`t1023-f64-control.json`](t1023-f64-control.json).

## Validation and remaining gap

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --all-features` passed: 326 unit tests, 4 extreme-scaling tests, 14 f32 integration tests, 1 workspace-compatibility test; 39 ignored.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `uv run --group dev pytest python/tests` passed (20 tests).
- The 67-lane normalization test covers SIMD groups, a scalar tail, tiny/large values and nonzero exponents; the full suite retained existing analytic, adjoint, exceptional and workspace-reset guards.

This removes repeated classification work, not the dominant scaled recurrence/accumulation itself. Continue profiling that f32 path with a new measured hypothesis. Keep B1 as the only active family; B0, B2 and B3 remain parked. Do not describe Callgrind instructions as cycles or relax numerical tests.
