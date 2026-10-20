# Adaptive Legendre strategy recheck at latest `main`

## Result

The compact north-hemisphere table remains useful for selected repeated GL
operations. The latest recurrence measurements change three selector
decisions:

- Scalar synthesis at exact-minimum T127 with one thread now uses recurrence. The table was consistently slower and added a cold build plus 4.03 MiB of
  retained values.
- Spin-1 STANDARD synthesis at exact-minimum T159 with one thread now uses the
  table. Five paired runs measured a 3.54% median warm gain; T144 remains on
  recurrence because its table was slower.
- Full-mmax spin-1 STANDARD analysis at T255 now uses recurrence for every
  pool with at least two threads. At q2 the table was effectively tied and
  carried a roughly 32 MiB payload; at q4, q8 and q16 it was slower. These production changes are in commit `08e84e8` on `main`. The table cap remains 34 MiB. Tables remain lazy and operation-specific. No
public API, transform convention, recurrence, FFT code or dense matrix was
added. The historical full strategy, cold-start, high-band and Torch-harmonics
comparison is in the [initial strategy report](adaptive-legendre-strategy-2026-10-14.md). The earlier [precomputation report](legendre-precomputation-2026-10-13.md)
compares the compact Rust table with Torch-harmonics' dense representation.

## Source and policy audit

