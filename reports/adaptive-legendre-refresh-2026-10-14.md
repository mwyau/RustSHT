# Adaptive Legendre policy refresh

## Why the selector was rechecked

The compact-table selector was originally measured before later f64 spin-1
kernel changes. The latest accepted spin-1 synthesis change removes repeated
coefficient-row bounds checks in the paired recurrence. I compared the current
automatic table path with a benchmark-only recurrence build to determine
whether the single-thread synthesis table range remained useful. The earlier
[adaptive strategy report](adaptive-legendre-strategy-2026-10-14.md) retains
the full scalar/spin-1 band, truncation, operator, thread-count, memory,
SHTns, and reuse-count study.

## Method

Baseline source was `79b9eff` on `main`. The automatic executable was built
from that source. A second executable was compiled from the same source with
temporary q1 spin-1 table gates disabled; no transform arithmetic was changed
in that build. The gates were removed before production edits. No public
benchmark control or environment variable was added.

The native comparison used f64 GL spin-1 STANDARD synthesis and analysis,
full `mmax`, `phi0=0.37`, exact-minimum grids, reused plans/workspaces, one
warm-up and five timed calls per process. Five fresh-process pairs per case
alternated run order, pinned to CPU 0. `m1b_bench` also recorded plan and
workspace creation and a first transform on a fresh plan. Warm synthesis
latency is `synthesis_one_thread_ms`; warm analysis latency is
`analysis_rayon_ms` in its one-thread Rayon pool. The first-call column is the
fresh-plan first-call measurement. Full process output is in
[`reports/data/adaptive-legendre-refresh-2026-10-14`](data/adaptive-legendre-refresh-2026-10-14/).

CPU: AMD Ryzen 9 5950X, Linux `7.0.14-20-pve`. Rust `1.99.0`
(`b940084d7`, 2026-09-28), LLVM `23.1.1`; release `opt-level=3`, 16 codegen
units, LTO off, Pulp runtime dispatch (AVX2/FMA), no `target-cpu=native`.
DUCC comparisons used installed `ducc0==0.41.0`, source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

Representative native commands were:

```text
taskset -c 0 target/release/examples/m1b_bench 1 159 1 synthesis GL STANDARD 160 321 159
taskset -c 0 target/release/examples/m1b_bench 1 159 1 analysis GL STANDARD 160 321 159
```

The recurrence executable used the same release build with the temporary
q1 control setting `SPIN1_TABLE_SYNTH_MAX_LMAX` to `0` and restricting
`allow_spin1_analysis_table()` to
`current_execution_thread_count() != 1`. The automatic executable used the
unmodified selector at `79b9eff`. All nine native cases, process ordering,
per-process medians and timed samples are in the data directory above.

## Current-source q1 results

Times are medians of five per-process medians in milliseconds. “Paired table
gain” is `(recurrence - table) / recurrence`, so positive values favor the
table. All synthesis and analysis cases in the table had five pairs.

| Operation | Band and grid | Table warm | Recurrence warm | Paired table gain | Table first call | Recurrence first call |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Synthesis | T127 `128x257` | 2.649 | 2.760 | +4.47% | 6.594 | 2.884 |
| Synthesis | T144 `145x291` | 3.824 | 3.282 | -16.36% | 9.642 | 3.468 |
| Synthesis | T159 `160x321` | 5.097 | 4.998 | -1.80% | 12.810 | 5.274 |
| Synthesis | T175 `176x353` | 6.563 | 7.144 | +8.11% | 16.884 | 7.578 |
| Synthesis | T191 `192x385` | 7.870 | 9.146 | +14.00% | 21.097 | 9.591 |
| Analysis | T85 `86x173` | 0.508 | 0.572 | +11.03% | 1.676 | 0.586 |
| Analysis | T127 `128x257` | 1.080 | 1.345 | +19.63% | 4.882 | 1.351 |
| Analysis | T159 `160x321` | 2.252 | 2.636 | +14.40% | 9.774 | 2.653 |
| Analysis | T191 `192x385` | 2.917 | 3.281 | +12.06% | 15.863 | 3.327 |

Spin-1 analysis table gains remain clear in this q1 recheck. The T85 and T191
series each had one shared-host outlier, but table medians favored the table
in four of five pairs. The separate T511 analysis scaling and q8 table results
remain in the prior reports; this synthesis-only selector change does not
alter those paths.

For synthesis, all T144 and T159 table pairs were slower. The table does not
amortize at either band because its warm transform is slower as well. T127,
T175 and T191 tables won in all five pairs. Using measured first-call and
warm medians, estimated break-even reuse is about 35 calls at T127, 18 at
T175 and 11 at T191. T144 and T159 have no warm break-even. Plan construction
remains about `0.01–0.05 ms`; the table is lazy, so its added setup appears in
the first transform rather than plan creation. Workspace creation did not
change. Table payloads are `4.094 MiB` at T127, `5.976 MiB` at T144,
`7.959 MiB` at T159, `10.576 MiB` at T175 and `13.711 MiB` at T191. These
are table payload sizes, not measured RSS. The existing 34 MiB per-table cap
is unchanged.

## Selector decision

