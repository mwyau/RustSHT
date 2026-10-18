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

No performance measurement has been collected for this source revision. The
previous T1023/T2047 speedups and cache-screen timing ratios apply to earlier
candidate sources only. This accumulator prototype is not accepted and does
not change production routing. The next decision requires a prospectively
frozen, validity-gated whole-transform comparison against both the previous
tile-128 candidate and the clean baseline.
