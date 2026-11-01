# B1 f64 seed normalizer bypass

## Finding

After successful f64 SIMD sectoral initialization, B1 checked every latitude lane for normal range twice: `initialize_normal_analysis_sectoral_dispatched` proves the seed is ordinary, then `FusedScalarAnalysisPairTyped::with_simd` called `normalize_analysis_pair_state` over the same previous/current/exponent state. The successful initializer also sets `previous = 0` and `exponents = 0`. The candidate carries that proof to the typed kernel and skips the duplicate scan for `lmax >= 510`.

The call remains guarded by successful initialization: exceptional or scaled seed blocks return before SIMD recurrence dispatch. Low-bandlimit setup retains its normalizer. The compile-time flag is true only for f64; the native-f32 path is unchanged.

## Source crosswalk and code generation

| Stage | RustSHT source |
|---|---|
| f64 seed initializer selection and successful-state initialization | [`analysis.rs`](../../../../src/legendre/scalar_paired/analysis.rs#L966), lines 975–1008 |
| Validation and dispatch after initialization | `fused_scalar_analysis_pair_block_preweighted_typed`, lines 1233–1340 |
| Redundant normalization previously performed at block entry | `FusedScalarAnalysisPairTyped::with_simd`, lines 1488–1525 |
| SIMD initializer wrapper and lane-by-lane range proof | `initialize_normal_analysis_sectoral_dispatched`, lines 2078–2091; `InitializeNormalAnalysisSectoral::with_simd`, lines 2455–2535 |

The 5950X worker dispatched to Pulp V3 AVX2/FMA with four f64 lanes. In the candidate V3 entry, `cmp $0x1fe,%rdi; ja ...+0x211` branches over the normalizer dispatch at T720. The parent entry makes that indirect dispatch before entering the ordinary recurrence path. The ordinary two-pair recurrence/accumulation and its FMA sequence are unchanged. The V3 function shrank from 7,236 to 7,102 bytes; V4 shrank from 9,902 to 9,639 bytes. Selected entry disassembly is preserved in `evidence/parent-v3-entry.asm.gz` and `evidence/candidate-v3-entry.asm.gz`.

The isolated normalization probe uses GL T720 northern-ring geometry and confirms successful seed state (`previous` and exponents are zero) remains unchanged. Nine pinned samples measured a median of 125.482 ns for 108 lanes and 289.106 ns for 256 lanes. Raw sample values and the test-only probe patch (`evidence/normalizer-microbench-tests.patch.gz`) are in `evidence/`.

## Cost profile

The latest retained B1 phase probe before this change (source `252a839`) measured f64 recurrence plus accumulation at 13.21–13.33 ms of 16.1–16.5 ms (80–83%); the FFT took 3.14–3.18 ms. This separates FFT from the combined Legendre phase but does not split recurrence from accumulation. `perf stat` and `perf record` cannot read hardware events on this host (`perf_event_paranoid=4`), so no cycle-level attribution is claimed.

The new microbenchmark isolates the duplicate lane scan, while whole-transform timing shows its impact: 3.8–6.3% lower B1 time across all four T720 cells. The remaining f64/DUCC gap is 5.6–10.5%; the reduced q1 interval includes parity and remains INCONCLUSIVE.

## Matched whole-transform results

Fifteen interleaved warm pairs per cell compare exact parent `ac88fe80` with the candidate and pinned DUCC 0.42.0 f64. Ratios are paired geometric means with 95% Student-t intervals on log ratios. The complete sanitized pair samples, fixture hashes, binary hashes, route, CPU and affinity are in [`screen15-summary.json`](evidence/screen15-summary.json); the earlier three-pair screen is in [`screen3-summary.json`](evidence/screen3-summary.json).

| Cell | Parent → candidate / DUCC median ms | Candidate / parent | Candidate / DUCC |
|---|---:|---:|---:|
| Full q1 | 16.177 → 15.549 / 14.445 | 0.962 [0.950, 0.974] | 1.087 [1.074, 1.100] GAP |
| Full q8 | 2.430 → 2.344 / 2.141 | 0.962 [0.947, 0.976] | 1.105 [1.086, 1.125] GAP |
| Reduced q1 | 6.815 → 6.422 / 5.946 | 0.937 [0.912, 0.963] | 1.056 [0.987, 1.129] INCONCLUSIVE |
| Reduced q8 | 1.246 → 1.188 / 1.059 | 0.952 [0.936, 0.967] | 1.096 [1.069, 1.123] GAP |

All 60 parent/candidate output comparisons were byte-identical. Full and reduced input fixture hashes are recorded with each cell. Rust workers used `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; q1 was pinned to CPU 0 and q8 to CPUs 0–7, with CPU 8 reserved for host monitoring.

The protected f64 GL T1023 spin-1 synthesis q1/q8 control used three pairs. Outputs were exact; candidate/parent medians were 1.0085 q1 and 1.0032 q8. Both controls are INCONCLUSIVE at three pairs and show no established regression. See `evidence/protected-t1023-control.json`.

## Validation

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: 329 unit tests, 4 extreme-range integration tests, 14 f32 integration tests, and 1 workspace compatibility test passed; 41 unit tests and one opt-in f32 T1023 test were ignored.
- Python API suites: 20 passed in `python/tests`; 7 passed and 1 skipped in `python/pyspharm-rs/tests`.
- Focused T511 scalar-analysis/analytic test passed.

## Provenance and next

- Parent commit: `ac88fe80fc2cd5f522b4ae4a37e8621a9d8ee6d6`.
- Parent/candidate `analysis.rs` SHA-256: `a23b7e73a806adcaa2a51b1758b1d8c56ff6d999ea83731dd06dddb3de429d4f` / `96bde2286f8ac6ec45c896e68a8e5acfff4546facf1c3c5f1601ebcf9ff72075`.
- Candidate production patch SHA-256 after decompression: `063775cae135b4d4162671c8a0a0d5c3160e8a21c8bb7b3a3a9ac187227a41cb` (`evidence/production-candidate.patch.gz`).
- Pinned DUCC revision: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Hardware: AMD Ryzen 9 5950X; `perf_event_paranoid=4`.

Retain this bounded f64 B1 improvement. It does not close parity. The active plan allows one or two B1 attempts; this is the first. Continue with a measured ordinary-loop or tail hypothesis only if a short microbenchmark identifies a distinct avoidable cost, then move to current-source f64 B3 profiling.
