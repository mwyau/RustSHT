# Current-source f32 GL T720 spin-1 analysis baseline

Source is exact current HEAD `e094d4f4b3a2d42f9ef7e94779ffc4afc078b198`; the previous pushed code source is `8472668b463e2fe06eae189800b83cf8d5620804`. The accepted native-f32 scalar-analysis leaf is included. Pinned DUCC is 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

Three interleaved fresh-process current-Rust/DUCC pairs ran on GL T720 full-mmax spin-1 analysis with `ntheta=721`, `nphi=1458`, `phi0=0.23`, native f32 inputs, AVX2/FMA, and warm reusable workspaces. q1 was pinned to CPU 3; q8 to CPUs 8–15.

| Threads | Rust median ms | DUCC median ms | Paired Rust/DUCC median | Per-pair ratios | Relative-L2 error | Max absolute error |
|---:|---:|---:|---:|---|---:|---:|
| 1 | 1071.505 | 29.013 | 36.932× | 37.096×, 36.932×, 36.930× | `1.569e-5` | `2.231e-7` |
| 8 | 140.883 | 4.014 | 35.138× | 34.822×, 35.269×, 35.138× | `1.569e-5` | `2.231e-7` |

All six workers passed finite-output, repeated-call, and workspace-reset checks. The metrics compare the Rust coefficients against pinned DUCC output for one deterministic fixture; they do not replace independent analytic-mode and adjoint tests. These are elapsed whole-transform timings, not instruction counts. No source change or stage-only profile was run.

This refresh confirms that native-f32 spin-1 analysis remains a large current-source GAP after the accepted scalar-analysis accumulation change. Exact fixtures, per-pair timing, output checks, build flags, binary hashes, and runner are in this folder. The next source hypothesis remains reflected-ring reuse for f32 scalar synthesis, after documenting the f32 family baselines.
