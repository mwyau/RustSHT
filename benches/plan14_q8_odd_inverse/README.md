# Plan 14 inverse-row comparison

This benchmark compares the production RustSHT inverse-row routine from
`src/fft.rs` with DUCC's `pocketfft_r` row routine. Rust compiles the exact
`src/fft.rs` and `src/error.rs` files into the benchmark package by path. It
does not substitute the separate `rustsht-fft` implementation.

The default case set is T720 synthesis geometry (`rows=721`, `mmax=180`) at
odd-minimum `nphi=361` and even-smooth `nphi=378`, with q1 and q8. Inputs are
deterministic positive-frequency spectra. Modes above `mmax` are zero. Plan,
workspace and thread-pool construction are outside timed batches; the first
execution is reported separately. Every call overwrites its output, and the
checksum/output comparison occurs outside timed batches.

## Build and run

Use Rust 1.99.1 and pinned DUCC 0.42.0 commit
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`:

```sh
cd benches/plan14_q8_odd_inverse
RUSTFLAGS='-C target-cpu=x86-64-v3' cargo build --release
DUCC_SRC="$DUCC_SOURCE_DIR" \
  RUSTSHT_PLAN14_DUCC_BUILD_DIR="$DUCC_BUILD_DIR" \
  ./ducc/build.sh
cd ../..
python3 benches/plan14_q8_odd_inverse/run_compare.py \
  --rust-bin benches/plan14_q8_odd_inverse/target/release/plan14-q8-odd-inverse \
  --ducc-bin "$DUCC_BUILD_DIR/ducc-q8-odd-inverse" \
  --output-dir reports/data/plan14-ducc-fft-rows-2026-10-21
```

Use repeated `--case q8-odd --case q8-even` arguments for a focused q8 rerun
after a noisy host interval.

The DUCC build checks the pinned source SHA and uses upstream C++17, GCC
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for the numerical
kernel. The standalone driver is compiled with `-fno-fast-math` and
`-ffp-contract=off`. Rust uses release optimization and
`-C target-cpu=x86-64-v3` without fast-math. The runner pins q1 to CPU 0 and
q8 to CPUs 0–7; change these with `--q1-cpus` and `--q8-cpus` when needed.

The run writes per-process batch logs, paired summary data, numerical
validation, and a local host snapshot. Keep the generated host snapshot
untracked; it contains transient machine data. Each backend receives
three independent process invocations by default, interleaved in alternating
order. Each process calibrates 30 batches to about 30 ms and records p10,
median and p90.
