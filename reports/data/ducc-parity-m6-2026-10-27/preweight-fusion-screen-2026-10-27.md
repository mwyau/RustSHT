# M6 — paired preweight and cosine-square fusion screen

**Disposition:** candidate removed; no supported f64 speedup.

The shared spin-1 recurrence setup copied four opposite-channel arrays into local scratch, traversed Q and U in separate cosine-preweight safety loops, then traversed cosine again to form `cos²`. The candidate replaced those operations with one per-lane pass that reads both channels, forms and validates the four weighted values, writes only to local scratch, and computes `cos²`. Any unsafe product still returns `None` before staged coefficients are committed, so the caller keeps the same decline and fallback path.

## Provenance and method

- Exact parent and candidate base: `85fd4998827619887c6d1642330ceb614b014935`.
- Measured candidate patch SHA-256: `a232afd292df1b9acc822fc93f301f42e34afcaa2e4e83b05cc88def12a9097c`.
- Recoverable zero-context patch SHA-256: `fcfe352b69c12de8a24a7aec11ae86467d31eda05cf9c3da9d90fec4bce2a404` (apply with `git apply --unidiff-zero`).
- Pinned DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust: rustc/Cargo 1.99.1, LLVM 23.1.1, release, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; parent and candidate used separate targets and an experiment-local Cargo home.
- DUCC: GCC 15.2.0; SHT source used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; driver used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`.
- Case: GL T720 full-mmax spin-1 analysis, `ntheta=721`, `nphi=1458`, `phi0=0.23`, seed `1416973757`. The deterministic fixtures match the M6 profile input hashes.
- Three balanced parent/candidate/DUCC triplets ran at each q. Each worker warmed, checked repeat/reset, and recorded ten warm batches targeting 25 ms. q1 used CPU 0; q8 used CPUs 0–7.

## Results

| q | Parent / candidate / DUCC median process time (ms) | Paired candidate/parent median | Candidate-faster pairs | Parent/DUCC; candidate/DUCC paired medians |
|---:|---:|---:|---:|---:|
| 1 | 36.177 / 36.068 / 29.063 | 1.001462 | 1/3 | 1.243921; 1.239544 |
| 8 | 5.248 / 5.217 / 4.230 | 0.998948 | 2/3 | 1.241512; 1.237274 |

Parent and candidate outputs were bitwise equal in all six triplets. Every backend produced finite outputs and passed repeat/reset checks. Candidate output had maximum DUCC relative-L2 error `4.760590e-14` and maximum absolute error `1.295838e-15`, matching the parent.

q1 runqueue wait was 6.0–6.9%; q8 wait was 0.6–1.2%. q8 frequency sampling included 1.746 GHz readings. The q1/q8 results are near-neutral and split across process pairs; three triplets do not support a repeatable gain or material regression. The candidate was not extended and does not close the Rust/DUCC gap.

Candidate validation passed `cargo test --release --locked --all-features --lib legendre::scalar_paired::analysis` (34 passed, 8 ignored), `cargo test --release --locked --all-features --lib spin1` (49 passed, 4 ignored), and `cargo fmt --all -- --check`; `git diff --check` passed. These suites cover scaled recurrence decline, subnormal cosine weighting, GL/CC/F1 fallback, poisoned scratch, adjoints, high-order cutoffs, and q1/q8 analysis. No production source changed.

**Decision:** remove the fused preweight setup because the matched whole-transform screen does not show a supported improvement. Preserve the source diff and sanitized numerical evidence in the evidence index.

## Cleanup

After saving and verifying the recoverable patch, sanitized triplet JSON, CSV, runner, and report, removed only the owned scratch root `rustsht-m6-preweight-fusion.2ccEyRrg` and its two registered worktrees. It contained 12,850 files, 1,329,721,344 allocated bytes, and 1,295,118,456 apparent bytes. The root and worktree registrations are absent; `/tmp` returned from 45 to 46 GiB available, and free inodes increased by 14,474 to 949,018. No experiment worker remained.
