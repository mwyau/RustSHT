# Sparse scaled-analysis fallback lane list — exact-source 15-pair screen

The native-f32 B1/B3 kernel now caches ascending lane IDs for hard scalar fallback contributions and skips the full-lane scan at each degree accumulation. Recurrence, arithmetic, SIMD lane behavior, accumulation sequence and outputs are otherwise unchanged.

## Provenance

- Measured candidate commit `deced73802a05d9b6c3c2bddf8ef2fc0cf8a0dc3`, tree `1d0138c5433e4ba58ef3b11497cb9e0d802b0e22`; integrated on `dev` as `a3cd896456ed307e2e0b0c4db3597d5dea4976eb` with the same tree.
- Parent source `47906d0966897cf2f04d33d38d71a467a7fafd7c`, tree `51922dd5de3b723cdfeb63b3202dd5cac9ed2649`.
- Source file SHA256 `1ab9fba32fc0ca5dedddba05000e39c6d6006fc0800150e5aadef2ac8f42cb7a`; patch SHA256 `d02483a33b8260a62fefbde02516163b94a78fb20761c7755d3a967fff4259f6`.
- DUCC 0.42.0 source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Ryzen 9 5950X, rustc 1.99.1, flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; q1 CPU 0; q8 CPUs 0–7. Candidate executed native-f32 V3/AVX2 with eight lanes.
- Fifteen interleaved 25 ms warm whole-call batches for each B1/B3 full/reduced q1/q8 cell. f64 fixtures are exact componentwise widenings of f32 fixtures.
- Worker, fixture and source hashes are in `metadata.json`; `paired-runs.csv` retains all 120 pair timings, ratios and candidate-parent output comparisons. Generated fixtures/output arrays and raw profiles are kept outside the repository.

## Results

Candidate/parent ratios and all absolute parity ratios have 95% percentile-bootstrap intervals over 15 paired values (50,000 resamples, seed 20261030). Candidate/parent intervals are below one in all eight cells, with medians 0.725–0.809, or 19–28% lower whole-call time. Outputs are bit-identical in all 120 pairs. Absolute parity remains open: median f32/f64 is 2.10–3.70× and f32/DUCC f32 is 2.86–4.96×. Rust f64/DUCC f64 is above one in seven cells; reduced B1 q1 is INCONCLUSIVE.

| Cell | Candidate/parent (95% CI) | f32/f64 | f32/DUCC32 | f64/DUCC64 | Status |
|---|---:|---:|---:|---:|---|
| GL-T720-M180-even-s0-analysis-q1 | 0.725 [0.612, 0.979] | 2.751 GAP | 3.635 GAP | 1.182 INCONCLUSIVE | bit-identical |
| GL-T720-M180-even-s0-analysis-q8 | 0.766 [0.738, 0.784] | 2.565 GAP | 3.589 GAP | 1.367 GAP | bit-identical |
| GL-T720-M180-even-s1-analysis-q1 | 0.768 [0.764, 0.774] | 2.317 GAP | 2.956 GAP | 1.276 GAP | bit-identical |
| GL-T720-M180-even-s1-analysis-q8 | 0.791 [0.763, 0.800] | 2.099 GAP | 2.860 GAP | 1.357 GAP | bit-identical |
| GL-T720-M720-even-s0-analysis-q1 | 0.789 [0.780, 0.796] | 3.696 GAP | 4.959 GAP | 1.319 GAP | bit-identical |
| GL-T720-M720-even-s0-analysis-q8 | 0.808 [0.770, 0.874] | 3.191 GAP | 4.586 GAP | 1.415 GAP | bit-identical |
| GL-T720-M720-even-s1-analysis-q1 | 0.792 [0.767, 0.811] | 2.625 GAP | 3.247 GAP | 1.237 GAP | bit-identical |
| GL-T720-M720-even-s1-analysis-q8 | 0.809 [0.783, 0.847] | 2.113 GAP | 3.324 GAP | 1.428 GAP | bit-identical |


## Profile evidence

| q1 profile | Parent Ir | Candidate Ir | Reduction |
|---|---:|---:|---:|
| B1 scalar analysis | 1,465,363,688 | 1,244,090,413 | 15.1% |
| B3 spin-1 analysis | 1,992,075,499 | 1,635,697,946 | 17.9% |

These are Callgrind instruction references, not cycles. Profile hashes and harness provenance are in `profile-provenance.json`; the raw captures remain local and untracked.

## Verification

- `cargo fmt --all -- --check`: passed.
- Strict all-target/all-feature Clippy with V3/native link flags: passed.
- Scalar-paired analysis unit tests: 36 passed, 8 ignored.
- f32 integration tests: 14 passed, 1 ignored.
- Protected f64 T1023 spin-1 synthesis regression: passed.
- Python tests: 20 passed.
- Existing tolerances unchanged; all candidate outputs match parent bit-for-bit.
- The host was contended during part of the run. Complete paired data are retained, but bootstrap resampling does not model temporal host-load correlation; repeat on a controlled host before final parity acceptance.
