# M20 radix-eleven mixed-pass optimization

Date: 2026-10-16. Baseline source: session-start revision
`cd5748bb0bf5a93d43143a0bb0b9119e4d672983`. The M20 candidate is commit
`343e5adc08e44cc5472032138dfb263bb34b4786`; its benchmark executable SHA-256
is `c760c0309e641c564cdfe5c7e68e4df27eeb0739cfa0efc307ff9d4973cf5da3`.

## Result

The new AVX2 radix-eleven pass improves full C2C forward execution by 34–36%
at N=726, 1452 and 2904, and inverse execution by 25–28%. N=1442, which has no
radix-eleven stage, remains at parity. The N=682 control remains at parity
because its original specialized kernel is retained.

The N=257 real transforms are slower by 3–5% in the final paired capture, an
absolute difference of 0.07–0.11 microseconds per transform. N=2049 R2C is
1.2% slower; N=2049 C2R is 1.7% faster. These are retained as measured control
results. The other tested real operations are near parity or faster.

## Method

The baseline executable was built from M18 production source and has SHA-256
`a97674f383732a07ee860887f5407c12659874ee70eb8d9ab0aa058d968dca17`. The
candidate and baseline used Rust 1.99.0 stable, the optimized bench profile,
portable target flags, runtime AVX2 dispatch and one transform thread. Each
process was pinned to logical CPU 30 on an AMD Ryzen 9 5950X. Each comparison
used six alternating process pairs and seven warmed samples per operation.
The host load averages recorded across the final target capture ranged from
2.26 to 3.62 at one minute. No separate SHT benchmark overlapped that capture.

The target capture used `bench-native-sht`, which measures the complete public
complex and real operations, including real packing, reconstruction and
normalization. The configured row counts were 128, 361, 512, 721, 1024 and
2048 where applicable. Reported ratios are M18 time divided by M20 time;
values above 1 favor M20. Intervals are bootstrap 95% intervals over the six
paired process ratios.

## Complex SHT targets

Times are microseconds per transform. The relative result is M18/M20.

| Operation | N | Rows | M18 | M20 | M18/M20 (95% interval) |
| --- | ---: | ---: | ---: | ---: | ---: |
| C2C forward | 726 | 361 | 6.44 | 4.74 | 1.361 (1.351–1.406) |
| C2C inverse | 726 | 361 | 7.37 | 5.79 | 1.275 (1.261–1.320) |
| C2C forward | 1,442 | 721 | 55.52 | 55.38 | 1.004 (0.973–1.030) |
| C2C inverse | 1,442 | 721 | 60.58 | 61.10 | 0.991 (0.965–1.030) |
| C2C forward | 1,452 | 721 | 13.07 | 9.70 | 1.349 (1.333–1.407) |
| C2C inverse | 1,452 | 721 | 14.90 | 11.86 | 1.258 (1.242–1.314) |
| C2C forward | 2,904 | 721 | 27.00 | 20.17 | 1.337 (1.332–1.393) |
| C2C inverse | 2,904 | 721 | 30.57 | 24.49 | 1.248 (1.242–1.301) |

The target route uses the first radix-eleven pass, where `ido == 11`, and keeps
the later `ido == 1` pass scalar. The planner stages are `[2, 3, 11, 11]` for
N=726, `[4, 3, 11, 11]` for N=1452 and `[8, 3, 11, 11]` for N=2904. N=1442
uses `[2, 7, 103]` and does not select the new kernel.

## Real transforms and regression controls

Times are microseconds per transform. These controls help identify changes
outside the optimized complex passes.

| Operation | N | Rows | M18 | M20 | M18/M20 (95% interval) |
| --- | ---: | ---: | ---: | ---: | ---: |
| R2C | 257 | 128 | 2.118 | 2.227 | 0.952 (0.894–0.960) |
| C2R | 257 | 128 | 2.416 | 2.484 | 0.967 (0.894–0.993) |
| R2C | 723 | 361 | 11.015 | 11.056 | 1.001 (0.983–1.128) |
| C2R | 723 | 361 | 11.762 | 11.908 | 0.990 (0.970–1.011) |
| R2C | 2,049 | 1,024 | 31.630 | 32.049 | 0.988 (0.982–0.993) |
| C2R | 2,049 | 1,024 | 34.818 | 34.401 | 1.017 (0.998–1.022) |
| R2C | 4,097 | 2,048 | 103.155 | 101.657 | 1.013 (1.008–1.073) |
| C2R | 4,097 | 2,048 | 117.689 | 116.928 | 1.007 (1.001–1.011) |

