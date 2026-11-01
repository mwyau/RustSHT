# B1 zipped-lane iteration screen — 2026-11-02

## Scope and provenance

This B1-only experiment uses exact parent `792c8f363ad73cd839a0df73965714b74623ac6e`, candidate patch SHA-256 `bed8d2a00071e36627889377ec4c5b934eb4ef1cbc02e941bce27d62b1f2574e`, and pinned DUCC source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The candidate changes only the ordinary two-degree SIMD loop in `accumulate_two_normal_scalar_analysis_pairs`: indexed access across equal-length SIMD slices becomes a chained `zip` traversal. FMA order, recurrence, accumulation, scale handling, tail path, FFT and scheduling are unchanged.

Host: AMD Ryzen 9 5950X; `-C target-cpu=x86-64-v3`, runtime Pulp V3 AVX2/FMA (f32: 8 lanes, f64: 4 lanes); q1 pinned to CPU 2, q8 to CPUs 0–7. DUCC uses the matching x86-64-v3 binaries with `-O3 -ffp-contract=fast`. `perf stat` and `perf record` are blocked by `perf_event_paranoid=4`; Callgrind `Ir` is dynamic instruction count, not cycles.

## Execution crosswalk and profile

Rust dispatch: `ShtPlan::analysis_into` → `analyze_map_to_coefficients` → `analyze_order_jobs_dynamic` → `analyze_order_group` → `analyze_symmetric_order_group` (`src/transform/scalar.rs:2119`) → `fused_scalar_analysis_pair_block_preweighted_typed` (`src/legendre/scalar_paired/analysis.rs:1233`) → `accumulate_two_normal_scalar_analysis_pairs` (`:1349`). The last loop advances one SIMD latitude vector through two degree pairs, accumulating same/opposite real/imag channels and updating recurrence state with AVX2 FMA. Each ordinary f32/f64 block uses the native type. `forward_modes` is the separate Fourier stage.

Pinned DUCC f32/f64 public transforms use `leg2alm<T>` → `leg2alm_internal<T>` (`src/ducc0/sht/sht.cc:777`) → `inner_loop_m2a<T>` (`sht_inner_loop.h:1752`). `fill_m2a<T>` (`:1660`) prepares ring data; `calc_map2alm` (`:636`) handles scaled state then calls `map2alm_kernel` (`:592`). That kernel updates two degree pairs per latitude-vector visit. In `leg2alm_internal`, Fourier spectra retain public `T`, while the recurrence and temporary alm array are `complex<double>` even for f32 maps; results narrow to public `T` at writeback.

One full T720/M720/q1 Callgrind run on parent source attributes 94.32% of f32 instructions and 81.37% of f64 instructions to the symmetric order group; the fused paired block accounts for 86.28% and 59.81% of whole-transform instructions. FFT accounts for 5.68% (f32) and 18.61% (f64); preparation is 2.96%/7.46%, factor fill 1.24%/3.36%. These are inclusive call-tree shares and are not additive. Whole-call Callgrind totals fell from 715,838,219 to 691,336,065 instructions for f32 (−3.42%) and from 278,584,228 to 265,147,211 for f64 (−4.82%). The kernel remains the largest avoidable region. Callgrind did not isolate recurrence, normal/scaled accumulation, exponent normalization and exceptional fallback from one another. An earlier trace on the unchanged f32 route counted 266,685 scaled pair calls and 159,441 factor epochs; these are work counts, not elapsed-time shares. q8 schedule and memory-system costs remain unisolated.

## Machine-code hypothesis

`objdump -d` confirms the parent f32 AVX2 loop emits four per-iteration slice-length comparisons/branches before its FMA body, and reloads several bases from stack slots in the loop. The zipped candidate retains one loop-bound comparison, keeps bases in GPRs, and retains the same `vfmadd*ps` sequence. Both have a 6,120-byte frame; the hot vector loop has no YMM stack spills. Function size changes from 22,532 to 22,307 bytes. The f64 four-lane FMA path also compiles with fewer per-iteration slice guards; its function size changes from 7,102 to 7,227 bytes and frame remains 504 bytes. Selected exact instructions are in `loop-disassembly.txt`.

The hypothesis is that redundant equal-length slice termination tests and pointer reloads add loop overhead at every SIMD vector. The 12,000-iteration f32 block microbenchmark measured indexed 45.34 ns versus zipped 27.78 ns median (9 samples); outputs and recurrence buffers were bit-identical. This microbenchmark is directional; whole-call timings are the acceptance evidence.

