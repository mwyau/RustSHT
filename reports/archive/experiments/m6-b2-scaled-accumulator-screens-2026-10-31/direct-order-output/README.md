# B2 direct q8 order output screen — rejected

This candidate bypassed per-worker contiguous Q/U rows for the prepared reflected latitude band and wrote that band directly into each worker's uniquely owned Fourier-order column. Polar/rejected rows retained the checked scratch route. The queue copied only rings outside the successful direct band. Phase and Nyquist operations were applied at the direct store.

## Source and workload

- Base source: `de6fc98e04dcbf56de125f4e1c0ec81aa402cb0f`, tree `da31dc2805f38c3fba96acfc1ccda170dedc16ad`.
- Tested patch SHA-256: `0893c1988893923893a400cc9ce82146d6b50695c92188316bfeaea59c354e80` (`source.patch.gz`).
- DUCC: pinned `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Ryzen 9 5950X, V3/AVX2/FMA, rustc 1.99.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- Three interleaved whole-call pairs per GL T720 spin-1 synthesis cell; q1 affinity `0`, q8 affinity `0-7`; 25 ms warm batches. Same deterministic fixtures and matched native Rust f32/f64 and DUCC f32/f64 workers as the current-source parity runner.

## Results

| Cell | Candidate/parent paired ratios | Median f32/f64 | Median f32/DUCC f32 | Median f64/DUCC f64 | Output delta vs parent |
|---|---|---:|---:|---:|---:|
| Full q1 | `1.0368, 1.0018, 0.9556` | 1.543 GAP | 1.430 GAP | 0.896 | 0 |
| Full q8 | `0.9959, 0.9944, 1.0012` | 1.458 GAP | 1.493 GAP | 0.977 | 0 |
| Reduced q1 | `1.0132, 1.0201, 1.0034` | 1.584 GAP | 1.464 GAP | 0.922 | 0 |
| Reduced q8 | `0.9810, 1.0102, 1.0031` | 1.548 GAP | 1.571 GAP | 1.001 | 0 |

This three-pair screen does not show a repeatable whole-call gain: full q8's median is about 0.4% faster, while reduced q8 is effectively neutral. q1 is outside the targeted route and its reduced cell moved about 1.3% slower. The direct-store complexity therefore was not extended to a 15-pair confirmation and is not promoted.

The route was native f32 with V3 eight-lane SIMD; the Rust f32/f64 and DUCC parity gaps remain. Per-pair timing, dispatch, fixture/binary hashes, and output checksums are in `results/metadata.json` and each case's `pair-N.json`. The benchmark runner is the shared `../run_b2.py`.

## Validation

Before the screen, `cargo fmt --all --check`, 51 spin-1 release library tests, and all 14 active f32 release integration tests passed. Candidate-vs-parent output relative L2 was exactly zero in all twelve pairs. Because timings were neutral, no code was promoted.

Patch and Callgrind artifacts use deterministic gzip compression; decompress them to recover the exact bytes and verify the recorded uncompressed SHA-256 values.
