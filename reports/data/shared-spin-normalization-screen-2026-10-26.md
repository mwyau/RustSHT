# Shared spin normalization: focused f64 screen

Candidate `7402fbb67de9beea1f5ebbeb1a1ebfb1ca171e9a` versus exact parent `376ac1c3317e6b887817fa4aa78cee5f580b461e`; native DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. This measures shared f32/f64 spin normalization on four f64 GL full-order spin-1 cells. Three fresh-process interleaved triplets per cell; no f32 baseline was rerun.

| Cell | Parent ms | Candidate ms | DUCC ms | Paired C/P median (range) | Paired C/DUCC |
|---|---:|---:|---:|---:|---:|
| GL T720 analysis q1 | 36.341 | 36.309 | 29.428 | 0.9927 (0.9895–1.0047) | 1.2338 |
| GL T720 analysis q8 | 5.330 | 5.326 | 4.272 | 0.9906 (0.9843–1.0079) | 1.2443 |
| GL T1023 synthesis q1 | 88.654 | 88.625 | 119.430 | 1.0061 (0.9957–1.0117) | 0.7387 |
| GL T1023 synthesis q8 | 14.105 | 14.057 | 20.085 | 0.9966 (0.9905–1.0079) | 0.7071 |

All 12 candidate/parent comparisons are bitwise identical; all 36 worker executions pass finite-output, repeat and poisoned-destination reset checks: `True`. Maximum candidate/DUCC relative L2 is `2.601774e-13`. Numerical error was measured directly; no tolerance was changed.

Milliseconds are medians of process medians. Each paired ratio uses the matched triplet; ranges are observed three-triplet dispersion, not confidence intervals. The rotating backend orders are parent/DUCC/candidate, candidate/DUCC/parent, and DUCC/parent/candidate; this is interleaving, not full positional balance. The CSV preserves every batch sample, per-triplet dispersion, fixture/binary/output hashes and check result. This small screen does not establish a runtime win or a complete performance matrix.

Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; `g++ (Ubuntu 15.2.0-16ubuntu1) 15.2.0`. Host: AMD Ryzen 9 5950X 16-Core Processor. Rust uses Cargo release, default+parallel features, and `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`. Both Cargo fingerprints were checked. DUCC was freshly built from the pinned source with the tracked native spin-1 CMake harness; no compiled reference objects were reused.
DUCC numerical flags: `-DDUCC0_NAMESPACE=ducc0 -DPKGNAME=ducc0 -DPKGVERSION=0.42.0 -O3 -DNDEBUG -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`.
DUCC driver flags: `-DDUCC0_NAMESPACE=ducc0 -DPKGNAME=ducc0 -DPKGVERSION=0.42.0 -O3 -DNDEBUG -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3 -fno-fast-math -ffp-contract=fast`.

Cases use existing `scripts/benchmark_ducc_spin1_native.py` PCG64 fixtures, exact GL latitudes, even smooth longitude count, and `phi0=0.23`: T720 is 721×1458; T1023 is 1024×2048. The existing common pinned-process harness runs q1 on CPU 0, q8 on CPUs 0–7, with monitoring on CPU 8. Target batches are 25 ms; actual short/long routes and sample counts remain in the CSV. Whole-worker scheduler-wait samples include validation and output serialization, so they are diagnostic rather than a correction to timed transform latency.

- GL T720 analysis q1 median whole-worker wait: parent 6.90%; candidate 6.88%; ducc 0.05%.
- GL T720 analysis q8 median whole-worker wait: parent 1.10%; candidate 1.15%; ducc 0.26%.
- GL T1023 synthesis q1 median whole-worker wait: parent 2.13%; candidate 2.06%; ducc 0.04%.
- GL T1023 synthesis q8 median whole-worker wait: parent 0.41%; candidate 0.36%; ducc 0.12%.

Owned scratch label: `rustsht-spin-norm-exp.JTpHnpaQ`. Symbolic targets: `CARGO_TARGET_DIR=$TMPDIR/rustsht-spin-norm-exp.JTpHnpaQ/parent-target`, `CARGO_TARGET_DIR=$TMPDIR/rustsht-spin-norm-exp.JTpHnpaQ/candidate-target`, and reference build `$TMPDIR/rustsht-spin-norm-exp.JTpHnpaQ/ducc-build`. Both Rust builds used `cargo build --release --locked --offline --jobs 1 --example ducc_spin1_full_transform_parity`; the fresh CMake reference build used `--parallel 1`. No builds or measurements overlapped. Durable evidence is this report and `shared-spin-normalization-screen-2026-10-26.csv`. After evidence commit `5ae2a655be2db4cc77616b7ccefb9d893a4ec4b3` was checksum-verified, the worker removed this scratch and both temporary worktrees: 1,636,384,768 bytes and 9,091 inodes reclaimed. Their absence and return to 46 GiB/949,018 free inodes were verified; no experiment processes remained.
