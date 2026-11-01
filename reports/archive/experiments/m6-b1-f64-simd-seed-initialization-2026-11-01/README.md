# B1 f64 SIMD sectoral seed initialization

## Finding

On GL T720 scalar analysis, f64 spent most of the isolated sectoral-init region computing each latitude seed through the scalar `scaled_power` routine, then used the fused SIMD recurrence. The new f64 path first tries the existing SIMD normal-range initializer. If any lane cannot be represented normally, it declines before recurrence and retains the existing scaled fallback. No recurrence or accumulation arithmetic changes.

Callgrind on the exact `80959a30` parent measured 36,249,111 instructions for region 1; 29,084,158 (80.23%) were in 195,204 `scalar::scaled_power` calls. The candidate region measured 1,800,451 instructions, including 1,639,114 in `initialize_normal_analysis_sectoral_dispatched`: 95.03% fewer instructions in this isolated region. These are Callgrind instruction counts, not cycles and not additive with whole-call timings.

`perf stat`/`perf record` are unavailable on this host (`perf_event_paranoid=4`, no accessible hardware events). The isolated microbenchmark below provides wall-time evidence for seed setup; the interleaved whole-transform results include FFT, recurrence, accumulation, scheduling and scratch work.

The exact-parent f32 region probes at GL T720 full q1 (map SHA-256 `d282e16a4680897b36f95290cf2770608319a248de24b497ef20bc6de4e57795`) separately recorded 34.0M Ir for scalar seed initialization, 153.4M for the ordinary two-pair body, 261.0M for scaled accumulation, 22.3M for recurrence advancement, and 43.9M for range normalization. Each number comes from a separate Callgrind gate around that code region; the two-pair body includes both accumulation and recurrence. They are instruction counts, not cycle shares, and must not be summed as a whole-call distribution. The current f32 V3 kernel body is byte-identical to this parent; this candidate changes only the f64 initializer.

## Focused source crosswalk