The production algorithm baseline was RustSHT `3369014`; `acb77db` added the
latest-head investigation plan and changed no transform code. This recheck
followed the fused spin-1 Q/U analysis reducer in `614db91` and the staged
spin-1 synthesis coefficient factorization in `3369014`. The scalar table stores northern-hemisphere triangular `(m,l)` values with
contiguous latitude lanes and reflects the south by parity. It supports f64
GL, `63 <= lmax <= 255`, near-full `mmax`, and latitude counts from the exact
minimum through twice that count, subject to the 34 MiB cap. Spin-1 stores an
additional scalar degree. Its STANDARD synthesis and analysis gates are
separate; synthesis additionally requires exact GL sampling, safe converted
coefficients, eligible polar rings and one executing thread. Analysis uses
the current execution pool. f32, CC/F1, GRAD_ONLY, DERIV1, unsafe direct
products and unsupported geometries continue through their existing
recurrence or specialized paths. The benchmark-only strategy override was kept in a detached worktree based on
`3369014`. A single release executable read `RUSTSHT_LEGENDRE_FORCE=auto`,
`recurrence` or `table` at runtime. All three modes therefore used the same
machine code and build flags. The control was removed with the worktree; no
override remains in the production source. Initial compile-time-forced runs
are retained as exploratory samples, but selector decisions below use the
same-binary results. SHTns `sht_auto` and `sht_gauss` use Gaussian grids; `sht_gauss_fly` selects an
on-the-fly path, and `sht_quick_init` skips timing-based selection. Its
operation-specific timing and accuracy checks support choosing by operation. Its initialization-time measurement is not a direct model for RustSHT's
immutable plan, whose Rayon pool is selected at execution time. The read-only
source revision and detailed notes are recorded in the [initial strategy
report](adaptive-legendre-strategy-2026-10-14.md). Historical GL/CC workloads
and grid dimensions are in [`sht_bench`](https://github.com/mwyau/sht_bench/tree/1328f83feb5dc95b3e6708b51228d954820ff15a). Torch-harmonics PR #322 stores a rectangular order-degree-latitude tensor;
its folded CC/F1 projection is built in bounded blocks but cached in a dense
layout. Rust's table is triangular and uses north/south symmetry, with about
one quarter of the dense scalar GL payload. The prior T127/T255 CPU comparison
did not find a Torch dense-table advantage over recurrence or the compact
table. No Torch-like dense or blocked implementation was added. These storage
and GPU-memory results are detailed in the [precomputation report](legendre-precomputation-2026-10-13.md).

## Measurement method

The source and workload were pinned to the same x86-64 CPU host. The native
harness was `examples/m1b_bench.rs`, using f64, full `mmax`, `phi0=0.37`, its
deterministic coefficient/map inputs, reused plans and workspaces, one warm-up
and five timed transforms per process. Fresh-process order rotated across the
three strategy settings. q1 used one CPU core; q2/q4/q8/q16 used 2 pinned CPU cores, 0–3, 0–7
or 16 CPU cores. GL dimensions were exact minimum: T72 `73x147`, T85 `86x173`, T127
`128x257`, T144 `145x291`, T159 `160x321`, T175 `176x353`, T191 `192x385`,
and T255 `256x513` (`ntheta x nphi`). All measured cells used `mmax=lmax`. The main table below gives the median of paired process medians. A positive
table gain means `(recurrence - table) / recurrence`; a negative value favors
recurrence. Initial comparisons used three to five process pairs. The q2 T255
analysis result uses ten pairs. Raw timed samples, run order, first-call
measurements, RSS and exact commands are in
[`the process data`](../../data/adaptive-legendre-latest-head-recheck-2026-10-14). The combined medians are in [`strategy-summary.csv`](../../data/adaptive-legendre-latest-head-recheck-2026-10-14/strategy-summary.csv).

| Operation | T / threads | Recurrence (ms) | Table (ms) | Paired table gain | Pairs |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar synthesis | T127 q1 | 0.883 | 0.897 | -1.58% | 5 |
| Scalar synthesis | T127 q8 | 0.395 | 0.354 | +20.51% | 3 |
| Spin-1 synthesis | T127 q1 | 2.768 | 2.472 | +10.69% | 3 |
| Spin-1 synthesis | T144 q1 | 3.256 | 3.567 | -9.50% | 5 |
| Spin-1 synthesis | T159 q1 | 4.961 | 4.806 | +3.54% | 5 |
| Spin-1 synthesis | T175 q1 | 7.045 | 6.169 | +12.47% | 5 |
| Spin-1 synthesis | T191 q1 | 7.696 | 7.435 | +3.39% | 3 |
| Spin-1 synthesis | T191 q8 | 1.926 | 2.303 | -20.98% | 3 |
| Spin-1 analysis | T72 q1 | 0.317 | 0.274 | +13.56% | 3 |
| Spin-1 analysis | T72 q8 | 0.171 | 0.141 | +17.54% | 3 |
| Spin-1 analysis | T85 q1 | 0.579 | 0.520 | +9.82% | 3 |
| Spin-1 analysis | T85 q8 | 0.373 | 0.319 | +16.05% | 3 |
| Spin-1 analysis | T127 q1 | 1.296 | 1.112 | +16.20% | 3 |
| Spin-1 analysis | T127 q8 | 0.680 | 0.502 | +26.91% | 3 |
| Spin-1 analysis | T159 q1 | 2.512 | 2.261 | +10.49% | 3 |
| Spin-1 analysis | T159 q8 | 1.310 | 1.052 | +19.69% | 3 |
| Spin-1 analysis | T191 q1 | 3.133 | 2.893 | +7.66% | 3 |
| Spin-1 analysis | T191 q8 | 1.054 | 0.866 | +13.19% | 3 |
| Spin-1 analysis | T255 q1 | 6.492 | 8.275 | -27.25% | 3 |
| Spin-1 analysis | T255 q2 | 4.970 | 5.018 | -0.26% | 10 |
| Spin-1 analysis | T255 q4 | 3.392 | 3.508 | -3.42% | 5 |
| Spin-1 analysis | T255 q8 | 2.549 | 2.740 | -7.30% | 5 |
| Spin-1 analysis | T255 q16 | 2.493 | 2.720 | -9.11% | 5 |

The T127 spin-1 q1 analysis series included one high recurrence and one high
table process; its paired median favored the table, but the evidence is less
stable than cells whose pairs all agreed. q2 and q8 T255 analysis also had
isolated process outliers. The raw values are preserved, and the thresholds
use paired medians rather than single fastest samples. q2 T255 is treated as
no proven table benefit, not as a measured recurrence speedup.

## Selector decisions and cold cost

The scalar T127 q1 table is slower in all five same-binary pairs, by a median
1.58%, and its fresh-plan first call was `4.666 ms` versus `0.956 ms` for
recurrence. The scalar table payload is 4.03 MiB. The q8 table remains selected
because its median warm gain is about 20%; other scalar table gates retain the
existing measured policy. Spin-1 T159 q1 synthesis changes from recurrence to the table. Its first call
was `12.407 ms` versus `5.209 ms` with recurrence; its warm medians were
`4.806` and `4.961 ms`. At those medians, the table repays its first-call
difference after about 48 total transforms. It retains a 7.86 MiB payload;
observed peak RSS was about 15.1 MiB with the table and 7.1 MiB with
recurrence. The T144 table remains slower, so T128–T158 are not newly admitted
by extrapolation. q8 T191 also favors recurrence, so spin-1 synthesis remains
table-backed only for its single-thread measured bands. Full-mmax T255 spin-1 analysis now uses recurrence for q2 and above; q1 already
used recurrence. At q2 the table first call was about `36.1 ms` versus `5.03
ms` for recurrence and the paired warm difference was a negligible 0.26% in
recurrence's favor. q4, q8 and q16 table warm medians were 3.4–9.1% slower. The table first call cost was about 30–33 ms higher and peak RSS increased
about 34 MiB. There is no measured warm break-even for these full-mmax cells. The gate applies only to full-mmax T255; previously measured reduced-mmax
cases retain their separate policy. Plan construction stayed around `0.02 ms`; the table is lazy, so construction
time is not charged to plans that remain on recurrence. The benchmark reports
workspace allocation separately. Representative workspace medians were
`0.022 ms` for scalar T127 q1, `0.283 ms` for spin-1 T159 q1, and
`0.86–1.23 ms` for spin-1 T255 q2–q16; these buffers did not vary with the
forced Legendre strategy. First-call timings include both table construction
and the transform, so the table-build increment is estimated by subtracting a
matched recurrence first call; this does not isolate the builder from the
first transform. RSS includes the executable, Python/runtime where applicable
and normal transform buffers; payload sizes are the direct table storage
measurement. The T159 spin-1 table addition and q2+ T255 recurrence gate were validated by
the current production binary after the edits. Five fresh-process medians
were: scalar T127 synthesis q1 `0.870 ms` (range `0.860–0.872`), spin-1 T159
synthesis q1 `4.803 ms` (`4.798–4.824`), and spin-1 T255 analysis q2/q4/q8
`5.089/3.413/2.532 ms` (ranges `4.988–6.553`, `3.332–3.457`,
`2.476–2.570`). The q2 range includes one noisy process.

## DUCC numerical comparison

The rebuilt Python extension was compared with installed `ducc0==0.41.0`;
the reference source revision is
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. All cases used f64, GL,
`phi0=0.37`, matching deterministic coefficients or maps and the exact grid
dimensions listed above.

| Operation | Band / pool | Max absolute error | RMS error | Relative L2 error |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | T127 q1 | `5.412e-16` | `3.800e-17` | `1.283e-14` |
| Spin-1 synthesis | T159 q1 | `2.921e-15` | `1.983e-16` | `5.210e-14` |
| Spin-1 analysis | T255 q2/q4/q8 | `8.238e-15` | `2.090e-16` | `2.048e-14` |

The T255 spin-1 analysis error was the same across thread pools to the reported
precision. The production selection test also compares q1/q2/q4/q8/q16
analysis results on one plan. Existing analytic, DUCC, adjoint, polar and
table-fallback tests passed; no tolerance changed. The f32 and high-band
recurrence paths are unchanged. Opt-in T1023/T2047 tests were not rerun in
this selector-only change because all table candidates are capped at T255;
their previous validation is recorded in the [milestone history](../../../plans/05-validation-and-milestones.md). Python-plan elapsed times are retained beside each numerical result. They
varied across process pairs on the shared host and are not used to select the
policy. Native reused-workspace timings decide the table thresholds.

## Reproduction and validation

Environment: x86-64 CPU (host topology omitted), Linux, x86-64 AVX2/FMA, no AVX-512. Rust `1.99.0`
(`b940084d7`), LLVM `23.1.1`; standard release `opt-level=3`, 16 codegen
units, LTO off, Pulp runtime dispatch and no `target-cpu=native`. DUCC's wheel
compiler flags are unknown. The host was shared and CPU frequency varied; no

```sh
CARGO_TARGET_DIR=<temporary-path> \
  cargo build --release --example m1b_bench
RUSTSHT_LEGENDRE_FORCE=table taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" \
  /usr/bin/time -f 'max_rss_kib=%M' -- \
  <temporary-path> \
  1 159 1 synthesis GL STANDARD 160 321 159
```

The other forced modes use `RUSTSHT_LEGENDRE_FORCE=recurrence|auto` and the
same binary; the native benchmark arguments encode spin, bandlimit, thread
count, operation, grid, mode, `ntheta`, `nphi` and `mmax`. The temporary
worktree and override were removed after data capture. The production post-
policy benchmark used `cargo build --release --example m1b_bench` with no
override. Checks completed after the selector changes:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: 153 Rust tests and 10 f32 tests passed; 9 ignored.
- `cargo test -p rustsht --no-default-features`: 131 Rust tests and 10 f32 tests passed; 7 ignored.
- `uv run pytest python/tests`: 20 passed.
- Release Python-plan DUCC comparisons passed for scalar T127 synthesis, spin-1 T159 synthesis and spin-1 T255 analysis at q2/q4/q8. The static policy has no hidden calibration, mutable global tuning, new
dependencies or caller hint. It chooses at operation time from the current
pool and measured workload gates. The compact table is retained only where
warm whole-transform measurements justify its first-call cost and memory.
