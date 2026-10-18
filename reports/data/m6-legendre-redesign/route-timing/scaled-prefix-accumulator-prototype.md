# Scaled-prefix accumulator prototype

## Hypothesis

The earlier tile-128 source's T2047 route diagnostic sampled 12.384 ms in
scalar complex accumulation, 0.618 ms in recurrence updates, and 3.365 ms in
range checks and normalization across 16,215 sampled prefix pairs. These
sampled values belong to that pre-accumulator source and are not extrapolated
to its complete prefix or transferred to the current candidate. Its
accumulation path called scaled complex multiply for every lane and degree
pair.

This isolated prototype accumulates prefix contributions in the recurrence's
binary scale. It rebases the partial sum only when a lane's recurrence exponent
changes, then materializes the sum once when the ordinary SIMD suffix begins.
The ordinary suffix and tile scatter remain unchanged. The experimental
`synthesis_fast` route is still isolated from production routing.

## Safety and implementation

The accumulator remains transactional. Products use ordinary arithmetic only
when the recurrence mantissa and both real and imaginary factors are within
the existing scalar normal range. Exponent subtraction is checked. A rebase
or final materialization that is nonfinite or loses a nonzero value into the
subnormal range declines the fast route; the caller discards staged sums and
runs the established scaled visitor for the tile. No degree prefix is skipped.

The accumulator exponent array is a fixed 128-element stack value, matching
the isolated active-tile bound. The prototype adds no workspace allocation or
public API. Its source and focused patch are identified below:

- Prior cache candidate source SHA-256:
  `da5fe000c53cc2d5a28aea4de79fc770709de1cde7d26671dec70aa1ecfc2c7e`.
- Prototype source SHA-256:
  `edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`.
- Patch SHA-256:
  `3a75e88d6c6d0640005caaa486e29d8fcbb8ecdd08c5419da9d015f39d736873`.
- Patch: [`scaled-prefix-accumulator-prototype.patch`](scaled-prefix-accumulator-prototype.patch).

## Validation and status

The focused tile, mixed-polar, fallback, high-order transition, scaled-reference,
and accumulator-rebase tests passed. The complete workspace suite passed with
all features (201 unit tests, 14 ignored; all integration targets passed) and
without default features (162 unit tests, 7 ignored; all integration targets
passed). The focused tests were rerun after extracting the rebase helper, and
`cargo fmt --all -- --check` plus workspace Clippy with warnings denied passed.

### T1023 one-pair screen

The frozen [screen protocol](scaled-prefix-accumulator-screen-v1-protocol.md)
authorized one clean-baseline/candidate pair. The baseline and candidate
RustSHT series both passed the fixed stability rules. DUCC and SHTns series
also passed for both processes. Both Rust outputs had relative-L2 error
`1.182e-13` and maximum absolute error `1.556e-13` against DUCC.

| Variant / backend | Five samples (ms) | Median (ms) | CV | Drift | Peak RSS (KiB) |
|---|---|---:|---:|---:|---:|
| Baseline RustSHT | 302.708, 301.823, 302.222, 301.171, 301.132 | 301.823 | 0.2012% | 0.3547% | 278,440 |
| Baseline DUCC | 50.757, 52.018, 50.689, 51.036, 50.253 | 50.757 | 1.1576% | 0.2216% | — |
| Baseline SHTns adapter | 35.442, 35.677, 35.654, 36.155, 35.980 | 35.677 | 0.7085% | 1.1590% | — |
| Accumulator candidate RustSHT | 173.243, 171.871, 171.850, 172.617, 172.660 | 172.617 | 0.3064% | 0.4446% | 274,896 |
| Accumulator candidate DUCC | 50.834, 51.112, 51.519, 51.219, 48.808 | 51.112 | 1.9139% | 2.1492% | — |
| Accumulator candidate SHTns adapter | 37.535, 36.900, 36.599, 36.093, 36.883 | 36.883 | 1.2730% | 1.1170% | — |

The single-pair RustSHT candidate/baseline median ratio was `0.571915`, a
42.81% latency reduction. The candidate was 3.377x DUCC and 4.680x SHTns.
These single-pair ratios are descriptive, not acceptance evidence. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Warning-band results did not invalidate the measurements. The benchmark used one worker, and sampled execution matched the configured placement. No competing build or benchmark was observed. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Peak RSS values are summarized above; raw GNU-time and host-state captures remain local and untracked.

The predeclared 3% screen gate passed, so this result authorizes a separate
six-pair q1 matrix. It does not accept the candidate or authorize q8 or
scalar/spin-1 regression tests. The earlier tile-128 results remain tied to
their own source and binary hashes.

### T1023 six-pair q1 matrix

All six new pairs passed the fixed gates. The median paired RustSHT ratio was
`0.568982` with a 95% bootstrap interval of `0.563779–0.571237`; the candidate
median was 171.624 ms versus 301.582 ms for the clean baseline. Relative-L2
error against DUCC remained `1.182e-13`. Exact backend timing samples, peak RSS,
and summarized CPU-pressure and interference decisions are in the
[T1023 matrix report](scaled-prefix-accumulator-t1023-q1.md).

The six-pair T2047 q1 matrix also passed every gate. Its median paired ratio
was `0.407892` (95% bootstrap interval `0.407274–0.408108`), with candidate
and baseline medians of 1370.613 ms and 3360.075 ms. The candidate output had
relative-L2 error `3.015e-13` and maximum absolute error `1.110e-12` against
DUCC. Full timing samples, resources, and CPU-pressure validity decisions are in the
[T2047 matrix report](scaled-prefix-accumulator-t2047-q1.md).

The high-resolution part of the q1 gate passes. The T511 scalar control stopped
at baseline pair 03 because the DUCC series failed stability (CV `5.6218%`,
drift `8.2650%`); RustSHT and SHTns passed in that process. No retry or
candidate mate ran. The T511 control remains unresolved, so q8 and broader
scalar/spin-1 regression work are gated. The candidate remains experimental
and is not routed in production. See the [stopped T511 record](scaled-prefix-accumulator-t511-q1-stopped.md).

A test-only T2047 route profile of the frozen source measured 715.931 ms in
the pre-suffix transition, 543.517 ms in the ordinary SIMD suffix, 41.953 ms
in scaled fallback calls, and 5.393 ms in coefficient-safety scans. The route
profile records exact counters, sampled prefix components, hashes, and timer
scope in the [T2047 profile report](scaled-prefix-accumulator-route-profile-t2047.md).
The pre-suffix transition is the next measured kernel target; its exponent
rebasing and normalization costs need finer separation.
