# Scalar GL T64–T256 performance discontinuities

**Date:** 2026-10-21  
**Branch:** `opt/sht-performance-parity`  
**Purpose:** bounded diagnostic and policy A/B before resuming Plan 15. Plan 14 remains closed.

## Findings

The large q8 ratios in the current scalar sweep were DUCC-relative deficits in the existing dispatch, not regressions introduced by a recent scalar code change. The production scalar sources in the sweep revision `e487b85b` and the Plan 15 adapter baseline `f3a2313c` are identical (`git diff` over the scalar plan/transform/Legendre sources is empty).

Two dispatch choices explain most of the T64–T256 q8 analysis and T64–T126 q8 synthesis excess:

1. **Row FFTs were gated by `ntheta >= 128 && nphi >= 255`.** This left high-work T64–T126 scalar GL grids serial for their longitude FFTs. It also left reduced-order grids at higher bandlimits serial when their `nphi` was below 255. Forced-route A/B showed the change is useful when the grid has enough work, but small reduced grids regress under worker scheduling.
2. **The compact Legendre-value table remained selected for q8 scalar analysis through T255.** It was markedly slower than the active-ring recurrence, with a growing penalty near the table's 34 MiB cap. The T256 recurrence route therefore appeared as a discontinuous improvement. At q1, table behavior is more mixed, so the policy keeps smaller tables and switches only above a measured payload crossover.

The integrated policies now use a 4096-sample row-FFT work floor for **f64 scalar GL only**, and select recurrence for exact-grid table-eligible scalar analysis at q8 and for q1 table payloads at or above 20 MiB. Spin-1, f32, CC/F1, synthesis table eligibility, oversampled table rules, and numerical safeguards remain unchanged.

### Boundary attribution and executed routes

The boundary changes have different causes; the old row-FFT gate did **not** cause the T63→T64 analysis jump. In the frozen baseline, T63 and T64 both executed serial row FFTs at q8. T63 exact-grid scalar analysis used its recurrence exception, while T64 selected the compact table: full-even analysis rose from 46.05 to 63.56 µs. The recurrence-only T64 control was 58.96 µs, and the row-FFT-only control was 37.24 µs; the combined policy was 30.09 µs. The current q8 policy therefore keeps both T63 and T64 on recurrence and enables row FFTs when there is enough grid work.

At T126→T127, both full-order analysis cases used the compact table, but the old row-FFT gate changed from serial to parallel at T127 (`ntheta` reaches 128 and `nphi` reaches 255/256). Baseline q8 even analysis fell from 190.09 to 108.99 µs across that boundary. Forcing row FFTs at T126 reduced it to 122.38 µs, while changing only T127's table route to recurrence reduced its 108.99 µs to 69.02 µs. Thus the observed boundary mixes two independent effects; neither alone explains all of the residual.

At T255→T256, full-order q8 row FFTs were already active on both sides. The baseline's table eligibility ended at T255, switching analysis from the slow compact table at T255 (604.71 µs) to recurrence at T256 (251.91 µs). Forcing row FFTs changed those medians by only −5.3% and −1.5%; the measured T255 recurrence candidate was 235.97 µs. This is a table eligibility cliff, not a row-FFT cliff.

The old longitude-size gate also excluded **reduced-order** q8 transforms well above T127 because their `nphi` stayed below 255. At T248–T256 quarter order, forced row FFTs cut full-transform analysis by about 48–63% and synthesis by about 23–58% across even/odd sampling. The current sample-count rule enables these cases when `ntheta*nphi >= 4096`; T64 quarter-order grids below that work floor remain serial because forcing workers cost 9–28% there. At q1, FFT execution remains serial because there is only one worker, regardless of the capability predicate.

The T127 compact-table SIMD report is a separate control: its AVX2 reflected-lane accumulator improved q1 by 5.5%, while q8 changed by +1.1% (within the observed spread). It did not account for the q8 table penalty; the recurrence A/B did. The old row-FFT gate, table-vs-recurrence choice, and table SIMD specialization must therefore be treated as separate mechanisms.

## Matched whole-transform A/B

