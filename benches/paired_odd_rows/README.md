# Paired odd-length longitude row experiment

This isolated benchmark measures two adjacent scalar longitude rows as
independent real transforms, two direct length-N complex transforms, one
length-N complex transform packing the row pair, and two pinned DUCC real
transforms. It includes odd target lengths and even controls. The paired
prototype is benchmark-only; it does not change the production FFT route.

The Rust `rust-real` path imports the current private `src/fft.rs` by path.
`rust-c2c` uses two independently planned full complex transforms. `rust-pair`
uses `z[j] = x[j] + i*y[j]` and extracts the two spectra from `Z[k]` and
`conj(Z[-k])`; synthesis packs the two Hermitian spectra into one full complex
frequency array and writes the inverse real and imaginary parts to adjacent
rows. `rust-pair-mixed` selects the existing 19×19 mixed-radix inverse plan at
N=361. Other paired transforms use RustFFT's ordinary planner.

The Rust executable preallocates each row transform and scratch area per
worker. The benchmark reports one first call, then calibrates 30 timed batches
to about 30 ms each. It records per-batch p10/median/p90, warmed allocation
count, workspace bytes per worker, total time and microseconds per row pair.
Rows are assigned to workers as contiguous adjacent pairs; an odd final row
uses the independent production route. Input setup, plan setup and output
comparison are outside timed batches.

`--profile ROUTE OPERATION N SHAPE INPUT` separately measures the packing,
FFT and extraction phases for one adjacent pair, with 30 calibrated batches
per phase. The FFT profile reloads a saved transform input before each in-place
FFT to prevent repeated unnormalized transforms from changing the input scale;
it reports both reload-only and reload-plus-FFT timings. Their median
difference estimates FFT execution time, while the end-to-end row benchmark
remains the primary performance result.

The fixture generator creates deterministic real maps for forward analysis and
deterministic Hermitian spectra for inverse synthesis. `full`, `reduced`, and
`polar` select full-order, half-order, and row-dependent sparse polar spectra.
The runner must validate every candidate against `rust-real` and a direct DFT
before timed comparisons. Full round trips must reproduce N times the source
map because both the RustSHT forward and inverse-row conventions are
unnormalized.

`run_matrix.py` interleaves three independent processes per route and case,
validates each result against RustSHT's independent real route, and records
per-process PSI, load average and top-process snapshots alongside the samples.
Host-contended runs remain visible in the process CSV so they can be excluded
or reported separately.

## Build

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3' \
  cargo build --manifest-path benches/paired_odd_rows/Cargo.toml --release

DUCC_SRC=/path/to/ducc-at-42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  RUSTSHT_PAIRED_ROWS_DUCC_BUILD_DIR="$DUCC_BUILD_DIR" \
  benches/paired_odd_rows/ducc/build.sh
```

The DUCC build rejects any source SHA except
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (DUCC 0.42.0). Rust uses strict
floating point with x86-64-v3 target code; DUCC transform code uses GCC 15.2
with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and its driver
uses strict floating point. Keep all matched measurements on this one DUCC
revision.

The Rust CLI accepts `--write-fixture` to create a shared deterministic input,
`--once` for untimed numerical validation, and a route name for timed runs.
See `run_matrix.py` for the pinned, interleaved q1/q8 procedure and required
odd/even lengths.