The N=682 comparison used one, 22 and 44 rows. At 22 rows, the forward
M18/M20 ratio was 0.995 (0.972–1.002), and inverse was 1.001 (0.980–1.007).
At 44 rows, the forward ratio was 0.994 (0.968–1.000), and inverse was 1.000
(0.979–1.011). The original N=682 radix-eleven kernel remains in use.

## Implementation and numerical checks

The new f64 AVX2 kernel applies the existing fixed radix-eleven butterfly to
two adjacent inner values in each vector. The caller processes the largest
even `ido` prefix with AVX2 and computes any odd final inner value with the
existing scalar routine. The kernel applies the pass twiddles and inverse
scale. It runs only after runtime AVX2 dispatch and only for eligible
radix-eleven passes. N=682 keeps its earlier `l1 == 2`, `ido == 31` kernel.
f32 continues to use the scalar radix-eleven pass.

Inverse inputs outside the deferred-normalization safety range fall back to
the scalar path. The added direct-pass test checks forward and inverse DFTs,
multiple `l1` and `ido` dimensions, odd tails and guard regions. Route tests
confirm AVX2 selection at N=726, 1452 and 2904, confirm the high-range inverse
fallback, and confirm f32 remains scalar. The existing full-plan direct-DFT
test covers forward and inverse transforms at all three optimized lengths.

The final workspace byte counts match M18 for all 34 operation/length cells.
Plan and workspace construction medians remain within the existing process
variation; this change adds no retained plan allocation, workspace storage or
steady-state allocation.

## Checks

On the final source, `cargo fmt --all -- --check`,
`cargo test --all-features` and
`cargo clippy --all-targets --all-features -- -D warnings` passed. The test run
reported 30 unit tests, 2 across-row tests, 31 complex tests and 40 real tests
passing. Release-mode regression tests remain part of M23 closeout.

## Independent review

The requested independent Sol High review found no blocking numerical,
dispatch, unsafe-memory, tail, scaling, source-notice, provenance or benchmark
validity issue. It confirmed the radix-eleven equations, twiddle indexing,
inverse scaling and scalar safety fallback. The N=257 and N=2049 R2C slowdowns
are measurable and disclosed above; no project threshold makes them blockers.
Release-mode numerical regression validation remains pending M23.

## Rejected dispatch variants

Routing N=682 through the generalized kernel reduced its row-batched forward
performance by about 1–3% in a separate control capture, with broad paired
variation. The final code retains the original specialized N=682 kernel.

An inline radix-eleven dispatch variant made the shared pass dispatcher
larger and slowed N=1442 inverse by 8–11% in paired captures. The final
implementation isolates radix-eleven handling in an out-of-line pass function;
the final N=1442 interval includes parity in both directions. All intermediate
configurations and raw process records remain under
[`benches/results/m20-radix11-mixed-pass/`](../benches/results/m20-radix11-mixed-pass/).

## Supporting data

- [Final six-pair target capture](../benches/results/m20-radix11-mixed-pass/m20-radix11-c760-quiet-six-pairs.jsonl)
  and [summary](../benches/results/m20-radix11-mixed-pass/m20-radix11-c760-quiet-summary.csv).
- [Target capture configuration](../benches/results/m20-radix11-mixed-pass/m20-radix11-c760-quiet-config.json).
- [Final six-pair N=682 control](../benches/results/m20-radix11-mixed-pass/m20-radix11-c760-n682-six-pairs.jsonl)
  and [summary](../benches/results/m20-radix11-mixed-pass/m20-radix11-c760-n682-summary.csv).
- [N=682 control configuration](../benches/results/m20-radix11-mixed-pass/m20-radix11-c760-n682-config.json).
