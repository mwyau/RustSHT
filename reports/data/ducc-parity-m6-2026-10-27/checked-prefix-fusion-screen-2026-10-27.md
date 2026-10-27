# M6 checked-scaled prefix/update fusion screen

**Decision:** reject and remove the candidate. It preserved Rust output bits, but the exact GL T720 f64 spin-1 analysis screen was slower in all six parent/candidate pairs. The f64 M6 parity gap remains open; this result does not certify other cells.

## Candidate and method

The candidate fused `MixedAnalysisPairChannelsAccumulation`'s vector pass with recurrence advancement in `FusedCheckedMixedAnalysisPairChannelsStep`. It retained scaled exceptional-value handling, scalar-tail arithmetic, and fallback behavior. It changed only `src/legendre/scalar_paired/analysis.rs` (213 additions, 19 deletions) on exact parent `67eb85151d0e91cca0fcc3bc6b89372f67e64dcd`; candidate source-file SHA-256 was `f0d4ce0cdc6111db1bd2757e3af264cf208605c5e923f6903d3acb192227af7f`. The recoverable zero-context patch SHA-256 is `aa01403775a9113be444d693c659d825f673875f53e7a87b275c36734b95bed7`.

Three interleaved fresh-process parent/candidate/DUCC triplets ran at q1 and q8 on GL T720 full-mmax spin-1 analysis (`ntheta=721`, `nphi=1458`, exact latitude mode, `phi0=0.23`, seed `1416973757`). Rust used rustc 1.99.1 / LLVM 23.1.1, release mode, and `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; q1 used CPU 0 and q8 CPUs 0–7. Pinned DUCC 0.42.0 was revision `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, built with GCC 15.2.0 and the SHT flags recorded in the JSON evidence. Each worker warmed and then collected ten batches targeting 25 ms. There are three triplets per q, not a confidence interval.

| q | Parent / candidate / DUCC median process time (ms) | Paired candidate/parent ratio, median (range) | Candidate slower | Paired candidate/DUCC ratio median |
|---:|---:|---:|---:|---:|
| 1 | 36.164 / 37.916 / 29.139 | 1.046406 (1.034628–1.059503) | 3/3 | 1.293809 |
| 8 | 5.391 / 5.963 / 4.288 | 1.130237 (1.016940–1.138137) | 3/3 | 1.372254 |

The q1 slowdown is consistent across all pairs. Parent and candidate scheduler wait was similar (about 6.5–6.9%), so runqueue wait does not explain the consistent slowdown. q8 also favored the parent in all three pairs, though one candidate pair had 3.50% wait; interpret that three-pair screen with its host variation in mind. This candidate does not improve the measured cell and was not applied to production.

## Correctness and provenance

Candidate and parent outputs were bitwise identical in all six triplets, finite, and repeat/reset consistent. Against DUCC, candidate output relative L2 error was `4.760590461219097e-14`, maximum absolute error `1.2958384365546749e-15`, and maximum relative error above the `1e-12` peak threshold `2.6248966535808065e-11`. Candidate binary SHA-256 was `3701e14a7829252dfc2d30a5a0a67b168c37c622c3080635e57121ae39cdb15b`; parent binary SHA-256 was `a9047ef0bf2723020f749bccdcffa405fb4ece052dc3f8b263f2364984d98e4a`; DUCC binary SHA-256 was `acb3fdf666cd855ce0186460265b7e032c425f0694c4fce3598f91fda2478c8d`.

Before timing, the candidate passed:

- `cargo test --locked --all-features --lib legendre::scalar_paired::analysis` — 34 passed, 8 ignored.
- `cargo test --locked --all-features --lib spin1` — 49 passed, 4 ignored.
- `cargo fmt --all -- --check`.

No broad suite or warning-denied Clippy run was needed for this rejected variant. The compact [JSON](checked-prefix-fusion-triplets.json) contains triplet medians, ratios, correctness checks, and build/fixture/output hashes; the [CSV](checked-prefix-fusion-triplets.csv) adds summarized wait and frequency diagnostics. Neither includes worker snapshots or batch-level samples. The original runner capture is retained locally and untracked (SHA-256 `1efc83e1ec8ca4681c4fc8d1a8ebd9d7af0c7986b68daa5a30caa071bd411ad0`). The exact candidate diff is [preserved separately](checked-prefix-fusion-candidate.patch). No benchmark parity claim follows from this single workload.

**Next:** pursue a new measured instruction-reduction hypothesis in the core fused recurrence/accumulation path. Keep the checked exceptional-value and fallback contract intact.

## Scratch cleanup

After saving and verifying the source patch, JSON, CSV, and report, removed only the experiment-owned scratch root `rustsht-m6-checked-prefix-fusion.gMZMJ8I2` and its registered parent/candidate worktrees. The scratch held 9,007 files and 924,896,044 apparent bytes. `/tmp` available space increased by 930,566,144 bytes; free inodes increased by 10,380 to 949,018. Both worktree registrations are absent, and no experiment worker remains. Other parked worktrees were left untouched.
