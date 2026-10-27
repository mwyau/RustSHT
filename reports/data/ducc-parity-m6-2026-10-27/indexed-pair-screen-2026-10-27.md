# Prepared paired-synthesis indexed traversal screen — rejected — 2026-10-27

## Candidate and decision

On exact parent `f70d9845edf6bba617b9d034b1f5e1d628c429e0`, the candidate replaced the six-slice nested `zip`/`enumerate` traversal with ordered `0..pair_count` indexing in both vector and scalar-tail loops of `FusedPreparedSynthesisPairBlock::with_simd`. Each pair still uses the same coefficient values in the same order; all recurrence and FMA expressions are unchanged. It touched only `src/legendre/scalar_paired/synthesis.rs`, whose candidate SHA-256 is `e7ac514825cab293c8b6225f9a0371b3a9724f8644b620c4329501cc6cad1e6a`. The recoverable patch SHA-256 is `007238b0fc312d2dfd885cfd03ec8ec21ccf5e019b7d482ba1259d9e24f730cb`.

Matched q1 Callgrind totals were **613,507,756 Ir parent** and **612,782,575 candidate** (725,181 fewer, **0.1182%**). The candidate removed the 61.71M direct `core::iter::adapters::zip` attribution, but moved much of the count into `core::cmp` (52.92M) and `core::iter::range` (8.88M). Direct FMA counts were unchanged. The whole-program Ir change is too small to support a performance claim.

The q8 whole-transform screens were slower for the candidate in all three CC pairs and in two of three GL control pairs, with low median scheduler wait. q1 GL showed a small paired lead, while q1 CC was highly variable; these were three-pair screens without confidence intervals. Given the small Ir change and q8 results, **reject the candidate and keep the parent implementation**. No production source change was made. The checked transition/fallback path remains the next source area to inspect; do not retry this indexed traversal unchanged.

## Validation and measurements

The candidate passed `cargo fmt --all -- --check`, `git diff --check`, and `cargo test --locked --release --all-features --lib spin1 -- --test-threads=1` (**49 passed, 4 ignored, 284 filtered**). Across 12 parent/candidate/DUCC triplets (3 per q per case), every Rust output was bitwise parent-equal; all 36 workers passed finite, repeat, and reset checks. Candidate-parent errors were zero. Candidate/DUCC maximum relative-L2 / maximum absolute error were `3.619e-13` / `3.025e-9` for CC and `2.602e-13` / `5.410e-9` for GL.

| Case | q | Paired C/P ratios (3 triplets) | Median paired C/P | Parent / candidate / DUCC median process time (ms) | Median parent/candidate wait |
|---|---:|---|---:|---:|---:|
| CC T720, mmax 180, 722×378 | 1 | 1.040792, 0.811324, 0.762314 | 0.811324 | 37.788 / 39.139 / 20.898 | 7.77% / 8.79% |
| CC T720, mmax 180, 722×378 | 8 | 1.001165, 1.047649, 1.014293 | 1.014293 | 4.608 / 4.737 / 2.671 | 0.70% / 0.60% |
| GL T1023, full mmax, 1024×2048 | 1 | 0.985753, 0.847663, 0.989605 | 0.985753 | 106.316 / 90.120 / 127.150 | 1.86% / 2.06% |
| GL T1023, full mmax, 1024×2048 | 8 | 1.011036, 0.899998, 1.014806 | 1.011036 | 14.838 / 14.962 / 20.201 | 0.89% / 0.65% |

The q1 CC pair spread is too wide for a stable timing conclusion. GL q1 was candidate-faster in all three pairs, but the median paired lead was 1.4% and this early screen has no confidence interval. Both q8 median ratios favor the parent; CC q8 favored the parent in all three low-wait pairs. The result does not establish a whole-transform win or improve the recorded DUCC parity claim.

## Provenance

- Parent and candidate base: `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust/Cargo 1.99.1, LLVM 23.1.1; release `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, `parallel` feature. Callgrind builds used the same flags plus `CARGO_PROFILE_RELEASE_DEBUG=2` and the local shim link argument.
- DUCC used GCC 15.2.0 with the pinned SHT `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` build and checked driver override `-fno-fast-math -ffp-contract=fast`. Host: AMD Ryzen 9 5950X.
- CC fixture seed `1416779915`; GL fixture seed `1436943300`. Exact fixture hashes, per-triplet samples/diagnostics, binary hashes, and output hashes are retained in the two triplet JSON files.
- Parent/candidate Callgrind profile hashes: `6405e466e013c4e8c6ba955a501ab551f8ce3d8b1794104c607de629e4864846` / `c72c6f4d8cb9a0d6b25a61b221cd62245178ba3130b2e8d1cc2edff4cfafb91d`. Callgrind summary CSV is [indexed-pair-callgrind.csv](indexed-pair-callgrind.csv).
- Runner SHA-256 `924ca5903415bbf5b9042efd1e275825160351df1e1c7b921a92bce083ee84cd`; it was based on the checked M6 triplet runner and changed only to accept explicit case/pair-count inputs.

## Cleanup

The report, patch, compact triplet data, and raw Callgrind profiles were copied and verified before cleanup. Removed only owned scratch root `rustsht-m6-cc-index-pairs.o0nSeyfV` (2,113,777,626 apparent bytes; 9,949 files) and its two registered worktrees. `temporary filesystem` available bytes rose by 2,138,640,384 (46,954,221,568 to 49,092,861,952); free inodes rose by 11,546 (937,470 to 949,016). The root and registrations are absent, and no build, profiler, or benchmark worker remains.