The q1 spin-1 synthesis table gate now selects the measured T127 point and
the `175 <= lmax <= 191` interval. It uses recurrence for T128–T174,
including the measured T144 and T159 regressions. The gap is deliberate:
unmeasured bands use the bounded-memory recurrence rather than extrapolating
across the observed change in performance. Existing requirements for exact
GL geometry, safe coefficient range, full/near-full truncation, spin-1
STANDARD mode, one executing thread, and the 34 MiB memory cap remain in force.
Multithread synthesis continues to use recurrence. Spin-1 analysis policy,
scalar policy, f32, CC/F1, GRAD_ONLY, DERIV1, and high-band recurrence
eligibility are unchanged.

This policy is deterministic and uses the executing pool, not plan-creation
thread count. It has no calibration, mutable global state, or new public API.

## DUCC and numerical validation

The rebuilt Python plan was compared with installed DUCC0 0.41.0 for T127,
T144, T159, T175 and T191 using matching deterministic coefficients, exact
GL grids, `phi0=0.37`, and one thread. Maximum absolute, RMS, and relative-L2
map errors were respectively:

| Band | Max absolute | RMS | Relative L2 | RustSHT / DUCC plan latency |
| --- | ---: | ---: | ---: | ---: |
| T127 | `1.757e-15` | `1.323e-16` | `4.344e-14` | 1.77x |
| T144 | `1.312e-15` | `5.811e-17` | `1.685e-14` | 1.39x |
| T159 | `1.356e-15` | `7.239e-17` | `1.902e-14` | 1.45x |
| T175 | `1.695e-15` | `1.133e-16` | `2.707e-14` | 2.25x |
| T191 | `2.733e-15` | `1.954e-16` | `4.278e-14` | 4.01x |

The exact Python command pattern was:

```text
for spec in '127 257' '144 291' '159 321' '175 353' '191 385'; do set -- $spec; RAYON_NUM_THREADS=1 taskset -c 0 uv run python examples/python_perf_baseline.py "$1" 1 1 GL 2 plan synthesis both STANDARD "$2"; done
```

The Python timings used two batches and establish current plan/DUCC context;
they were not used to choose the table gate. The native table-vs-recurrence
measurements decide the selector. Full Python output and commands are in
[`python-ducc-checks.txt`](data/adaptive-legendre-refresh-2026-10-14/python-ducc-checks.txt).

The release `spin1_gl_table_synthesis_matches_recurrence_and_adjoint` test
now covers T72, T127, T128, T144, T159, T175, T191 and T255. Table-selected
outputs match the recurrence within `3e-11` absolute sample error and preserve
synthesis adjoint identities within `2e-11` relative error. The new selection
does not change table arithmetic. T511 spin-1/2 DUCC and round-trip checks on
GL/CC/F1 passed; T2047 sparse synthesis for spins 1–4 on GL/CC/F1 passed.
Spin-1 T511 DUCC synthesis errors remained at most `4.885e-15`; T2047 spin-1
DUCC middle-ring error was `2.137e-13` and analytic map error was
`1.344e-11`.

## Spin-1 synthesis table range preflight

A read-only review of the selector at `2d18ff8` found that q1 spin-1
synthesis materialized its table before the per-order converted coefficients
were checked. Extreme inputs fell back to recurrence correctly, but a first
call could still allocate and retain a 4–14 MiB table. The code now checks
each raw E/B real and imaginary component against one quarter of the table
input limit before first construction. The spin-1 coefficient conversion
forms each scalar component from at most three input components with factors
no larger than one; the remaining order-level check still decides whether a
converted order may use direct table multiplication. Once a table exists, the
raw coefficient scan is skipped and the order-level checks remain active.

The focused T127 test uses one finite `1e150` E coefficient. It confirms that
the first synthesis leaves the table uninitialized, produces finite maps, and
matches a separate recurrence-only plan exactly. Three alternating native
process pairs measured normal f64 GL STANDARD synthesis before and after the
preflight at T127 (`128x257`), T175 (`176x353`) and T191 (`192x385`), full
`mmax`, `phi0=0.37`, q1, with one warm-up and five timed transforms per
process. Warm medians changed from `2.651` to `2.659 ms` at T127, `6.587` to
`6.595 ms` at T175, and `7.905` to `7.841 ms` at T191. Fresh-plan first-call
medians changed from `6.552` to `6.545 ms`, `16.566` to `16.465 ms`, and
`20.736` to `20.674 ms`, respectively. These differences are below the
process variation; the added scan did not produce a measured transform
regression. Avoided table payloads are `4.094`, `10.576` and `13.711 MiB` at
those bands. Raw process outputs are in
[`spin1-table-safety-processes.txt`](data/adaptive-legendre-refresh-2026-10-14/spin1-table-safety-processes.txt).

The review found no other selector, numerical, race, or workspace issue.
Focused recurrence/table equivalence and the unsafe-input regression passed.
After the fix, the full all-feature suite passed (145 Rust and 10 f32 tests;
8 ignored), and the full no-default-feature suite passed (129 Rust and 10 f32
tests; 7 ignored). The T2047 sparse synthesis release regression passed for
spins 1–4 on GL, CC and F1. Formatting and workspace Clippy passed. The review
and its disposition are recorded in the validation notes.

Full checks passed: fmt, workspace Clippy with warnings denied, all-feature
tests (145 Rust and 10 f32 passed; 8 ignored), no-default-feature tests (129
Rust and 10 f32 passed; 7 ignored), and Python tests (16 passed). The
T511/T2047 opt-in release tests above were run separately. The scalar and
spin-1 table-versus-recurrence policy remains internal; no FFT code changed.
