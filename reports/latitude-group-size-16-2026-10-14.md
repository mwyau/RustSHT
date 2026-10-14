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
