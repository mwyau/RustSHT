# Independent native-f32 accuracy screen for scalar analysis and spin-1 (2026-10-30)

## Scope and provenance

This diagnostic extends the saved scalar-synthesis causality trace with independent end-to-end references for the current scalar-analysis, spin-1-analysis, and spin-1-synthesis paths. Rust source is `b849b7b321b85c96263b6fe122f603584f22a847`; the pinned DUCC source identity is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The new comparison is against an independently coded binary128 reference, not DUCC output. No tolerance was supplied, so the measurements below do not assign an accuracy PASS or FAIL.

The fixture is GL T720 (`lmax=720`, `ntheta=721`), `mmax=16`, `nphi=34`, `phi0=0`. Geometry is emitted as raw f64 bits. Scalar-analysis and spin-analysis inputs are deterministic f32 maps. Spin-synthesis inputs have only `m=0..16`, `l=600..720` nonzero. The Rust build uses `--release --features parallel -C target-cpu=x86-64-v3`, with q1 and q8. The host is an AMD Ryzen 9 5950X; Rust is `1.99.1 (58cf8f9c3 2026-10-13)`.

The C++ reference evaluates normalized scalar harmonics in binary128, forms direct Fourier sums, and independently applies the spin-1 E/B conversion and its transpose. Rust f64 agrees with this reference to relative L2 `6.45e-14` for scalar analysis, `6.49e-14` / `6.60e-14` for spin-analysis E/B, and `6.68e-13` / `6.75e-13` for spin-synthesis Q/U. This cross-check supports the reference on this fixture; it does not establish an application accuracy contract.

## Results

Relative L2 is measured against the binary128 output. High-band rows select output degrees `l=600..720`.

| Operation | q1 relative L2 | q8 relative L2 | High-band relative L2 | Maximum f32 absolute error |
|---|---:|---:|---:|---:|
| Scalar analysis | 3.655895e-5 | 3.655895e-5 | 6.026887e-5 | 4.088139e-6 at `m=0,l=705` |
| Spin-1 analysis E | 3.815288e-5 | 3.815288e-5 | 5.876293e-5 | 2.889098e-6 at `m=0,l=712` |
| Spin-1 analysis B | 3.203094e-5 | 3.203094e-5 | 5.315537e-5 | 3.067824e-6 at `m=0,l=714` |
| Spin-1 synthesis Q | 3.461760e-4 | 3.030359e-4 | — | 0.104124 at ring 0, longitude 32 |
| Spin-1 synthesis U | 3.579517e-4 | 3.078474e-4 | — | 0.0857291 at ring 0, longitude 28 |

The analysis FFT is not a material contributor on this fixture. Replacing the actual f32 Fourier input with its direct Fourier reference changes scalar-analysis relative L2 from `3.655895e-5` to `3.655806e-5`, spin-analysis E from `3.815288e-5` to `3.815235e-5`, and B from `3.203094e-5` to `3.203014e-5`. The forward-FFT relative residual is about `1.09e-7`.

Spin-synthesis error is present in the spectrum before the inverse FFT: q1 Q/U spectrum relative L2 is `3.470281e-4` / `3.619901e-4`; q8 is `3.042871e-4` / `3.122808e-4`. The inverse-FFT residual is about `1.15e-7` to `1.18e-7`. On this high-band input, recurrence, channel preparation, or their accumulation order dominate the map error; the FFT does not explain it.

The spin-synthesis f32 results depend on q1/q8 dispatch. The captured q1 workspace has capacity 1, ring group size 768, and no cached scalar coefficients. The q8 workspace has capacity 8, ring group size 64, and 24,548 cached scalar coefficients across 17 orders. At `mmax=16`, the order-major queue eligibility condition is not met; q8 uses grouped parallel ring work while q1 takes the single-thread reflected all-ring route. Their Fourier spectra already differ before FFT: 21,612 of 25,956 Q-spectrum f32 components differ, with max absolute difference `9.13477e-3`; the corresponding U counts are 21,677 and `1.00074e-2`. The map outputs differ in 22,644/24,514 Q samples and 22,592/24,514 U samples; q1/q8 RMS absolute differences are `1.9813e-3` / `2.1104e-3`. Q1/q8 scalar-analysis and spin-analysis coefficient outputs are bitwise identical.

