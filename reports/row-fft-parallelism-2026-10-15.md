# Parallel row FFT scheduling

## Method

The analysis phase profile found that serial longitude row FFTs consumed 52% of scalar and 57% of spin-1 GL T1023 q6 analysis time. This experiment scheduled those independent SHT rows with Rayon and a private root `RealFftWorkspace` for each worker. Each FFT call, its packing, and its scaling remain unchanged. Capacity-one execution retains the original serial loop, including the interleaved Q/U row order for spin 1.

The parallel path is enabled for grids with at least 128 latitude rows and 257 longitude samples. `prepare_workspace` reserves up to `min(execution_capacity, ntheta)` FFT workspaces for those grids. Smaller grids and capacity-one workspaces allocate no row-FFT worker scratch. The existing `allocated_worker_count()` continues to count recurrence scratch only.

The baseline is commit `4e8e8b4a55688f855815f6b0b8e9fec11b647674`, immediately before the implementation. The candidate was built from the final uncommitted source in this report. `examples/m1b_bench.rs` measured f64 GL STANDARD analysis with reused plans and workspaces, full `mmax`, `phi0=0.37`, and `nphi=2*lmax+3`. Each process performed one warm-up and five timed calls. Five baseline/candidate process pairs alternated order for every q6 case. q1 controls used the same method and five process pairs. Processes were pinned to CPUs 0–5 for q6 and CPU 0 for q1; the host-wide benchmark lock was held throughout.

The CPU was an AMD Ryzen 9 5950X with 16 cores and 32 logical CPUs, running Linux `7.0.14-20-pve`. Builds used rustc `1.99.0` and LLVM `23.1.1`, default release settings, and Pulp runtime dispatch. The CPU supports AVX2 and FMA; the benchmark did not log the selected Pulp ISA. No `target-cpu=native` flag was used. The 1/5/15-minute load averages changed from 2.39/2.76/4.05 at q6 start to 2.66/2.80/4.05 at q6 end; process snapshots showed no concurrent CPU-intensive job.

## Results

Each table entry reports the median of five process medians. The paired change is the median of the five candidate/baseline ratios; ranges are the minimum and maximum paired changes.

| Operation | Degree | Threads | Baseline (ms) | Candidate (ms) | Paired change |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar | T127 | q6 | 0.383 | 0.269 | −29.3% (−32.9% to −24.1%) |
| Scalar | T511 | q6 | 7.514 | 5.664 | −26.1% (−33.9% to −24.0%) |
| Scalar | T1023 | q6 | 65.274 | 39.954 | −36.1% (−53.2% to −34.6%) |
| Spin 1 | T127 | q6 | 0.540 | 0.288 | −44.9% (−52.5% to −43.5%) |
| Spin 1 | T511 | q6 | 13.555 | 8.686 | −36.3% (−44.9% to −32.9%) |
| Spin 1 | T1023 | q6 | 117.525 | 66.893 | −43.9% (−45.8% to −42.0%) |

All paired q6 results favored the candidate. q1 medians changed by +0.2%/+0.5% for scalar T511/T1023 and +1.8%/+1.3% for spin 1. Each q1 five-pair range crossed zero, so the measurements resolve no repeatable q1 change. q1 peak RSS differences also stayed within process variation.

At q6, median peak RSS increased by 200–912 KiB across the T511/T1023 cases; the largest measured paired increase was 1,280 KiB. The exact additional FFT vector payload was 196,800 bytes at T511 and 393,408 bytes at T1023. T127 adds 49,344 bytes. A q1 workspace allocates zero additional FFT worker bytes. Fresh-workspace allocation medians were within process variation in these runs; the q6 T1023 spin-1 candidate added 0.535 ms.

The payload values were measured from the six worker FFT workspaces' vector capacities. The temporary diagnostic was removed after collection; it did not change the root FFT implementation. Raw timing, allocation and peak-RSS samples are in [`paired.log`](data/row-fft-parallelism-2026-10-15/paired.log), [`q1-control.log`](data/row-fft-parallelism-2026-10-15/q1-control.log), and [`t127-paired.log`](data/row-fft-parallelism-2026-10-15/t127-paired.log). Payload counts are in [`workspace-bytes.txt`](data/row-fft-parallelism-2026-10-15/workspace-bytes.txt). Reproduction scripts are in the same data directory.

## Validation and scope

The q1-to-q4 workspace-migration test compares scalar analysis coefficients after explicit preparation. The cross-pool q1 reference test compares scalar and spin-1 analysis at capacities q1/q2/q3/q4/q5/q6/q8/q16. Existing prepared-workspace tests exercise scalar and spin-1 operations on GL, CC and F1. The full f32 suite also passes; f32 performance was not measured.

Release DUCC 0.41.0 and analytic checks pass for scalar T511/T1023/T2047 synthesis and analysis on GL, CC and F1, and for spin-1/2 exact T1023 analysis on all three grids. The DUCC source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

Final checks passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features` (182 Rust tests and 10 f32 integration tests passed, 12 ignored); and `cargo test -p rustsht --no-default-features` (144 Rust tests and 10 f32 integration tests passed, 7 ignored). The release scalar and spin high-band DUCC checks above passed with `RAYON_NUM_THREADS=5`. The Python suite was not run because bindings did not change.

This benchmark establishes a q6 whole-analysis gain for GL T127/T511/T1023. It does not measure CC/F1 performance, other q>1 capacities, reduced `mmax`, or f32 performance. Numerical tests cover CC/F1 and odd/even longitude paths. The implementation changes SHT row scheduling and workspace allocation only; no code in the standalone `rustsht-fft` crate was inspected or changed, and no RustFFT algorithm or dependency changed.
