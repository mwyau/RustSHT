# Workspace capacity and independent-field control (2026-10-15)

## Control result

The existing independent-field control remains explicit: `m5_batch_bench` creates each `parallel-one` workspace inside a one-thread pool, so each has execution capacity 1 even when the outer pool has eight threads. The T1023 worker probe also confirmed that an unprepared q1 workspace remains on one active worker after migration to q8 or q16. The q8, eight-field T511 GL f64 `parallel-one` control was screened for scalar and spin-1 STANDARD synthesis and analysis. Three fresh release processes per operation each collected five warm batch samples. The median of process medians was `49.300 ms` for scalar synthesis, `27.533 ms` for scalar analysis, `60.203 ms` for spin-1 synthesis and `42.966 ms` for spin-1 analysis. The 2026-10-14 report records `48.827`, `29.529`, `68.666` and `44.976 ms`, respectively, from a different source revision and collection window. The new scalar synthesis median is 0.97% higher; the other three are lower. Within-process sample spreads overlap the prior study's process variability. These measurements do not establish a performance gain or a formal no-regression result because they are not paired runs of the same baseline and candidate commits. The current runs used the x86-64 CPU, eight threads pinned to CPU IDs `0-7`, Rust `1.99.0` (`b940084d7`), LLVM `23.1.1`, Cargo's release profile and runtime Pulp dispatch. Load-average telemetry was retained only in local diagnostics. The process list showed no CPU-intensive competing job during collection. The workspace RSS increase was `66.7–66.9 MiB` across runs, compared with `65.1–65.2 MiB` in the earlier study; process RSS includes allocator and runtime effects. Full warm samples are in [`native-batch-samples.txt`](data/workspace-capacity-control-2026-10-15/native-batch-samples.txt). The later same-host paired parent/candidate screen is in [`the paired capacity comparison`](workspace-capacity-paired-2026-10-15.md). Its result supports the q8 independent-field spin-1 synthesis case, while scalar synthesis changes by about 2% and other cells are variable or near baseline. These controls do not establish broad performance acceptance.

## Command

```sh
cargo build --release --example m5_batch_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m5_batch_bench 0 511 8 synthesis 8 parallel-one 512 1025 511 5
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m5_batch_bench 0 511 8 analysis 8 parallel-one 512 1025 511 5
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m5_batch_bench 1 511 8 synthesis 8 parallel-one 512 1025 511 5
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m5_batch_bench 1 511 8 analysis 8 parallel-one 512 1025 511 5
```

Each command was run in three separate processes. The benchmark source was unchanged. RustSHT source was at `4f54934` plus the uncommitted workspace-capacity changes; the historical comparison is the independent-field study at source revision `5813be9f14bfc37422b48448201bcfeea88fcdaa`.
