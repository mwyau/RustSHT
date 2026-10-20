# Scalar-analysis kernel comparison

`src/legendre/scalar_paired_kernel_bench.rs` contains the test-only Rust
fixture and kernel harness. `ducc-ordinary-kernel.cpp` invokes DUCC's native
`map2alm_kernel` from the recorded revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. The runner defaults to the current
reference pin; set `RUSTSHT_DUCC_REVISION` explicitly to reproduce this
historical comparison:

```sh
RUSTSHT_DUCC_REVISION=80388904b8cfc32400ba84af8a6a45aabfe60f86 \
DUCC_SRC=/path/to/ducc-source-snapshot \
DUCC_GIT=/path/to/ducc-git-repository \
python3 scripts/benchmark_scalar_analysis_kernel.py
```

The source snapshot and Git object store may be different directories. The
script verifies SHA-256 equality for the pinned `sht_inner_loop.h`, `sht.h`,
`simd.h` and `constants.h` before compiling. Set `CXX` to select the C++
compiler; the default is `g++`. The script builds optimized release code with
Rust `-C target-cpu=x86-64-v3` and C++ `-O3 -DNDEBUG -march=x86-64-v3`. By
default, the checked Rust route uses the production post-group scan. To
benchmark fused group-state validation instead, set
`RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_fused_analysis_pair_range_check -Aunexpected_cfgs'`.
The script verifies that
Rust selects Pulp V3 and DUCC uses four f64 lanes, and runs one fixture
(`m=1`, `lmax=720`) at 64, 128 and 256 active latitude lanes.

To benchmark the scalar reduction variant, set
`RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_fused_analysis_pair_range_check --cfg rustsht_scalar_fused_analysis_pair_range_check -Aunexpected_cfgs'`.
This selects the same checked recurrence with one scalar mask reduction per
four-lane vector chunk.

The script warms and calibrates each specialization in sustained batches. Each
timed Rust process warms both variants three times; each timed DUCC process
performs three reset/kernel/checksum warmups before measurement, and the runner
validates that warmup record. It chooses separate operation counts for
recurrence and commit-only work so every measured batch targets 35 ms.
Calibration predicts a shared 20–95 ms interval; the runner then verifies every
recorded batch is between 20 and 100 ms. It
records 60 batches in each of ten independent process invocations per
implementation and lane count, with six batches per process. Rust/DUCC process
order alternates at each process pair; checked/unchecked Rust variants
alternate within each mode. Each child is pinned to the same allowed logical
CPU. Frequency and scheduler counters are sampled outside the benchmark core
where a second physical core is available. Output includes batch median,
p10/p90 and full-range distributions, per-process medians and p10/p90,
frequency and scheduling summaries.

The benchmark prints only a sanitized summary. Fixtures, binaries and raw
samples live in a temporary directory that is removed at exit.

## Timed boundaries

- Rust `kernel_only` times the fused recurrence, SIMD accumulation and writes
  into `staged_coefficients`. It is a Rust subphase and is not compared directly
  with DUCC, whose ordinary kernel updates its output coefficients directly.
- Rust `commit_only` times repeated additions from the staged array into the
  output array. The output is cleared once per batch, then consumed after the
  repeated commit operations finish.
- `kernel_plus_commit` is the matched boundary. Rust restores identical input
  state and clears output outside the timer, then runs the fused recurrence and
  commits staged coefficients inside it. DUCC performs the direct coefficient
  accumulation inside `map2alm_kernel`. `reset_inclusive` includes the same
  input copies and output clearing for both implementations.
- Coefficient preparation, ring packing, FFT, scheduling and complete-transform
  costs are outside all direct-kernel boundaries. Rust's `next` scratch is
  overwritten before use. Output arrays are compared outside timed regions.

Rust passes mutable state through `std::hint::black_box`; DUCC uses compiler
barriers. Both adapters validate recurrence coefficients and compare final
states outside timed regions. Checked and unchecked Rust outputs are compared
exactly.

The ignored `benchmark_fused_scaled_scalar_analysis_prefix` emits synthetic
seeds and T720 GL sectoral seeds (`m=500`) at 64, 128 and 256 lanes.
The DUCC adapter initializes from those same Rust-generated recurrence states
and verifies the resulting state and scale epochs after `iter_to_ieee`. The
256-lane GL case includes significant seeds, so DUCC correctly skips prefix
advancement. Run the calibrated prefix comparison with:

```sh
DUCC_SRC=/path/to/ducc-source-snapshot \
DUCC_GIT=/path/to/ducc-git-repository \
python3 scripts/benchmark_scalar_analysis_prefix.py
```

The runner verifies the pinned source, selects Pulp V3 and four-lane DUCC, and
collects 30 batches in three independent interleaved process pairs per case.
Kernel-only and reset-inclusive batches use separately calibrated repetition
counts, target 32 ms and must remain within 20–100 ms. Rust dispatches its SIMD
specialization once per batch. Kernel-only samples time the recurrence after
reset, then subtract per-call clock overhead measured in the same binary.
Reset-inclusive samples use one outer timer around state resets and recurrence;
they do not call the per-operation timer. Rust's scale/correction preflight is
measured separately. Output states and scale epochs are compared after timing.
Rust also resets its correction scratch, which has no DUCC counterpart, so
wall timings include this extra Rust buffer copy. Host frequency and scheduling
summaries are printed without saving raw host captures.
`split-check-candidate.patch` preserves the earlier split-loop prototype for
review; it is not part of the timed candidate.

## Scalar-analysis pair-tile screen

The production scalar-analysis pair tile defaults to 256. For controls, build
the same release example with either
`--cfg rustsht_scalar_analysis_pair_tile_64` or
`--cfg rustsht_scalar_analysis_pair_tile_128` in `RUSTFLAGS`; omitting both
selects 256. For example:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_scalar_analysis_pair_tile_64 -Aunexpected_cfgs' \
  cargo build -p rustsht --release --example m1b_bench
```

The options change only the f64 paired scalar-analysis block size. They do not
change shared synthesis/spin block sizes; f32 analysis continues to use 64.
The ignored `report_scalar_analysis_pair_tile_utilization` test reports the
active block and SIMD-tail counts for the T720 full-order and T300 reduced-order
cases. Run it with
`cargo test -p rustsht --release --lib report_scalar_analysis_pair_tile_utilization -- --ignored --nocapture`.
