# Original `ce710` whole-transform baseline

This is a matched native f64 scalar GL analysis baseline for RustSHT source
`ce710d3622e3ca37783e0666b8bb800426fd72e0` against DUCC 0.42.0 source
`80388904b8cfc32400ba84af8a6a45aabfe60f86`.

The harness used Rust 1.99.1 / LLVM 23.1.1, Pulp runtime V3 (four f64
AVX2/FMA lanes), strict Rust floating-point behavior and
`-C target-cpu=x86-64-v3`. DUCC numerical translation units used GCC 15.2 with
`-O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3`; its driver
was compiled with `-fno-fast-math`. The benchmark used the same deterministic
fixtures and direct `ShtPlan` transform boundary as the current-source T720
comparison in the parent report. It ran three interleaved Rust/DUCC process
pairs and ten calibrated 20–100 ms batches per process and backend. q1 was
pinned to one physical CPU; q8 used eight physical CPUs on one L3 domain.
Plan/workspace setup and first-call timings were reported outside warm batches.

All six requested workloads (T720, `mmax` 1/180/720, q1/q8, even-smooth
longitude sampling) completed. The harness recorded 18 numerical/identity
comparisons with no failures. Maximum relative L2 error versus DUCC was
`6.13e-14`. Median CPU runqueue wait was 0.64%, with p90 17.0%; interpret
small q8 deltas cautiously.

`results.csv` contains compact rows; `summary.json` contains sanitized case
distributions, correctness metrics and build metadata. Raw per-process
samples and host snapshots remain local. The production library source is the
clean `ce710` commit; the temporary checkout also contained test-only
benchmark instrumentation, which was not linked into the native example.
The copied harness sources are identified by SHA-256 in the summary metadata.

## Reproduction

Use a checkout at the `ce710` SHA, the pinned DUCC source and the benchmark
worker/harness files identified in `summary.json`. Run from the Rust source root:

```sh
RUSTUP_TOOLCHAIN=1.99.1 \
RUSTSHT_DUCC_REVISION=80388904b8cfc32400ba84af8a6a45aabfe60f86 \
RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
CARGO_TARGET_DIR="$CARGO_TARGET_DIR" \
TMPDIR="$SCRATCH" \
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 80388904b8cfc32400ba84af8a6a45aabfe60f86 \
  --production-source-revision ce710d3622e3ca37783e0666b8bb800426fd72e0 \
  --lmax 720 --operation analysis --threads 1 --threads 8 \
  --longitude-mode even_smooth \
  --results-dir "$RESULTS_DIR" --no-report
```

Set `DUCC_SRC` to the pinned checkout and set `CARGO_TARGET_DIR`, `SCRATCH`,
and `RESULTS_DIR` to writable paths before running. The recorded Rust and
DUCC executable SHA-256 values are in `summary.json`; a rebuild may produce
different binary hashes if toolchain patch-level details change.
