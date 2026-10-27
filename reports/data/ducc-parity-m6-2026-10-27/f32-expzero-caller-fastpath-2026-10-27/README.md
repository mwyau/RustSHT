# Native-f32 exponent-zero caller fast path — 2026-10-27

The candidate moves the common exponent-zero branch from `multiply_complex_scaled_f32` into its sole paired scalar-synthesis caller. Zero-exponent lanes perform the same two native-f32 multiplies directly; nonzero exponents keep the existing scaled helper unchanged. The patch is based on exact source parent `a30537bdcfa0fc5273985abfd592998325a608de`; pinned DUCC is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Matched GL T720 results

Three alternating fresh-process parent/candidate/DUCC triplets per thread count used the deterministic full-mmax f32 scalar-synthesis fixture. Times are median milliseconds per transform; candidate/parent and candidate/DUCC are medians of paired ratios.

| GL T720 full M720 scalar synthesis | Parent / candidate / DUCC ms | Candidate / parent (range) | Candidate / DUCC |
|---|---:|---:|---:|
| q1 | 904.456 / 881.213 / 19.228 | 0.974301 (0.966349–0.990875) | 45.854× |
| q8 | 370.554 / 366.436 / 2.670 | 0.986955 (0.982199–0.991703) | 137.242× |

All six candidate/parent pairs were faster. Parent and candidate output hashes were identical in every pair, and each backend passed finite, repeat, and reset checks. Candidate/DUCC relative L2 was `1.72343e-4`, matching the existing source-level numerical error. The prior focused `a30537bd` report measured 140.90× DUCC q8 for this workload; this screen measured 137.242×. These are still substantial f32 gaps, not parity. This triage screen covers only GL T720 full-mmax scalar synthesis.

## Validation and decision

On the candidate worktree, `cargo test --locked --release --features parallel --lib scalar` passed (168 passed, 16 ignored), and `cargo test --locked --release --features parallel --test f32` passed (11 passed, 1 ignored). Formatting and `git diff --check` passed. The patch's `git apply --check` passed against a clean exact-parent worktree. The exact candidate source hash and binary hashes are recorded in `provenance.json`; `candidate.patch.gz` preserves the source diff and `screen-runner.py.gz` preserves the benchmark runner. Pair medians, error metrics, fixture hashes, and output hashes are in `screen-results.json` and `triplets.csv`.

The small caller change was applied to `dev` after exact-parent numerical checks and the matched q1/q8 screen. Keep the f32-specific path; do not infer behavior for other grids, operators, or sizes from this cell.

## Experiment scratch cleanup

After verifying the source patch, checksums, pair results, and test record outside scratch, removed only the owned scratch root `rustsht-m6-f32-expzero-caller-7VW2Zw` and its clean parent and candidate worktrees. The root measured 855,679,325 apparent bytes across 9,123 files. The scratch root and registrations are absent, and no experiment worker remained. Temporary filesystem available space increased from 48,214,937,600 to 49,092,861,952 bytes (877,924,352); free inodes increased from 938,479 to 949,016 (10,537).
