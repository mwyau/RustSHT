# B0 native-f32 scaled-prefix fast path — 2026-10-31

**Disposition:** promote the shared generic scaled-prefix skip and remove the dormant f32-to-f64 scalar-synthesis sidecar. The native f32 path now beats pinned DUCC f32 in all four GL T720 cells. It still runs 1.03–1.18× matching Rust f64, so the final f32/f64 parity target remains open, especially on full-order grids.

## Source and execution

The isolated parent was commit `179835a559f00dd2add6721b22f579822849bfce` (`feat: enable native f32 scalar synthesis kernel`). The scaled-prefix core patch measured in the 15-pair screen is SHA-256 `0718cdfc5e415870ede45886cef2fa8687aeb93696001753f46f06c2089c7207`. After that screen, the final patch also removed the unreachable child-plan, promotion, narrowing and conversion-buffer code; the complete final patch is `patches/final-source.patch`, SHA-256 `a243bc1e1b2ab741897204e903119746f9c08bdce74a83dac799ff5a52498b1c`.

Before the change, native f32 declined the optimized skip callback and processed each scaled-prefix degree through the full per-pair rescale-and-accumulate path. The f64 order-major path already advanced two degree pairs with fused SIMD recurrence while their scale epoch remained below one and omitted contributions beneath its precision-specific accumulation threshold. The candidate generalizes that control flow over `T` and the SIMD type, uses native f32 scaling and `vmul_add`, and enables the same skip with f32's own threshold and exponent increments. AVX2 dispatch used eight f32 lanes. No runtime promotion occurred. The f64 output stayed bitwise identical to the parent in the measured T720 cells.

The exact run data is in `results/`. The 15-pair screen used the unchanged native production route at source parent `179835` and the three-file core patch. The final source, after sidecar cleanup and workspace-layout changes, was rebuilt and confirmed in three interleaved pairs per T720 cell. Its outputs have the same reported error metrics as the core candidate. Do not conflate the final three-pair confirmation with the 15-pair confidence intervals.

Rust: 1.99.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, parallel feature, AMD Ryzen 9 5950X. T720 workers pinned q1 to the first allowed CPU and q8 to the first eight allowed CPUs. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; f32 and f64 DUCC binaries and every fixture are hashed in the JSON. Workers time warm whole transforms, including recurrence, scheduling and FFT. The machine had shared-host CPU activity during the runs; per-worker pressure and affinity telemetry remain in the raw result JSON.

## Matched GL T720 results

Times are whole-transform medians in milliseconds over 15 pairs. Ratio intervals are 50,000 paired bootstrap resamples of within-pair ratios. `parent f32` is the current native route at the isolated parent; `Rust f64` and both DUCC precisions use matching fixtures and geometry.

| Cell | Rust f32 candidate | Parent f32 | Rust f64 | DUCC f32 | DUCC f64 | f32/f64 [95%] | f32/DUCC32 [95%] | f64/DUCC64 [95%] | f32/parent [95%] |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 19.086 | 26.512 | 16.416 | 20.463 | 20.619 | 1.149 [1.132, 1.168] GAP | 0.927 [0.883, 0.939] PASS | 0.786 [0.778, 0.809] PASS | 0.721 [0.691, 0.734] |
| Full q8 | 2.845 | 4.056 | 2.454 | 2.978 | 3.007 | 1.179 [1.140, 1.213] GAP | 0.959 [0.912, 0.995] PASS | 0.792 [0.776, 0.831] PASS | 0.701 [0.673, 0.727] |
| Reduced q1 | 6.592 | 9.168 | 6.352 | 8.594 | 8.627 | 1.029 [1.023, 1.054] GAP | 0.768 [0.755, 0.780] PASS | 0.741 [0.719, 0.751] PASS | 0.730 [0.706, 0.733] |
| Reduced q8 | 1.139 | 1.527 | 1.076 | 1.334 | 1.347 | 1.044 [1.023, 1.128] GAP | 0.824 [0.794, 0.894] PASS | 0.783 [0.764, 0.810] PASS | 0.723 [0.694, 0.778] |

