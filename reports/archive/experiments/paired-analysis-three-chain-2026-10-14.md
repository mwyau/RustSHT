# Three-chain paired-analysis reduction experiment

## Result

The third SIMD accumulation chain was rejected. It shortened the independent
reduction chains further, but added register pressure and per-call chunk
indexing. It slowed one-thread spin-1 STANDARD analysis by 7.21% at T511 and
6.80% at T720. At eight threads, T720 was 2.71% slower and T511 was within
measurement noise. The committed two-chain reducer remains unchanged.

## Prototype and machine-code hypothesis

The current f64 normal-range paired reducer uses two partial sums for each of
four components: same-parity real/imaginary and opposite-parity
real/imaginary. The AVX2 loop therefore keeps eight YMM accumulators and
processes two latitude vectors per iteration. The experiment added a third
partial sum per component, processing three vectors per iteration. This
shortened each dependency chain from eight vector updates to about five for a
64-lane block. The expected gain was lower dependency-chain latency. The cost was twelve live
accumulator vectors, plus vectors used for the products. AVX2 exposes sixteen
YMM registers. The candidate AVX2 assembly used a `0x98` byte stack frame
versus `0x38` for the two-chain kernel and emitted stack-mediated vector
loads/stores inside the reduction loop. It also used multiply-high reciprocal
arithmetic to calculate the dynamic slice lengths for three-element chunks;
the two-element chunks use shifts. These costs explain why the shorter
dependency chain did not improve whole-transform time.

## Measurements

The baseline was committed source `6ee8930`; the prototype changed only
`src/legendre/scalar_paired.rs` and was never committed. Both binaries used
Rust `1.99.0`, LLVM `23.1.1`, the optimized release profile with debug symbols,
16 codegen units, LTO disabled, and portable Pulp runtime dispatch. The CPU
was an x86-64 CPU running Linux; AVX2/FMA was available
and AVX-512 was not. No `target-cpu=native` flag was used. The workload was f64 spin-1 STANDARD GL analysis, `phi0=0.37`, full `mmax`,
reused plan/workspace, T511 `512x1025` and T720 `721x1443`. The harness ran one
warm-up and five timed operations in each fresh process. Five ABBA process
blocks alternated baseline and candidate. q1 was pinned to one CPU core and q8 to
8 pinned CPU cores. Table entries are medians of five process medians; paired ranges are
the minimum and maximum block percentage changes.

| Band | Threads | Two-chain ms | Three-chain ms | Change | Paired range |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | 38.037 | 40.778 | +7.21% | +6.30% to +7.96% |
| T511 | 8 | 12.369 | 12.414 | +0.36% | -0.81% to +3.23% |
| T720 | 1 | 101.344 | 108.239 | +6.80% | +6.32% to +7.62% |
| T720 | 8 | 33.841 | 34.758 | +2.71% | +1.71% to +4.11% |

The q1 regressions were consistent in all five process blocks. q8 T511 was
inconclusive; q8 T720 was consistently slower. Raw per-process samples are in
[`reports/data/paired-analysis-three-chain-2026-10-14`](../../data/paired-analysis-three-chain-2026-10-14).

## Numerical checks

On the prototype, all seven focused `paired_analysis_` tests passed. They
cover the SIMD reduction against the scaled-lane reference, mixed normal and
scaled lanes, subnormal basis values and extreme finite products. The
release-only `m3_spin_t511_all_grids_matches_ducc_and_roundtrips` test also
passed against stored DUCC 0.41.0 references for spins 1 and 2 on GL, CC and
F1. For spin 1, maximum absolute DUCC synthesis error was at most
`4.885e-15`, analysis coefficient error at most `4.970e-18`, and
synthesis-analysis coefficient error at most `1.277e-14`. The altered
reduction order is not bitwise identical. No tolerance changed. The experiment adds no workspace memory because it reuses the existing SIMD
vectors. It does increase code size and stack traffic. The temporary source
and benchmark control were removed; no production API, FFT code or recurrence
arithmetic changed. No fresh DUCC timing was collected for this experiment;
the timings above compare complete native RustSHT transforms.
