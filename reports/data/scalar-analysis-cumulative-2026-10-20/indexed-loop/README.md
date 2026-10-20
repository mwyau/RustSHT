# Indexed SIMD loop ablation

Baseline source: `68381d3d`. Candidate source: `2d4fd6b6`. The candidate
uses the compile-time cfg `rustsht_indexed_analysis_pair_loop`; the default
zipped implementation remains selected otherwise. Both versions use Pulp
0.22.3 runtime V3 and Rust 1.99.1 strict FP. DUCC was pinned to
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, built with GCC 15.2, numerical
kernel flags `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and a
strict validation driver.

`summary.csv` records T720 GL `m=1` direct-kernel medians and p10/p90 ranges
for 64/128/256 active-ring lanes. The runner used ten interleaved
Rust/DUCC process pairs, six calibrated 20–100 ms batches per process/backend,
and identical fixtures. Rust kernel-only excludes final coefficient commit;
kernel-plus-commit includes it and is compared with DUCC's direct output
accumulation. Numerical errors are measured against pinned DUCC. A single
indexed 64-lane process outlier is retained; the runner marks that subset
unstable.

Reproduce from the repository root with `DUCC_SRC` and `DUCC_GIT` pointing at
the pinned checkout:

```sh
export DUCC_SRC=/path/to/ducc-42ce8ab5
export DUCC_GIT="$DUCC_SRC"

RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  CARGO_TARGET_DIR=build/kernel-zipped \
  python3 scripts/benchmark_scalar_analysis_kernel.py > build/kernel-zipped.txt

RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_indexed_analysis_pair_loop -Aunexpected_cfgs' \
  CARGO_TARGET_DIR=build/kernel-indexed \
  python3 scripts/benchmark_scalar_analysis_kernel.py > build/kernel-indexed.txt
```

The optimized V3 benchmark binaries can be inspected with `nm -S -C` and
`objdump -d` using the `FusedScalarAnalysisPair<true, false>` symbol. The
checked-in CSV is a sanitized summary. Raw timings, scheduling records,
frequency samples, executable paths, and command output stay local.
