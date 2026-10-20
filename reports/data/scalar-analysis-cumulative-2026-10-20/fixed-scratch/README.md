# Scalar-analysis fixed-scratch comparison

The matched source baseline is `b1ebc31c`; the candidate adds the experimental
`rustsht_scalar_analysis_fixed_simd_arrays` layout. Both use Rust 1.99.1,
Pulp 0.22.3 runtime V3, pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`
(0.42.0), and GCC 15.2.0. Rust uses strict floating-point semantics and
`-C target-cpu=x86-64-v3`. DUCC numerical sources use
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its validation driver
is separately compiled with `-fno-fast-math`.

The selection contains ten GL geometries: T300 at `mmax=75,300` and T720 at
`mmax=1,180,720`, with odd-minimum and even-smooth longitude sampling. Each
case measures analysis and synthesis at q1/q8. Rust and DUCC run in alternating
order in three independent process pairs. Each short operation has ten
calibrated batches per process, targeting 25 ms; the summary retains the
30-batch median and p10/p90 for each backend. The two builds use the same
fixture seeds and benchmark code. DUCC was compiled once and reused for both
runs.

The run completed 40 rows and 120 Rust/DUCC output comparisons. Candidate
maximum relative L2 error was `1.841e-13`; maximum absolute error was
`3.742e-10`. Host contention was high in both runs, so q8 attribution is
provisional; all timing samples were retained. Raw host/process samples remain
local. The committed `summary.csv` contains per-row timings and numerical
errors only.

Reproduce from the repository root with the pinned DUCC checkout available as
`DUCC_SRC`:

```sh
RESULT_ROOT=build/scalar-analysis-fixed-layout
DUCC_SRC=/path/to/ducc-42ce8ab5
export DUCC_SRC
RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
CARGO_TARGET_DIR="$RESULT_ROOT/target-baseline" \
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 300 --lmax 720 --results-dir "$RESULT_ROOT/baseline-results" \
  --no-report

RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_scalar_analysis_fixed_simd_arrays -Aunexpected_cfgs' \
CARGO_TARGET_DIR="$RESULT_ROOT/target-candidate" \
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 300 --lmax 720 --results-dir "$RESULT_ROOT/candidate-results" \
  --no-report

python3 scripts/summarize_scalar_analysis_fixed_layout.py \
  "$RESULT_ROOT/baseline-results/results.json" \
  "$RESULT_ROOT/candidate-results/results.json" \
  reports/data/scalar-analysis-cumulative-2026-10-20/fixed-scratch/summary.csv
```

The benchmark uses its production GCC DUCC flags and reports the selected Rust
architecture. For final interpretation, retain the actual compile commands,
the reported Rust dispatch and host-pressure summary from each run.
