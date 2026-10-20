# Scalar-analysis pair-grouping fixtures

The Rust fixture writer and direct-kernel microbenchmarks are in
`src/legendre/scalar_paired_kernel_bench.rs` and are excluded from production
builds. For the candidate, run:

```sh
BENCH_TMP="${TMPDIR:-.}"
RUSTFLAGS='-C target-cpu=x86-64-v3' \
RUSTSHT_KERNEL_BENCH_BASE="$BENCH_TMP/rustsht-analysis-kernel" \
cargo test -p rustsht --release --lib \
  legendre::scalar_paired::kernel_bench::benchmark_fused_scalar_analysis_pair_kernel \
  -- --ignored --nocapture --test-threads=1
```

Build the DUCC adapters against the pinned source tree, then pass the matching
Rust `.in` fixture and `9` repetitions:

```sh
BENCH_TMP="${TMPDIR:-.}"
g++ -std=c++17 -O3 -DNDEBUG -march=x86-64-v3 -I"$DUCC_SRC/src" \
  reports/data/scalar-analysis-pair-grouping-2026-10-19/ducc-ordinary-kernel.cpp \
  -o "$BENCH_TMP/ducc-analysis-kernel-bench"
"$BENCH_TMP/ducc-analysis-kernel-bench" "$BENCH_TMP/rustsht-analysis-kernel-64.in" 9
```

For the synthetic prefix fixture, run the ignored Rust test
`benchmark_fused_scaled_scalar_analysis_prefix`, then build and run its DUCC
adapter:

```sh
BENCH_TMP="${TMPDIR:-.}"
g++ -std=c++17 -O3 -DNDEBUG -march=x86-64-v3 -I"$DUCC_SRC/src" \
  reports/data/scalar-analysis-pair-grouping-2026-10-19/ducc-scaled-prefix.cpp \
  -o "$BENCH_TMP/ducc-analysis-prefix-bench"
"$BENCH_TMP/ducc-analysis-prefix-bench" "$BENCH_TMP/rustsht-analysis-kernel-64.in" 9
```

To reproduce the Rust parent timing, use a worktree at `d8b40fd`, copy
`src/legendre/scalar_paired_kernel_bench.rs` into it, and add the test-only
module declaration beside the other scalar-paired module imports before
running the same Rust command. The drivers validate
coefficients, output or recurrence state before printing timings. The DUCC
source revision is `80388904b8cfc32400ba84af8a6a45aabfe60f86`.
