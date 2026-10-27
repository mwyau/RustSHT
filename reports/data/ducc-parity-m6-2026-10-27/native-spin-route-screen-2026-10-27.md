# Direct native-spin analysis route screen — rejected

**Decision:** do not route GL T720 full-mmax spin-1 analysis through the general direct-spin traversal. The existing scalar-conversion/paired-reflection route is 22.5–26.0× faster in the matched cells, despite the candidate producing finite output with DUCC-relative L2 error 5.94e-14.

## Phase 0 checkpoint

Before this experiment, dev was clean and matched origin/dev at f70d9845edf6bba617b9d034b1f5e1d628c429e0; there were no unpushed commits or active Cargo/Rust/benchmark workers. /tmp had 46 GiB available and 949,016 free inodes. The checkout's existing target (3.7 GiB), .venv (44 MiB), registered parked worktrees, and older /tmp experiment directories were left untouched because their ownership was not provable. No unrelated host artifact was deleted.

## Candidate and method

The candidate was based on exact parent f70d9845edf6bba617b9d034b1f5e1d628c429e0, whose production Rust files are unchanged from the source measured in the M6 ledger at 26941cf45564a9637b72b0eca87ff880472c3505. A compile-time experimental gate set fast_range to None, routing analysis through the already-existing direct spin recurrence and SIMD accumulator instead of the GL scalar-conversion/paired-reflection route. It changed no recurrence equations. A route-specific test expectation verifies that this path leaves the paired-route safety cache untouched; the numerical, workspace-reuse, poisoned-scratch, and extreme-value assertions remain enabled.

The pinned DUCC revision was 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b (0.42.0). Rust used rustc/Cargo 1.99.1, LLVM 23.1.1, release mode, and -C target-cpu=x86-64-v3 -Aunexpected_cfgs; the candidate additionally used --cfg rustsht_force_native_spin_analysis. DUCC used GCC 15.2.0, SHT -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3, and the driver overrode fast math. Each q used three interleaved parent/candidate/DUCC process triplets on the deterministic GL T720, mmax=720, ntheta=721, nphi=1458, phi0=0.23 fixture (seed 1416973757).

## Results

| q | Parent/candidate/DUCC median ms | Paired candidate/parent ratios | Median paired parent/DUCC | Median paired candidate/DUCC |
|---:|---:|---|---:|---:|
| 1 | 35.946 / 927.170 / 29.122 | 26.047, 25.770, 25.751 | 1.225 | 31.539 |
| 8 | 5.285 / 121.786 / 4.227 | 22.539, 23.027, 23.101 | 1.248 | 28.739 |

The candidate's runqueue wait was 1.1–1.2% at q1 and 0.06–0.08% at q8, so contention does not explain the slowdown. Parent q1 wait was 6.7–6.9%; one DUCC q8 sample had 3.7% wait. The candidate was slower in all six matched triplets.

All candidate outputs were finite, repeat-consistent, and reset-consistent. Candidate vs DUCC maximum relative L2 was 5.9393e-14, maximum absolute error 1.7168e-15, and maximum relative error above the 1e-12 peak threshold 3.3929e-11. The different algorithm is not bitwise equal to the parent. Candidate focused validation passed cargo test --locked --release --all-features --lib spin1 (49 passed, 4 ignored); cargo fmt --all -- --check and git diff --check passed. The inherited triplet runner returned status 2 only because it treats parent/candidate bitwise equality as its final criterion; all 18 backend workers completed and their numerical and repeat/reset records are preserved.

**Disposition:** rejected and removed from production source. The ordinary fast GL path remains unchanged. Do not retry this route gate for the selected full-mmax workload. The M6 GL T720 analysis gap remains open.

## Evidence and cleanup

- Per-triplet JSON: native-spin-route-triplets-2026-10-27.json, SHA-256 9f84e4a140e5ce2c9b7618e562ea8bfe45de6a1fbb9e812d05b63ec418696d4d.
- Per-triplet CSV: native-spin-route-triplets-2026-10-27.csv, SHA-256 7539275fbf934d1abd0466e5d7884bd199af02e8a17bd7d5d6b83d9672d98ff7.
- Recoverable candidate and test patch: native-spin-route-candidate.patch, SHA-256 8e0d496aac44c14b368d239586d75486c0728a7ed2e00f4adc47ad1385b5d072.

The exact owned scratch root measured 919,242,263 apparent bytes across 9,034 files before cleanup. After the evidence checkpoint was verified, its two clean worktrees, isolated Cargo home/targets, DUCC build, runner, fixtures, and raw outputs were removed. Post-cleanup free-space and inode counts are recorded in the repository progress entry.
