# B3 one-channel scaled recurrence fusion

## Decision

Retain as an incremental B3 performance improvement. It reduces native-f32 whole-call time in all four GL T720 spin-1 analysis cells, with 15-pair candidate/parent intervals below 1 and bit-identical f32 output. It is not parity: Rust f32 remains 2.64–2.99x DUCC f32, and Rust f64 remains 1.21–1.35x DUCC f64. Keep all parity gates open.

The change fuses one recurrence advance into channel 0's f32 scaled SIMD accumulation pass. Channel 1 keeps the existing pass. The B3-only `B3ScaledRecurrencePrecision` leaf leaves f64 unfused. The scalar-analysis entrypoints and B1 arithmetic were restored to their parent implementation; the B1 full-q8 whole-binary timing check is nevertheless inconclusive and is recorded below.

## Source and build provenance

- Parent source: `95bcbecc862e7e0c08fa7e7769ba30ea7c59f86a`.
- Candidate `analysis.rs` SHA-256: `514932cdb18b6a8ac0f77fc6b12822145249e5ba9497436c48e90447976e7a7a`.
- Candidate patch SHA-256: `0c7a20981b1f3279a5d9c2208571df329e399487f5562d2f85f32385e9ce6b6f`.
- DUCC: pinned `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- CPU: AMD Ryzen 9 5950X; q1 and q8 runs were pinned with taskset. Exact affinity is preserved in the compressed manifests.
- Rust: `1.99.1 (58cf8f9c3 2026-10-13)`, flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- Fixtures: deterministic GL T720 spin-1 analysis fixtures, `phi0=0.23`; widened f64 controls use the same f32 inputs. Per-pair fixture hashes, worker checksums and binary hashes are in the compressed run manifests; uncompressed manifest checksums are beside each file.
- The executed f32 route reports AVX2 V3 with eight f32 lanes. The worker includes scheduling, Legendre analysis, FFT and output handling; target batch is 25 ms.
- Exact binary hashes and complete paired worker data are in `b3-15pair/metadata.json.gz`.

Generated fixture arrays and output files are omitted from this archive. The runner deterministically regenerates the fixtures, while the recorded hashes permit checking a reproduction; pairwise output errors and worker checksums are preserved. The exact source patch is retained compressed as `source.patch.gz`; its uncompressed SHA-256 is recorded above. The run scripts use `RUSTSHT_CANDIDATE_ROOT`, `RUSTSHT_PARENT_ROOT`, `RUSTSHT_DUCC_ROOT`, `RUSTSHT_BASELINE_ROOT`, `RUSTSHT_RESULTS_DIR` and `RUSTSHT_CANDIDATE_PATCH` for portable paths. The manifests replace local paths with those labels while preserving executable hashes.

## B3 results

Times are medians in milliseconds. Candidate/parent intervals are deterministic 95% percentile-bootstrap intervals for the median paired ratio (50,000 resamples; see `summarize.py`). Rust/DUCC and f32/f64 ratios are medians of paired ratios. `GAP` means the ratio remains above 1.

| GL T720 cell | Rust f32 / f64 ms (ratio) | Rust f32 / DUCC f32 (ratio) | Rust f64 / DUCC f64 (ratio) | Candidate / parent (95% interval) |
|---|---:|---:|---:|---:|
| Full q1 | 93.815 / 38.642 (2.438 GAP) | 2.987 GAP | 1.213 GAP | 0.928 [0.915, 0.940] PASS vs parent |
| Full q8 | 14.136 / 6.649 (2.098 GAP) | 2.944 GAP | 1.346 GAP | 0.907 [0.881, 0.935] PASS vs parent |
| Reduced q1 | 34.921 / 16.142 (2.149 GAP) | 2.644 GAP | 1.236 GAP | 0.930 [0.913, 0.939] PASS vs parent |
| Reduced q8 | 5.409 / 2.656 (2.033 GAP) | 2.656 GAP | 1.295 GAP | 0.937 [0.909, 0.974] PASS vs parent |

All 60 candidate/parent f32 outputs are bit-identical (`relative L2 = 0`). Relative L2 against DUCC f32 is unchanged from the parent and reaches `1.85e-5` on full-order fixtures and `2.17e-5` on reduced-order fixtures. All workers reported repeat/reset consistency. The candidate f64 output is also bit-identical to parent in a separate three-pair control.

### Controls and caveats

- **f64 route:** candidate/parent medians for full q1, full q8, reduced q1 and reduced q8 were `1.004`, `1.030`, `1.001` and `0.992`. The three-pair bootstrap intervals all include 1; output is bit-identical. Timing is INCONCLUSIVE, with no evidence of a stable f64 change.
- **B1 scalar analysis full q8:** 15-pair candidate/parent median `1.037`, 95% interval `[1.000, 1.112]`; output is bit-identical. Mark this whole-binary control INCONCLUSIVE. The B1 route has no changed callsite, but the median trends slower and needs a quieter/repeated control before claiming no performance effect.
- The host was shared during screening. Immediately after the run, load average was `3.73, 5.44, 6.18` with 21 users and another CPU-heavy workload active. The wide q8 control intervals reflect that limitation.
- The AVX2 V3 scaled-channel kernel grew from 2,476 to 5,270 disassembled instructions; stack-based operand references grew from 416 to 676. These counts are a code-size/stack-traffic warning, not a claim that every reference is a spill. The measured whole-call improvement is positive, but further fusion should avoid blindly growing this kernel.

## Validation

- `cargo fmt --check` — pass.
- `cargo clippy --all-targets --all-features -- -D warnings` — pass.
- `cargo test --release --test f32` — 14 passed, 1 ignored.
- `cargo test --release --lib` — 323 passed, 38 ignored.
- `uv run --group dev --with maturin maturin develop --release` — pass.
- `uv run --group dev pytest -q python/tests` — 20 passed.
- Protected `spin1_t1023_order_major_cache_is_bounded_and_reused` passed in the full library suite.

## Next

Return to B1 scalar analysis with a new measured hypothesis. Recheck the B1 full-q8 control under lower host load; do not claim it passes while its interval overlaps both sides of 1. Continue to compare all changes against the exact source parent and pinned DUCC.
