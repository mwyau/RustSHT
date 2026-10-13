# M4 contiguous batch API

## API behavior

The public complex and real plans accept flat row-major buffers through
`forward_batch` and `inverse_batch`. The plan length defines each row. Real
forward rows contain `n` samples and produce `n / 2 + 1` complex bins; real
inverse rows use the reverse shape. Complex rows have `n` values in both
directions. The operations validate batch shapes and the caller-owned
workspace before processing rows. They allocate no batch scratch and do not
create worker threads. Output rows retain input order. The current implementation calls the one-transform pass schedule for each
row with a shared workspace. It does not vectorize across rows. The contiguous
layout supports a later across-row SIMD implementation. This API change alone
does not provide a measurable execution speedup.

## Measurement method

The benchmark compares the public batch operation with a loop over the
existing one-transform public operation. Both use the same plan, workspace,
input data, output storage, and row order. The timer includes only the FFT
calls. Complex input is restored from a prebuilt source buffer before timing;
that copy is outside the timed region. Real inputs and spectra use the
existing deterministic `real_value` and `spectrum_value` formulas. Plan and
workspace construction occur outside the timing interval. The benchmark uses **f64** transforms. It was run with
`DUCCFFT_PROCESS_SAMPLE=<1..6> cargo bench --bench rustfft --
bench-native-batch-api N ROWS` for each workload below. The benchmark binary
was built from source commit
`7568775f814560117bcd91f34e3d68309ac72b80` with Rust 1.99.0
(`b940084d7`, LLVM 23.1.1), Cargo 1.99.0, and the default Cargo `bench`
profile (`opt-level=3`). `RUSTFLAGS`, LTO, and `target-cpu` were unset. Runtime
AVX2 dispatch remained enabled. The host was an x86-64 CPU; each process
was pinned to logical one CPU core. The Legendre workload was active. A load sample
during the sweep was 3.44, 4.17, and 3.77 for the 1-, 5-, and 15-minute
averages, respectively. Treat the results as provisional under shared-host
load. For each workload, six process runs each performed one warm-up per API and
seven paired timed samples. The first API in each pair alternated. The tables
report median per-transform times across the 42 paired samples. The gain is
the median of `(single-call loop time / batch API time) - 1` for each pair;
positive values favor the batch API. Checksums matched in all 1,008 paired
records. Raw process samples are in
[`m4-batch-api-process-samples.csv`](../benches/results/m4-batch-api-process-samples.csv).

## RustSHT longitude workloads

The row counts match T127, T511, T1023, and T2047 in
[`rustsht-workloads.md`](rustsht-workloads.md). Times are microseconds per
transform. Each cell gives `single-call loop / batch API (median paired gain)`.

| N | Rows | C2C forward | C2C inverse | R2C | C2R |
|---:|---:|---:|---:|---:|---:|
| 257 | 128 | 12.322 / 12.330 (+0.04%) | 12.745 / 12.742 (+0.11%) | 13.416 / 13.417 (−0.14%) | 13.844 / 13.851 (−0.04%) |
| 1,025 | 512 | 36.992 / 36.963 (+0.06%) | 38.076 / 38.094 (−0.05%) | 33.242 / 33.286 (−0.15%) | 49.706 / 49.692 (+0.03%) |
| 2,049 | 1,024 | 100.745 / 100.783 (−0.02%) | 103.797 / 103.834 (−0.08%) | 120.118 / 120.343 (+0.01%) | 117.678 / 117.883 (−0.09%) |
| 4,097 | 2,048 | 245.363 / 245.256 (+0.05%) | 251.616 / 251.783 (−0.04%) | 245.796 / 245.311 (+0.00%) | 249.767 / 250.047 (−0.08%) |

Across these workloads, median paired gains range from −0.15% to +0.11%; no
operation shows a consistent improvement. Per-process median gains range
from −1.26% to +1.04%, which exceeds the overall median changes and reflects
the shared-host variation. These data support retaining the batch API for its
contiguous-row interface and single upfront shape validation. The measured
medians do not support a speedup claim.

## Smooth-length controls

These controls use 32 rows per call and the same measurement method.

| N | C2C forward | C2C inverse | R2C | C2R |
|---:|---:|---:|---:|---:|
| 1,024 | 3.792 / 3.788 (+0.12%) | 5.225 / 5.225 (+0.06%) | 2.921 / 2.928 (−0.21%) | 2.773 / 2.768 (+0.03%) |
| 4,096 | 19.706 / 19.687 (+0.16%) | 25.422 / 25.442 (+0.00%) | 13.526 / 13.472 (+0.27%) | 12.539 / 12.458 (+0.36%) |

Median paired gains across these controls range from −0.21% to +0.36%. The
single-transform and sequential batch-of-32 comparisons against RustFFT and
DUCC remain in [`m4-c2c-avx2.md`](m4-c2c-avx2.md) and
[`m4-real-multipass.md`](m4-real-multipass.md); those measurements do not use
the new batch API.

## Decision

Keep the serial batch methods as a public row-major interface. Their reusable
workspace and output ordering match the longitude-row storage used by
RustSHT, and the validation tests cover f32/f64, inverse scaling, Bluestein,
real complexify, incomplete rows, and workspace mismatches. Future work should
measure an implementation that vectorizes across independent rows. Across-row
SIMD remains a separate optimization.
