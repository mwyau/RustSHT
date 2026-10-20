# Spin-1 analysis leaf overpartitioning

## Decision

Keep the current weighted order partition with one leaf per executing worker. A temporary two-leaf-per-worker policy did not reduce T511 q8 analysis time; it was slower in four of five process pairs. T511 q16 results were mixed, and T720 q8 was consistently slower. The code change was reverted.

## Hypothesis and profile

The weighted order splitter balances its active degree-lane estimate, but earlier diagnostics measured 1.5–2.8x variation in leaf completion time. The candidate doubled the order-leaf count when f64 spin-1 STANDARD analysis ran on a full-band exact GL plan at `lmax >= 511` with at least eight executing Rayon workers. It was bounded by existing worker scratch and left the recurrence, accumulation order and output slices unchanged. `rayon::join` work stealing could then assign multiple smaller leaves to each worker and reduce the final tail. AVX2 disassembly of `AnalysisPairAccumulation` showed eight YMM accumulator chains and the existing two-vector unroll. The loop had no YMM stack spills. The paired visitor still calls its accumulation closure for each degree pair. This supports recurrence and accumulation as a substantial analysis cost, but it does not establish that more order leaves will reduce it.

## Paired native measurements

The baseline executable was built from RustSHT commit `803ccb1` and saved with SHA-256 `32bd7feef42651f5867d57f2fa3103f94d17372200a24a1f5edb8656595d91dd`. The candidate was built from the same source plus the temporary leaf-count change. Both used Rust `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO disabled, and portable Pulp AVX2/FMA dispatch. The CPU was an x86-64 CPU running Linux; q8 processes were 8 pinned CPU cores and q16 to 16 pinned CPU cores. Each process used `examples/m1b_bench` with f64 spin-1 STANDARD analysis, a reused plan/workspace, one warm-up, and five timed transforms. The workload used full `mmax`, `phi0=0.37`, and exact GL latitude counts. Baseline and candidate processes alternated within each pair. Values are medians in milliseconds. The change is the median, across process pairs, of the candidate/baseline ratio of each pair's process medians.

| Grid | Threads | Process pairs | Baseline / candidate medians | Paired change | Candidate faster |
|---|---:|---:|---:|---:|---:|
| T511 `512x1025` | 8 | 5 | `12.122 / 12.567` | `+3.19%` slower | 1/5 |
| T511 `512x1025` | 16 | 3 | `13.163 / 13.723` | `-1.68%` | 2/3 |
| T720 `721x1443` | 8 | 3 | `34.486 / 38.403` | `+11.55%` slower | 0/3 |
| T511 `512x1025` control | 4 | 3 | `15.988 / 15.971` | `+1.13%` | branch disabled |

The T511 q8 candidate was slower in four process pairs and faster in one. T511 q16 had one large candidate slowdown despite two faster process medians. T720 q8 slowed by 10–13% in all three pairs. The q4 control exercised the baseline path on both sides and shows expected process noise. The candidate reused the workspace's existing recurrence buffers; it requested no extra workspace allocation. Per-process timing vectors, first-call times and workspace allocation samples are in [`more-leaves-paired-samples.txt`](../../data/spin1-analysis-thread-scaling-2026-10-14/more-leaves-paired-samples.txt). The command for each process was:

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" examples/m1b_bench 1 511 8 analysis GL STANDARD 512 1025 511 requested-only
```

For q16, affinity matched the requested worker count; T720 used `1 720 8 analysis GL STANDARD 721 1443 720 requested-only`. The baseline executable and candidate executable ran in separate processes with identical inputs and release flags.

## Numerical check and disposition

With `RAYON_NUM_THREADS=8`, the temporary candidate passed the opt-in T511 spin DUCC/roundtrip check on GL, CC and F1. The GL spin-1 DUCC analysis coefficient maximum absolute error was `4.970e-18`; synthesis-analysis coefficient maximum absolute error was `1.255e-14`. CC/F1 were outside the candidate selector. The scheduling change assigns disjoint order slices and does not change per-order summation. The candidate did not provide a whole-transform gain. Smaller leaves add recursive joins and repeat order-group setup; the available measurements do not identify which cost caused the slowdown. The baseline remains unchanged. A scheduling revision would need path-specific worker imbalance that persists under repeated process measurements before another granularity change is justified.