The q1/q8 spin-synthesis channel preparation is bitwise equal for the captured `m=16` order: each converted scalar channel has 722 complex values, and each of the four prepared coefficient-pair arrays has 353 complex values. Their SHA-256 hashes match across q1 and q8 for all six arrays (listed in the JSON and fixture-hash file). This is a post-call workspace capture, so it establishes equality of the values produced by preparation on this fixture, not a general invariant for all inputs. It rules out different converted channels and prepared pair coefficients as the cause of this sampled q1/q8 output gap. The first arithmetic divergence is therefore inside the dispatched recurrence/summation routes, or in how they initialize and reconstruct ring values.

## Isolated timing-candidate overlays

The B1 tile32 and B3 range80 patches were applied only in their source-matched diagnostic worktrees. Their raw candidate f32 outputs were compared with the same binary128 reference rows. These are numerical observations, not performance or acceptance claims.

- **B1 scalar-analysis tile32:** q1 and q8 all-mode relative L2 is `3.655776e-5`; high-band relative L2 is `6.026787e-5`; maximum absolute error is `4.087207e-6`. This is effectively the baseline error on this fixture. Parent source `797d2720cfb17cd443c1ccdf835063b2433006be`; uncompressed patch SHA-256 `471f15a6f57a5d47ff9569dcc554def7a6b5886ccc6d157374e4b58df8b240ef`.
- **B3 spin-analysis range80:** all 15 emitted f32 outputs are bitwise identical to the current-source outputs for q1 and q8 on this fixture. Parent source `9cb2788cd3f18c0f262e24dff6fac58236e5a18d`; compressed patch SHA-256 `623b905c5604bee5488db4b6775dd219fe776aabbe5d0f92b621964fd26cb1d6`.

These two bounded patches do not change the conclusion that B1 and B3 remain timing GAPs. They also do not establish broader numerical equivalence outside the recorded fixture.

## Source crosswalk

The Rust dataflow exercised is:

- Scalar analysis: `ShtPlan::analysis_into` (`transform/scalar.rs:1254`) → `analyze_map_to_coefficients` (`:1422`) → forward longitude FFT → `analyze_order_group` / `analyze_symmetric_order_group` (`:2212`, `:2270`) → `fused_scalar_analysis_pair_channels_block_native<T: FusedAnalysisPrecision>` (`legendre/scalar_paired/analysis.rs:4954`).
- Spin analysis: `ShtPlan::analysis_spin_into` (`transform/spin.rs:292`) → `map_to_spin_coefficients` (`:356`) and Fourier rows → `accumulate_spin_scalar_analysis_pair_block` (`transform/spin/analysis.rs:406`) and paired spin/scalar accumulation into E/B coefficients.
- Spin synthesis outer routing: `synthesis_spin_core` (`transform/spin/synthesis.rs:38`) → cached (`:199`, `:521`) or local (`:1199`) spin-1-to-scalar preparation (`legendre/spin_fast.rs:23`) → grouped ring execution (`synthesize_spin_ring_group_orders`, `:781`) or order-major queue (`synthesize_spin1_order_major_queue`, `:608`) → f32 spectra → inverse longitude FFT (`:493`, `:502`).
- The captured q1 GL T720, `mmax=16` path satisfies the single-thread reflected all-ring predicate in `transform/spin/synthesis.rs:1374`. It enters `try_synthesize_spin1_prepared_component_order_to_strided_spectra` (`:2362`), which runs the first and second prepared scalar components separately (`:2501`, `:2539`). `synthesize_spin1_prepared_scalar_component_batch` (`transform/spin.rs:894`) adapts each call to `ScalarOrderedScratch`, then the f32 `ShtScalar` implementation dispatches to `scalar_ordered::synthesize_prepared_component_batch_dispatched` (`legendre/mod.rs:2148`; kernel entry at `legendre/scalar_ordered/kernel.rs:202`). Thus the reflected order-major recurrence and accumulators are executed twice, once per component.
- The captured q8 path is not eligible for the order-major queue: only 3 order tiles are available at 17 orders, below the q8 threshold of 16. It uses 64-ring groups with cached scalar channels. `synthesize_spin1_scalar_block_to_order` (`transform/spin/synthesis.rs:1797`) prepares both coefficient channels and calls `synthesize_prepared_scalar_pair_block_dispatched` (`:1955`), which reaches the f32 implementation in `legendre/mod.rs:2409` and `scalar_paired::synthesize_prepared_order_pair_block_dispatched` (`legendre/scalar_paired/synthesis.rs:1014`). That initializes and normalizes a paired sectoral state (`:1060–1079`), then dispatches the fused paired-channel SIMD recurrence/accumulation (`FusedPreparedSynthesisPairBlock`, `:1090`, implementation at `:2296`). It computes both spin channels together in one pair loop.

