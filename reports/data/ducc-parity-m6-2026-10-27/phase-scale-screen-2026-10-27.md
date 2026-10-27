# Shared phase-scale and FMA rotation screen — 2026-10-27

## Candidate

On exact parent `f70d9845edf6bba617b9d034b1f5e1d628c429e0`, the isolated candidate combines the common per-ring scale with the order phase once, then rotates both Q and U spectra with explicit scalar FMA operations. It changes only the four reflected-spectrum multiplications in `analyze_spin_scalar_reflected_order`; recurrence, safety checks, fallback, and coefficient staging are untouched. The recoverable patch SHA-256 is `062113589c586100665fd4b60776fe1de4609ece9f29e29638a4743a9aaba139`. It was not applied to production.

## Validation and method

Candidate validation passed `cargo test --locked --release --all-features --lib spin1` (49 passed, 4 ignored), `cargo fmt --all -- --check`, and `git diff --check`. The tests exercise spin-1 analysis and adjoints, GL/CC/F1, fallback and extreme inputs, poisoned workspace reuse, and route equivalence. Parent and candidate differ at roundoff scale: relative L2 (1.07\times10^{-15}), maximum absolute (4.17\times10^{-17}). Candidate/DUCC error was relative L2 (4.76\times10^{-14}), maximum absolute (1.30\times10^{-15}).

Rust used rustc/Cargo 1.99.1, LLVM 23.1.1, release mode, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; runtime Pulp dispatch was retained. DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2 with SHT `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, while the driver overrides fast math. The host was an AMD Ryzen 9 5950X. The GL T720 full-mmax case used `ntheta=721`, `nphi=1458`, `phi0=0.23`, seed `1416973757`, and the same finite fixture as prior M6 screens. Each worker checked warm repeat/reset and collected ten 25 ms target batches. Fifteen balanced parent/candidate/DUCC triplets ran at each of q1 and q8.

| q | Parent / candidate / DUCC median process time (ms) | Paired C/P median [bootstrap 95% CI] | Candidate faster | Paired P/D; C/D medians |
|---:|---:|---:|---:|---:|
| 1 | 36.325 / 36.109 / 29.309 | 0.995288 [0.984092, 1.007003] | 11/15 | 1.238098; 1.233333 |
| 8 | 5.342 / 5.330 / 4.259 | 0.998563 [0.976639, 1.024761] | 8/15 | 1.243247; 1.239262 |

The q1 parent/candidate median scheduler wait was 6.65%/6.67%, balanced but elevated. q8 median wait was 1.23%/0.89%; the q8 candidate/parent ratio spread was wide. All 30 triplets had finite outputs and passed repeat/reset checks. Candidate outputs were not bitwise identical to the parent, as expected from the arithmetic reordering; every candidate/parent relative-L2 and absolute error stayed below the spin-1 test tolerance. Per-triplet timings, diagnostics, hashes, and numerical errors are in the [CSV](phase-scale-triplets-2026-10-27.csv) and [JSON](phase-scale-triplets-2026-10-27.json).

## Decision and cleanup

The candidate's medians trend slightly faster, but both q1/q8 bootstrap intervals include 1 and q8 had only 8/15 faster pairs. The measured M6 gap remains about 1.23× DUCC. The small neutral result does not justify changing f64 phase arithmetic; candidate is rejected and preserved for provenance. No production source changed.

Cleanup removed only owned scratch root `rustsht-m6-phase-scale.9C0pr88s` and its two registered worktrees after verifying the report, CSV, JSON, runner, and candidate patch. The scratch held 9,453 files and 1,615,943,895 apparent bytes. The root and worktree registrations are absent; no benchmark/build worker remained. Free temporary filesystem space increased by 1,639,022,592 bytes and free inodes increased by 10,890.
