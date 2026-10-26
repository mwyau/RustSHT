# Shared checked f64 lane-pair normalization: focused regression screen

Measured starting `2026-10-26T22:56:50.728837+00:00`. Candidate `e3b9b7fe3866ab7ed3e1bb1315e5f1527b8d6414` versus exact parent `f8ab2a3d7e835ecff099b8b5fe9c20cfa16a14e8`. The source change shares the always-inlined checked f64 scalar/spin lane-pair normalizer. Eight GL full-mmax f64 cells, three alternating fresh-process parent/candidate pairs per cell. Scalar and spin-1 T720 use 721×1458; protected spin-1 T1023 synthesis uses 1024×2048; phi0=0.23.

Reference context only: immutable pre-refactor source `062218639bc2f097f004c617da0ce46a3a4d2941` and pinned native DUCC source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Neither reference was rebuilt or measured here; no immutable f32 baseline or broad matrix was rerun.

| Family / T / operation / q | Pair | Parent ms | Candidate ms | C/P | Parent wait | Candidate wait |
|---|---:|---:|---:|---:|---:|---:|
| scalar T720 analysis q1 | 1 | 15.375373 | 15.050739 | 0.978886 | 10.93% | 10.78% |
| scalar T720 analysis q1 | 2 | 15.047283 | 15.031345 | 0.998941 | 10.92% | 10.94% |
| scalar T720 analysis q1 | 3 | 15.015856 | 15.016442 | 1.000039 | 10.93% | 10.75% |
| scalar T720 analysis q8 | 1 | 2.377401 | 2.283858 | 0.960653 | 2.91% | 1.59% |
| scalar T720 analysis q8 | 2 | 2.199582 | 2.214961 | 1.006992 | 0.58% | 0.80% |
| scalar T720 analysis q8 | 3 | 2.226082 | 2.245192 | 1.008585 | 2.13% | 0.94% |
| scalar T720 synthesis q1 | 1 | 14.640632 | 14.507545 | 0.990910 | 7.73% | 7.76% |
| scalar T720 synthesis q1 | 2 | 14.600684 | 14.538406 | 0.995735 | 7.79% | 7.76% |
| scalar T720 synthesis q1 | 3 | 14.472221 | 14.629408 | 1.010861 | 7.77% | 7.77% |
| scalar T720 synthesis q8 | 1 | 2.020337 | 2.056527 | 1.017912 | 0.26% | 0.51% |
| scalar T720 synthesis q8 | 2 | 2.031949 | 2.033309 | 1.000669 | 0.27% | 0.37% |
| scalar T720 synthesis q8 | 3 | 2.035122 | 2.026454 | 0.995740 | 2.46% | 0.39% |
| spin1 T720 analysis q1 | 1 | 37.139088 | 36.204086 | 0.974824 | 6.61% | 6.63% |
| spin1 T720 analysis q1 | 2 | 36.119076 | 36.512323 | 1.010888 | 6.45% | 6.68% |
| spin1 T720 analysis q1 | 3 | 36.614877 | 36.290043 | 0.991128 | 6.65% | 6.47% |
| spin1 T720 analysis q8 | 1 | 5.350306 | 5.388233 | 1.007089 | 0.85% | 0.95% |
| spin1 T720 analysis q8 | 2 | 5.260145 | 5.378881 | 1.022573 | 0.85% | 0.59% |
| spin1 T720 analysis q8 | 3 | 5.264286 | 5.283786 | 1.003704 | 0.55% | 1.08% |
| spin1 T1023 synthesis q1 | 1 | 91.249031 | 88.703650 | 0.972105 | 2.09% | 2.08% |
| spin1 T1023 synthesis q1 | 2 | 90.149768 | 90.417399 | 1.002969 | 2.15% | 2.10% |
| spin1 T1023 synthesis q1 | 3 | 89.892882 | 88.547566 | 0.985034 | 2.05% | 2.07% |
| spin1 T1023 synthesis q8 | 1 | 14.471319 | 14.242517 | 0.984189 | 0.29% | 0.42% |
| spin1 T1023 synthesis q8 | 2 | 14.259880 | 14.359853 | 1.007011 | 0.66% | 0.68% |
| spin1 T1023 synthesis q8 | 3 | 14.115449 | 14.241580 | 1.008936 | 0.42% | 0.29% |

| Cell | Parent median ms | Candidate median ms | Paired C/P median | Observed paired range |
|---|---:|---:|---:|---:|
| scalar T720 analysis q1 | 15.047283 | 15.031345 | 0.998941 | 0.978886–1.000039 |
| scalar T720 analysis q8 | 2.226082 | 2.245192 | 1.006992 | 0.960653–1.008585 |
| scalar T720 synthesis q1 | 14.600684 | 14.538406 | 0.995735 | 0.990910–1.010861 |
| scalar T720 synthesis q8 | 2.031949 | 2.033309 | 1.000669 | 0.995740–1.017912 |
| spin1 T720 analysis q1 | 36.614877 | 36.290043 | 0.991128 | 0.974824–1.010888 |
| spin1 T720 analysis q8 | 5.264286 | 5.378881 | 1.007089 | 1.003704–1.022573 |
| spin1 T1023 synthesis q1 | 90.149768 | 88.703650 | 0.985034 | 0.972105–1.002969 |
| spin1 T1023 synthesis q8 | 14.259880 | 14.242517 | 1.007011 | 0.984189–1.008936 |

All 24 parent/candidate output comparisons are bitwise identical, with zero relative L2 and max absolute difference. All 48 outputs are finite and native repeat checks pass: `True`. All 24 spin-worker poisoned-destination reset checks pass: `True`. The scalar worker does not explicitly poison destinations; those 24 scalar checks are unsupported/not run. No tolerance was changed.

Each cell is recorded separately; small differences and three-pair observed ranges are not confidence intervals or runtime-win proof. A consistent positive direction is a possible regression to assess under the f64 performance guardrail, rather than a maintainability waiver. Whole-worker wait/frequency diagnostics include fixture/setup, validation and output serialization and do not isolate or correct timed transform latency. Pair orders are parent/candidate, candidate/parent, parent/candidate. Every timed batch, actual sample count, p10/p90 and fixture/binary/output hash is retained in the CSV.

- spin1 T720 analysis q8: all three paired ratios exceed parity (1.003704–1.022573, median 1.007089); possible small regression requiring an explicit f64 decision.

Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1; host: AMD Ryzen 9 5950X 16-Core Processor. Both source builds freshly compile scalar and spin workers with Cargo release default+parallel and `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`, verified by all four fingerprints. Build options: `--locked --offline --jobs 1`. Existing common scalar and native spin-1 PCG64 fixtures and worker/pinning harnesses were reused. q1 CPU 0; q8 CPUs 0–7; monitor CPU 8; target batches 25 ms. No builds or measurement processes overlap.

Owned scratch label: `rustsht-f64-pair-normalizer-exp.978DkWIC`. Temporary worktrees, private `CARGO_HOME=$TMPDIR/rustsht-f64-pair-normalizer-exp.978DkWIC/cargo-home`, parent/candidate targets, caches, logs, fixtures and binaries were confined to this scratch. Only lockfile source archives/index were copied; compiled objects were not reused. Compact evidence is written and read back outside scratch before immediate owned-artifact cleanup. The main agent owns the source decision and evidence commit.

Cleanup verified: scratch `rustsht-f64-pair-normalizer-exp.978DkWIC` and both registered worktrees are absent; no owned processes remain. Reclaimed 1,521,147,904 allocated bytes and 11,191 inodes after the durable CSV/report were written and read back. No unrelated paths were changed.