The paired q1/q8 capture places the first algorithmic split after converted channels and coefficient-pair preparation. The kernel code has different initialization and summation structures, so matching prepared inputs alone does not yet identify the first different f32 result. The next trace should compare the same north ring, order, and degree on both routes: sectoral seed; normalized previous/current state; `a*cos²+b`; recurrence next-state; each channel's accumulation; and reflected Fourier reconstruction.

For line-numbered DUCC scalar-recurrence, preparation, FMA and extraction crosswalks, use the saved [DUCC 0.42.0 causality report](../numerical-causality-ducc-0.42.0-2026-10-29/REPORT.md) and [source crosswalk](../numerical-causality-ducc-0.42.0-2026-10-29/SOURCE_CROSSWALK.md). This follow-up does not instrument or modify DUCC.

## Reproduction and files

`phasec_accuracy_dump.rs` regenerates maps, coefficients, geometry bits and Rust f32/f64 outputs. `phasec-spectrum-capture.patch` adds an ignored, test-only observer for pre-FFT spectra, selected spin-synthesis dispatch, and the `m=16` converted channels and prepared coefficient pairs. `phasec_accuracy_reference.cpp` performs the independent binary128 comparison. The two Python scripts summarize the baseline and overlay isolated candidates. `phasec-cross-family-results.json` contains machine-readable metrics; `phasec-fixture-sha256.txt` contains fixture and key-output hashes. Raw per-row CSVs are intentionally not included; the deterministic harness regenerates them.

Build and run from a clean worktree at the stated Rust source SHA after applying the test-only patch and copying the example into `examples/`:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=1 \
  cargo run --release --features parallel --example phasec_accuracy_dump -- diagnostics/q1
PHASEC_SPECTRA_DIR=diagnostics/q1 RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=1 \
  cargo test --release --features parallel --lib phasec_capture_native_f32_spectra -- --ignored --nocapture
RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=8 \
  cargo run --release --features parallel --example phasec_accuracy_dump -- diagnostics/q8
PHASEC_SPECTRA_DIR=diagnostics/q8 RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=8 \
  cargo test --release --features parallel --lib phasec_capture_native_f32_spectra -- --ignored --nocapture
c++ -std=c++20 -O2 -march=x86-64-v3 phasec_accuracy_reference.cpp -lquadmath -o diagnostics/phasec_accuracy_reference
diagnostics/phasec_accuracy_reference diagnostics/q1
diagnostics/phasec_accuracy_reference diagnostics/q8
PHASEC_SOURCE_SHA=b849b7b321b85c96263b6fe122f603584f22a847 \
  python3 summarize_phasec_accuracy.py diagnostics/q1
```

The candidate overlay script takes a candidate output directory and a reference directory produced above. Candidate patch sources and exact build runners remain in the B1 and B3 experiment packages linked from the active M6 ledger.

## Decision and NEXT

This screen confirms substantial f32-versus-binary128 differences in all three families and localizes the spin-synthesis difference before FFT. It shows equal q1/q8 `m=16` converted channels and prepared coefficient-pair arrays, then distinct q1 reflected scalar-ordered and q8 fused paired-channel kernels. It does not yet identify their first differing arithmetic operation or establish an acceptable scientific tolerance. Continue Phase C with a same-ring, same-order recurrence trace across those kernels, then extend independent high-degree and exceptional/adjoint references. Do not treat the timing candidates as numerically accepted or select a compensation based on this aggregate alone.
