# Mixed-lane f32 scalar synthesis fusion screen — 2026-10-27

The candidate tried the existing fused recurrence on lanes whose initial normalized state was safe inside an otherwise mixed GL T720 scalar-synthesis block. Unsafe lanes were zero-masked for that attempt; a failed range or finite check restored staged sums and sent all lanes through the unchanged scaled fallback. If fusion succeeded, its safe lanes were skipped in the fallback accumulation. The source patch was based on exact parent `a30537bdcfa0fc5273985abfd592998325a608de`; pinned DUCC was `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Matched results

Three alternating fresh-process parent/candidate/DUCC triplets ran at each thread count with the same deterministic native-f32 fixture. Times are per-transform medians in milliseconds.

| GL T720 full M720 scalar synthesis | Parent / candidate / DUCC | Paired candidate / parent | Candidate / DUCC |
|---|---:|---:|---:|
| q1 | 893.171 / 885.095 / 18.899 | 0.993786 (range 0.980393–1.003015) | 46.878× |
| q8 | 373.917 / 381.385 / 2.672 | 1.019974 (range 1.016261–1.025878) | 142.984× |

The q1 result is mixed and near parity. All three q8 pairs were 1.63–2.59% slower for the candidate, and its DUCC ratio worsened. The screen does not justify the extra branch and second fused traversal, so the candidate was removed. This is not a parity result.

All 18 workers passed finite, repeat, and reset checks. Candidate/parent relative L2 error was `1.240014e-7`; candidate/DUCC relative L2 was `1.723430e-4` with maximum absolute error `9.644032e-4`. The recorded candidate map-0 SHA-256 was `539778faaf22455e94ae6f26d2825027c54526f88b3760423a15dfa9cc3a1f32`; parent was `260e35799c6f8d3c505fc9d6f2cc5bdbd5443e03d4e75e4d488044904066d45d`; DUCC was `8b0ebf91d2af1961a2fc66eacb82329b7612f11813f2850daa28c5b28cae77d4`.

## Validation and provenance

- The new mixed-lane direct-reference unit test passed on scalar and runtime SIMD dispatch.
- `cargo test --locked --release --features parallel --lib scalar`: 169 passed, 16 ignored.
- `cargo test --locked --release --features parallel --test f32`: 11 passed, 1 ignored.
- Formatting and `git diff --check` passed.
- Rust 1.99.1 release workers used runtime Pulp dispatch; pinned DUCC used GCC 15.2.0 with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`. q1 used CPU 0; q8 used CPUs 0–7. The workers were run sequentially in alternating order.

`screen-results.json` contains matched per-pair median timings, errors, fixture/output hashes, and build/source identifiers; raw worker snapshots and host telemetry are omitted. `triplets.csv` is the compact timing table. `candidate.patch.gz` expands to the exact source/test diff from the parent; its uncompressed SHA-256 is `863f87b445e0ddc5876ec8444c37b3773f760d78c68d3d70a08d0add742b1653`. The patch SHA was verified and `git apply --check` passed against the exact `a30537bd` parent worktree. `executed-runner.py.gz` preserves the exact runner, and `provenance.json` records source, binaries, fixtures, and cleanup.

After saving and verifying the evidence, removed only the exact scratch root `rustsht-m6-f32-mixedmask-3948wn9b` (1,028,386,228 apparent bytes) and its clean registered parent worktree. The scratch path and registration are absent, with no matching worker process. Temporary filesystem available space increased from 48,049,979,392 to 49,092,861,952 bytes (1,042,882,560); free inodes increased from 942,227 to 949,016 (6,789). Three pre-existing untracked Callgrind files in the repository were untouched.
