# Native-f32 zero-exponent multiply screen — 2026-10-27

## Decision

Keep the small f32-only fast path in `multiply_complex_scaled_f32`: when the tracked scale exponent is zero, return the direct f32 component products. It passed the exact split-reference unit test and the broader scalar/f32 suites. Across 15 matched q1 triplets, candidate/parent median was **0.96164** (95% bootstrap interval **0.94802–0.97563**); every pair favored the candidate. Across q8, median was **0.99531** (**0.98789–1.00310**), with 11/15 pairs faster. The q8 interval includes parity, so report q8 as near parity, not as a measured speedup.

Parent and candidate outputs were byte-identical in all 30 triplets and matched the current-head baseline Rust output hash for this fixture. Finite, repeat, and reset checks passed for every worker. The changed branch is confined to `multiply_complex_scaled_f32`; no f64 route or source was changed.

## Measurement and limits

The exact parent is RustSHT `98adf1e6b5334021bfe6ae9abb618c6be167a822`. Parent and candidate used separate release targets, the same Rust f32 worker from `examples/ducc_f32_full_transform_parity.rs`, the same deterministic GL T720 full-mmax fixture, and pinned q1/q8 affinity sets. Fifteen parent/candidate triplets ran per q, with the order alternated. Each process reported five warm batches. The machine-readable results preserve batch samples, outputs, worker diagnostics, frequency snapshots, and confidence intervals.

The prior same-source DUCC 0.42.0 baseline remains q1 1,066.473 ms Rust / 19.079 ms DUCC and q8 373.030 ms / 2.660 ms. The candidate screen did not rerun DUCC, so its exact-parent gains are not a fresh candidate/DUCC comparison. Per-process scheduler-wait fractions were not collected. Post-process CPU pressure snapshots were captured, but exact host-telemetry strings are omitted from the committed JSON; frequency snapshots and all timing pairs remain. The q8 interval excludes a material regression larger than about 0.31% at its upper end, but does not establish a speedup.

## Validation

- `cargo test --locked --release --all-features --lib f32_scaled_complex_multiply_matches_split_reference` — 1 passed.
- `cargo test --locked --release --all-features --lib scalar` — 167 passed, 16 ignored.
- `cargo test --locked --release --all-features --test f32` — 11 passed, 1 ignored.
- `cargo fmt --all -- --check` — passed.
- `cargo clippy --locked --release --all-features --lib -- -D warnings` — passed.

The [candidate patch](candidate.patch) and [15-pair results](screen-results.json) retain exact provenance. The screen used only experiment-local worktrees, targets, Cargo home, and results. After verifying the durable evidence, both owned worktrees and the experiment root were removed; the scratch measured 1,119,836,402 bytes, `/tmp` available space rose 1,142,824,960 bytes, and free inodes rose 10,780. No experiment worker remained.