| Stage | RustSHT | Pinned DUCC 0.42.0 |
|---|---|---|
| Public analysis and longitude FFT | `ShtPlan::analysis_into` and `analyze_map_to_coefficients`, [`src/transform/scalar.rs`](../../../../src/transform/scalar.rs#L1103) lines 1103, 1271, 1307–1369 | Pinned DUCC `sht.cc` (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`), `analysis_2d<T>` builds `leg`, calls `map2leg`, then `leg2alm_internal`, lines 1729–1799 |
| Order/ring preparation | `analyze_symmetric_order_group` traverses orders and reflected ring pairs, prepares weighted same/opposite channels, and dispatches the typed block; [`src/transform/scalar.rs`](../../../../src/transform/scalar.rs#L2119) lines 2119–2290 | Pinned DUCC `sht_inner_loop.h` (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`): `fill_m2a` packs ring geometry, phases and weights then calls `init_lambda`; `inner_loop_m2a` chunks rings (`nv0 * VLEN = 256` doubles on AVX2) and advances degree blocks, lines 1660–1699, 1752–1795 |
| Coefficients and sectoral seed | Rust prepares recurrence factors per order, then `FusedAnalysisPrecision for f64` now tries `initialize_normal_analysis_sectoral_dispatched` before scalar fallback; [`analysis.rs`](../../../../src/legendre/scalar_paired/analysis.rs#L965) lines 965–1007, 2064–2128 | Pinned DUCC `sht_inner_loop.h` at the same source revision: `YlmBase` stores double `mfac`; `Ylmgen::prepare` fills double `eps`, `alpha`, and `(a,b)`; `init_lambda` uses `mypow` and scale normalization, lines 100–122, 186–205, 413–424 |
| Recurrence and accumulation | `fused_scalar_analysis_pair_block_preweighted_typed` and `FusedScalarAnalysisPairTyped::with_simd`; AVX2 f64 has four lanes, f32 eight. The ordinary loop uses fused operations and accumulates the paired degrees; [`analysis.rs`](../../../../src/legendre/scalar_paired/analysis.rs#L1231) lines 1231–1318, 1389–1464, 1486 onward | Pinned DUCC `sht_inner_loop.h`: `Tv = native_simd<double>` and `VLEN = 4`; AVX2 horizontal reduction is `vhsum_cmplx_special`; `map2alm_kernel` advances two recurrence pairs and accumulates their coefficients; `calc_map2alm` handles scale correction/handoff, lines 252–275, 592–684 |
| f32 execution difference | Rust f32 remains native f32, including recurrence, accumulation and eight-lane AVX2. Its seed dispatch is compile-time false in this candidate; its hot V3 function bytes match the parent. | The public templated map/FFT input is f32, but `leg2alm_internal` accumulates in `vmav<complex<double>>`, and `Tv` is `native_simd<double>`; output narrows only when writing `complex<T>` at `sht.cc` lines 925–945 and 943. DUCC f32 therefore executes the Legendre stage in double. |

## Microbenchmark

The test-only probe uses the actual GL T720 grid, its 105 northern latitudes at rings 256–360, four representative orders, and 8,000 initializations per paired sample. It compares the exact old `initialize_pair_state` route with the candidate's `try_initialize_analysis_pair_state` route. Seven paired samples were run on pinned CPU 0; both routes produced the same normal-range seed state.

| Order m | Scalar setup median | SIMD setup median | SIMD / scalar |
|---:|---:|---:|---:|
| 1 | 384.5 ns | 60.2 ns | 0.156 |
| 180 | 989.8 ns | 132.8 ns | 0.134 |
| 360 | 1,053.8 ns | 141.4 ns | 0.134 |
| 720 | 1,116.2 ns | 155.3 ns | 0.139 |

The AVX2 disassembly contains four-lane `vbroadcastsd`, `vmovupd %ymm`, and `vmulpd %ymm` operations in the dispatched initializer. The runtime worker records Pulp V3; the V4/AVX-512 specialization is not executed on this Ryzen 9 5950X. The f64 V3 recurrence/accumulation body remains 7,236 bytes. The f32 V3 body remains 22,532 bytes and hashes byte-for-byte to the exact parent (`42a975b2067583be5a139ec9b34d7e4c15eafdc2e935c0244946dedbffc77d17`).

## Matched GL T720 whole calls

Fifteen interleaved pairs per cell; warm complete transforms; Rust `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, runtime V3 AVX2/FMA; pinned DUCC sources at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` and a non-fast-math benchmark driver. q1 pins CPU 0; q8 pins physical CPUs 0–7 on one L3. Ratios use paired geometric means and 95% t intervals on log ratios.

| Cell | Rust f32 parent → candidate / DUCC f32 (ms) | f32 / DUCC f32 (95% CI) | Rust f64 parent → candidate / DUCC f64 (ms) | f64 / DUCC f64 (95% CI) | Candidate / parent (95% CI), f64 |
|---|---:|---:|---:|---:|---:|
| Full q1 | 40.148 → 39.763 / 14.114 | 2.842 [2.815, 2.870] GAP | 18.311 → 16.311 / 14.579 | 1.136 [1.100, 1.173] GAP | 0.899 [0.871, 0.929] PASS |
| Full q8 | 5.410 → 5.463 / 2.050 | 2.654 [2.611, 2.698] GAP | 2.696 → 2.412 / 2.133 | 1.132 [1.115, 1.149] GAP | 0.895 [0.885, 0.905] PASS |
| Reduced q1 | 13.668 → 14.164 / 5.766 | 2.424 [2.380, 2.468] GAP | 7.307 → 6.718 / 5.906 | 1.142 [1.127, 1.158] GAP | 0.917 [0.906, 0.929] PASS |
| Reduced q8 | 1.971 → 1.956 / 0.888 | 2.181 [2.146, 2.216] GAP | 1.154 → 1.036 / 0.921 | 1.111 [1.085, 1.137] GAP | 0.906 [0.894, 0.919] PASS |

All f64 candidate outputs and all f32 candidate outputs were bitwise equal to the exact parent. The f32 candidate/parent intervals include 1.0 in all four cells (0.983–1.017, 0.999–1.015, 1.000–1.035, 0.956–1.012): this change leaves f32 timing statistically unresolved and does not improve it. f32 remains 2.18–2.84× DUCC; f64 now remains 1.11–1.14× DUCC. No precision-matched parity cell passes yet.

The protected f64 T1023 spin-1 synthesis q1/q8 check used three pairs per cell and bitwise-equal outputs. Candidate/parent was 0.998 [0.982, 1.016] q1 and 1.000 [0.828, 1.333] q8; both are INCONCLUSIVE at three pairs, with no regression established.

## Provenance and reproduction

- Parent source: `80959a30d6965a6b0765aabbab7a37d16be402d2`.
- Screened production patch: `evidence/production-candidate-measured.patch.gz` (decompressed SHA-256 `6f68e74d4746db18b13c88dc192625e5e6bd8274ad3943581735c0f69dd5aa50`); screened `analysis.rs` SHA-256 `de61c62cd5e52c1f83fab427ee0b1158c3050c131008b83f6e4b6267f196e1f1`.
- Final source adds only a Clippy `#[expect]` annotation after the screen: current `analysis.rs` SHA-256 `a23b7e73a806adcaa2a51b1758b1d8c56ff6d999ea83731dd06dddb3de429d4f`. The f64 candidate `.text` is identical to the screened binary (`dbbecfcd0189f82d2f38ce004ce37c0d7ba0c788ccceee90d60f18f156fbb822`); the f32 hot V3 body is byte-identical to the parent.
- Screened Rust worker hashes: f64 parent `edd9a4d0dd7d4a436c47e623fead55ae2dd051de7aee90df735051643a8087f2`, candidate `c87d65b7322efab3fe67b999aea1c15fef6c1acbb3000fab925bdb4e2d1cac60`; f32 parent `ee7f9bee5c6a871292d6f9d91c0306cf614cc80deab48cb31da072630c2e7ca0`, candidate `c6a3499e81daa63adc7e340d74e2b13db86e7fbf0ef979390ff71e7aa178fcaa`.
- Pinned DUCC worker hashes: f64 `7a698915281354f62fde7afa0fc2ccec66201a33a80f6e8f08170fdd7818cc11`; f32 `a191b94696361fbbb2ca665715870024243c5fbc9577f6a13cccfe0206be18c8`.
- Fixture hashes and every paired sample are preserved in `evidence/f64-15pair-results.json.gz` and `evidence/f32-15pair-results.json.gz`. T1023 fixture/output hashes and source/binary metadata are in `evidence/t1023-*.json`.
- Rebuild the source patch on the parent with `gzip -dc evidence/production-candidate-measured.patch.gz | git apply -`. The focused seed microbenchmark source and valid module patch are in `evidence/seed-init-microbenchmark.rs` and `evidence/seed-init-microbenchmark-module.patch.gz`; decompress the module patch before applying it, then run `cargo test --release --features parallel --lib b1_f64_sectoral_seed_initialization_microbenchmark -- --ignored --nocapture`. `evidence/callgrind-instrumentation.patch.gz`, the Callgrind harness and shim reproduce the isolated instruction regions; they remain test-only.
- Callgrind raw profiles, compressed disassembly, microbenchmark transcript, test-only patches and exact results have SHA-256 entries in [`SHA256SUMS`](SHA256SUMS). Full binaries and map fixtures are not duplicated.

## NEXT

Keep this f64 seed initializer. Profile current f32 exceptional/scaled accumulation and normalization with wall-time microbenchmarks against its exact matched DUCC worker; do not assume the f64 seed result explains the remaining f32 deficit. For f64, inspect the next largest typed ordinary-loop cost and scheduling only after measuring it separately. Do not claim DUCC parity from this improvement.
