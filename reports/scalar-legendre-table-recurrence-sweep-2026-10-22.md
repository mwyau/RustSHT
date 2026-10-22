# Scalar Legendre table versus recurrence sweep — 2026-10-22

## Scope and source

This is a forced-route comparison on one production source revision. The
numerical and transform implementation is the production code at
`13b82470`; the benchmark-only route override, counters and timing harness are
at `c9de73237274f5a0a1a45be67fd53391701e1035`. No DUCC comparison is involved
in this report. The complete benchmark and coverage audit are in
[`reports/data/legendre-policy-2026-10-22/`](data/legendre-policy-2026-10-22/).

The Rust toolchain was 1.99.1 (`rustc 58cf8f9c3`, LLVM 23.1.1), with
`-C target-cpu=x86-64-v3`, Pulp 0.22.3 and the runtime AVX2/FMA backend. The
host was an AMD Ryzen 9 5950X, performance governor. q1 ran on physical CPU 0;
q8 ran on CPUs 0–7 (one CCD). Those CPUs are physical cores. There were no
source or compiler changes between the forced routes.

## Coverage and method

The full screen examined every integer `lmax=0…256` for scalar f64 exact-grid
GL, analysis and synthesis, q1 and q8, full `mmax`, and both odd-minimum and
even-smooth longitude sampling (`phi0=0.23`). Reduced `mmax=max(1,lmax/4)` was
also included at 16 boundary and transition degrees. The manifest contains
2,184 case rows, no missing cases and no unsupported table routes. Per-sample
route counters confirm that both forced algorithms actually ran in every
complete case. The maximum table payload estimate was 34,213,896 bytes, below
the 34 MiB table cap.

The exhaustive screen used five interleaved calibrated batches targeting
1 ms and is exploratory. Stronger checks used 30 interleaved batches targeting
20 ms per route, with independent test processes. Two q1 process repeats cover
the full focused crossover set; a third q1 run rechecks T120–128. Two q8
processes cover overlapping dispatch boundaries. An earlier q1 process is
preserved but excluded from the primary conclusions because system CPU-pressure
monitoring showed pronounced waiting during that process. The q8 second process
reported a brief late rise in pressure; its overlapping paired ratios match the
first q8 run.

`table_samples_us` and `recurrence_samples_us` retain all 30 per-call batch
observations. The table is constructed before the first table transform and
its build time and storage are recorded separately. The first table transform
then precedes the first recurrence transform on the same plan/workspace, so
those first-call values are sequence-coupled diagnostics, not independent
cold-process comparisons. Warm measurements alternate route order and are the
basis for the route decision. The reported reuse count is an estimate from the
measured build/first-call and warm costs, not a general guarantee.

## Existing production dispatch

For full-order, exact-grid f64 GL scalar plans, the route selected before the
follow-up selector change was:

| lmax | q1 analysis | q8 analysis | q1 synthesis | q8 synthesis |
|---|---|---|---|---|
| 63 | table | recurrence | recurrence | table |
| 64 | table | recurrence | recurrence | recurrence |
| 127 | table | recurrence | recurrence | recurrence |
| 128 | table | recurrence | recurrence | recurrence |
| 255 | recurrence (20 MiB payload rule) | recurrence | recurrence | recurrence |
| 256 | recurrence (table range ends at 255) | recurrence | recurrence | recurrence |

The decision is independent of odd-minimum versus even-smooth `nphi`. Reduced
order is rejected by the production table eligibility rule (at least 75% of
`lmax`), so those production routes use recurrence. Spin-1 has a separate
strategy and is not changed by the scalar policy work.

## Measured warm costs

Ratios below are table latency divided by recurrence latency; less than 1 favors
the table. Band summaries use the exhaustive screen and geometric means of
per-degree medians. They exclude T0 from throughput summaries. Both longitude
shapes show the same broad trend.

| q | operation | T63–96 | T97–127 | T128–191 | T192–222 | T223–256 |
|---:|---|---:|---:|---:|---:|---:|
| 1 | analysis | 0.77 | 0.91 | 1.09 | 1.47 | 2.03 |
| 1 | synthesis | 1.43 | 1.81 | 2.30 | 2.68 | 2.94 |
| 8 | analysis | 1.47 | 1.82 | 2.15 | 2.47 | 2.91 |
| 8 | synthesis | 2.71 | 4.06 | 5.58 | 7.04 | 7.47 |

