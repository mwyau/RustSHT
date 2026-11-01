# B1 normal-suffix accumulation screen

## Decision

Reject the f32 suffix bypass. It removes power-of-two factor multiplication for a long ordinary tail, but the direct microbenchmark was slower in both representative cases. No production code changed and no whole-transform candidate was run.

## Provenance and workload

- Rust base: `252a839c038fb93a68d5ae40a9166b728b0f2ce0` (`dev` and `origin/dev`). The f32 B1 kernel is the implementation introduced by `80959a30`; the intervening `252a839` change is the f64 seed initializer.
- DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- CPU: AMD Ryzen 9 5950X. The microbenchmark was pinned to logical CPU 0. Runtime SIMD dispatch used V3/AVX2/FMA; the f32 lane width was eight. `perf stat` and `perf record` are unavailable (`perf_event_paranoid=4`, no PMU access).
- Candidate: `252a839` plus [`candidate.patch.gz`](candidate.patch.gz). Decompressed patch SHA-256: `02706adaf65aa828a685f19583b5434953f1b8bc34e15fc8210c81a6f98e45f8`. The patch includes the test-only probe and temporary phase/route counters; it was restored after the screen.
- Microbenchmark binary SHA-256: `56c3f31a8c3b1bf6f531bcc9ace4aa69715959ebb531743273383915257c5dde`.

The probe uses real GL T720 latitude geometry and recurrence states, 256-lane coefficient-pair tiles, nonzero deterministic spectra, 2,000 calls per sample and nine samples. For `m=8,l=8` at ring 0, the scaled prefix has 8 lanes and the ordinary suffix 248. For `m=192,l=720` at ring 29, the scaled prefix has 216 lanes, the ordinary suffix 40, and 11 vector groups need hard-range recovery. The production candidate activated only when the suffix covered at least half the tile.

## Measured B1 distribution

Current-source test-only phase probes at `252a839` put the full-order f32 Legendre recurrence plus accumulation at 36.6–36.9 ms of 39.2–39.5 ms (about 93–94%); its FFT took 2.57–2.61 ms. For f64, the Legendre stage took 13.21–13.33 ms of 16.1–16.5 ms (about 80–83%); its FFT took 3.14–3.18 ms. The full-order f32 q8 probe also puts most time in Legendre work. These are warm phase timings, not hardware cycle attribution. Reduced-order phase decomposition was not captured at `252a839`; whole-call ratios for all current cells are in the live ledger.

The Rust source crosswalk is `ShtPlan::analysis_into` and `analyze_map_to_coefficients` in `src/transform/scalar.rs:1103,1271`, then `analyze_symmetric_order_group` at line 2119. The typed B1 loop is `fused_scalar_analysis_pair_block_preweighted_typed` and `FusedScalarAnalysisPairTyped::with_simd` in `src/legendre/scalar_paired/analysis.rs:1231,1466`; factor and hard-lane preparation is `fill_b1_f32_analysis_factors` at line 433. Pinned DUCC enters `alm2leg` in `src/ducc0/sht/sht.cc` and runs `inner_loop_m2a` / `map2alm_kernel` in `src/ducc0/sht/sht_inner_loop.h` (revision above).

The exact f32 implementation at `80959a30` (unchanged in the f32 monomorphization at `252a839`) has separate Callgrind gates on the same full-q1 map, SHA-256 `d282e16a4680897b36f95290cf2770608319a248de24b497ef20bc6de4e57795`:

| Region gate | Instructions (Ir) | Scope |
|---|---:|---|
| Seed setup | 34.0M | Sectoral initialization |
| Ordinary two-pair body | 153.4M | Includes accumulation and recurrence advancement |
| Scaled accumulation | 261.0M | Scaled coefficient accumulation |
| Recurrence advancement | 22.3M | Separate recurrence gate |
| Range normalization | 43.9M | Exponent/state normalization |

These are separate instrumented region counts, not time shares; the ordinary two-pair body includes recurrence and accumulation, so the rows are not additive. Callgrind does not provide CPU cycles. The current whole-call matrix is 2.842, 2.654, 2.424 and 2.181x DUCC f32 for full q1/q8 and reduced q1/q8. f64 is 1.136, 1.132, 1.142 and 1.111x DUCC f64. Exact 15-pair times and intervals remain in [`reports/DUCC_PARITY_M6.md`](../../../DUCC_PARITY_M6.md).

## Code generation and hypothesis

The Rust V3 ordinary loop uses YMM f32 vectors, four channel accumulators and `vfmadd231ps`; the two-pair traversal reuses each latitude vector for adjacent degrees. The f64 specialization uses four-lane YMM `vfmadd231pd`. The Rust f32 V3 body is 22,532 bytes and the f64 body is 7,236 bytes. Pinned DUCC's `map2alm_kernel` is a 722-byte AVX2 double-precision loop; DUCC's public f32 analysis builds double-precision Legendre work arrays and accumulates into `complex<double>`. Rust's larger kernel contains the ordinary, scaled and exceptional routes; size alone is not evidence of instruction-cache cost, and an earlier cold-path outlining change slowed whole calls.

The strongest current f32 hypothesis is the cost of scaled accumulation and state normalization inside the Legendre stage. A warm 256-lane microbenchmark measured the ordinary accumulator at 23.56 ns per pair and the scaled `m=192,l=720` accumulator at 177.88 ns (7.55x). This supports a large per-call cost difference between ordinary and scaled work, but does not establish how much of the full-transform gap it explains. The f64 residual gap is smaller; its SIMD seed initializer already improved all four cells by 8.3–10.6% against its exact parent.

## Suffix candidate result

The proposed path accumulates a contiguous zero-exponent suffix directly with ordinary f32 FMA, then adds that partial sum to the scaled-prefix result. With CPU 0 pinned:

| Case | Existing scaled accumulator | Split suffix replay | Split / existing | Result difference |
|---|---:|---:|---:|---:|
| `m=8,l=8`, 8 scaled + 248 ordinary lanes | 39.96 ns | 42.54 ns | 1.064x | bitwise equal |
| `m=192,l=720`, 216 scaled + 40 ordinary lanes | 176.53 ns | 182.51 ns | 1.034x | relative `5.07e-8` |

The high-order suffix is too short for the production threshold and the low-order case is slower. The candidate therefore fails its isolated screen. No whole-call timing or numerical promotion test was run. The sanitized sample summary and exact test-only patch are retained here; the raw console log remains in local experiment scratch.

## NEXT

Keep production unchanged. The phase profile says B1 time is in Legendre work, while the f32 scaled accumulator is much slower per 256-lane pair than the ordinary path. Next isolate factor preparation, hard-lane recovery, accumulation and normalization with a current-source pinned microbenchmark; use those results to form a materially different candidate. For f64, isolate its remaining ordinary-loop cost separately. Hardware cycle attribution remains open on this host.