The f32 path cuts the current native parent by 26.3–29.9%. Each f32/DUCC f32 interval is below 1.0. The f32/Rust f64 interval remains above 1.0 in every cell, so B0 has not reached full cross-precision parity. The candidate f64/parent ratios did not show a consistent regression; their paired intervals are inconclusive in three cells and below 1 in reduced q8.

The final cleaned-source three-pair confirmation is separately recorded under `results/t720-final-source-3pair/`. Its f32/DUCC ratios were 0.940 full q1, 0.945 full q8, 0.786 reduced q1 and 0.793 reduced q8. All candidate outputs were finite, repeat-consistent and reset-consistent. The final source dispatched V3/AVX2 with eight f32 lanes and used native f32 coefficients, spectra, FFT and map output.

The deterministic random f32 maps changed relative to the pre-screen parent: candidate-versus-parent relative L2 was `1.7448e-4` full and `1.6811e-4` reduced; candidate-versus-DUCC f32 was `1.8028e-4` full and `1.9160e-4` reduced. Existing analytic, high-degree, range-edge, grid, adjoint, SIMD-tail, workspace-reset and independent DUCC regression tests pass. No tolerance was changed. These fixture differences are recorded, not used to assert a new numerical tolerance.

## Protected f64 T1023 spin-1 synthesis

The final cleaned source was compared with its parent and pinned DUCC f64 on GL `T1023/M1023`, q1 and q8, using complete E/B transforms and matching deterministic fixtures. Q1 used three pairs: candidate/parent median `0.999`, with all three outputs bitwise equal. Q8 used 15 pairs: candidate/parent median `0.987`, bootstrap 95% interval `[0.974, 1.072]`, and candidate/DUCC median `0.775`. Every parent/candidate output pair was bitwise equal, and repeat/reset and finite checks passed. The q8 control is **INCONCLUSIVE** because its interval crosses 1; it establishes no f64 regression. Full pairs, fixture hashes, telemetry and binary provenance are in `results/t1023/`.

## Validation

On the final cleaned candidate:

- `cargo fmt --all` and `cargo fmt --all -- --check` passed.
- Strict all-target Clippy with the parallel feature passed: `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo clippy --features parallel --all-targets -- -D warnings`.
- `cargo test --release --features parallel`: 325 unit tests passed, 38 ignored; 4 extreme-range integration tests, 14 f32 integration tests, and 1 workspace-compatibility test passed. One opt-in T1023 f32 reference test remains ignored as before. One initial run transiently failed the parallel worker-count assertion; the focused test and repeated full suite passed.
- Python tests: `uv run --group dev pytest python/tests`, 20 passed; `uv run --project python/pyspharm-rs --group dev pytest python/pyspharm-rs/tests`, 7 passed and 1 skipped.
- `cargo check --no-default-features --lib` still fails at the pre-existing unconditional re-export of a `parallel`-gated function in `src/legendre/scalar_paired.rs:924`; this source is unchanged by this B0 patch.

## Reproduction and artifacts

`patches/scaled-prefix-core.patch` reproduces the 15-pair kernel candidate on the source parent. `patches/final-source.patch` reproduces the promoted source including sidecar removal. The final-source T720 confirmation runner and the modified T1023 screen runners are in `runners/`. The original 15-pair T720 runner is preserved at [`m6-b0-native-scalar-synthesis-2026-10-31/results/run_t720_pairs.py`](../m6-b0-native-scalar-synthesis-2026-10-31/results/run_t720_pairs.py). `SHA256SUMS.txt` covers this package. The original sidecar route and its numerical caveats remain in the earlier B0 package.

**NEXT:** B0 reaches DUCC f32 parity but not Rust f32/f64 parity on full-order cells. Record this bounded improvement, then resume B1's scaled recurrence and exceptional-lane overhead from a new measured bottleneck; retain B3 fusion and do not repeat rejected B1/B2 variants.
