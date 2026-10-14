# Two-way SIMD accumulation for paired analysis

## Change

The f64 normal-range `AnalysisPairAccumulation` previously processed one latitude vector at a time into four YMM reduction chains. The AVX2 loop covered 16 vectors in a 64-lane block, with each partial sum depending on the previous loop iteration. It now processes two vectors per iteration into two independent partial sums for each complex component, then combines the partial sums before the horizontal reductions. This shortens each dependency chain from 16 vector updates to 8 without changing recurrence, Fourier preparation, cutoff logic or the scaled fallback. The Pulp V3 assembly on the x86-64 CPU uses packed `vmulpd` and `vaddpd` operations, eight YMM accumulators, and no vector-register spills in the accumulation loop. The source uses `as_chunks::<2>()`; incomplete vector pairs and scalar latitude tails retain explicit handling. The change is confined to the f64 normal-range reducer. f32 and scaled-lane paths are unchanged. No workspace or plan memory was added.

## Native performance

The baseline is source commit `e642b6c`; the candidate changes only `src/legendre/scalar_paired.rs`. Both builds used Rust `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO disabled, portable Pulp runtime dispatch, and no `target-cpu=native`. The machine was an x86-64 CPU on Linux. Native measurements used `m1b_bench`, reused plans and workspaces, and pinned q1 to one CPU core and q8 to 8 pinned CPU cores. Each process warmed the transform and recorded five timed executions. Samples below are medians of process medians; paired changes use adjacent baseline/candidate process pairs. Exact samples and commands are in [`data/paired-analysis-two-way-unroll-2026-10-14`](data/paired-analysis-two-way-unroll-2026-10-14/paired-process-samples.txt).

| Operation / bandlimit | Threads | Baseline | Candidate | Median paired change | Paired range |
| --- | ---: | ---: | ---: | ---: | ---: |
| Spin-1 STANDARD analysis T511 | 1 | 39.681 ms | 39.102 ms | -1.55% | -2.53% to +3.94% |
| Spin-1 STANDARD analysis T511 | 8 | 12.428 ms | 12.580 ms | +0.69% | -2.84% to +23.06% |
| Spin-1 STANDARD analysis T720 | 1 | 104.216 ms | 102.073 ms | -1.68% | -3.00% to -1.29% |
| Spin-1 STANDARD analysis T720 | 8 | 34.083 ms | 33.926 ms | -0.46% | -2.24% to +0.86% |
| Scalar analysis T511 | 1 | 24.290 ms | 23.803 ms | -2.00% | -11.23% to -1.28% |
| Scalar analysis T511 | 8 | 7.358 ms | 7.383 ms | +0.08% | -17.73% to +12.77% |

The clearest result is the q1 analysis gain: spin-1 improves at T511 and T720, and scalar analysis improves at T511. q8 did not show a repeatable change; its process spread includes substantial host scheduling variation, so no multicore speedup is claimed. The improvement is retained for the repeated q1 gains and instruction-count reduction, with the q8 results documented as neutral/inconclusive.


## DUCC and numerical validation

The candidate Python plan was compared with installed `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, for f64 GL T511 spin-1 STANDARD analysis (`512x1025`, full `mmax`, `phi0=0.37`). One Python process per thread setting used five timed calls after warm-up. The q1 RustSHT/DUCC plan times were `40.412/13.677 ms`; q8 was `18.284/3.245 ms`. These Python timings include binding and per-call workspace costs and are contextual, not the native acceptance measure. Maximum absolute, RMS and relative-L2 coefficient errors were `1.411e-14`, `1.910e-16` and `3.740e-14` at both thread counts. Validation passed:

- Seven focused paired-analysis tests, including vector tails, mixed normal/scaled lanes and extreme finite product fallback.
- Spin-1 fast/general analysis and adjoint comparison across GL, CC and F1 through T255.
- Release spin-1/2 T511 DUCC sample and roundtrip regression on GL, CC and F1.
- Release spin-1/2 T1023 exact-analysis DUCC regression on GL, CC and F1.
- `cargo fmt --all -- --check` and workspace Clippy with all targets/features and warnings denied.
- `cargo test --workspace --all-features`: 143 passed, 8 ignored; 10 f32 integration tests passed.
- `cargo test -p rustsht --no-default-features`: 128 passed, 7 ignored; 10 f32 integration tests passed.
- `uv run pytest python/tests`: 16 passed. The changed summation grouping is not bitwise identical. The tested coefficient errors remain within existing tolerances, and no tolerance was changed. No FFT code, public API, grid semantics or higher-spin recurrence changed.
