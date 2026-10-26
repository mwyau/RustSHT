# Shared native-spin seed construction: focused f64 regression screen

Candidate `25b877edacce5c99c1a361ded2694c831bc29749` versus exact parent `c4ab9b6d8f58a816f4b30eaf91889760c754c3ee`. GL full-mmax f64 spin-1: T1023 synthesis q1/q8 (1024×2048) and T720 analysis q1/q8 (721×1458), phi0=0.23. Three alternating fresh-process parent/candidate pairs per cell; no DUCC rebuild/reference measurement, f32 baseline or broader matrix.

| Cell | Pair | Parent ms | Candidate ms | C/P | Parent wait | Candidate wait |
|---|---:|---:|---:|---:|---:|---:|
| T1023 synthesis q1 | 1 | 88.946553 | 87.814730 | 0.987275 | 2.14% | 2.16% |
| T1023 synthesis q1 | 2 | 87.874066 | 88.268123 | 1.004484 | 2.18% | 2.14% |
| T1023 synthesis q1 | 3 | 87.449232 | 87.763087 | 1.003589 | 2.13% | 2.15% |
| T1023 synthesis q8 | 1 | 14.327208 | 14.232606 | 0.993397 | 0.64% | 0.61% |
| T1023 synthesis q8 | 2 | 14.077547 | 14.048547 | 0.997940 | 0.34% | 0.55% |
| T1023 synthesis q8 | 3 | 14.072638 | 14.026270 | 0.996705 | 0.55% | 0.25% |
| T720 analysis q1 | 1 | 36.520904 | 36.606682 | 1.002349 | 6.85% | 6.90% |
| T720 analysis q1 | 2 | 36.374858 | 36.116644 | 0.992901 | 6.86% | 6.92% |
| T720 analysis q1 | 3 | 36.484625 | 36.102147 | 0.989517 | 6.62% | 6.86% |
| T720 analysis q8 | 1 | 5.287043 | 5.327304 | 1.007615 | 1.38% | 0.77% |
| T720 analysis q8 | 2 | 5.362704 | 5.327725 | 0.993477 | 0.53% | 0.87% |
| T720 analysis q8 | 3 | 5.430601 | 5.481834 | 1.009434 | 0.95% | 1.64% |

| Cell | Parent median ms | Candidate median ms | Paired C/P median | Observed paired range |
|---|---:|---:|---:|---:|
| GL T1023 synthesis q1 | 87.874066 | 87.814730 | 1.003589 | 0.987275–1.004484 |
| GL T1023 synthesis q8 | 14.077547 | 14.048547 | 0.996705 | 0.993397–0.997940 |
| GL T720 analysis q1 | 36.484625 | 36.116644 | 0.992901 | 0.989517–1.002349 |
| GL T720 analysis q8 | 5.362704 | 5.327725 | 1.007615 | 0.993477–1.009434 |

All 12 candidate/parent output comparisons are bitwise identical, with zero relative L2 and max absolute difference. All 24 worker executions pass finite-output, repeat and poisoned-destination reset checks: `True`. No tolerance was changed. The CSV retains every timed batch, actual sample counts, p10/p90 dispersion, whole-worker wait/frequency diagnostics and fixture/binary/output hashes.

This is a three-pair regression screen; small differences and observed ranges do not establish a runtime win or a complete performance certification. Ranges are not confidence intervals. Whole-worker scheduler diagnostics include fixture/setup, validation and output serialization, so they do not isolate or correct timed transform latency. Pair orders are parent/candidate, candidate/parent, parent/candidate.

Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1; host: AMD Ryzen 9 5950X 16-Core Processor. Both fresh Cargo release builds use default+parallel features and `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`, confirmed by fingerprints. Build options: `--locked --offline --jobs 1`. Existing native spin-1 PCG64 fixtures and pinned-worker harness were reused. q1 CPU 0; q8 CPUs 0–7; monitor CPU 8; 25 ms target batches. No builds or measurements overlap.

Owned scratch label: `rustsht-spin-seed-exp.BVezdahs`. All worktrees, private `CARGO_HOME=$TMPDIR/rustsht-spin-seed-exp.BVezdahs/cargo-home`, parent/candidate targets, caches, logs, fixtures, binaries and raw output remain within this scratch pending tracked evidence verification on origin/dev. Only source archives/index were copied into the private Cargo home; no compiled objects were reused.
