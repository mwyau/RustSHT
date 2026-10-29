# Shared scalar-analysis rescale-mask screen — rejected

This Stage C candidate replaced the horizontal `reduce_max` used to ask whether any AVX2 f64 lane exceeded the rescale threshold with the existing lane-mask helper and `vmovmskpd`. The recurrence, scale updates, checked fallback, and coefficient accumulation were unchanged. Timings came from a test-only f32-FFT/f64-recurrence diagnostic route, not the shipped f32 public analysis dispatch; that hook also lacked a guarded decline to native f32 for exceptional values. The candidate is rejected: it did not produce a repeatable whole-call gain and all four f32-I/O/f64-recurrence cells remain above native-f32 DUCC. No production source was changed.

## Provenance and method

- Exact production source parent: `e3ae78e75e4d1883db3ae6680d9be2832b725e5e`. The production files in the parent worker were byte-identical at `64133b570865712b26a2ed8bf1b1d5ea9061a6d3`; intervening commits recorded evidence only.
- Measured source patch: [`candidate-measured.patch.gz`](candidate-measured.patch.gz), uncompressed SHA-256 `162d40416102f8fb068140e8072c7ce47327919bb3151f6712d1fe432f8d6773`. [`candidate-formatted-equivalent.patch.gz`](candidate-formatted-equivalent.patch.gz) contains the same expression after `rustfmt` line wrapping.
- The test-only f32-FFT/f64-recurrence hook is reused from the [current-source boundary package](../f32-scalar-analysis-f32fft-boundary-64133b57/README.md); its patch SHA-256 is `4cce560395a8499cda2d2e579ab0cb8f24adc455da33882546fe76a4d08dd692`. The test-only spin profiling overlay is also archived there. Neither hook is production code.
- DUCC 0.42.0 is pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Worker and test binary hashes are in [`binary-hashes.txt`](binary-hashes.txt).
- AMD Ryzen 9 5950X; Rust 1.99.1 / LLVM 23.1.1, release `parallel`, `opt-level=3`, no explicit `RUSTFLAGS`; Pulp selected AVX2/V3 at runtime. DUCC used GCC 15.2, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its worker driver used `-fno-fast-math`.
- The f32 row FFT, requested-spectrum promotion, shared f64 recurrence/accumulation, and Complex64-to-Complex32 writeback were inside the timed route. Plans/workspaces were made before timing. Rust results are medians of seven warmed calls per fresh worker; DUCC used ten 25 ms batches. q1/q8 process affinity followed the deterministic policy in the committed [`paired-screen.py`](benchmark/paired-screen.py) runner.
- Input hashes are in [`fixtures.json`](fixtures.json). Per-worker samples, route status, reset status and output hashes are in [`paired-results.json`](paired-results.json). Portable runner copies in [`benchmark/`](benchmark/) take source, target, fixture, DUCC and output paths from environment variables.

## Whole-call results

Ratios are paired medians; bracketed ranges are observed pair ranges. `GAP` indicates the end-to-end test route remains slower than DUCC.

| GL T720 f32 scalar analysis | Parent / candidate / DUCC ms | Candidate/parent median [range] | Candidate/DUCC median [range] | Candidate vs parent output | Status |
|---|---:|---:|---:|---|---|
| Reduced q1 (3 pairs) | 6.176 / 6.131 / 5.771 | `0.998` [`0.992–1.005`] | `1.061` [`1.060–1.079`] | bitwise equal, all pairs | **GAP; neutral** |
| Reduced q8 (15 pairs) | 1.002 / 0.991 / 0.887 | `0.991` [`0.901–1.025`] | `1.115` [`1.033–1.150`] | bitwise equal, all pairs | **GAP; no stable gain** |
| Full q1 (3 pairs) | 15.157 / 15.159 / 13.923 | `1.000` [`0.993–1.017`] | `1.089` [`1.082–1.099`] | bitwise equal, all pairs | **GAP; neutral** |
| Full q8 (3 pairs) | 2.538 / 2.552 / 2.049 | `1.005` [`1.001–1.007`] | `1.243` [`1.216–1.245`] | bitwise equal, all pairs | **GAP; slight regression** |

The reduced-q8 cell was extended from three to 15 pairs because its initial screen looked about 2.8% faster. Across 15 pairs, candidate/parent was `0.9915`, with a fixed-seed paired bootstrap 95% interval of `0.9716–1.0002`; the interval includes parity and the observed range is wide. Candidate/DUCC was `1.1146x`, so the route still misses. Full q8 candidate/parent was `1.0054x`, with all three pairs slower. Both q1 cells were effectively neutral.

Native-f32 controls had candidate/parent ratios of `0.999` reduced q1, `1.006` reduced q8, `0.995` full q1 and `0.998` full q8 medians; no repeatable adjacent f32 regression appeared. Candidate coefficients were bitwise identical to parent in every paired case. P3 candidate-vs-DUCC relative L2 / maximum absolute error was `1.319e-7 / 1.397e-9` reduced and `1.432e-7 / 6.985e-10` full. The focused shared analysis tests passed 34 tests (8 ignored), covering scaled transitions, tails, range declines and existing reference cases; `cargo fmt --check` passed on the formatted-equivalent source. No workspace storage was added. Allocation counts were not separately instrumented.

## Decision and next action

Reject this mask reduction. It replaces a horizontal maximum reduction with an AVX2 `vmovmskpd` check, but the only apparent whole-call improvement disappears in 15 pairs and full q8 slows slightly. The emitted instruction excerpt is in [`assembly/avx2-mask-conversion.txt`](assembly/avx2-mask-conversion.txt). The follow-up Stage D f32 reflected leaf variants and f32-I/O/f64-internal reflected/order-major routes have also been screened and rejected; see the [spin-1 reflected-route screen](../f32-spin1-reflected-route-screen/README.md) and the [reusable f64-internal spin-1 report](../f32-spin1-f64-internal-backend-v1/README.md). Keep the checked analysis path and target the measured shared scaled-analysis recurrence bulk path next. M6 remains open.

To reproduce, apply the test-hook patch and build overlay linked above, then `candidate-measured.patch.gz`. Set `M6_PARENT_SOURCE`, `M6_PARENT_TARGET`, `M6_CANDIDATE_SOURCE`, `M6_CANDIDATE_TARGET`, `M6_FIXTURES`, `M6_DUCC_BINARY`, and `M6_OUTPUT_ROOT` before running `benchmark/paired-screen.py`; `benchmark/extend-q8-reduced.py` runs pairs 4–15 for reduced q8.