RustSHT candidate `985fa844` and baseline `f3a2313c` were measured against DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. For each case, runs used q1/q8, both operations, full and quarter `mmax`, even-smooth and odd-minimum `nphi`, three process pairs, and ten calibrated ~25 ms batches per process pair. RustSHT and DUCC were interleaved within each process pair. q1 used CPU 0; q8 used CPUs 0–7 on one L3 domain. Rust used `-C target-cpu=x86-64-v3`; DUCC numerical sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with its validation driver compiled using `-fno-fast-math`.

The table shows representative full-order cases. Latencies are medians in microseconds; ratios are medians of the per-process paired Rust/DUCC ratios. The full/reduced, odd/even, q1/q8 summary for all 96 matched configurations is in [policy-ab-summary.csv](data/scalar-t64-256-rowfft-2026-10-21/policy-ab-summary.csv).

| Case | Op / q | Baseline Rust | Candidate Rust | DUCC | Baseline ratio | Candidate ratio |
|---|---:|---:|---:|---:|---:|---:|
| T64 full even | A / 8 | 63.56 | 30.09 | 36.82 | 1.76 | 0.82 |
| T64 full odd | A / 8 | 90.32 | 34.08 | 47.81 | 1.87 | 0.71 |
| T64 full even | S / 8 | 42.17 | 33.61 | 39.56 | 1.05 | 0.85 |
| T64 full odd | S / 8 | 94.76 | 40.58 | 47.40 | 1.95 | 0.86 |
| T124 full even | A / 8 | 206.18 | 70.06 | 73.79 | 2.75 | 0.95 |
| T124 full odd | A / 8 | 327.97 | 90.63 | 132.86 | 2.52 | 0.68 |
| T124 full even | S / 8 | 131.87 | 74.46 | 69.50 | 1.89 | 1.08 |
| T124 full odd | S / 8 | 312.66 | 110.49 | 149.23 | 2.16 | 0.73 |
| T127 full even | A / 8 | 108.99 | 68.90 | 72.73 | 1.49 | 0.95 |
| T248 full even | A / 1 | 3190.18 | 1488.67 | 1135.90 | 2.73 | 1.30 |
| T248 full even | A / 8 | 624.59 | 239.22 | 209.79 | 2.91 | 1.14 |
| T248 full odd | A / 8 | 716.37 | 316.71 | 313.39 | 2.32 | 1.01 |
| T255 full even | A / 8 | 604.71 | 235.97 | 223.23 | 2.74 | 1.06 |
| T255 full odd | A / 8 | 716.57 | 294.36 | 339.42 | 2.08 | 0.86 |
| T256 full even | A / 8 | 251.91 | 247.33 | 229.33 | 1.08 | 1.08 |
| T248 full even | S / 8 | 278.38 | 278.40 | 219.94 | 1.27 | 1.27 |
| T256 full even | S / 8 | 294.32 | 289.35 | 237.81 | 1.24 | 1.22 |

The table-route repair also helped q1 where the table is large: T248 full even analysis fell from 3.190 ms to 1.489 ms. For a crossover control, q1 T191 has a 13.57 MiB table and recurrence was 12.6% slower; q1 T223 has a 21.53 MiB table and recurrence was 9.3% faster for even-smooth and 12.6% faster for odd-minimum sampling. The 20 MiB cutoff keeps the measured T191 table while selecting recurrence from T223. The T255 q1 full-order recurrence exception was already present.

The row-FFT effect is separate from table analysis. At T124 full q8, forcing only parallel row FFTs changed analysis from 206.18 to 120.86 µs (even) and 327.97 to 140.05 µs (odd); forcing only analysis recurrence changed it to 161.43 and 287.17 µs. The integrated route measured 70.06 and 90.63 µs. For synthesis, the recurrence route already applied, and row FFT alone changed T124 from 131.87 to 74.76 µs (even) and 312.66 to 108.07 µs (odd). A test-only phase probe independently saw T124 forward-FFT medians fall from 99.1 to 17.3 µs (even) and 273.2 to 51.4 µs (odd); inverse-stage medians fell from 58.3 to 10.4 µs and 236.6 to 46.3 µs. These phase probes are diagnostic, not the whole-transform timing source.

