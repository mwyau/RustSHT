# Plan 14 n=361 inverse comparison

Baseline Rust source: `3fcbaee6b3b8d3c707ee4c981ef95388946ccbdb`.
Candidate source: `a6cfdc49`. DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

The baseline and candidate were separate clean Rust worktrees, built with
Rust 1.99.1 / LLVM 23.1.1, release mode, strict FP and
`-C target-cpu=x86-64-v3`. The selected backend was Pulp V3 (four f64 lanes).
The pinned DUCC SHT source used GCC 15.2 with
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its validation driver
used strict floating-point settings. The machine was a Ryzen 9 5950X.
q1 used CPU 0; q8 used the eight physical cores 0–7.

`comparison.csv` contains medians and p10/p90 timing summaries. The row
comparison used the existing `benches/plan14_q8_odd_inverse` harness: 721
rows, `mmax=180`, n=361, 30 calibrated batches per process, and three
interleaved Rust/DUCC process pairs per source. The end-to-end comparison
used `scripts/benchmark_ducc_full_transform_parity.py`, identical
deterministic coefficients, three interleaved Rust/DUCC process pairs per
run, and 10 calibrated batches per process. q8 source comparisons were
repeated in three alternating baseline/candidate runs. Plan and workspace
setup were outside warm timing; first call and setup were recorded
separately. All transform outputs were checked outside the timed region.

Reproduction commands for each source worktree:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo build --release --example ducc_full_transform_parity --features parallel

CXX=g++-15 python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 720 --mmax 180 --fixed-ntheta 721 \
  --operation synthesis --threads 1 --threads 8 \
  --longitude-mode odd_minimum --results-dir "$RESULTS_DIR" --no-report

RUSTFLAGS='-C target-cpu=x86-64-v3' \
  cargo build --release --manifest-path benches/plan14_q8_odd_inverse/Cargo.toml
```

For the row harness, build its pinned DUCC executable with
`benches/plan14_q8_odd_inverse/ducc/build.sh`, then run
`benches/plan14_q8_odd_inverse/run_compare.py` with the resulting Rust and DUCC
binaries and `--case q1-odd --case q8-odd --pairs 3 --q1-cpus 0 --q8-cpus 0-7`.
The original detailed batch logs and transient host snapshots remain local.
