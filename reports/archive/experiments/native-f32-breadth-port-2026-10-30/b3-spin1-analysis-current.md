# B3 spin-1 analysis, current scaled SIMD screen (2026-10-30)

## Candidate and provenance

- Base source: `b2648901580196d7e3af76db7d549354f994346c`.
- Isolated branch: `candidate/m6-native-f32-spin1-analysis-scaled-current`.
- Final combined B1+B3 source patch SHA-256: `94d6a64c8602b8ae0dab9b76ba9e5f5c05da0ef221c15966e5750e5bea6cbbd5`; compressed patch: [`b1-b3-spin1-analysis-shared-simd-current.patch.gz`](patches/b1-b3-spin1-analysis-shared-simd-current.patch.gz). It contains the current B1 scalar-analysis changes plus B3 spin-analysis changes; B1's standalone current-dev patch remains available separately.
- Rust f32 worker SHA-256: `797e43ff14ba291ad4533b78fafeb0331d6fbcb89fa66fb4f348af394eb93490`; matched Rust f64 worker: `38393d523a1c829c6859ce38d747d2c94fe34a0d6a76a6986f08117d3403f59f`.
- Pinned native-f32 DUCC 0.42.0 revision: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; worker SHA-256: `1e8412966f7bf306a1fea8141969208f661d5a705f7737e7a8f0dc1cc74cc36d`.
- CPU: AMD Ryzen 9 5950X; Rust `1.99.1`; `-C target-cpu=x86-64-v3`; q1 pinned to CPU 0, q8 to CPUs 0–7. The native-f32 worker reports Pulp V3 AVX2/FMA and eight f32 lanes. f32 fixtures are passed directly to f32 Rust and DUCC; the same inputs are widened component-wise only for the matched Rust f64 diagnostic.
- Three interleaved process pairs per cell. This is a screen, not a 15-pair performance claim. Full pair timings, source/compiler/binary/fixture hashes and output errors are in [`b3-spin1-analysis-final-t720-3pair.json`](b3-spin1-analysis-final-t720-3pair.json).

## T720 whole-call screen

| Geometry / threads | Rust f32 ms | matched Rust f64 ms | f32/f64 | DUCC f32 ms | f32/DUCC | Interim gate |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 170.261 | 38.391 | 4.571x (4.421–4.596) | 30.920 | 5.543x (5.489–5.625) | GAP |
| Full q8 | 25.051 | 6.274 | 3.969x (3.944–3.994) | 4.549 | 5.507x (5.493–5.525) | GAP |
| Reduced q1 | 79.439 | 15.603 | 5.126x (5.066–5.139) | 12.297 | 6.484x (6.460–6.501) | GAP |
| Reduced q8 | 11.567 | 2.429 | 4.762x (4.512–4.899) | 1.937 | 6.034x (5.844–6.161) | GAP |

Ranges show the three paired process ratios, not confidence intervals. Relative to the preceding scaled-lane B3 candidate, the shared SIMD scaled accumulator cut the full-q1 Rust f32 time from about 501 ms to 170 ms and reduced the f32/f64 ratio from 13.2x to 4.57x. It still misses the <=2.0x target in every cell and is not DUCC parity.

## Numerical and functional status

- The f32/DUCC random T720 fixture distance is `1.84695e-5` relative L2 / `3.51342e-7` max absolute (full) and `2.17137e-5` / `9.84408e-7` (reduced). The same f32 outputs differ from matched Rust f64 by `1.84695e-5` / `2.17137e-5` relative L2. These random comparisons are diagnostic, not an accuracy PASS.
- `f32_spin1_gl_t511_analysis_matches_ducc_0410` passes with its existing tolerance. The all-grid f32 spin-1 operation/adjoint integration test passes. The f64 `spin1` unit subset passes 49 tests with 4 ignored; scalar-analysis subset passes 17 with 8 ignored. The independent T720 spin-1 accuracy matrix remains incomplete.
- T720 scalar-analysis analytic/adjoint/workspace-reset test also passes for B1: relative L2 `4.2773e-5` (M180) and `4.2758e-5` (M720), maximum absolute `5.60e-6`.
- `cargo fmt --all`, `git diff --check`, and strict release Clippy (`--features parallel --lib -- -D warnings`) pass on the final source.

## Bounded blocker and next

The old scaled two-channel kernel restored every ring lane after each component product, then reduced scalar products. Replacing that with the shared native-precision SIMD accumulator (cached scale factors, FMA vector sums and the opposite-ring cosine within the same generic leaf) yields a 2.5–3x whole-call speedup. The remaining candidate path still checks paired recurrence range after every degree step and normalizes lane mantissas/exponents when any ring lane crosses its precision-specific range. This is the remaining source-visible numerical overhead; its exact time share is unmeasured. Linux performance counters cannot be read here (`perf_event_paranoid=4`), so no counter-based attribution is claimed. Stop this B3 microkernel pass at the bounded screen and retain the timing GAP.

All three missing transform families now have isolated native-f32 implementations and matched T720 screens. Per the updated owner sequence, resume the saved scalar-synthesis high-degree accuracy investigation next; then return to the measured B1/B3 timing gaps and complete the broader numerical matrix. Do not promote this candidate as a production timing or T720 accuracy pass.
