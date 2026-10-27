# M6 — reflected active-lane mask screen

**Disposition:** candidate removed; no supported f64 speedup.

The candidate reused active-lane decisions already made while staging each reflected-latitude block. The existing compaction pass reloaded north and south ring-order limits and recomputed the same predicate. The candidate recorded the first-pass decisions in a `u64` when compaction was enabled, then used the mask during compaction. Block size is 64; recurrence, arithmetic order, safety checks, decline behavior, and fallback routing were unchanged.

## Provenance and method

- Exact parent and candidate base: `914e68fbdf4385b3a7e74304e2bcdc77cfd6e4c1`.
- Candidate source patch SHA-256: `a2a399159a825272db5b04cad6d3e41fdd0fe93f7a478cf13edd4dceb47ffa04`.
- Pinned DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust: rustc/Cargo 1.99.1, LLVM 23.1.1, release, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; parent and candidate used separate targets and the same isolated Cargo home.
- DUCC: GCC 15.2.0; SHT source used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; driver used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`.
- Case: GL T720 full-mmax spin-1 analysis, `ntheta=721`, `nphi=1458`, `phi0=0.23`, seed `1416973757`. The deterministic input fixture matches the M6 profile fixture hashes.
- Three balanced parent/candidate/DUCC process triplets ran at q1 and q8. Each worker warmed, checked repeat/reset, and recorded ten warm batches targeting 25 ms. q1 used CPU 0; q8 used CPUs 0–7.

One preceding pass completed all workers but failed before writing its ledger because the runner expected the candidate patch under a different filename. That pass was excluded. The results below are from the corrected, fully recorded rerun.

## Results

| q | Parent / candidate / DUCC median process time (ms) | Paired candidate/parent median | Candidate-faster pairs | Parent/DUCC; candidate/DUCC paired medians |
|---:|---:|---:|---:|---:|
| 1 | 36.438 / 36.418 / 29.075 | 0.992072 | 2/3 | 1.255473; 1.247802 |
| 8 | 5.236 / 5.293 / 4.201 | 1.008278 | 0/3 | 1.250409; 1.260760 |

Parent and candidate outputs were bitwise equal in all six triplets. Every backend produced finite outputs and passed repeat/reset checks. Candidate output had maximum DUCC relative-L2 error `4.760590e-14` and maximum absolute error `1.295838e-15`, matching the parent.

The q1 parent/candidate runqueue wait was 6.6–6.9%. q8 frequency samples included 1.746 GHz readings and varied across process pairs. The small q1 improvement did not repeat at q8; the three-pair screen does not establish a gain or a material regression. It also leaves the Rust/DUCC gap open.

Candidate validation passed `cargo test --release --locked --all-features --lib spin1` (49 passed, 4 ignored) and `cargo fmt --all -- --check`; `git diff --check` passed. The spin tests cover the high-order polar cutoff, mixed active/inactive blocks, GL/CC reflected fallback, poisoned scratch reuse, adjoint identities, and q1/q8 compacted analysis. No production source changed.

**Decision:** do not keep the mask branch and shift work without a supported whole-transform improvement. Preserve the negative result and exact patch in the evidence index. This is not a parity result.

## Cleanup

After verifying the durable patch, JSON, CSV, runner, and report, removed only the owned scratch root `rustsht-m6-active-mask.7W3HgJPa` and its two registered worktrees. It contained 12,841 files, 1,329,602,560 allocated bytes, and 1,294,995,959 apparent bytes. The root and worktree registrations are absent; `/tmp` returned from 45 to 46 GiB available, and free inodes increased by 12,430 to 949,018. No experiment worker remained.
