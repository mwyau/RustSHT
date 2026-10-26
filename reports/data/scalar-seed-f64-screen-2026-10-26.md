# Shared scalar seed initialization: focused f64 regression screen

Measured starting `2026-10-26T22:31:29.798777+00:00`. Candidate `66145f8724426bff234897433744cd6102784228` versus exact parent `2475a80949a9aa5e548795f2d981750abc70c536`. GL T720 full-mmax f64 scalar analysis and synthesis, q1/q8, 721×1458, phi0=0.23. Three alternating fresh-process pairs per cell. No pre-refactor baseline rerun, DUCC rebuild/reference measurement or broader matrix.

| Operation | q | Pair | Parent ms | Candidate ms | C/P | Parent wait | Candidate wait |
|---|---:|---:|---:|---:|---:|---:|---:|
| analysis | 1 | 1 | 15.007794 | 15.502433 | 1.032959 | 10.92% | 10.91% |
| analysis | 1 | 2 | 14.879312 | 14.973019 | 1.006298 | 10.98% | 10.94% |
| analysis | 1 | 3 | 14.989485 | 14.848399 | 0.990588 | 10.98% | 10.58% |
| analysis | 8 | 1 | 2.166266 | 2.185186 | 1.008734 | 0.68% | 0.19% |
| analysis | 8 | 2 | 2.174120 | 2.184001 | 1.004545 | 0.52% | 0.46% |
| analysis | 8 | 3 | 2.176490 | 2.235885 | 1.027289 | 0.56% | 0.25% |
| synthesis | 1 | 1 | 14.601083 | 14.640382 | 1.002692 | 7.49% | 7.41% |
| synthesis | 1 | 2 | 14.502215 | 14.552575 | 1.003473 | 7.77% | 7.62% |
| synthesis | 1 | 3 | 14.584670 | 14.581011 | 0.999749 | 7.72% | 7.75% |
| synthesis | 8 | 1 | 2.037247 | 2.029649 | 0.996270 | 0.49% | 0.34% |
| synthesis | 8 | 2 | 2.069871 | 2.032849 | 0.982114 | 0.57% | 0.44% |
| synthesis | 8 | 3 | 2.056815 | 2.043023 | 0.993295 | 0.63% | 0.39% |

| Operation | q | Parent median ms | Candidate median ms | Paired C/P median | Observed paired range |
|---|---:|---:|---:|---:|---:|
| analysis | 1 | 14.989485 | 14.973019 | 1.006298 | 0.990588–1.032959 |
| analysis | 8 | 2.174120 | 2.185186 | 1.008734 | 1.004545–1.027289 |
| synthesis | 1 | 14.584670 | 14.581011 | 1.002692 | 0.999749–1.003473 |
| synthesis | 8 | 2.056815 | 2.032849 | 0.993295 | 0.982114–0.996270 |

All 12 parent/candidate output comparisons are bitwise identical, with zero relative L2 and max absolute difference; all 24 outputs are finite and native repeat/value-reset checks pass: `True`. The existing scalar worker does not explicitly poison destinations; poisoned-destination validation is unsupported/not run. Its optional broader validation route was disabled. No tolerance was changed.

Analysis q8 was slower in all three recorded pairs (0.45–2.73%, paired median +0.87%); this is a possible small regression that merits confirmation before a material f64 performance decision. Analysis q1 was mixed (+3.30%, +0.63%, −0.94%). These observations are retained per cell rather than averaged away.

This three-pair screen records dispersion rather than confidence intervals; small differences do not establish a runtime win or a complete performance certification. Every batch sample, p10/p90, fixture/binary/output hash and check flag is retained in the CSV. Whole-worker scheduler wait/frequency samples include fixture/setup and output serialization, and do not isolate or correct timed transform latency. Pair orders are parent/candidate, candidate/parent, parent/candidate.

Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1; host: AMD Ryzen 9 5950X 16-Core Processor. Fresh release builds use default+parallel features and `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`, confirmed by fingerprints, with `--locked --offline --jobs 1`. Existing common scalar PCG64 fixtures and `examples/ducc_full_transform_parity.rs` worker were reused. q1 CPU 0; q8 CPUs 0–7; monitor CPU 8; 25 ms target batches. No builds or measurement processes overlap.

Owned scratch label: `rustsht-scalar-seed-exp.hmDnOjmJ`. Temporary worktrees, private `CARGO_HOME=$TMPDIR/rustsht-scalar-seed-exp.hmDnOjmJ/cargo-home`, parent/candidate targets, caches, logs, fixtures and binaries were confined to this scratch. Only lockfile source archives/index were copied; compiled objects were not reused. Per experiment hygiene, compact evidence is saved and verified outside scratch before all owned transient artifacts are removed. The main agent owns the production decision and evidence commit.

Cleanup verified: owned scratch `rustsht-scalar-seed-exp.hmDnOjmJ` and both registered worktrees are absent; no owned processes remain. Reclaimed 935,116,800 allocated bytes and 11,105 inodes after the durable CSV/report were written and read back. No unrelated paths were changed.
