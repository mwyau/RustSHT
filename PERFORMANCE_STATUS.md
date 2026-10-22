# RustSHT performance checkpoint — current findings

**Active sequence (2026-10-22):** the scalar-analysis cumulative ablation is
closed; Plan 16 is active at Milestone E. Plans 14 and 15 are complete. Plan 17
is queued; Plans 12 and 13 remain deferred. **Branch:**
`opt/sht-performance-parity`.

## Scalar-analysis cumulative ablation

Production remains on staged output, the checked range-proof cache, and the
fixed 256 reflected-pair tile. The original `ce710` baseline, fixed workspace
layout, packed reduction, range-cache control, direct-output path, and
cumulative 64/128/256-tile variants were compared. The packed reduction and
layout change lacked supported whole-transform gains; 64/128 tiles were
slower; direct output was 2.42–2.81× slower in the final matrix and violated
cold/warm reset consistency. The no-cache route was 25–40% slower. See the
[closeout report](reports/scalar-analysis-cumulative-ablation-2026-10-21.md)
and [sanitized final matrix](reports/data/scalar-analysis-cumulative-ablation-2026-10-21/final-combined/README.md).

## Plan 16 — spin-1 recurrence and dataflow

Milestone A's executed-path crosswalk and matched stage profile are in
[the crosswalk](reports/plan16-spin1-executed-path-crosswalk-2026-10-22.md).
Milestone B tested fused two-channel and one-channel parity-separated
recurrences. Neither improved whole transforms. One-channel removed YMM spills
but duplicated recurrence, regressing q1 synthesis about 23%; q8 was neutral
to slower. Production remains unchanged. See the [A/B report and data](reports/plan16-spin1-parity-separated-ab-2026-10-22.md).

Milestone C tested reusing prepared paired coefficients across latitude blocks
within each ring group. At T720 CC q1, preparation calls fell from 5,293 to 721
full order and from 1,784 to 181 at `mmax=180`; matched q1/q8 whole-transform
changes stayed within ±0.92% and were not repeatable. The q8 busy-worker cost
was at most about 1.24 ms of 23–25 ms full calls; a transform-wide cache would
add about 8 MiB at full order. The prototype was removed. See the [report and
timing data](reports/plan16-spin1-paired-coefficient-preparation-2026-10-22.md).

Milestone D's order-major q8 synthesis queue is now the production route when
the prepared coefficient cache and dynamic group queue are eligible and at
least two 8-order tiles are available per worker. On T720 GL/CC/F1 it reduced
q8 warmed synthesis latency by 19–59% across full/reduced order and even/odd
longitude cases. Same-binary phase profiles attribute the gain to cutting
worker-summed recurrence-factor preparation (13.5→1.3 ms full CC; 6.4→0.6 ms
reduced CC) and improving worker balance (2.69×→1.01× full; 5.48×→1.30×
reduced). The remaining Rust/DUCC q8 ratio is 2.03–2.84×. See the [Milestone D
report](reports/plan16-spin1-order-major-dataflow-2026-10-21.md).

**Next:** Plan 16 Milestone E: test T1023 coefficient-cache eligibility and,
separately, CC/F1 spin-1 analysis resampling overhead.

## Other measured context

### Owner-directed scalar table/recurrence dispatch sweep (2026-10-22)

The forced-route screen covers all 257 degrees with verified table and
recurrence execution, q1/q8, scalar f64 GL analysis/synthesis, and odd/even
longitude shapes. Independent 30-batch repeats support the current q1 analysis
20 MiB table-footprint threshold; they show recurrence faster for q8 analysis
and for q1/q8 synthesis across the production-eligible region. The only exact
grid scalar production route at T63 that contradicts this evidence is q8 table
synthesis. The q1 T255 and q8 T63 analysis exceptions are redundant with the
existing byte/thread rules. The table minimum and maximum are retained because
the low-T warm results do not establish a safe eager-build/reuse policy.
No production code changed in the measurement commit. See the
[forced-route report](reports/scalar-legendre-table-recurrence-sweep-2026-10-22.md)
and its coverage manifest.

The completed scalar GL T64–T256 dispatch diagnosis attributes the q8 analysis
cliffs to compact-table selection and serial row FFT eligibility, not a recent
source regression. The current policy uses recurrence for q8 exact-grid
analysis and a 4096-sample scalar GL row-FFT work floor. T124 full q8 analysis
improved from 2.52–2.75× to 0.68–0.95× DUCC. The remaining full-order
T248–T256 synthesis gap is about 1.15–1.45×. See the [diagnostic report](reports/scalar-t64-256-performance-discontinuities-2026-10-21.md).

Plan 15's GL→CC→F1 sequence is complete. Its remaining CC/F1 synthesis excess
was attributed to recurrence/spectrum work; tested dataflow candidates did not
justify another production kernel. See the [Plan 15 closeout](reports/plan15-spin1-synthesis-dataflow-closeout-2026-10-21.md).

**Constraints:** preserve strict Rust floating-point semantics, checked
exceptional-value fallbacks, portable Pulp SIMD, the stable API, and the
odd-length FFT hold. Keep DUCC 0.42.0 pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` for current comparisons.
