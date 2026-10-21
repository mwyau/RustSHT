# Plan 14: Direct ring-major output for ordered synthesis

**Status:** validated source change at `fe0590ac7c918c3d6aeea9fa25ff283b2974d80d`.
The preceding production source is `a6059cd60723c2a4a85c28710a6094a899ff7592`.

## Change

Parallel scalar GL synthesis previously accumulated each harmonic order into
an order-major scratch array, then launched another parallel pass to scatter
that array into the ring-major spectrum consumed by the inverse FFT. The new
path writes each uniquely scheduled order directly into its ring-major
column. The workspace spectrum is zeroed once before scheduling, so polar
and inactive entries remain zero. If any order declines the fast route, all
partial output is cleared before the existing robust fallback runs.

This removes an intermediate spectrum clear and the separate output-scatter
pass/parallel region. It retains dynamic one-order jobs, recurrence scratch,
the numerical preflight, fallback and inverse FFT. The source uses a private
writer with an explicit disjoint-column safety contract; jobs join before
the spectrum is read. Public APIs and serial synthesis are unchanged.

## Matched results

The parent and candidate were built from clean source checkouts with Rust
1.99.1 / LLVM 23.1.1, `-C target-cpu=x86-64-v3`, runtime Pulp V3 AVX2/FMA,
and DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` with GCC 15.2.
DUCC SHT sources used `-O3 -ffast-math -ffp-contract=fast
-march=x86-64-v3`; the driver used strict FP. The benchmark reuses each
library's plans/workspaces as supported, times complete native synthesis
calls, and checks output buffers outside the timed regions. Each run pinned
q8 workers to the same eight physical CPUs, alternated Rust/DUCC process
order over three independent process pairs, and collected ten calibrated
25 ms batches per process pair (30 batches per backend and case).

The candidate was run twice, once before and once after the parent run. The
table shows the parent median, both candidate Rust medians, mean candidate
latency, and mean Rust/DUCC ratio. Percent change is candidate mean versus
parent Rust latency. Errors are relative L2 against DUCC.

| Case | Parent Rust ms | Candidate Rust ms (run 1 / 2) | DUCC ms (parent → candidate mean) | Rust/DUCC (parent → candidate mean) | Rust change | Relative L2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T720, m=1, odd-minimum | 0.1539 | 0.1009 / 0.1037 | 0.1314 → 0.1315 | 1.171 → 0.778 | -33.6% | 1.08e-13 |
| T720, m=1, even-smooth | 0.1514 | 0.0987 / 0.0977 | 0.1281 → 0.1281 | 1.182 → 0.766 | -35.1% | 1.84e-13 |
| T720, m=180, odd-minimum | 2.0265 | 1.6804 / 1.7228 | 1.2653 → 1.2565 | 1.602 → 1.354 | -16.0% | 5.19e-14 |
| T720, m=180, even-smooth | 1.7673 | 1.3959 / 1.4247 | 1.2136 → 1.2214 | 1.456 → 1.155 | -20.2% | 4.71e-14 |
| T720, m=720, odd-minimum | 5.0354 | 4.1814 / 4.2382 | 3.8941 → 3.8971 | 1.293 → 1.080 | -16.4% | 4.45e-14 |
| T720, m=720, even-smooth | 3.9478 | 3.0969 / 3.0982 | 2.7104 → 2.6961 | 1.457 → 1.149 | -21.5% | 4.41e-14 |
| T2047, m=2047, ntheta=2049, nphi=4096 | 75.2385 | 54.6950 / 55.0764 | 57.5523 → 57.7729 | 1.307 → 0.950 | -27.1% | 1.19e-13 |

Each source's paired process distributions support these changes. Across the
two candidate runs, T2047 Rust p10/p90 were 54.44–55.04 ms and 54.71–55.97
ms; parent p10/p90 were 74.62–77.08 ms. Candidate T720 Rust p10/p90 ranges
for each run are retained in the data. Numerical errors are unchanged between
the two candidate runs.

The machine reported an AMD Ryzen 9 5950X with AVX2/FMA and runtime Pulp V3.
For the T720 process groups, sampled runqueue-wait medians were 0.57% for the
parent and 0.32%/0.38% for the two candidate runs; p90 values were
4.95%/0.89%/1.71%. Involuntary-switch outliers occurred, so these are
matched bounded comparisons rather than claims about every host. Raw
process distributions, checksums, backend/lane dispatch, errors and host
samples are in [`data`](data/plan14-ordered-spectrum-output-2026-10-21/).

The T2047 phase probe before this change measured about 7.1 ms in the
standalone spectrum-scatter stage. The 20.4 ms whole-call reduction includes
removing both the intermediate-array pass and the second parallel region;
the benchmark does not isolate their individual shares. The T720 m=1 gain
also shows that avoiding the extra parallel scatter region matters even when
the order array itself is small.

## Validation and next work

The release parallel and no-default-feature suites, strict release Clippy,
formatting, ordered-versus-ring synthesis, and subnormal/extreme fallback
tests passed on this source. The native benchmark compared all seven
configurations above against DUCC; maximum measured relative L2 was
`1.84e-13`. The Rust test suites separately exercised roundtrip, basis,
zero, exceptional-value, and adjoint behavior.

Keep this direct-output path. Next, follow Plan 14 Phase A: obtain a clean,
current-source q1 small-band baseline and phase attribution for T4/T8/T16/T32
(with T48/T63/T64 controls). Attribute the largest absolute fixed cost
before changing production code. Then remeasure the residual q8 synthesis
stages from the direct-output source; do not attribute the entire remaining
T720 reduced-order excess to recurrence without stage evidence.

Reproduction commands (from the repository root):

```sh
export DUCC_SRC="${DUCC_SRC:?set this to the pinned DUCC checkout}"
export PLAN14_RESULTS_DIR="${PLAN14_RESULTS_DIR:?choose an output directory}"
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo build --release --features parallel --example ducc_full_transform_parity
CXX=g++-15 \
  python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --skip-build --lmax 720 --operation synthesis --threads 8 \
  --results-dir "$PLAN14_RESULTS_DIR/t720" --no-report
CXX=g++-15 \
  python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --skip-build --lmax 2047 --fixed-ntheta 2049 --fixed-nphi 4096 \
  --full-mmax-only --operation synthesis --threads 8 \
  --results-dir "$PLAN14_RESULTS_DIR/t2047" --no-report
```

For the parent measurement, create a detached worktree at `a6059cd`, build
the same example into an isolated target directory with the same Rust flags,
then run the same two commands from that worktree with its target directory
selected by `CARGO_TARGET_DIR`. The retained JSON records the source SHA and
confirms the worktree was clean for each run.
