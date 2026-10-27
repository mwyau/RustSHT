# Fuse inverse-sine scaling into paired SIMD synthesis — rejected

## Candidate and scope

The isolated candidate follows exact parent `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; patch SHA-256 `e0725761add848c53cf4032d0b68f3865aa6141703b9758cda5e338d78e4c6fe`. It overwrites the paired synthesis kernel's `previous` lane scratch with `1/sin(theta)` after successful lane normalization, scales all four accumulated Q/U components in the SIMD kernel, and removes the fused branch's later per-ring spectrum-scale loop. The scalar tail uses the same multiply. Recurrence arithmetic and FMA order are unchanged. This route's Q/U output order starts cleared, so scaling its newly accumulated sums is bitwise equivalent to scaling the output after accumulation.

The candidate was tested only in an isolated worktree. It was not applied to production. The target was f64 spin-1 synthesis on exact CC geometry with `lmax=720`, reduced `mmax=180`, `ntheta=722`, `nphi=378`, and `phi0=0.23`.

## Validation and measurement

Candidate validation passed `cargo test --locked --release --all-features --lib spin1 -- --test-threads=1` (49 passed, 4 ignored, 284 filtered), including spin-1 synthesis references, CC fallback, extreme inputs, and workspace reuse. `cargo fmt --all -- --check` and `git diff --check` passed. Parent and candidate both used rustc/Cargo 1.99.1, LLVM 23.1.1, release mode, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. Pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2.0, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for numerical sources and strict driver flags. Host: AMD Ryzen 9 5950X.

A deterministic fixture (seed `1416779915`) ran three balanced parent/candidate/DUCC triplets at q1 and q8. Each worker checked repeated calls and poisoned-output reset, then collected ten 25 ms target batches. Parent and candidate outputs were bitwise equal in all six triplets; all 18 workers were finite and passed repeat/reset. Candidate-parent relative-L2 and max absolute error were exactly zero. Candidate/DUCC maximum relative-L2 was `3.619e-13`, maximum absolute error `3.025e-9`.

| q | Parent / candidate / DUCC median process time (ms) | Paired C/P ratios; median | Candidate faster | Median q1/q8 host wait, parent/candidate |
|---:|---:|---:|---:|---:|
| 1 | 30.623 / 31.429 / 20.664 | 1.024277, 1.026317, 1.034464; **1.026317** | 0/3 | 9.75% / 9.84% |
| 8 | 5.704 / 5.479 / 3.016 | 0.977610, 0.877852, 1.002711; **0.977610** | 2/3 | 3.54% / 3.41% |

The q1 result is the protected target: every pair favored the parent by 2.4–3.4%, despite balanced median scheduler wait. q8 is inconclusive; its apparent lead depends on one unusually fast candidate-first pair, while the remaining ratios straddle parity. This three-pair screen does not establish a new DUCC parity ratio; its host waits differ from earlier matched cells.

**Decision:** reject this candidate and keep the existing post-scale loop. The repeated q1 slowdown is not justified by the noisy q8 result. No production source changed. Exact per-worker batches, host diagnostics, hashes, and errors are in the [triplet JSON](simd-scale-triplets.json) and [CSV](simd-scale-triplets.csv); the [candidate patch](simd-scale-candidate.patch) and [runner](simd-scale-triplet-runner.py) preserve the experiment.

## Cleanup

The experiment used only owned scratch root `rustsht-m6-simd-scale.BfvcbdyW`, including its two registered worktrees, local Cargo home/targets, pinned DUCC source/build, fixture, binaries, and raw outputs. Before cleanup, `du` reported 853,016,576 apparent bytes across 9,034 files. After verifying the durable report, patch, CSV, JSON, and runner, cleanup removed only that exact root and its two worktree registrations. `temporary filesystem` available bytes rose from 48,239,845,376 to 49,092,861,952 (+853,016,576); free inodes rose from 938,608 to 949,016 (+10,408). The root and registrations are absent, and no experiment worker remains.
