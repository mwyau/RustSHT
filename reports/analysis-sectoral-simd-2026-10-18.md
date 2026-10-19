# Scalar analysis SIMD initializer checkpoint

**Source:** RustSHT candidate `cbb1ea6b`, compared with prior production source `9c6f7cb9`; pinned DUCC source `80388904b8cfc32400ba84af8a6a45aabfe60f86`. Installed `ducc0 0.41.0` corresponds to source `c1ed01b8fda69d2a378190f0915f1a19762dddbf` for the relevant routines. **Host:** AMD Ryzen 9 5950X, x86_64, AVX2/FMA. Rust 1.99.0, release `-C target-cpu=x86-64-v3 -C debuginfo=1 -C force-frame-pointers=yes`; Python 3.11.15, NumPy 2.4.6.

## Change and numerical checks

The fused f64 scalar-analysis block computes ordinary-range sectoral powers across latitude SIMD lanes when `lmax >= 511`. The initializer is shared by scalar analysis and the two-channel Q/U recurrence used by spin-1 analysis. If any lane fails the range check, it uses the checked scalar initializer.

The initializer matched `scalar::sectoral_value` at order 720 across 21 lanes, including the SIMD tail. Fused one- and two-channel blocks passed paired-reference checks at `lmax=511`; range-decline tests passed. The pinned-DUCC spin analysis test passed for GL, CC and F1. A T720 GL scalar analysis comparison had max absolute error `1.528e-14` and relative L2 error `6.039e-14`.

## Rust candidate screen

Three process-paired release screens compared reused-plan Rust analysis before (`9c6f7cb9`) and after (`cbb1ea6b`) the change. Entries are medians of process medians; the short screen is directional, especially for q8.

| Operation and workload | Prior Rust (ms) | Candidate Rust (ms) | Change |
| --- | ---: | ---: | ---: |
| Scalar analysis, T720, q1 | 42.866 | 40.351 | -5.9% |
| Scalar analysis, T720, q8 | 7.256 | 6.930 | -4.5%; noisy, one pair was +2.7% |
| Spin-1 analysis, T511, q1 | 30.224 | 28.290 | -6.4% |
| Spin-1 analysis, T720, q1 | 73.582 | 69.457 | -5.6% |
| Spin-1 analysis, T720, q8 | 12.215 | 12.304 | Inconclusive |

The q8 samples do not establish a repeatable gain. A T255 q8 control was noisy and showed no reliable benefit, so the vector initializer is gated at `lmax >= 511`.

## Current scalar analysis comparison with DUCC

At T720 GL, full `mmax`, 721×1442, `phi0=0.37`, one process per thread count used one warmup and five timed calls. Rust reused a plan and its lazily created workspace and wrote into a caller-owned output. DUCC used `analysis_2d` with a caller-owned output; that API does not expose an equivalent reusable plan. The timings are full calls, not isolated Legendre phases. Python reports the median of five calls; calls were measured Rust then DUCC, not interleaved.

| Threads | RustSHT (ms) | DUCC (ms) | Ratio | Relative L2 error |
| ---: | ---: | ---: | ---: | ---: |
| q1 | 41.815 | 19.294 | 2.17× | 6.039e-14 |
| q8 | 6.827 | 3.212 | 2.13× | 6.039e-14 |

The comparison used the installed DUCC wheel; its compiler flags were not independently verified in this screen. No host-pressure capture was made, so small differences should not be interpreted as stable. This full-call ratio includes different setup boundaries and does not identify the remaining kernel-stage gap.

Validation after the source change: `cargo fmt --all -- --check`; focused all-features and no-default-features analysis tests; high-order initializer test; workspace Clippy with warnings denied; pinned-DUCC spin analysis test across GL/CC/F1. The full release workspace suite was not rerun for this increment.

## Next implementation task

The shared SIMD initializer gives a modest q1 gain, while current full-call analysis remains about 2.1× DUCC. Compare the scalar `FusedScalarAnalysisPair` degree loop with DUCC `calc_map2alm`/`map2alm_kernel`, then test a four-degree SIMD accumulation step that reuses latitude input loads across two recurrence pairs. Keep `A`, `S*` and `A*` semantics distinct and retain the scaled fallback.