At T256 quarter-order q8, where the table is ineligible but the old `nphi >= 255` gate prevented parallel row FFTs, the combined policy reduced analysis from 203.67 to 99.17 µs (even) and 305.61 to 114.29 µs (odd); synthesis fell from 181.04 to 125.38 µs and 349.94 to 148.13 µs. Conversely, T64 quarter-order cases below 4096 samples stay on the serial FFT route and showed no material change. This guard avoids the 9–28% regressions seen when those small cases were forced parallel.

## Remaining gaps and discontinuities

- The T63→T64 full-order q8 analysis jump was caused by the selector changing from its T63 recurrence exception to the compact table at T64; row FFTs were serial on both baseline cases. The recurrence selector removes that table detour.
- The T126→T127 change combined the old row-FFT eligibility boundary with the compact-table analysis path. The sample-work predicate removes the FFT geometry cliff, and q8 recurrence removes the table accumulation penalty. At T127 the compact-table SIMD specialization alone did not produce a q8 gain.
- The T255→T256 full-order q8 analysis change came from the compact-table limit, with row FFTs already active on both sides. The recurrence policy makes T255 q8 analysis close to parity and removes the table cliff. T248–T255 q8 table-route gaps were likewise not caused by row-FFT eligibility for full-order grids.
- The new policy does **not** close all synthesis gaps. T248–T256 full-order q8 synthesis remains about 1.15–1.45× DUCC, and T127 odd q8 synthesis remains about 1.35×. These cases already used row FFTs and recurrence; they need a separately measured recurrence/output-stage investigation, not a dispatch change in this bounded task.
- Small q1 differences under roughly 2% are within the observed run variation and are not promoted as regressions or gains.

## Numerical and host checks

The integrated candidate completed 312 Rust/DUCC numerical comparisons with no failed configurations. Maximum relative L2 error was `1.81e-14`; maximum absolute error was `2.76e-11`. The harness also checked basis modes, roundtrips, adjoints, zero, subnormal, extreme-finite inputs, finite outputs and output reset. No tolerance was changed. Release tests passed (244 passed, 31 ignored); release no-default tests passed (201 passed, 16 ignored); formatting and strict Clippy passed.

The host was an AMD Ryzen 9 5950X with AVX2/FMA. CPU-frequency readings varied across invocations, and runqueue wait was low in the median but materially elevated in the tail; one process also had many involuntary context switches. Timed batches had 30 samples per case, and representative candidate q8 p10–p90 ranges were narrow (for example T124 full-even analysis 69.6–71.9 µs), but high-contention tails mean small changes should be treated cautiously. The changes retained here have much larger effects than those tails.

## Source and reproduction

- Baseline production: `f3a2313cfa6ec1665208daaa3dffe1dc3773003a`; its scalar production source matches sweep source `e487b85b`.
- Candidate measured binary/source: `985fa844ba2de495615b6b96af17dc41ab48315e` (RustSHT policy candidate).
- Final f64-only guard/test follow-up: `ba9d3de9`; it adds `T::supports_precomputed_scalar()` around the area policy, which is true for the measured f64 route and keeps f32 unchanged.
- Benchmark filter/route metadata fix: `b70850b4`.
- Rust: 1.99.1, LLVM 23.1.1, Pulp 0.22.3, runtime V3, four f64 lanes.
- DUCC: 0.42.0, immutable source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; fixed x86-64-v3 GCC 15.2 build with the flags above.

The candidate run was:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --production-source-revision 985fa844 \
  --lmax 64 --lmax 124 --lmax 127 --lmax 248 --lmax 255 --lmax 256 \
  --longitude-mode even_smooth --longitude-mode odd_minimum \
  --threads 1 --threads 8 \
  --results-dir reports/data/scalar-t64-256-rowfft-2026-10-21/combined-policy \
  --report-path reports/scalar-t64-256-performance-discontinuities-2026-10-21.md \
  --no-report
```

Set `DUCC_SRC` to a checkout at the immutable revision listed above.

The compact matched summary CSV contains all 96 full/quarter-order combinations. Full raw process/host records remain in the local diagnostic directory and are not part of this report commit.

## Next step

This scalar interruption is complete. Resume Plan 15 Phase A on the same branch/session: establish the native spin-1 GL, CC, then F1 baseline using its existing adapter, preserving the Plan 15 source and numerical contracts.
