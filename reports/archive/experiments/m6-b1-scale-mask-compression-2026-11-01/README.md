# B1 f32 scale-mask lane update — 2026-11-01

**Decision: retain the B1-only f32 update.** The native f32 full-call times improve significantly in all four T720 cells. The remaining f32/DUCC gap is still 2.43–3.04×, so this is a narrow improvement rather than a parity result.

## Route and hypothesis

The production path enters `ShtPlan::analyze_symmetric_order_group` at `src/transform/scalar.rs:2119`, then calls the paired scalar recurrence in `src/legendre/scalar_paired/analysis.rs:1165,1381`. The f32 normalizer is in `src/legendre/scalar.rs:1445,1498`; the selected-lane update is reached only by the B1 `SHORT_CIRCUIT_RESULT=true` specialization.

The exact-parent route recorded 1,327,943 full-order range checks and 158,692 normalization triggers; reduced order recorded 668,654 checks and 35,084 triggers. A test-only lane counter found 1.32 selected exponent updates per eight-lane vector pass at full order and 1.35 at reduced order. Every triggered vector had mixed exponents. The previous path materialized a float mask and scanned all eight lanes for each trigger.

The candidate uses Pulp's typed `select_u32s` and `partial_store_u32s` operations, compresses the selected flags, then visits only set lanes with `trailing_zeros`/`blsr`. It retains checked i64 exponent arithmetic and the exceptional fallback. It does not reinterpret Pulp's opaque mask representation. Compile-time specialization leaves other f32 transform families on their existing path; f64 source and behavior are unchanged.

## Profile and generated code

`perf stat` and `perf record` are blocked by `/proc/sys/kernel/perf_event_paranoid=4`. The exact-4ce q1 Callgrind profile is transform-only and includes cache/branch simulation; those events are instruction-simulator counts, not hardware cycles.

| Profile | Instructions (Ir) | D1 misses | Conditional branches | Conditional mispredicts |
|---|---:|---:|---:|---:|
| f32 parent, source `4ce0ed5` | 803.36M | 1.0329M | 145.91M | 2.714M |
| f32 candidate | 793.18M | 1.0186M | 143.54M | 2.404M |
| f64 candidate, same map promoted for this diagnostic profile only | 320.34M | 2.0227M | 45.30M | 0.864M |

The f32 candidate reduces Callgrind Ir by 1.27%, conditional branches by 1.62%, and simulated conditional mispredicts by 11.4%. Its order-group inclusive count is 752.52M Ir (94.87% of the whole call); the V3 f32 typed kernel accounts for 615.42M Ir self (77.59%). The f64 order-group inclusive count is 268.45M Ir (83.80%). These nested values are not additive. The profile shows the paired recurrence/accumulation body remains the dominant work; exponent-mask traversal is a smaller avoidable cost.

The release microbenchmark at 1.25 selected lane updates per vector measured 3.403 ns for the original full-lane scan and 2.610 ns for the candidate set-lane update, a 23.3% reduction in that helper. The AVX2 excerpt shows typed mask selection, vector compression (`vpsrlvd`, `vpand`, `vpor`), then `tzcnt`/`blsr` and the checked exponent update. It also shows stack staging for the compressed lane flags. The f32 ordinary accumulation uses eight-lane YMM `vfmadd231ps` and horizontal reductions; f64 uses four-lane YMM `vfmadd231pd`. See [assembly excerpts](v3-f32-mask-and-recurrence.asm.txt).

