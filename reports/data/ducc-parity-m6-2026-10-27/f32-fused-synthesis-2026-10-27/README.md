# Shared fused scalar synthesis recurrence — 2026-10-27

Source `a30537bdcfa0fc5273985abfd592998325a608de` follows exact parent `972bacdaef57388509738d1c2cdebbb7973b89be`. The numerical reference is native DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. This is a focused T720 scalar-synthesis milestone, not full f32/f64 parity certification.

## Change

`FusedSynthesisBlock` now owns one generic degree-pair recurrence and complex accumulation loop. The f32 and f64 precision leaves supply their native Pulp vectors and arithmetic. The f32 prepared route enters the fused block only for a normal seed block; it checks recurrence values as it advances and keeps sums staged until they are finite. If a check fails, it restores the original staged sums and uses the existing scaled paired visitor. The distinct f64 paired-channel kernel and its f64 SIMD path remain separate.

The trace used a deterministic full-mmax GL T720 q8 fixture: 74,402,963 degree/latitude contributions; 89.39% had exponent zero and 79.80% were all-lane normal. The monomorphized V3 f32 kernel contains AVX2/FMA vector arithmetic and vector range comparisons. The trace totals and assembly excerpt are preserved below.

## Matched Rust/DUCC measurements

All times are per-transform medians in milliseconds. Candidate/parent is the median of the interleaved paired ratios. The initial q1 cells and q8 acceptance cells use separate experiments and fixtures recorded in their JSON sidecars.

| GL T720 scalar synthesis | Parent / candidate / DUCC | Candidate / parent | Candidate / DUCC | Triplets |
|---|---:|---:|---:|---:|
| f32 full M720 q1 | 1068.290 / 906.982 / 19.003 | 0.8490 | 47.66× | 3; all candidate runs faster |
| f32 reduced M180 q1 | 416.851 / 300.078 / 8.708 | 0.7207 | 34.36× | 3; all candidate runs faster |
| f32 full M720 q8 | 383.834 / 377.008 / 2.680 | 0.9846 | 140.90× | 15; all 15 candidate runs faster |
| f32 reduced M180 q8 | 151.467 / 142.968 / 1.218 | 0.9458 | 117.24× | 15; all 15 candidate runs faster |

Across the 15 q8 triplets, full-mmax candidate/parent ratios ranged 0.9686–0.9982; reduced-mmax ratios ranged 0.9145–0.9760. The f32 DUCC gap remains large: the accepted route is 140.90× DUCC at full M720 q8 and 117.24× at reduced M180 q8. No parity claim is made.

The q8 output errors stayed at the parent level. Full-mmax relative L2 was `1.72342e-4` for the parent and `1.72343e-4` for the candidate; max absolute error was `9.64403e-4` for both. Reduced-mmax relative L2 was `2.72551e-4` for the parent and `2.72546e-4` for the candidate; max absolute error was `1.54793e-3` for both. Every parent, candidate, and DUCC worker in the 15-triplet acceptance screen reported finite, repeat-consistent, and reset-consistent outputs.

## Precision and adjacent controls

The generic f64 scalar synthesis control at GL T720 full M720 was neutral within the three-triplet screen: candidate/parent `0.9955` q1 and `1.0028` q8. Parent and candidate had the same DUCC relative L2, `4.40501e-14`, in both cells. This control found no material f64 slowdown; it is not the protected T1023 spin-1 benchmark.

Adjacent f32 q8 controls showed candidate/parent `0.9898` for scalar analysis and `0.9999` for spin-1 synthesis. The separate Rust `target-cpu=x86-64-v3` control retained the scalar-synthesis improvement: candidate/parent `0.9876` full M720 and `0.9647` reduced M180. In that fixed-ISA screen, candidate/DUCC remained `121.19×` full and `93.15×` reduced.

## Validation

- `cargo test --locked --release --all-features --lib scalar`: 168 passed, 16 ignored. The new `f32_fused_synthesis_matches_reference_and_declines_overflowing_sums` test exercises scalar and runtime SIMD blocks against an independent recurrence reference and checks transactional decline on overflowing staged sums.
- `cargo test --locked --release --all-features --test f32`: 11 passed, 1 ignored; includes all-grid scalar adjoint checks, poles, Nyquist handling, and spin 1–4 adjoints.
- `cargo check --locked --all-features`, `cargo clippy --locked --all-features --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check` passed.
- A first broad scalar-filtered run hit one unrelated parallel-worker probe assertion. The exact test passed on its own, and the full scalar-filtered rerun then passed.

The f32 harness used Rust 1.99.1 release + `parallel` with runtime Pulp V3 selection; DUCC used `-march=x86-64-v3`. The f64 control and fixed-ISA f32 control used Rust `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. Host: AMD Ryzen 9 5950X. Timings are warm worker batches, and the report preserves paired observations rather than pooling q1 and q8.

## Evidence files

- [`candidate.patch.gz`](candidate.patch.gz) expands losslessly to the exact source/test diff from the parent; the uncompressed SHA-256 is `1f31e89b2fa26e21e6ae0fb1e93afe417ab7e892a883c77b44f62d715fd515c7`.
- [`triplets.csv`](triplets.csv) has all 60 per-triplet rows across initial q1/q8 screens, q8 acceptance, f64 controls, adjacent f32 controls, and the fixed-ISA control.
- [`provenance.json`](provenance.json) records source, toolchain, DUCC, binary and artifact hashes.
- [`trace-summary.json`](trace-summary.json) and [`trace-instrumentation.patch.gz`](trace-instrumentation.patch.gz) retain the f32 exponent/range distribution evidence; the instrumentation patch expands losslessly.
- [`screen-three-triplets.json`](screen-three-triplets.json), [`screen-fifteen-q8-triplets.json`](screen-fifteen-q8-triplets.json), [`f64-controls.json`](f64-controls.json), [`f32-adjacent-controls.json`](f32-adjacent-controls.json), and [`f32-native-v3-control.json`](f32-native-v3-control.json) contain machine-readable per-cell details.
- `executed-runner-*.py.gz` preserve the exact executed runner sources losslessly; decompress a copy before running. Their original runner hashes and binary hashes are listed in the corresponding JSON manifests. The committed adjacent-control JSON omits transient host-pressure fields.

## Experiment scratch cleanup

After verifying the source hashes, the saved patch against its exact parent, all JSON sidecars, and the 60-row CSV, removed only the owned experiment scratch root and its two registered parent/candidate worktrees. The scratch contained 2,493,204,587 apparent bytes across 11,784 files. Available space on its filesystem increased by 2,523,496,448 bytes and free inodes by 13,987. Both worktree registrations and the scratch directory are absent; no experiment worker process was active. The three pre-existing untracked Callgrind captures in the main checkout were left untouched.

## Next work

Continue the remaining shared scalar traversal work under the active refactor order, then proceed to native-spin recurrence traversal and spin-analysis accumulation. This change does not retire `scalar_f32.rs` or `spin_f32.rs`; M6 DUCC parity also remains open.