Selected full-order even-smooth q1 analysis results from two independent
30-batch processes were stable: table/recurrence was 0.59 at T63, 0.79 at T64,
0.85 at T85, 0.93 at T96, 1.02 at T104 and T112, 1.05 at T120, 1.07 at T124,
1.16 at T126, 0.73 at T127, 1.09 at T128, 0.77 at T159, 0.86 at T191, 1.20
at T223, and 2.20–2.34 at T248–256. T126/T127/T128 is a real local variation
on this host, not a monotonic threshold; it does not justify a new exact-T
exception without a source-grounded explanation.

Across two q8 processes, table analysis was consistently slower from T63 on:
1.26–1.30× at T63–64, 1.60× at T127, 1.77× at T128, 1.58–1.79× at T159–223,
and 2.46–2.74× at T248–256. Table synthesis was 1.56–1.58× slower at T63,
2.43× at T64, 3.3–5.4× through T85–128, and 5.8–9.5× at T255–256. q1 table
synthesis was slower at every full-order sampled T≥63, increasing from 1.22–1.36×
at T63–64 to 3.2–3.4× at T255–256.

At low degrees, warm table routes can win even where the production policy does
not permit them. The table-build cost changes the first-use result: for
example, q1 table analysis at T31 saved about half the warm latency but needed
roughly three subsequent calls to recover the measured build cost; table
synthesis at T31 needed about 15 calls. T0–62 therefore does not support
eagerly lowering the production minimum without an operation-specific reuse
contract. At q8, the warm table advantage mostly disappears by T31–40.

Table-versus-recurrence output error remained small over the full screen:
maximum relative L2 error `8.23e-14`, maximum scaled error `1.60e-14`. This is
a route-agreement check, not an independent DUCC or analytic validation.

## Decision

The supported policy is a compact, resource-based rule for exact-grid scalar
GL: use recurrence for synthesis and for q8 analysis; use the table for q1
analysis only within the current eligible band and below the existing 20 MiB
payload cutoff. Keep the current minimum/maximum table range and reduced-order
eligibility: the low-degree warm wins do not establish cold/reuse behavior
well enough to expand eager table construction, and T256 remains recurrence
even though its experimental table fits under the allocation cap.

The q8 T63 exact-grid synthesis table exception is measured to lose and will
be removed in the production selector. The exact-grid q8 T63 analysis and q1
T255 exclusions are already implied by the general thread-count and payload
rules; their scalar-specific duplicates can be removed while preserving
shared spin-1 exclusions. The T63/64 and T255/256 boundaries remain as table
eligibility/resource boundaries, not hand-tuned exact-T performance hacks.

## Reproduction and data

The benchmark is the ignored Rust test
`transform::tests::benchmark_scalar_legendre_table_vs_recurrence_dense` in
`src/transform/tests.rs` at `c9de7323`. It accepts explicit `lmax` lists,
reduced-degree lists, q1/q8, batch duration, batch count and output path via
the `RUSTSHT_LEGENDRE_BENCH_*` environment variables. A representative run is:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
RUSTSHT_LEGENDRE_BENCH_THREADS=1,8 \
RUSTSHT_LEGENDRE_BENCH_MIN=0 RUSTSHT_LEGENDRE_BENCH_MAX=256 \
RUSTSHT_LEGENDRE_BENCH_TARGET_US=1000 RUSTSHT_LEGENDRE_BENCH_BATCHES=5 \
RUSTSHT_LEGENDRE_BENCH_OUTPUT=legendre-screen.csv \
cargo test --release --lib transform::tests::benchmark_scalar_legendre_table_vs_recurrence_dense \
  -- --ignored --exact --nocapture
```

The checked-in `table-recurrence-screen.csv`, `coverage-manifest.csv`,
`coverage-audit.json`, `screen-summary.csv`, and independent repeat CSVs under
`reports/data/legendre-policy-2026-10-22/` are the numerical evidence. Local
console, frequency and PSI captures are retained untracked on the benchmark
host; the relevant contention summary is recorded above.
