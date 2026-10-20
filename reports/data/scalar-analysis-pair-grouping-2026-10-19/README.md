# Scalar-analysis kernel comparison

`src/legendre/scalar_paired_kernel_bench.rs` contains the test-only Rust
fixture and kernel harness. `ducc-ordinary-kernel.cpp` invokes DUCC's native
`map2alm_kernel` from revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. Run the calibrated ordinary-kernel
comparison with:

```sh
DUCC_SRC=/path/to/ducc-source-snapshot \
DUCC_GIT=/path/to/ducc-git-repository \
python3 scripts/benchmark_scalar_analysis_kernel.py
```

The source snapshot and Git object store may be different directories. The
script verifies SHA-256 equality for the pinned `sht_inner_loop.h`, `sht.h`,
`simd.h` and `constants.h` before compiling. Set `CXX` to select the C++
compiler; the default is `g++`. The script builds optimized release code with
Rust `-C target-cpu=x86-64-v3` and C++ `-O3 -DNDEBUG -march=x86-64-v3`, verifies
that Rust selects Pulp V3 and DUCC uses four f64 lanes, and runs one fixture
(`m=1`, `lmax=720`) at 64, 128 and 256 active latitude lanes.

The script warms and calibrates each specialization in a sustained batch
(5,000 calls at 64 lanes, 3,000 at 128, and 1,400 at 256). It then chooses one
shared operation count per lane count so the fastest variant targets 35 ms per
batch and the slowest is projected below 95 ms. It records 30 batches in each of
three independent process invocations per implementation and lane count. The
Rust/DUCC process order alternates by invocation, and checked/unchecked Rust
batches alternate their within-process order. Each child is pinned to the same
allowed logical CPU; frequency and scheduler counters are sampled outside the
benchmark core where a second physical core is available. Frequency sampling
uses the cpufreq policy covering the pinned CPU. Output includes batch
distributions, process medians, frequency and scheduling summaries.

The benchmark prints only a sanitized summary. Fixtures, binaries and raw
samples live in a temporary directory that is removed at exit.

## Timed boundaries

- `kernel_only` restores identical recurrence input state and clears the
  coefficient output before each operation, outside that operation's timer.
  The reported kernel duration sums per-operation intervals; the surrounding
  batch wall time also includes resets. Rust's `next` buffer is recurrence
  scratch, overwritten before it is read, so it is not part of the input reset.
- `reset_inclusive` times the input copies, output clear and kernel together.
  Rust and DUCC restore the same `previous/current` recurrence values and zero
  the corresponding coefficient output on every operation. DUCC's padded
  output buffer is cleared only over its active `m..=lmax` coefficient span.
- Coefficient preparation, ring packing, FFT, scheduling and complete-transform
  costs are outside both boundaries. The benchmark compares the ordinary
  recurrence and coefficient accumulation only.

Rust passes mutable state through `std::hint::black_box` and consumes outputs
after each batch. DUCC uses a compiler fence and post-batch checksum. Both
adapters validate recurrence coefficients and compare final coefficients
outside timed regions. Checked and unchecked Rust outputs are compared exactly.

The ignored `benchmark_fused_scaled_scalar_analysis_prefix` and the separate
scaled-block adapter remain available for focused experiments. Their older
single-call measurements are historical and are not comparable to the
calibrated ordinary-kernel results. `split-check-candidate.patch` preserves the
earlier split-loop prototype for review; it is not part of the timed candidate.
