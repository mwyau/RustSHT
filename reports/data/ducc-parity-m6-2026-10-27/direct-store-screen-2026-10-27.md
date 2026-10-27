# Direct writes to cleared paired-synthesis spectrum slots — rejected

## Candidate and scope

The candidate follows exact parent `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; patch SHA-256 `de5863798bf82495d2a235c77ea4d9e7e75682019e26c57b8e5383d2d888a037`. In `synthesize_prepared_order_pair_block_dispatched`, it replaces Q/U `Complex64 +=` with direct stores of the newly computed pair sums. The `0.0 + sum` form preserves the existing zero-plus-sum signed-zero behavior. The call-site audit found the destination order is cleared before synthesis, and each ring/order segment is written once. No recurrence, factor, fallback, or scaling arithmetic changed. The candidate stayed in an isolated worktree and was not applied to production.

## Validation and measurement

Candidate validation passed `cargo test --locked --release --all-features --lib spin1 -- --test-threads=1` (49 passed, 4 ignored, 284 filtered), `cargo fmt --all -- --check`, and `git diff --check`. Rust used rustc/Cargo 1.99.1, LLVM 23.1.1, release mode, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2.0 with the checked SHT and driver flags. Host: AMD Ryzen 9 5950X.

Fifteen balanced parent/candidate/DUCC triplets per q ran on exact CC T720 reduced-mmax spin-1 synthesis (`mmax=180`, `ntheta=722`, `nphi=378`, `phi0=0.23`; fixture seed `1416779915`). Each worker checked repeated calls and poisoned-output reset, then recorded ten 25 ms target batches.

| q | Parent / candidate / DUCC median process time (ms) | Paired C/P median [100k-resample percentile bootstrap 95% CI] | Candidate faster | Median parent/candidate wait |
|---:|---:|---:|---:|---:|
| 1 | 27.823 / 27.796 / 18.559 | 0.997470 [0.992019, 1.002035] | 9/15 | 10.34% / 10.34% |
| 8 | 4.426 / 4.436 / 2.657 | 1.008259 [0.982173, 1.010737] | 5/15 | 0.69% / 0.71% |

Parent and candidate outputs were bitwise equal in all 30 triplets. All 90 workers were finite and passed repeat/reset. Candidate-parent errors were zero; candidate/DUCC maximum relative-L2 was `3.619e-13`, maximum absolute error `3.025e-9`. Both q intervals include parity; the small q1 lead was only 9/15 wins, and q8 had no supported gain.

**Decision:** reject the direct-store candidate. The matched measurements do not establish a whole-transform gain, so retain the current `+=` implementation. The screen does not establish a new DUCC parity ratio. Full per-worker timings, diagnostics, hashes, and errors are in the [triplet JSON](direct-store-triplets.json) and [CSV](direct-store-triplets.csv); the [candidate patch](direct-store-candidate.patch) and [15-pair runner](direct-store-triplet-runner.py) preserve the experiment.

## Cleanup

The experiment used only owned scratch root `rustsht-m6-direct-store.fCds2w2R`, including its two registered worktrees, isolated Cargo home/targets, pinned DUCC source/build, fixture, binaries, runner, and raw outputs. Before cleanup, `du` reported 1,168,195,584 apparent bytes across 9,177 files. Cleanup removed only that exact root and its two worktree registrations after evidence verification. `temporary filesystem` available bytes rose from 47,924,666,368 to 49,092,861,952 (+1,168,195,584); free inodes rose from 938,466 to 949,016 (+10,550). The root and registrations are absent, and no experiment worker remains.
