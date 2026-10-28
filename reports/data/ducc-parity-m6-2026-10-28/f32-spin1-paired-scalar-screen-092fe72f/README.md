# f32 spin-1 separate paired-scalar dispatch screen

**Decision: no promotion.** The candidate changes one dispatch gate in `src/transform/spin/synthesis.rs`, allowing native-f32 spin 1 to use the existing fast scalar eligibility and separate-channel paired-scalar conversion/accumulation path. It does not change the recurrence or paired-spin capability. Polar/declined routes retain the general fallback.

Exact parent is `092fe72f322bd482dac73b020785b096772bc901`; pinned DUCC is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The candidate patch SHA-256 is `b35cedbc12ad9ee0e86efa1749a7e7e76bf2be3a6327dbe65fb04108de5fb2ed`. Parent and candidate were separately built in release mode with rustc/cargo 1.99.1, `-C target-cpu=x86-64-v3`, Pulp runtime V3; DUCC used GCC 15.2 with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` and a `-fno-fast-math` driver. q1 used CPU 3; q8 used CPUs 8–15. The workers warmed, checked finite/repeat/reset behavior, then timed repeated transforms at a 35 ms batch target.

## Initial GL T720 spin-1 synthesis screen

Three interleaved parent/candidate/DUCC pairs ran for each full/reduced mmax q1/q8 cell (`ntheta=721`, `phi0=0.23`, `nphi=1458` full and `378` reduced). Timings below are medians of per-process warm batch medians; the candidate/parent and Rust/DUCC columns are paired per-triplet medians.

| Cell | Parent / candidate / DUCC ms | Candidate/parent | Parent/DUCC | Candidate/DUCC |
|---|---:|---:|---:|---:|
| full q1 | 1484.907 / 1493.662 / 40.528 | 1.006 | 36.58× | 37.10× |
| full q8 | 271.436 / 268.161 / 5.563 | 0.983 | 49.24× | 48.53× |
| reduced q1 | 615.820 / 613.619 / 17.682 | 0.995 | 34.82× | 34.64× |
| reduced q8 | 123.724 / 101.717 / 2.459 | 0.897 | 49.98× | 41.36× |

Parent/candidate outputs were bitwise equal in all initial pairs; every worker passed finite, repeat, and workspace-reset checks. Maximum candidate/DUCC relative-L2 error was `1.4264e-4` full and `1.41495e-4` reduced; maximum absolute error was `1.17165e-3` and `9.13277e-4`. This screen did not test analytic modes, adjoints, exceptional values, or protected f64 cells.

## Reduced q8 extension

The initial three-pair reduced q8 median suggested a 10.3% gain, but pair ratios were `0.897`, `1.005`, and `0.807`. A 15-pair extension found paired candidate/parent median `1.0059`; a 100,000-resample percentile bootstrap interval was `[0.8225, 1.0362]`, with only `6/15` pairs faster. The observed paired ratios ranged `0.796–1.244`; no samples were discarded. The standalone parent/candidate process medians (`122.540/102.338 ms`) are not used as the candidate effect because the paired measurements are highly dispersed. Candidate/DUCC remained a large gap (paired ratio median `41.17×`); the parent/candidate paired result controls the decision.

The repeated workers stayed bitwise identical to the parent and passed finite/repeat/reset checks. No candidate-specific route-success count, equivalent-work instruction count, or stage-only timing was collected. Thus this screen does not establish that the intended fused work increased or that a repeatable whole-transform gain exists. No extended f32 analysis/spin controls or f64 T1023 control were run after the targeted cell failed to confirm the apparent gain.

**Next:** return to the native-f32 scalar-synthesis bottleneck. The earlier suffix routes are documented blockers; measure the exact-current-source scalar-synthesis baseline and use the existing reflected-ring work census only as a source-equivalent lead before selecting a different bounded dataflow change. M6 remains open.

The initial matrix, all 15 reduced-q8 per-pair timings and correctness checks, fixture hashes, binary hashes, runner snapshot and exact patch are retained in this folder. Raw host telemetry was reviewed but excluded from committed data; the wide paired spread is preserved above and no samples were dropped. The runner expects the three rebuilt binaries under `cargo-parent/`, `cargo-candidate/`, and `ducc-build/` inside an experiment-owned build root; set `RUSTSHT_M6_BUILD_ROOT` and `RUSTSHT_M6_RESULTS_DIR` before running it. It imports the repository's `scripts/benchmark_ducc_f32_native.py` by locating the repository relative to this report.
