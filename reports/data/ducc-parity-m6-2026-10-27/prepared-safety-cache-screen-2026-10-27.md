# Prepared synthesis safety-check cache screen — rejected — 2026-10-27

## Candidate and decision

The candidate memoized the existing prepared-pair coefficient range check across adjacent latitude blocks sharing one prepared coefficient run. It evaluates the check lazily, only after the ordinary fused synthesis declines, so successful fused blocks do not pay the extra scan. The check remains in the low-level transition API when no cached result is supplied. This changed five files by 68 insertions and four deletions. The exact-parent patch SHA-256 is `36553db57b80244ac39325f8049097fa5d9c405b8a9449412c8c3b6380f3c341`.

**Reject the candidate.** CC T720 reduced-mmax synthesis medians were effectively neutral at q1 and q8. GL T1023 q1 had a small lead, but GL q8 did not confirm it. These were three interleaved triplets per cell with no confidence intervals; the CC q1 workers also had about 10% scheduler wait. The results do not support a repeatable end-to-end gain for the additional cache plumbing. No production source change was made.

## Validation and measurements

The candidate passed the two focused prepared-transition tests and the release all-features spin-1 suite (**49 passed, 4 ignored**). Across 12 parent/candidate/DUCC triplets, candidate and parent outputs were bitwise identical; all 36 workers passed finite-output, repeat, and reset checks. Candidate-parent numerical error was zero. Candidate/DUCC maximum relative-L2 / maximum absolute error was `3.619e-13` / `3.025e-9` for CC and `2.602e-13` / `5.410e-9` for GL.

| Case | q | Paired candidate/parent ratios | Median C/P | Median parent / candidate / DUCC process time (ms) | Median parent/candidate wait |
|---|---:|---|---:|---:|---:|
| CC T720, mmax 180, 722×378 | 1 | 0.984706, 1.009754, 1.001844 | 1.001844 | 27.833 / 27.884 / 18.621 | 10.35% / 10.30% |
| CC T720, mmax 180, 722×378 | 8 | 1.000910, 1.002108, 0.972276 | 1.000910 | 4.406 / 4.414 / 2.648 | 1.25% / 0.47% |
| GL T1023, full mmax, 1024×2048 | 1 | 1.008117, 0.983103, 0.991833 | 0.991833 | 88.352 / 87.818 / 119.909 | 2.09% / 2.07% |
| GL T1023, full mmax, 1024×2048 | 8 | 1.000487, 1.008950, 0.960905 | 1.000487 | 14.164 / 14.291 / 19.579 | 0.72% / 0.72% |

The small GL q1 lead is unconfirmed and the CC q1 screen is wait-limited; low-wait q8 results are neutral in both controls. No larger timing screen was justified for this candidate.

## Provenance

- Parent and candidate base: `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust/Cargo 1.99.1, LLVM 23.1.1; release `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, `parallel` feature. DUCC used GCC 15.2.0, pinned SHT `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and the driver override `-fno-fast-math -ffp-contract=fast`.
- Reused checked triplet runner SHA-256 `924ca5903415bbf5b9042efd1e275825160351df1e1c7b921a92bce083ee84cd`. CC fixture seed `1416779915`; GL fixture seed `1436943300`. Binary hashes and fixture/output hashes are retained in the triplet JSON.
- Candidate source file SHA-256 values: `src/legendre/mod.rs` `9f1a270e69ab8c2c7dfbd24bc7c96b1af8bb04199c2893f1e718a391d1b4d57e`; `src/legendre/scalar_paired.rs` `bd3b36ca7ad4be1e3c959903bdde6f5d2cdaac2a12436efb7648a16a1389b7a9`; `src/legendre/scalar_paired/synthesis.rs` `c98e8e06867487f26da34b720e1526d17905a1e995c3e2f1e955eab59a6e6e2a`; its existing tests file `src/legendre/scalar_paired/analysis/tests.rs` `12b91301aabbdcb037cd802cd2cc397f5f0281dfb24919da1825ceaa61f2d5c0`; and `src/transform/spin/synthesis.rs` `120bfe48a45f3370bb1d1b94c9d1c68531bdd80668988d3d39c838397c6315e2`.

## Cleanup

The patch, report, sanitized triplet JSON/CSV, fixture hashes, and binary hashes were verified before cleanup. Removed only the experiment-owned scratch root labeled `rustsht-m6-prepared-check.bDPRWW6M` and its two registered worktrees. The scratch measured 1,568,382,753 apparent bytes across 10,791 paths. Available temporary-filesystem space rose from 47,501,451,264 to 49,092,861,952 bytes (+1,591,410,688); free inodes rose from 938,225 to 949,016 (+10,791). The root and worktree registrations are absent, and no build, benchmark, or profiler process remains.
