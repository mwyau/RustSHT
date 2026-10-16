# Aggregating recurrence range checks did not give a stable high-band q8 result

The candidate accumulated a SIMD minimum for each vector group and performed
one horizontal reduction after the recurrence update. This replaced one
horizontal reduction per SIMD vector group. Recurrence values and coefficient
accumulation were unchanged. A focused regression covered failures in the
first vector, a later vector, and the scalar tail.

## Paired source comparison

Five alternating release-process pairs compared candidate source with commit
`26449b5` using `examples/m1b_bench.rs`. Each process ran one warmup and five
timed f64 scalar GL STANDARD analysis calls, with full `mmax`, at T511
(`512 x 1025`) and T1023 (`1024 x 2049`). q1 ran on CPU 0; q8 ran on CPUs 0–7.
Rust was `1.99.0`, LLVM `23.1.1`, with portable Pulp runtime dispatch (AVX2/FMA
on this AMD Ryzen 9 5950X host). No CPU quota applied. The source builds used
release defaults and did not set `target-cpu=native`.

Negative paired changes favor the candidate. The paired change is the median
of the five within-pair percentage changes; the range is the minimum and
maximum pair.

| Grid | Threads | Baseline median | Candidate median | Paired change median (range) | Faster pairs |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | q1 | 18.737 ms | 17.503 ms | -6.64% (-8.50% to -5.25%) | 5/5 |
| T511 | q8 | 3.627 ms | 3.327 ms | -5.60% (-15.83% to -0.36%) | 5/5 |
| T1023 | q1 | 148.039 ms | 139.337 ms | -6.20% (-6.78% to -3.83%) | 5/5 |
| T1023 | q8 | 25.431 ms | 27.428 ms | +3.59% (-0.05% to +17.46%) | 1/5 |

The q1 result was consistently faster. T511 q8 also favored the candidate in
all five pairs, though the range was wide. T1023 q8 did not show a repeatable
gain, so that cell was measured in five additional alternating pairs using
the same CPU affinity. The repeat had a `-4.93%` median paired change and a
`-13.76%` to `+15.40%` range, with three of five pairs faster. Across all ten
T1023 q8 pairs, the median paired change was `+2.40%` (`-13.76%` to `+17.46%`),
with four of ten pairs faster. These runs disagree in direction and are too
variable to establish either a q8 gain or regression at T1023.

The candidate was removed. The improvements at q1 and T511 q8 did not provide
a stable result across the high-band q8 workload. The raw process logs, peak
RSS files, hashes, CPU affinity, load records and repeat commands are in
[`the benchmark data`](data/scalar-analysis-range-check-min-aggregation-screen-2026-10-15/).
The initial screen summary is in `paired-results.tsv`; the additional and
pooled T1023 q8 summaries are in `t1023-q8-repeat-results.tsv`.

## Validation

With the candidate present, formatting passed and the focused range-failure
regression passed:

```text
cargo test -p rustsht --all-features fused_scalar_analysis_block_declines_on_late_recurrence_range_failure -- --nocapture
  1 passed

cargo fmt --all -- --check
  passed
```

Full workspace suites and Clippy were not run for this rejected candidate.
No workspace storage, FFT code or numerical recurrence changed.
