# Native-f32 vector-group fusion screen — 2026-10-27

The candidate retried the paired f32 fused synthesis kernel one native f32 vector at a time when a mixed block could not pass the existing whole-block fast path. Groups with ordinary seed state used the fused recurrence; exceptional groups used the existing scaled callback. The candidate was based on exact parent source `3189fe8ee49f125d0d4569549fb880a6b6468bb8`; pinned DUCC 0.42.0 was `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Matched GL T720 scalar-synthesis results

Three interleaved fresh-process parent/candidate/DUCC triplets ran for each full/reduced mmax q1/q8 cell. The Rust workers used native f32, V3/AVX2 (8 f32 lanes), and the same deterministic fixture per cell. q1 used CPU 0; q8 used CPUs 0–7. Timings are medians of each worker's warm batch samples; the ratio is the median of the three paired candidate/parent ratios.

| GL T720, spin-0 synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent (range) | Candidate/DUCC |
|---|---:|---:|---:|---:|---:|
| Full M720 q1 | 722.334 | 714.263 | 18.883 | 0.99044 (0.97457–0.99914) | 37.83× |
| Full M720 q8 | 319.399 | 318.370 | 2.677 | 0.99864 (0.99678–1.00003) | 119.50× |
| Reduced M180 q1 | 216.216 | 212.824 | 8.405 | 0.98358 (0.98186–0.99353) | 25.32× |
| Reduced M180 q8 | 110.236 | 109.640 | 1.217 | 1.00002 (0.99206–1.00589) | 89.87× |

The candidate gained 0.1–1.6% in the median q1 cells, with the strongest result at reduced M180. Both q8 medians were effectively unchanged; reduced q8 included a 0.59% candidate regression. This is too small and narrow to retain the added f32-specific control path. The source candidate was rejected after the three-pair triage; no 15-pair acceptance screen was run.

Outputs were not bitwise equal to the parent. Candidate/parent relative L2 was (1.1787\times10^{-7}) at full mmax and (9.9735\times10^{-8}) at reduced mmax. Candidate/DUCC relative L2 and maximum absolute error were (1.72343\times10^{-4}) / (9.64403\times10^{-4}) at full mmax and (2.72546\times10^{-4}) / (1.54787\times10^{-3}) at reduced mmax; these match the parent results to the shown precision. All 36 workers passed finite-output, repeated-call, and workspace-reset checks.

## Validation and provenance

The focused mixed-scale test passed (1 passed); the scalar unit-test filter passed (169 passed, 16 ignored); the f32 integration suite passed (11 passed, 1 ignored). Formatting, `git diff --check`, and release all-feature library Clippy with warnings denied passed. The C++ worker used pinned DUCC 0.42.0; Rust and DUCC build flags, binary hashes, deterministic fixture hashes, per-worker batch timing, checks, and output hashes, and all triplet results are in [provenance.json](provenance.json), [triplets.json](triplets.json), and [triplets.csv](triplets.csv). [candidate.patch.gz](candidate.patch.gz) preserves the exact rejected source/test diff; [executed-runner.py](executed-runner.py) preserves the screen runner.

The candidate production diff was removed after preserving and checking this evidence. This screen covers only GL T720 spin-0 scalar synthesis; it does not establish parity or results for other f32/f64 workloads.

## Scratch cleanup

After verifying the report and patch checksums, removed only the owned experiment scratch (measured as 1.5 GiB by `du -sh`) and its clean registered parent worktree. The scratch and worktree registration are absent, and no experiment worker remains. Free inodes increased by 61,574.
