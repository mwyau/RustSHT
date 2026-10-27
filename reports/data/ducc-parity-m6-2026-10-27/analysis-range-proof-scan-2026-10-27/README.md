# M6 cached-proof recurrence-state scan screen

**Decision: not promoted.** The candidate reused an existing checked recurrence proof to skip the scalar finite/upper-bound scan after the shared reflected spin-1 recurrence. The cached proof is populated only after the same plan/order/block recurrence has passed that scan. The initial checked traversal still performs the scan, and per-call input/product safety checks remain in place. This differs from B02, which removed per-degree SIMD range tracking.

## Provenance

- Exact parent: RustSHT `4c2455317f66a85e071cb6a43e0292b79bd3d82c`; production source matches the 22-cell baseline at `98adf1e6b5334021bfe6ae9abb618c6be167a822` (the intervening commit changed reports only).
- Candidate patch SHA-256: `50a148e2a320314dabd225895a72954d38508a832724f0d01213dfcc38dc7241`.
- Candidate source file SHA-256: `7a76bd0dded8db0b6862f94727c85d431d0abb0a5823dbf2df10ee3d42e5502c`.
- Pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- Rust release flags: `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC SHT flags: `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; driver adds `-fno-fast-math -ffp-contract=fast`.
- Candidate and parent workers were built separately. The measured worker used the same 721×1458 GL T720 full-mmax fixture as the current-head baseline.

The [compressed candidate patch](candidate.patch.gz) is recoverable. [Machine-readable results](screen-results.json) retain paired worker medians, batch spread, scheduler waits, frequency summaries, output hashes and DUCC errors without local CPU identifiers or scratch paths.

## Correctness

Every parent/candidate output compared byte-for-byte across the six initial cells and both 15-triplet confirmations. All workers passed finite, repeat and reset checks. Candidate-to-DUCC max relative L2 for GL T720 analysis was `4.76059e-14` at both thread counts; max absolute error was `1.29584e-15`. Each benchmark fixture hash matched the current-head baseline.

Candidate validation passed:

- `cargo test --locked --release --all-features --lib spin1_analysis_reuses_poisoned_workspace_for_extremes_on_all_grids` — 1 passed.
- `cargo test --locked --release --all-features --lib spin1` — 49 passed, 4 ignored.
- Parent and candidate release example builds completed with the flags above.

## Timing

Each paired ratio is candidate median / exact-parent median within a fresh-process triplet. The 95% intervals use 50,000 bootstrap resamples of the 15 paired ratios. The first set used the checked benchmark's default physical-core placement; the confirmation used a separate physical-core set with the frequency monitor outside the worker set.

| GL T720 full spin-1 analysis | Triplets | Candidate/parent median | 95% bootstrap interval | Candidate faster | Max wait, parent/candidate/DUCC |
|---|---:|---:|---:|---:|---:|
| Initial core set, q1 | 15 | 0.99202 | 0.97772–1.00866 | 8/15 | 6.91% / 6.92% / 0.94% |
| Initial core set, q8 | 15 | 0.94441 | 0.89117–1.03908 | 9/15 | 10.53% / 9.97% / 20.56% |
| Alternate core set, q1 | 15 | 0.98331 | 0.97886–0.99329 | 12/15 | 6.63% / 6.67% / 0.05% |
| Alternate core set, q8 | 15 | 1.01836 | 0.97894–1.09460 | 7/15 | 3.57% / 3.39% / 1.87% |

The q1 result suggests a small gain on the alternate set, but the first-set interval includes parity. q8 is inconclusive on both sets and its median changes direction. Three-pair control screens were near parent parity: GL T1023 synthesis C/P `1.00065` q1 and `1.00297` q8; CC T720 reduced synthesis `0.99435` q1 (elevated wait); GL T720 over-resolved reduced synthesis `1.00051` q8.

**Disposition:** leave production unchanged. The screen does not establish a stable q1/q8 gain, and no candidate-specific whole-call instruction reduction was measured. Reconsider only with new attribution showing this scan is material and a quiet matched q8 result.

## Cleanup

The experiment-owned scratch and two temporary worktrees were removed after the patch and results were verified in this report directory. Measured free space increased by 2,608,795,648 bytes and free inodes by 3,004. No benchmark worker or experiment worktree registration remained.
