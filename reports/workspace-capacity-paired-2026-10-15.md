# Paired workspace-capacity batch comparison (2026-10-15)

## Scope

This screen compares parent commit `4f54934` with workspace-capacity commit `b6f2415` using the existing `m5_batch_bench` harness. It measures eight independent T511 fields in a q8 outer Rayon pool, with scalar and spin-1 STANDARD synthesis and analysis.

`parallel-one` creates one-thread workspaces before executing the fields in q8. On the parent, transforms could grow those workspaces after migration; on the candidate, capacity 1 keeps each transform serial. `parallel-pool` creates q8 workspaces and serves as the control for pool-sized execution.

Three process pairs were run per cell in baseline/candidate, candidate/baseline, baseline/candidate order. Each process used one initial batch, a warmup batch, then five timed batches. The reported baseline and candidate values are medians of three process medians. The paired change is the median of the three within-pair percentage changes.

## Results

| Schedule | Operation | Baseline ms | Capacity candidate ms | Paired change | Paired range |
| --- | --- | ---: | ---: | ---: | ---: |
| One-thread workspaces | Scalar synthesis | 46.626 | 47.567 | +2.02% | +1.88% to +3.38% |
| One-thread workspaces | Scalar analysis | 29.752 | 28.496 | -8.50% | -27.19% to +21.99% |
| One-thread workspaces | Spin-1 synthesis | 88.324 | 57.280 | -35.11% | -36.63% to -34.51% |
| One-thread workspaces | Spin-1 analysis | 41.473 | 40.713 | -1.08% | -13.02% to -0.54% |
| Pool-sized workspaces | Scalar synthesis | 47.195 | 47.288 | +0.20% | -1.52% to +14.97% |
| Pool-sized workspaces | Scalar analysis | 27.931 | 28.655 | +3.60% | -3.01% to +6.77% |
| Pool-sized workspaces | Spin-1 synthesis | 87.594 | 78.099 | -10.70% | -11.03% to -10.64% |
| Pool-sized workspaces | Spin-1 analysis | 42.288 | 40.810 | -2.39% | -3.52% to +0.39% |

The capacity-1 independent-field schedule reduced spin-1 synthesis time by about 35% in all three pairs. Scalar synthesis was about 2% slower in all pairs. Scalar analysis had wide process variation, and the small spin-1 analysis change is not conclusive. With q8 workspaces, scalar timings stayed near baseline; spin-1 synthesis improved about 11% in every pair. This one-host screen does not establish a general speedup or full M4 performance acceptance.

The largest relative output-checksum difference within a pair was `6.5e-14`. Checksums only confirm that each benchmark produced output; they are not a numerical accuracy test. The Rust numerical suite supplies independent DUCC, analytic, and adjoint checks.

## Environment

The runs used the AMD Ryzen 9 5950X with AVX2/FMA, Rust `1.99.0` (`b940084d7`), LLVM `23.1.1`, f64, release profile, portable Pulp runtime dispatch, and CPUs 0–7 pinned with `taskset`. The grid was GL `512x1025`, `lmax=mmax=511`, `phi0=0.37`; the pool size and field count were both eight. The start and end load averages were `1.61, 3.36, 3.79` and `1.69, 3.10, 3.67`. No CPU-intensive process was visible in the top-process samples at either boundary.

Median RSS increase during workspace allocation, aggregated over the six synthesis and analysis processes for each schedule/spin/variant, was 66,654/66,760 KiB (baseline/candidate) for scalar one-thread workspaces, 66,784/66,970 KiB for spin-1 one-thread workspaces, 73,946/74,132 KiB for scalar pool-sized workspaces and 76,014/76,188 KiB for spin-1 pool-sized workspaces. This is measured before the first transform and excludes later lazy allocations.

The paired process medians, five within-process batch samples and output checksums are in [`paired-samples.tsv`](data/workspace-capacity-paired-2026-10-15/paired-samples.tsv). The SHT benchmark uses the library FFT as part of each transform; no FFT-specific profiling or benchmark was performed.

## Commands

Both source revisions were built with:

```sh
cargo build --release --example m5_batch_bench
```

Each benchmark used this command form:

```sh
taskset -c 0-7 target/release/examples/m5_batch_bench <spin> 511 8 <synthesis|analysis> 8 <parallel-one|parallel-pool> 512 1025 511 5
```

The baseline binary came from `4f54934`; the candidate binary came from `b6f2415`.
