# Shared ordinary scalar traversal — focused f64 timing screen

**Date:** 2026-10-26 23:16 UTC (screen start)  
**Candidate:** `f438918faf96876050cf523a3a8aaa62d87af9a2`  
**Exact parent:** `0a74927f5c12765ffaf24cb5c7d02895fbef6a2a`  
**Immutable pre-refactor baseline:** `062218639bc2f097f004c617da0ce46a3a4d2941` (not rebuilt for this screen).

## Method

Three interleaved exact-parent/candidate pairs per cell, GL T720, full mmax (L=M=720), ntheta=721, nphi=1458, even smooth longitude mode, native f64 scalar, q1/q8. Each process reported 10 warmed batch measurements (2 transforms per batch; 25 ms target batch). Parent/candidate execution order alternated. Fixture SHA-256: alm `3287233ee711f55c89f036cedeb3e73dcee3397b8059444063e048f024496bc0`, map `e5929b5372ec1d08c3b5500993a38b5a8661d4fa26c80cb07f3434a24b40d469`. No DUCC build/reference was measured. Rust 1.99.1 / Cargo 1.99.1, AMD Ryzen 9 5950X, `RUSTFLAGS="-C target-cpu=x86-64-v3 -Aunexpected_cfgs"`; parent/candidate binaries used separate isolated target directories.

Binary SHA-256: parent `0970c7e9fde922c2e4afd9493b7657253637396b338493763fb767f8a4aac61b`, candidate `5014babfbc6767f3f8646302eaf5176ce52ad6c687be7388a461b90214d8e0d9`.

## Results

| Route | Median paired C/P ratio | Per-pair C/P ratios | Wait / interpretation |
|---|---:|---|---|
| analysis q1 | 0.991249 | 0.993558, 0.991249, 0.974173 | high host wait (10.8%–11.0%); timing is wait-affected |
| analysis q8 | 1.003761 | 1.003761, 1.003445, 1.004698 | low host wait (0.2%–0.7%); candidate slower in 3/3 pairs |
| synthesis q1 | 0.993349 | 0.993349, 0.994279, 0.983620 | high host wait (7.7%–7.8%); timing is wait-affected |
| synthesis q8 | 1.004351 | 1.003605, 1.004351, 1.012975 | low host wait (0.3%–0.7%); candidate slower in 3/3 pairs |

The q1 medians are not used to claim a speedup because host wait was high. At q8, the candidate was slightly slower in all three pairs: analysis median +0.376% (range +0.345% to +0.470%) and synthesis median +0.435% (range +0.360% to +1.298%). These are small observed differences; this three-pair screen does not establish a material regression or a general runtime conclusion. The shared traversal is retained for its substantial removal of the separately maintained f32 ordinary traversal. Preserve the typed recurrence/normalization leaves and the exact f64 sectoral seed.

All 12 outputs were byte-identical and finite; all 24 repeat/reset consistency checks passed. The worker does not support poisoned-destination validation, so that check was not run. The complete per-pair batch timings, wait/frequency diagnostics, output hashes, and checks are in the adjacent CSV.

## Cleanup

The source candidate and its validation record are committed on `dev`. After verifying this report and CSV were readable, removed the exact owned tmpfs scratch labeled `rustsht-scalar-traversal-exp.GcJhfbGD`, including its clean parent/candidate worktrees, isolated targets, Cargo home and raw output. The scratch and both worktree registrations are absent. It measured 912,689,723 apparent bytes / 9,343 files before cleanup; available tmpfs space rose by 465,141,760 bytes and free inodes rose by 3,158 across the final removal step. Historical unrelated/branch-backed worktrees and persistent checkout targets/venvs were left untouched.
