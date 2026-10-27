# M6 two-vector recurrence pipeline screen

**Decision:** reject and remove this candidate. It preserved exact Rust outputs, but regressed the selected f64 GL T720 spin-1 analysis cell by about 10.5% q1 and 7.7% q8 on the paired medians. The M6 gap remains open.

## Candidate and method

The candidate interleaved two independent SIMD-latitude vectors in the fused two-degree recurrence loop. It scheduled both first-step coefficient FMAs before accumulation, then advanced both recurrence chains while preserving each output accumulator's vector order and the exact per-lane arithmetic. It changed only `src/legendre/scalar_paired/analysis.rs` (105 additions, one deletion) on exact parent `30a17ee82154d761ccbbb40a256e27a97d3c2946`; the changed-file SHA-256 is `bbf7e381165fd703f73909374a91d6047ec697dbae1d26ca9a6ca5ec04a3bfda`. The recoverable zero-context patch SHA-256 is `f786de45bb2e09854146ad1259324e89619b4278b0fdb423351802f8e69f18a6`.

Three interleaved fresh-process parent/candidate/DUCC triplets ran at q1 and q8 on GL T720 full-mmax f64 spin-1 analysis (`ntheta=721`, `nphi=1458`, exact latitude mode, `phi0=0.23`, seed `1416973757`). Rust used rustc 1.99.1 / LLVM 23.1.1, release mode, and `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; q1 used CPU 0 and q8 CPUs 0–7. Pinned DUCC 0.42.0 was revision `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, built with GCC 15.2.0 and the SHT flags in the compact JSON. Each worker warmed and collected ten batches targeting 25 ms. Three pairs are an early screen, not a confidence interval.

| q | Parent / candidate / DUCC median process time (ms) | Paired candidate/parent ratio, median (range) | Candidate slower | Paired candidate/DUCC ratio median |
|---:|---:|---:|---:|---:|
| 1 | 36.299 / 39.869 / 29.307 | 1.105232 (1.078546–1.106919) | 3/3 | 1.356371 |
| 8 | 5.266 / 5.679 / 4.216 | 1.077169 (1.076884–1.082783) | 3/3 | 1.342385 |

All six pairs favored the parent. q1 parent/candidate scheduler wait was similar at about 6.4–6.9%. q8 scheduler wait was low in five workers; one candidate worker had 1.42% wait, while its parent pair had 0.52%. This difference does not explain the slower candidate across all six pairs. The code was not applied to production and was not extended.

## Correctness and provenance

Parent and candidate outputs were bitwise identical in all six triplets; all outputs were finite and repeat/reset consistent. Candidate versus DUCC relative-L2 error was `4.760590461219097e-14`, maximum absolute error `1.2958384365546749e-15`, and maximum relative error above the `1e-12` peak threshold `2.6248966535808065e-11`. Candidate binary SHA-256 was `dd09a466251be00b1540855115e94598be4b0ed78924d35403f3ad4e349e4ea7`; parent binary SHA-256 was `d3bf04c844f41872424052a3b8a107ca10fcdd70d77d742f08ddcbe731150aa5`; DUCC binary SHA-256 was `2d202a944f14facd1e5e209c0324049b47ca22f25caa18b53909ea809a489250`.

Before timing, candidate validation passed:

- `cargo test --locked --all-features --lib legendre::scalar_paired::analysis` — 34 passed, 8 ignored.
- `cargo test --locked --all-features --lib spin1` — 49 passed, 4 ignored.
- `cargo fmt --all -- --check`.

The compact [JSON](two-vector-pipeline-triplets.json) stores triplet medians, ratios, correctness and build/fixture/output hashes; the [CSV](two-vector-pipeline-triplets.csv) adds summarized scheduler-wait diagnostics. Neither includes worker snapshots or batch-level samples. The original runner capture is retained locally and untracked (SHA-256 `95312da1c6130384f5d055584c048de1727c461861f2df705753b3f9b4f0f060`). The exact candidate source diff is [preserved](two-vector-pipeline-candidate.patch). No performance parity claim follows from this single workload.

**Next:** investigate the existing GL T720 full versus reduced CC synthesis gap with a grid-specific cause; keep GL and CC resampling and quadrature paths distinct.

## Scratch cleanup

After saving and verifying the source patch, compact JSON, CSV, and report, removed only the experiment-owned scratch root `rustsht-m6-two-vector-pipeline.6ZaMVKzS` and its registered parent/candidate worktrees. The scratch held 9,557 files and 1,697,144,643 apparent bytes. `/tmp` available space increased by 1,667,055,616 bytes; free inodes increased by 11,004 to 949,016. Both worktree registrations are absent, and no experiment worker remains. Other parked worktrees were left untouched.
