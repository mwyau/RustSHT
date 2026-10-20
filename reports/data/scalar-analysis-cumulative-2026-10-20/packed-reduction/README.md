# Packed complex reduction comparison

Source baseline: `1548b5c5`. Candidate: `0db88bd5`, with
`rustsht_packed_complex_pair_reduction`. DUCC was pinned to
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0). Rust was 1.99.1,
Pulp 0.22.3, and runtime dispatch selected V3 on an AMD Ryzen 9 5950X. The
kernel comparison used GCC 15.2 with a DUCC numerical kernel built using
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the benchmark driver
used `-fno-fast-math`. Rust used strict FP with
`-C target-cpu=x86-64-v3`. Rust and DUCC each used four f64 SIMD lanes.

`kernel-summary.csv` contains the matched T720 GL `m=1` direct-kernel
comparison at 64/128/256 active-ring lanes. Ten interleaved Rust/DUCC process
pairs collected six calibrated batches per process/backend and lane count;
each batch was 20–100 ms. The kernel-only boundary excludes final coefficient
commit. `kernel_plus_commit` includes it and is compared with DUCC, which
accumulates directly to output. Outputs were checked after timing. Numerical
error fields report Rust output against DUCC.

`t720-full-transform-screen.csv` contains a bounded GL `mmax=1`, odd-minimum
`nphi=3` analysis/synthesis screen at q1/q8. It used three process pairs and
ten 25 ms-target batches per backend. The reduction-only run was separated in
time from baseline and combined-candidate runs; both backends moved into a
different timing regime, so that run is labeled for host-regime context and
must not be used to estimate candidate effect. q8 is inconclusive because of
run-queue interference.

The direct-kernel harness records the actual compile commands and timing
diagnostics. Reproduce it with the DUCC checkout at the pinned revision:

```sh
export DUCC_SRC=/path/to/ducc-42ce8ab5
export DUCC_GIT="$DUCC_SRC"

RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  python3 scripts/benchmark_scalar_analysis_kernel.py > build/kernel-baseline.txt

RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_packed_complex_pair_reduction -Aunexpected_cfgs' \
  python3 scripts/benchmark_scalar_analysis_kernel.py > build/kernel-packed.txt
```

For the full-transform check, set `DUCC_SRC` to the same pinned checkout and
use distinct target and result directories for each variant:

```sh
DUCC_SRC="$DUCC_SRC" CARGO_TARGET_DIR=build/full-baseline-target \
RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 720 --case-limit 1 --results-dir build/full-baseline --no-report

DUCC_SRC="$DUCC_SRC" CARGO_TARGET_DIR=build/full-reduction-target \
RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_packed_complex_pair_reduction -Aunexpected_cfgs' \
  python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 720 --case-limit 1 --results-dir build/full-reduction --no-report

DUCC_SRC="$DUCC_SRC" CARGO_TARGET_DIR=build/full-combined-target \
RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_scalar_analysis_fixed_simd_arrays --cfg rustsht_packed_complex_pair_reduction -Aunexpected_cfgs' \
  python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 720 --case-limit 1 --results-dir build/full-combined --no-report
```

The selected first case is T720, `mmax=1`, odd-minimum longitude sampling;
the harness measures A/S at q1/q8 and validates independent basis, edge-input
and adjoint cases outside timed regions.

The checked-in CSVs contain sanitized summary data only. Raw benchmark output,
host scheduling samples, CPU-frequency samples, command captures and local
build paths remain outside the repository.
