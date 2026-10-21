# Plan 14 q1 small-transform baseline and T127 analysis attribution

**Date:** 2026-10-21. **Purpose:** rebaseline the small serial cases after
Plan 11 and identify whether the reduced-`mmax` analysis gap is front-end,
scaled-prefix, or ordinary recurrence work. These measurements do not compare
against the earlier Rust 1.99.0/DUCC `80388904` sweep.

## Benchmark contract

- RustSHT source: `cb09b04f` for T4–T64 and `dc63adba` for the T48/T127
  follow-ups. The intervening `fe0590ac` change is confined to ordered q8
  synthesis output; the q1 paths measured here are unchanged. The T48 and
  T127 run used a clean tracked worktree.
- Rust 1.99.1, LLVM 23.1.1, Pulp 0.22.3, `-C target-cpu=x86-64-v3
  -Aunexpected_cfgs`; the worker reports runtime Pulp V3. DUCC 0.42.0 at
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, SHT kernel flags
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, strict native
  driver. Both execute four f64 lanes on AVX2/FMA.
- Reusable plans and workspaces, deterministic maps/coefficients, q1 pinned
  to one physical CPU, interleaved Rust/DUCC process pairs. Each case has 30
  calibrated batches across three process pairs, targeting 25 ms per batch;
  cold first-call/setup and warmed transform timings are separate. Numerical
  outputs are compared outside timed regions.
- 64 cases cover T4, T8, T16, T32, T48, T63, T64 and T127, full and quarter
  `mmax`, odd-minimum/even-smooth longitude sampling, analysis and synthesis.
  Maximum relative L2 error across these rows was `2.35e-14`; the T127
  subset's maximum was `9.15e-15`.

Raw CSV/JSON results:

- [T4–T64](data/plan14-small-q1-2026-10-21/t4-t64/)
- [T48 control](data/plan14-small-q1-2026-10-21/t48-control/)
- [T127 control](data/plan14-small-q1-2026-10-21/t127-control/)

## Selected warmed q1 results

Means below combine the odd/even longitude cases for the same operation and
`mmax`. They summarize this small screen; individual case distributions are
in the CSV files.

| Case | Rust µs | DUCC µs | Rust/DUCC | Excess µs |
| --- | ---: | ---: | ---: | ---: |
| T32 full analysis | 31.60 | 21.61 | 1.46× | +9.98 |
| T32 quarter analysis | 10.61 | 10.36 | 1.04× | +0.25 |
| T48 quarter analysis | 23.85 | 16.94 | 1.41× | +6.90 |
| T48 full analysis | 75.39 | 102.54 | 0.89× | −27.15 |
| T63 quarter analysis | 42.12 | 32.94 | 1.33× | +9.18 |
| T63 full analysis | 73.82 | 202.38 | 0.50× | −128.56 |
| T64 quarter analysis | 42.53 | 28.75 | 1.48× | +13.78 |
| T64 full analysis | 84.74 | 98.43 | 0.89× | −13.68 |
| T127 quarter analysis (`mmax=31`) | 192.46 | 99.55 | 1.94× | +92.92 |
| T127 full analysis | 380.21 | 276.73 | 1.37× | +103.48 |
| T127 quarter synthesis | 118.41 | 103.93 | 1.14× | +14.47 |
| T127 full synthesis | 373.73 | 305.16 | 1.22× | +68.57 |

The dispatch policy explains some, but not all, of the non-monotonicity:
reduced-order T48/T63/T64/T127 analysis uses recurrence; full-order T63/T64/
T127 analysis is table-eligible at q1. T64 and T127 synthesis use recurrence
under the existing q1 policy. The T63 full-order advantage and the T127
full-order excess show that “table versus recurrence” alone does not explain
the remaining gap. These are small absolute times, and the table records
process distributions and contention indicators rather than treating any
one ratio as a general crossover.

## T127 reduced-order stage evidence

For T127, `mmax=31`, `ntheta=128`, `nphi=65`, Rust warmed analysis was
189–196 µs across longitude shapes. A temporary test-only phase probe on the
same plan measured a 189.4 µs median whole call:

| Rust phase | Median µs |
| --- | ---: |
| Forward FFT | 19.96 |
| Order spectrum preparation | 4.95 |
| Latitude setup | 2.49 |
| Legendre recurrence and coefficient accumulation | 160.95 |
| Coefficient initialization/cleanup and dispatch | 0.57 |

The profile observed one leaf for orders 0–31, 4,096 active physical
ring/order pairs, estimated recurrence work of 460,800 active
ring-degree terms, 32 normal blocks and zero scaled blocks. The warmed
reduced-order route therefore does not pay a scaled-prefix fallback.

A temporary timer-only DUCC build, with pinned `sht_inner_loop.h` and only
non-kernel `sht.cc` timing hooks changed, measured 34.34 µs in `map2leg` and
69.84 µs in `leg2alm_internal` per call. The whole native DUCC worker was
about 108 µs. Stage boundaries are not identical: Rust's `legendre` phase
excludes order/latitude preparation, while DUCC's `leg2alm_internal` includes
ring packing and initialization. Even adding Rust's 7.44 µs of order and
latitude preparation leaves roughly 98 µs in Rust's Legendre side versus
69.84 µs in DUCC's, while Rust's FFT/frontend phases are about 6.5 µs faster
than DUCC's `map2leg`. This locates the whole-call excess in Legendre work,
not the FFT or scaled-prefix path.

## Executed V3 loop inspection

The exact Rust specialization is
`fused_scalar_analysis_pair_block_impl::<true, false>` on the production
fused path. Its recurrent SIMD body processes four f64 latitude lanes and
uses packed `vfmadd*pd` operations for both recurrence and accumulation. The
two-pair loop body contains 12 packed FMA instructions, eight coefficient
accumulator chains and two recurrence-state updates. Horizontal reductions
and the one-to-three-lane tail are outside the repeated packed body. The
inspected vector loop has no scalar `sd` recurrence operation and no
stack-based vector spill/reload; the function reserves 568 bytes for its
larger setup/tail/error-path state, which alone is not evidence of a hot-loop
spill.

The pinned GCC DUCC `map2alm_kernel` loop also uses four f64 lanes and packed
FMA, with eight accumulation chains and two recurrence states in its
two-coefficient group. Its inspected body contains 13 packed FMA
instructions; it reduces into the output after the SIMD lane traversal using
the AVX2 horizontal-sum helper. Neither hot loop is scalarized or visibly
spilling. The assembly does not support a missing-AVX2 or obvious scalar
instruction explanation for the nearly 2× T127 reduced-order stage gap.
Both loop bodies perform similar mathematical work; the remaining difference
needs a narrower same-input kernel/dataflow comparison before a production
rewrite. Raw disassembly and profiler logs remain local, not committed.

Runqueue wait in the calibrated T127 processes had a 7.2% median and 27.0%
p90, with about 34 involuntary switches per worker process. The batch protocol
was calibrated and process-paired, but host scheduling adds uncertainty; the
stage gap is large enough to prioritize, not precise enough to attribute to a
single instruction sequence.

## Decision and next work

No small-q1 source change is justified by this screen. The T127 reduced-order
case is the strongest q1 analysis lead, but its packed recurrence already
matches DUCC's basic lane width and degree grouping, so the next useful step
is to compare the same active-ring fixture through the isolated ordinary
kernel and account for per-order initialization/data movement. Continue
Plan 14's separate q8 synthesis stage attribution from the current direct
ring-spectrum-output source; do not change table cutoffs or recurrence
arithmetic based on the small screen alone.
