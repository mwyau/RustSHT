# Plan 14: T64 checked fused scalar-analysis route

**Implementation:** `7e51ae43b5cca74fd716511493df3b20262dad13`.
**Baseline source:** `f63c83d5e5a7fe9e155685378cf946a351b06179`.
**DUCC:** 0.42.0, pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Change and motivation

The preceding q1 screen showed a consistent T64 reduced-order analysis gap:
`mmax=16` was 10.0 µs slower than DUCC at odd-minimum `nphi=33` and 16.8 µs
slower at even-smooth `nphi=36`. Calibrated Rust phase probes attributed
33.1–33.5 µs per call to recurrence and coefficient accumulation. The
existing checked fused analysis route had already removed most of the T127
reduced-order gap, so this experiment lowers only the f64 eligibility floor
from T127 to T64. It does not alter recurrence arithmetic, numerical checks,
scaled transitions, proof caching or fallback. f32 remains at T255; spin-1
and table selection are unchanged.

The test verifies actual route execution at T64, preserves the callback
route at T63, and checks f32's separate floor. The native transform checks
compare all outputs against DUCC; standard safety tests cover extreme,
subnormal, polar and adjoint behavior.

## Matched whole-transform A/B

The baseline source was clean commit `f63c83d5`; the candidate was measured
from that source plus exactly the cutoff/test diff later committed as
`7e51ae43`. Candidate raw JSON records the worktree as dirty at capture.
The baseline and candidate used Rust 1.99.1 / LLVM 23.1.1, Pulp 0.22.3,
release mode, `-C target-cpu=x86-64-v3`, and runtime Pulp dispatch. DUCC
used the pinned source with GCC 15.2, x86-64-v3, fast-math SHT compilation
and a strict benchmark driver. Host: AMD Ryzen 9 5950X. The native harness
reused plans/workspaces, validated outside timing, interleaved Rust/DUCC
process order, and ran three independent process pairs with 30 calibrated
20–100 ms batches per backend and case. The harness selected the same q1
physical core. Timings below are warmed full analysis calls; first-call and
setup fields remain in raw JSON.

| Case | Rust p10 / median / p90, before → after (µs) | DUCC median, before → after (µs) | Rust/DUCC, before → after | Excess, before → after (µs) | Rust change | Candidate relative L2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T63, m15, 64×31 | 42.646 / 43.525 / 44.109 → 43.579 / 43.686 / 44.073 | 41.859 → 41.639 | 1.040x → 1.049x | +1.666 → +2.047 | +0.37% | `3.62e-15` |
| T63, m15, 64×32 | 37.919 / 38.028 / 38.590 → 37.830 / 38.106 / 39.973 | 23.408 → 23.432 | 1.625x → 1.626x | +14.620 → +14.674 | +0.21% | `3.50e-15` |
| T64, m16, 65×33 | 40.975 / 41.262 / 41.553 → 26.256 / 26.374 / 26.532 | 31.259 → 30.863 | 1.320x → 0.855x | +10.003 → −4.489 | **−36.08%** | `2.86e-15` |
| T64, m16, 65×36 | 42.766 / 42.912 / 43.078 → 27.630 / 27.793 / 28.075 | 26.082 → 26.447 | 1.645x → 1.051x | +16.830 → +1.346 | **−35.23%** | `2.90e-15` |

The T63 controls did not materially move. Both T64 geometries improved by
about 15 µs, with nonoverlapping Rust p10–p90 ranges. The odd-minimum case
is faster than DUCC in the candidate; the even-smooth case is close but
still 1.35 µs slower. These results support the T64 floor for this checked
f64 route, not universal transform parity.

## Stage attribution and limits

The test-only phase batch probe used 30 batches on CPU 0 for T64 `mmax=16`.
At `nphi=33`, recurrence/accumulation fell from 33.087 to 19.465 µs; at
`nphi=36`, it fell from 33.510 to 19.252 µs. The corresponding full-call
reductions were 14.887 and 15.120 µs. FFT, spectrum preparation and
latitude setup stayed within roughly 0.1 µs between source versions for the
same shape, tying the improvement to recurrence/accumulation rather than
the FFT. Stage timings include the test-only phase probe and are diagnostic;
raw stage distributions are in `phase-summary.csv`.

The host was not idle: sampled q1 runqueue-wait medians were about 12–14%
and p90 values 27–32% across the four cases. The candidate and baseline
had similar CPU frequency and context-switch samples. Despite that
contention, both T64 candidate batch distributions were tightly grouped and
well separated from their baseline distributions. The benchmark's numerical
comparisons were stable across all three process pairs; maximum absolute
error was at most `1.01e-15`.

Release tests passed with parallel features (233 unit, 4 extreme-scaling,
11 f32 and 1 workspace test) and without default features (195 unit, 4
extreme-scaling, 11 f32 and 1 workspace test). Formatting and strict
all-feature/no-default Clippy passed.

## Disposition and next work

Keep the f64 fused-analysis eligibility floor at T64. The source change
removes about 14–15 µs of measured T64 recurrence time without changing
T63, f32, spin-1, table policy or safety behavior. Continue Phase A on the
remaining low-degree q1 excesses, beginning with T32 full-order analysis;
then re-rank q8 whole-transform excess from current-source measurements.

Raw JSON/CSV and stage summaries are in
[`reports/data/plan14-t64-fused-analysis-cutoff-2026-10-21/`](data/plan14-t64-fused-analysis-cutoff-2026-10-21/).
The candidate/ baseline paired comparison is `comparison.csv`.
