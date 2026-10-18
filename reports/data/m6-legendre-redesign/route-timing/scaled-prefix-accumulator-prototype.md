# Scaled-prefix accumulator prototype

## Hypothesis

The T2047 route diagnostic sampled 12.384 ms in scalar complex accumulation,
0.618 ms in recurrence updates, and 3.365 ms in range checks and normalization
across 16,215 sampled prefix pairs. These sampled values are not extrapolated
to the complete prefix. The accumulation path called scaled complex multiply
for every lane and degree pair.

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
These single-pair ratios are descriptive, not acceptance evidence. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The values in
the 0.50–2.00% band are warnings under the unchanged protocol.

Observed benchmark-thread affinity and placement stayed within the requested
single-CPU set. No competing build or benchmark and no cgroup throttling were
observed. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. GNU-time context-switch counts remain local and peak RSS values shown above. Raw local paths,
process captures, CPU identifiers, frequency samples, PSI records, and GNU-time
output remain untracked.

The predeclared 3% screen gate passed, so this result authorizes a separate
six-pair q1 matrix. It does not accept the candidate or authorize q8 or
scalar/spin-1 regression tests. The earlier tile-128 results remain tied to
their own source and binary hashes.

### T1023 six-pair q1 matrix

All six new pairs passed the fixed gates. The median paired RustSHT ratio was
`0.568982` with a 95% bootstrap interval of `0.563779–0.571237`; the candidate
median was 171.624 ms versus 301.582 ms for the clean baseline. Relative-L2
error against DUCC remained `1.182e-13`. Exact backend samples, all PSI avg10
readings, peak RSS, and aggregate placement/frequency review are in the
[T1023 matrix report](scaled-prefix-accumulator-t1023-q1.md).

This passes the T1023 part of the q1 gate, not the full q1 gate. T2047 q1 is
the next measurement under the frozen matrix protocol. T511 remains the
required control before q8 or broader regression work. The candidate remains
experimental and is not routed in production.