For the matching DUCC baseline, the [pinned DUCC `map2alm_kernel` AVX2 disassembly](../m6-b1-profile-410c-2026-10-31/hot-loops.asm.txt) shows a four-double-lane YMM loop using `vfmadd*pd`; that capture is from DUCC source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` and binary SHA-256 `efb42fd69e47bd39f0060208202efab326c337f3c0f434506ee556b5984e3836`. The new f32 helper reduces mask-lane update work, while the main Rust B1 accumulation still performs its native eight-lane recurrence/accumulation path. This assembly comparison is structural evidence only; it does not attribute hardware cycles.

Existing stage probes at source `410c9ec` attributed 94–97% of f32 wall time and 82–91% of f64 wall time to recurrence plus accumulation, with FFT at 3–6% and 8–18%. Those probes predate later B1 traversal changes and are included as context, not current-source stage timing. The current profile's inclusive order-group values corroborate that the Legendre stage remains dominant.

## Fifteen-pair whole-transform screen

The exact parent was `4ce0ed5efa2e36f46aa2f34537821211ff17ef16`; candidate source was that parent plus patch `38bac5b72e5602c046021a2ecbbb739ed7960176bcb3773e58dd24701c13cc23`. Rust release builds used `-C target-cpu=x86-64-v3`; runtime dispatch was Pulp V3 AVX2/FMA, eight f32 lanes. The host was an AMD Ryzen 9 5950X. q1 used CPU 2 and q8 used CPUs 0–7. Pinned DUCC 0.42.0 source was `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The Rust path uses native f32 coefficients, recurrence, spectrum, FFT and output.

| GL T720 B1 | Parent ms | Candidate ms | DUCC f32 ms | Candidate / parent (95% CI) | Candidate / DUCC (95% CI) | Matching Rust f64 / DUCC f64 |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 44.391 | 43.371 | 14.201 | 0.967 [0.955, 0.979] | 3.040 [3.013, 3.067] | 1.310 GAP |
| Full q8 | 6.037 | 5.872 | 2.065 | 0.974 [0.968, 0.981] | 2.811 [2.770, 2.852] | 1.273 GAP |
| Reduced q1 | 15.834 | 15.547 | 5.853 | 0.978 [0.962, 0.994] | 2.653 [2.623, 2.683] | 1.278 GAP |
| Reduced q8 | 2.219 | 2.178 | 0.895 | 0.966 [0.945, 0.988] | 2.431 [2.414, 2.449] | 1.226 GAP |

All 15 paired candidate/parent intervals are below 1.0. Candidate and parent outputs are bitwise identical in all pairs; repeat and workspace-reset checks passed. Candidate/DUCC relative L2 was `1.839e-5` full and `2.220e-5` reduced; this performance screen did not change numerical tolerances or claim new independent accuracy. f64 was not modified; its matching-DUCC results are the exact-source 15-pair baseline in the [live ledger](../../../../reports/DUCC_PARITY_M6.md).

The protected f64 GL T1023 spin-1 synthesis q1/q8 smoke controls each used one pair. Outputs were exact; candidate/parent ratios were 0.995 q1 and 0.935 q8. The q8 repeat/reset flags passed. These single-pair controls are not statistically meaningful performance claims; the f64 transform source is unchanged.

## Validation and provenance

- `cargo fmt --all -- --check` passed.
- `cargo test --all-features`: 329 passed, 40 ignored; integration tests passed.
- Strict Clippy passed with `-D warnings`.
- Python API suite: 20 passed.
- Existing T720 scalar-analysis analytic, adjoint, and workspace-reset test passed; no test tolerance was changed.

The exact per-pair results, uncertainty, fixtures, Rust binary hashes, DUCC binary hash, and runtime dispatch are in [screen15.json](screen15.json). The current-source profile totals and raw profile hashes are in [profile-summary.json](profile-summary.json). Exact Callgrind captures remain local and untracked under `raw/` per the repository artifact policy. The test-only profile harness and screen runner are under [repro](repro/). The uncompressed B1 patch SHA-256 is `38bac5b72e5602c046021a2ecbbb739ed7960176bcb3773e58dd24701c13cc23`; restore it with `gzip -dc candidate.patch.gz | git apply`.

File checksums are listed in [SHA256SUMS](SHA256SUMS).

## Next

Keep this f32 lane-update reduction. The measured gain is 2.2–3.4% per T720 cell, while the fused recurrence/accumulation remains the dominant cost and f32 remains 2.43–3.04× DUCC. Next isolate normal/scaled recurrence and accumulation work on current source, then test a materially different execution strategy. Do not repeat the rejected scale-threshold, recurrence-fusion, tile, or sparse-index variants.
