# Matched RustSHT–DUCC profile at `41b309a5`

**Date:** 2026-10-18. **Host:** AMD Ryzen 9 5950X, x86_64, 16 cores/32 threads, AVX2/FMA. Rust 1.99.0 (LLVM 23.1.1), release build with `-C target-cpu=x86-64-v3 -C debuginfo=1 -C force-frame-pointers=yes`. GCC 15.2.0 DUCC build flags recorded in DWARF: `-D_FORTIFY_SOURCE=3 -march=x86-64-v3 -mtune=generic -g -O3 -std=c++17 -fno-omit-frame-pointer -fasynchronous-unwind-tables -fstack-protector-strong -fstack-clash-protection -fcf-protection -fzero-init-padding-bits=all`. Rust source `scalar_ordered.rs` SHA256: baseline `0491f24c896d959da9ac274f6bfd75de00ebe958f1b3515b232cff1d0d6bb932`; candidate `5e0d372d7078ba67fa18a862a822e564dfe76507fc984dc0540b5373d52ef79b`. DUCC source: `80388904b8cfc32400ba84af8a6a45aabfe60f86`; `sht_inner_loop.h` SHA256 `1d0364497c4f4a53fa9c2e652c86794e33725dfa6f5af2e86cbcfc5cf3e17646`, `sht.cc` SHA256 `6ec04499ef514fb4582f207516cc5e643698ca6b964aac231572d43ab417ec33`. Installed comparison package ducc0 0.41.0 corresponds to `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, with only namespace qualification changes in the relevant scalar files.

## Matched end-to-end timing

Both sides used identical packed f64 coefficients, GL grids, `phi0=0.37`, map layout and reusable plans/workspaces where the API permits. Rust used public `ShtPlan::synthesis_into`; DUCC used a symbolized native C++ runner calling pinned `synthesis_2d`. Each run warmed both sides, then alternated five timed Rust/DUCC samples. Values are medians in milliseconds; ratios use the displayed medians. Errors compare the final maps.

| Workload | HEAD Rust / DUCC (ratio) | Candidate Rust / DUCC (ratio) | Candidate max abs / relative L2 error |
| --- | ---: | ---: | ---: |
| T720, full mmax, 721×1442, q1 | 26.281 / 25.201 (1.043×) | 25.560 / 24.671 (1.036×) | 1.873e-12 / 8.662e-15 |
| T720, full mmax, 721×1442, q8 | 5.827 / 3.585 (1.625×) | 4.718 / 3.621 (1.303×) | 1.873e-12 / 8.662e-15 |
| T300, mmax=150, 301×603, q1 | 3.268 / 2.407 (1.357×) | 3.085 / 2.246 (1.373×) | 2.952e-13 / 3.586e-15 |
| T300, mmax=150, 301×603, q8 | 0.904 / 0.434 (2.083×) | 0.819 / 0.513 (1.598×) | 2.952e-13 / 3.586e-15 |

The candidate is the current `scalar_ordered.rs` change in this worktree: sectoral initialization, insignificant-prefix advancement and scaled accumulation now cover every vector group in an active ring batch under one shared degree index. The q8 T720 Rust median fell 19%; q1 did not regress. The q8 samples were noisy: DUCC's first T720 sample was 7.55 ms versus 3.58–3.63 ms for the remaining four; T300 q8 had a 0.92 ms first sample versus 0.49–0.58 ms later. All five samples remain in the medians. Treat q8 T300's ratio change as inconclusive because its DUCC median shifted between screens.

## Phase timings and execution evidence

The following q1 internal stage measurements use a separate timer-instrumented build at HEAD `41b309a5`, before the batch-prefix change. Timers run at order, batch or vector boundaries; the Rust probe increased whole-call time about 30% at T720 and 18% at T300, so these values identify dominant work but are not additive wall-time decompositions. DUCC stages use temporary order/batch-boundary timers in a pinned-source build. Boundaries differ slightly.

| Stage, milliseconds | T720 Rust / DUCC | T300 Rust / DUCC | Evidence and boundary |
| --- | ---: | ---: | --- |
| Plan/workspace setup | 6.82 / 2.03 (q1); 9.31 / 2.24 (q8) | 1.70 / 0.43 (q1); 2.34 / 0.31 (q8) | Measured separately; Rust constructs a plan and reusable workspace, DUCC runner prepares coefficients and geometry. Not comparable setup contracts. |
| Coefficient preparation | 1.370 / 1.779 | 0.183 / 0.217 | Instrumented q1 order boundaries; Rust uses `prepare_order`, DUCC uses `Ylmgen::prepare` and coefficient transformation. |
| Active-ring scan/packing | 0.389 / 0.848 | 0.059 / 0.070 | Instrumented q1; Rust active-list packing versus DUCC `fill_a2m`. |
| Sectoral initialization | 1.881 / 0.523 | 0.212 / 0.044 | Instrumented q1; Rust clears accumulators inside this boundary. |
| Insignificant prefix + scaled accumulation | 4.908 / 0.094 | 0.415 / 0.011 | Instrumented q1; Rust values are prefix advancement plus scaled recurrence/rescaling, DUCC value is `iter_to_ieee` plus its scaled correction loop. Largest measured Rust excess. |
| Ordinary recurrence | 14.018 / 12.747 | 0.942 / 0.801 | Instrumented q1; Rust `ordinary_two_pair_step`, DUCC `alm2map_kernel`. |
| Reflection and spectrum scatter | 1.011 / 0.596 | 0.113 / 0.057 | Instrumented q1; Rust includes phase/output writes, DUCC measures `extract_a2m`. |
| Legendre phase / inverse FFT and map | Rust 21.173 / 5.077; DUCC 15.79 / 8.53 | Rust 1.665 / 1.255; DUCC 1.177 / 1.103 | Coarse Rust phase probes and direct uninstrumented DUCC `alm2leg` / `leg2map` medians. Similar operation boundaries, not a paired subtraction. |

For q8, a Rust phase probe measured T720 setup 0.556 ms, outer Legendre 4.163 ms, ordered recurrence 3.067 ms, spectrum scatter 0.284 ms and inverse FFT/map 0.786 ms; active order/ring-pair work was balanced across workers (46.6k–49.5k pairs per worker per call). At T300 the corresponding measurements were 0.186, 0.369, 0.258, 0.035 and 0.197 ms, with 26.4k–28.7k pairs per worker. These per-worker work estimates describe Rust only; no per-thread CPU-time profile was available for DUCC.

`perf stat` hardware counters and `perf record` call-graph sampling were denied by `perf_event_paranoid=4`, including for the pinned DUCC runner. No cycles, instruction counts, IPC, cache misses or sampled self-time are claimed. Optimized symbolized disassembly shows four-lane YMM AVX2 and FMA in both implementations. Source and assembly confirm a remaining loop-organization difference: Rust keeps one ring vector's ordinary recurrence in registers through degree pairs, while DUCC broadcasts each four-degree coefficient group and applies it across vector blocks. The available profile does not quantify broadcast, spill or cache costs; this is not evidence of Rust language overhead.

## Ranked bottlenecks and next changes

1. **Repeated prefix and scaled-state work — addressed by this candidate.** At T720 q1, the instrumented Rust prefix/scaled region was 4.91 ms versus DUCC's 0.094 ms. The candidate advances all active-batch vector groups together, matching DUCC's shared degree index and avoiding a fresh prefix decision for every vector call. The q8 T720 full-call median improved from 5.83 to 4.72 ms; q1 and numerical checks remained stable.
2. **Ordinary recurrence — largest remaining q1 kernel stage.** Rust measured 14.02 versus DUCC 12.75 ms at T720, and 0.942 versus 0.801 ms at T300. Keep the next recurrence experiment bounded: test coefficient-broadcast reuse across a small number of ring vectors while retaining a register-resident state, then compare the same two workloads before changing dispatch.
3. **q8 execution and output layout.** Rust remains 1.30× DUCC at T720 and 1.60× at T300 in the candidate screen. Rust's active work is balanced, while its q8 phase shows recurrence and order-major spectrum staging/scatter consume most Legendre time. Scheduler and memory-traffic costs are plausible contributors, not yet isolated; do not attribute the gap to imbalance or cache behavior without counters.

Correctness: `cargo fmt --all -- --check` and `cargo test --release --lib` passed (209 passed, 15 ignored); the latter includes pinned DUCC scalar references, active-batch/tail tests, extreme/subnormal fallback and serial/parallel comparisons. This checkpoint covers only f64 scalar GL at the stated two workloads; it does not establish broad performance parity.

## Current-source P0/P1 follow-up (18 October 2026)

The q1 P0 stage probe used source `021884dd` (after shared-prefix batching, before the two-vector change) in a temporary instrumented worktree. It ran five calls per case with order/batch-boundary timers. Production source has no profiling hooks. The P1 candidate `scalar_ordered.rs` SHA256 is `36309e3e114f8e8c29a61bca108e993f1c60b346998c659227874b5485e3e7b2`.

| q1 stage, ms | T720 before two-vector grouping | T300 before two-vector grouping | DUCC T720 | DUCC T300 |
| --- | ---: | ---: | ---: | ---: |
| Coefficient preparation | 1.377 | 0.186 | 1.779 | 0.217 |
| Active-ring packing | 0.386 | 0.045 | 0.848 | 0.070 |
| Sectoral initialization | 0.680 | 0.074 | 0.523 | 0.044 |
| Prefix advancement | 0.364 | 0.048 | 0.094* | 0.011* |
| Scaled accumulation | 0.118 | 0.015 | included above* | included above* |
| Ordinary recurrence | 15.394 | 1.001 | 12.747 | 0.801 |
| Reflection/output scatter | 0.890 | 0.102 | 0.596 | 0.057 |

`*` DUCC's timer combines `iter_to_ieee` and the scaled correction loop; boundaries differ. Compared with the pre-fix Rust sum for initialization plus prefix/scaled work (6.79 ms at T720 and 0.63 ms at T300), current work is 1.16 ms and 0.14 ms. The original excess has shrunk enough to end P0. The q1 probe counted 1,139 empty and zero nonempty prefix-rescale checks at T720, 151 empty and zero nonempty at T300, and zero prefix recurrence steps. The ordered route succeeded and no fallback fired.

P1 compared the same q8 work with reusable plans/workspaces and a temporary phase/worker probe. Worker order jobs were balanced by elapsed time. At T720 each worker spent about 2.4–2.9 ms inside order jobs; the dynamic queue and utilization do not explain the main gap. The wall time for ordered recurrence fell from a 2.92 ms median to 2.50 ms; scatter fell from 0.27 ms to 0.22 ms. Inverse FFT/map was about 0.8–1.3 ms and was not the principal T720 excess. At T300 the recurrence and scatter stages were about 0.27 ms and 0.03 ms. `perf_event_paranoid=4` prevents hardware-counter sampling; no cache-miss or cycle claims are made.

The P1 change pairs two adjacent SIMD ring vectors in the ordinary suffix so both use the same degree coefficients before advancing. In the q1 stage probe, ordinary recurrence fell from 15.394 to 12.318 ms at T720 and from 1.001 to 0.779 ms at T300. It keeps each recurrence and four accumulation vectors local; scalar SIMD retains the one-vector route, and an unpaired SIMD tail uses the original helper. Existing all-feature tests cover reflected batches, SIMD tails and unsafe-value fallback.

A matched, interleaved, five-sample release screen compared the prior source, the two-vector source and pinned DUCC. Plans and workspaces were reused; Rust called public `ShtPlan::synthesis_into`, and DUCC called pinned native `synthesis_2d`. Times are medians in milliseconds. Map errors were unchanged by the source change.

| Workload | Prior Rust | Two-vector Rust | DUCC | New/prior | New/DUCC | Max abs / relative L2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T300, mmax=150, q1 | 3.199 | 3.044 | 2.322 | 0.952× | 1.311× | 2.952e-13 / 3.586e-15 |
| T720, full mmax, q1 | 25.686 | 23.613 | 25.244 | 0.919× | 0.935× | 1.873e-12 / 8.662e-15 |
| T300, mmax=150, q8 | 0.926 | 0.859 | 0.560 | 0.927× | 1.534× | 2.952e-13 / 3.586e-15 |
| T720, full mmax, q8 | 6.401 | 5.023 | 3.631 | 0.785× | 1.384× | 1.873e-12 / 8.662e-15 |

The q1 T720 result approached DUCC in this screen; q1 T300 and both q8 cases remain behind. q8 samples were noisy, notably DUCC's first T720 sample (7.33 ms versus 3.58–3.64 ms for the remaining four), and Rust's T300 q8 samples also had outliers. Treat the ratios as a bounded checkpoint, not a broad parity claim. Full all-feature workspace tests passed; the ordered-kernel focus passed with default and no-default features, and Clippy/formatting passed.
