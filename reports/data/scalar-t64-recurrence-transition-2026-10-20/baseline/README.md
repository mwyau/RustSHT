# Scalar dispatch experiment: pre-change baseline

This dataset records the production implementation before changing the spin-0
table-selection policy. Rust production code is from `980e8e4f`; the benchmark
case-selection and fixture-fingerprint harness is `d68d8e48`. Rust is 1.99.1
with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, using runtime Pulp dispatch.
DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` and compiled for
x86-64-v3 with `-O3 -ffast-math -ffp-contract=fast`; the native benchmark driver
uses `-O3 -fno-fast-math -ffp-contract=fast`. The host is an AMD Ryzen 9 5950X
with AVX2/FMA and no AVX-512.

The run used `--max-lmax 256`: 212 configurations over 61 degrees, including
all T0–T32, full and quarter-order cases, odd-minimum and even-smooth longitude
sampling, and the requested transition samples. It produced 848 operation / q
rows, 2,544 Rust-versus-DUCC output comparisons, and 5,088 backend process
records. Each backend row has 30 calibrated batches across three interleaved
process pairs. First-call and warm timings, complete output errors, selected
Pulp backend, and deterministic map/coefficient fixture hashes are in
`results.json` and `results.csv`. The same case IDs and hashes are required in
the candidate run.

Command:

```sh
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "${DUCC_SRC:?set DUCC_SRC to the pinned checkout}" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --max-lmax 256 \
  --results-dir reports/data/scalar-t64-recurrence-transition-2026-10-20/baseline \
  --no-report
```

## Pre-change full-order GL dispatch

The table-support column assumes f64, exact GL sampling (`ntheta=lmax+1`),
full `mmax`, default automatic reuse, and finite moderate inputs. An unsafe
table input still selects the robust recurrence route. The analysis and
synthesis strategy checks have distinct q- and degree-specific exceptions.

| lmax | Table supported | q1 synthesis | q1 analysis | q8 synthesis | q8 analysis |
|---:|:---:|:---:|:---:|:---:|:---:|
| 63 | yes | recurrence | table | table | recurrence |
| 64 | yes | recurrence | table | table | table |
| 127 | yes | recurrence | table | table | table |
| 128 | yes | table | table | table | table |
| 255 | yes | table | recurrence | table | table |
| 256 | no | recurrence | recurrence | recurrence | recurrence |

The table requires `mmax >= lmax - floor(lmax/4)`, so the benchmark's quarter-
order configurations and `mmax=1` do not select it. Other exceptions include
single-thread synthesis through T127, T63 q8 exact-grid analysis, and T255
single-thread full-order analysis. f32 does not currently support the compact
table. Spin-1 uses its own eligibility and operation policy; this scalar
experiment does not change it.

The following full-order, even-smooth warm medians and first-call times are in
milliseconds. They expose the measured transition cases before dispatch
changes; the host limitation below makes the q8 numbers provisional.

| lmax | operation | q | Rust warm | DUCC warm | Rust/DUCC | Rust first call | DUCC first call |
|---:|---|---:|---:|---:|---:|---:|---:|
| 63 | A | 1 | 0.0971 | 0.0626 | 1.550 | 0.6103 | 0.2948 |
| 63 | A | 8 | 0.0610 | 0.0446 | 1.367 | 0.0886 | 0.5613 |
| 63 | S | 1 | 0.0848 | 0.0638 | 1.330 | 0.1179 | 0.2916 |
| 63 | S | 8 | 0.0597 | 0.0451 | 1.324 | 0.5951 | 0.6256 |
| 64 | A | 1 | 0.2055 | 0.1523 | 1.349 | 1.0339 | 0.5279 |
| 64 | A | 8 | 0.1810 | 0.3458 | 0.523 | 0.9988 | 1.2349 |
| 64 | S | 1 | 0.1882 | 0.1453 | 1.295 | 0.2313 | 0.5006 |
| 64 | S | 8 | 0.1538 | 0.3352 | 0.459 | 1.0641 | 1.3052 |
| 255 | A | 1 | 2.7258 | 2.2403 | 1.217 | 3.1289 | 3.4260 |
| 255 | A | 8 | 5.5217 | 2.5917 | 2.131 | 50.3916 | 4.1124 |
| 255 | S | 1 | 9.0186 | 2.1868 | 4.124 | 52.8528 | 3.3740 |
| 255 | S | 8 | 5.0098 | 2.4239 | 2.067 | 53.6469 | 4.3521 |
| 256 | A | 1 | 2.8601 | 2.3675 | 1.208 | 3.2920 | 3.4964 |
| 256 | A | 8 | 1.8452 | 2.6672 | 0.692 | 3.4375 | 4.4306 |
| 256 | S | 1 | 2.8486 | 2.2268 | 1.279 | 3.1673 | 3.4414 |
| 256 | S | 8 | 1.6866 | 2.5348 | 0.665 | 3.6758 | 4.3371 |

## Measurement limitation

This is the saved pre-change baseline, but the host was highly contended. The
runner's aggregate run-queue wait was 20.4% median and 79.2% p90; sampled CPU
frequency was 4.02 GHz median (3.96–4.61 GHz p10–p90). Only 93.3% of short
operation batches were in the requested 20–100 ms window at the median case,
and the least-compliant case was 35%. All 30 samples remain present; none were
removed. Treat individual latency and q8 ratios as provisional and compare the
candidate distributions with this limitation in view.

All 424 per-case numerical validation suites produced finite outputs. Maximum
Rust/DUCC relative L2 error was `9.027e-14`; maximum absolute error was
`2.228e-10`. These comparisons include the complete permitted harmonic triangle
for each configuration. T0 remains in raw data and correctness coverage, but
will be excluded from throughput averages.