## Whole-transform screen

Fifteen interleaved parent/candidate/DUCC process pairs per cell, each Rust worker collecting 10 warm batches; GL T720, full M720 and reduced M180, q1/q8. Parent and candidate outputs were bit-identical in all eight cells. Candidate/parent ratios use paired geometric means and 95% t intervals on log ratios (df=14); `results-15pair.json` contains every pair, per-batch timing, output error, fixture hash and binary hash.

| Cell | f32 candidate/parent | f32 candidate/DUCC | f64 candidate/parent | f64 candidate/DUCC |
|---|---:|---:|---:|---:|
| Full q1 | 0.972 [0.865, 1.092] inconclusive | 2.717 [2.638, 2.800] gap | 0.918 [0.828, 1.018] inconclusive | 1.059 [1.002, 1.120] gap |
| Full q8 | 0.951 [0.926, 0.977] improvement | 2.589 [2.531, 2.649] gap | 0.972 [0.935, 1.010] inconclusive | 1.131 [1.097, 1.166] gap |
| Reduced q1 | 0.935 [0.918, 0.953] improvement | 2.203 [2.169, 2.237] gap | 0.934 [0.851, 1.025] inconclusive | 1.047 [0.988, 1.110] inconclusive |
| Reduced q8 | 0.947 [0.922, 0.973] improvement | 2.100 [1.316, 3.351] gap, noisy DUCC pairs | 1.004 [0.938, 1.075] inconclusive | 1.033 [0.998, 1.068] inconclusive |

The three-pair current-source control measured f32/Rust-to-DUCC at 2.26–2.79× and f64 at 1.02–1.14×; the owner-provided historical ranges were not reproduced by these exact fixtures and binaries. Do not interpret this screen as DUCC parity. The zip change is a confirmed native-f32 improvement in three cells, with full q1 unresolved. f64 has no statistically resolved candidate regression or gain; its Callgrind instruction reduction is diagnostic and must not substitute for warm timing. The largest remaining f32 gap is still at least 2.1× DUCC.

## Checks and artifacts

Focused `f32_t720_scalar_analysis_matches_analytic_modes_adjoint_and_workspace_reset` passed at q1/q8 and full/reduced orders. Three `fused_scalar_analysis_block*` SIMD-tail, scaled-transition and range-failure tests passed. Parent/candidate outputs matched exactly over all 120 benchmark pairs. In a clean candidate worktree, `cargo fmt --all -- --check`, `cargo test --release --features parallel`, `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, and the Python API suite (20 tests) passed. The protected f64 GL T1023/M1023 spin-1 synthesis control used three pairs at q1/q8: parent/candidate medians were 1.0005x/0.9886x, with exact outputs, successful repeat/reset checks and finite values. Three pairs do not establish a timing effect; the B1-only loop is not called by this route. Full control rows and binary/fixture hashes are in `protected-t1023-spin1-synthesis-3pair.json`.

Included: `results-15pair.json` (15-pair candidate), `baseline-f32-3pair.json` and `baseline-f64-3pair.json` (three-pair current-source baseline), `production-candidate.patch.gz`, test-only microbenchmark patch `.gz`, compressed Callgrind raw profiles, and `loop-disassembly.txt`. The uncompressed production patch SHA-256 is `bed8d2a00071e36627889377ec4c5b934eb4ef1cbc02e941bce27d62b1f2574e`; the uncompressed microbenchmark patch SHA-256 is `dd4f825a79f82702eed9a2a879ecb0cc73f2bf91a8fe678a82126e1911b419a1`. `SHA256SUMS` covers them. Rust/DUCC binaries and fixtures are not embedded; their hashes and fixture hashes are in the JSON, and benchmark scripts are in the parent diagnostic directory.

## Decision / NEXT

Retain this bounded generic loop change for f32: three cells show a paired whole-call improvement, the fourth is unresolved, and the arithmetic/output is unchanged. f64 timing effects remain inconclusive, so recheck f64 under a quieter host before asserting a performance gain. Next, profile B1 f32 scaled accumulation and exceptional fallback separately on a nonzero fixture; distinguish wall time from instruction counts, then profile f64 q8 against DUCC. Do not retune unrelated transform families.
