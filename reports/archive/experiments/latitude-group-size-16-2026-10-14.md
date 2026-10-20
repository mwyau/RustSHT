# Sixteen-ring synthesis group experiment

## Decision

Rejected a universal 16-ring synthesis group size. It improved scalar T511 eight-thread synthesis but slowed spin-1 T511 by about 14%, which is the higher-priority wind operation. The candidate also doubled recurrence worker groups and increased process peak RSS. The default 32-ring schedule remains unchanged.

## Hypothesis and implementation

The current q8 T511 workspace creates 16 contiguous groups of 32 latitudes. A 16-ring group creates 32 tasks, increasing scheduling granularity and potentially balancing work across polar and scaled recurrence regions. The temporary prototype changed the existing single-block group heuristic from `LATITUDE_BLOCK_SIZE / 2` (32) to `/ 4` (16) for grids with at least 256 rings. It changed no kernel arithmetic. At q8 T511, the prototype increased `RecurrenceWorkspace` groups from 16 to 32. Plan and map dimensions remained `512x1025`, full `mmax=511`, f64 GL, STANDARD, `phi0=0.37`.

## Method

- Baseline source: `cd65415` (`Unroll paired analysis vector accumulation`); candidate differed only in the group-size heuristic.
- CPU: x86-64 CPU, Linux; q8 processes 16 pinned CPU cores.
- Compiler: rustc 1.99.0, release build, default release flags, runtime Pulp AVX2/FMA dispatch.
- Each process warmed the operation and collected five steady-state samples. Five process pairs were run for T511 and three for T255, with baseline followed by candidate in each pair.
- Commands:

```sh
cargo build --release --example m1b_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 511 8 synthesis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 511 8 synthesis GL STANDARD 512 1025 511 requested-only
```

## Results

Values are medians of the five timed calls per process, followed by the median across processes. The process median ranges overlap where shown.

| Band | Operation | 32-ring baseline (ms) | 16-ring candidate (ms) | Change |
| --- | --- | ---: | ---: | ---: |
| T511 | Scalar synthesis q8 | `12.129` (`11.521–12.248`) | `11.096` (`10.853–11.845`) | `-8.5%` |
| T511 | Spin-1 synthesis q8 | `20.774` (`20.391–23.700`) | `23.713` (`23.463–23.955`) | `+14.1%` |
| T255 | Scalar synthesis q8 | `1.635` (`1.595–1.667`) | `1.579` (`1.541–1.596`) | `-3.4%` |
| T255 | Spin-1 synthesis q8 | `4.521` (`4.418–4.540`) | `4.688` (`4.503–4.738`) | `+3.7%` |

At T511, spin-1 was slower in four of five paired processes; the remaining pair was approximately tied. Scalar was faster in four of five pairs, with one slower result. T255 changes were small and ranges overlapped. Workspace allocation medians at T511 rose from about `3.9 ms` to `4.4–4.7 ms`. A separate `/usr/bin/time -v` q8 spin-1 pair measured peak RSS of `38,840 KiB` for 32-ring groups and `40,952 KiB` for 16-ring groups, a `2,112 KiB` increase. In that single pair, allocation rose from `3.974` to `4.857 ms` and transform time from `20.474` to `23.819 ms`.

## Interpretation

Smaller groups help scalar T511 scheduling but duplicate per-order setup across more groups and are substantially slower for spin-1. A single `ShtWorkspace` currently serves all supported operators for a plan, so adopting this policy would regress the main spin-1 path. A scalar-only schedule would require an operation-specific worker sizing design and separate evidence for scalar analysis, DERIV1, and other grids. That complexity is not justified by the current scalar-only result. The prototype was removed. No new correctness test was needed for this rejected scheduling-only change; existing baseline numerical paths remain unchanged. The benchmark output exercised both candidate transform paths but was not compared against DUCC. No FFT code changed.

## Spin-1 q8 64-ring group control

The coefficient-cache path copies each prepared E/B scalar row into the
private recurrence scratch for every latitude group. At T511 q8, the current
32-ring groups create 16 groups for eight workers. A temporary spin-1-only
schedule used one 64-ring group per worker, halving those row copies and
scheduled groups while leaving the scalar schedule unchanged. Five alternating release-process pairs used the f64 GL T511 grid `512x1025`,
full `mmax`, `phi0=0.37`, reused plan/workspace and `m1b_bench` with one warm-up
and five timed samples. The baseline was the source at `fbbe483`; the
candidate changed only the q8 spin-1 group size. Processes were pinned to
8 pinned CPU cores on the x86-64 CPU, Linux, using Rust 1.99.0 / LLVM
23.1.1, release `opt-level=3`, 16 codegen units, LTO off and Pulp AVX2/FMA
runtime dispatch without `target-cpu=native`. Baseline q8 spin-1 synthesis process medians were `20.048, 20.068, 18.407,
20.100, 19.943 ms` (median `19.943 ms`). Candidate medians were `23.014,
22.771, 22.873, 22.818, 22.702 ms` (median `22.818 ms`), a `14.4%`
increase using the unpaired medians. All five paired process medians favored
the 32-ring baseline. First-call medians increased from `25.335 ms` to
`28.742 ms`. The q8 scalar control was measured only on baseline; the
operation-specific candidate condition does not apply to scalar synthesis. The T511 q8 DUCC release regression passed under the candidate, with spin-1
GL analysis coefficient max absolute error `4.970e-18` and
synthesis-analysis coefficient max absolute error `1.255e-14`. The scheduling
change preserves coefficient arithmetic and output ownership. The candidate
was reverted: halving cached row copies did not repay the loss in group
parallelism. Raw within-process samples are in
[`ring64-process-samples.txt`](../../data/spin1-ring64-synthesis-2026-10-14/ring64-process-samples.txt).
