# RustSHT performance checkpoint — current findings

**Updated:** 2026-10-21 UTC. **Branch:** `opt/sht-performance-parity`.
**Active objective:** [Plan 14 — small serial and large parallel SHT parity](plans/14-small-serial-parallel-performance.md).
This is a **measured checkpoint, not a standalone instruction queue**.
The previous long status, including intermediate experiments and historical
next-step text, is preserved
[here](reports/archive/performance-status-2026-10-21-pre-refresh.md).
For reproducible raw benchmark tables see [reports/README](reports/README.md).

**Plan transition (2026-10-21):** Plan 11's reduced-`mmax` frontend study
is closed at `a37f74d68631ab5fd373a4097640895e9f0c0c23`. The input scan,
row FFT, spectrum pruning, order preparation and recurrence stage evidence
is recorded in the cumulative report and CSV. Plan 14 is now active on the
same branch/session. Its first clean-source q8 synthesis comparison is
published. The small-q1 baseline, T127 reduced-analysis profile and T720 q8
synthesis and analysis stage profiles are now recorded. The odd-length
complex inverse arithmetic is the largest identified q8 synthesis substage;
the current reprofile found no faster supported replacement. Plans 12/13
remain deferred.

**Plan 14 q1 rebaseline (2026-10-21):** the 64-case T4–T127 warmed q1
screen and T127 reduced-order phase profile are in
[the small-q1 report](reports/plan14-small-q1-baseline-2026-10-21.md).
The initial same-active-ring attribution found that T127 `mmax=31` was still
using the per-degree callback path: the fused symbol in the original assembly
inspection was compiled, but not eligible at T127 under the 255 cutoff. The
corrected executed-path comparison and production dispatch are in the
[T127 report](reports/plan14-t127-fused-route-2026-10-21.md). At the same
geometry, fused Rust now measures 100.249 µs versus 103.999 µs DUCC (0.964x),
with `8.77e-15` relative L2 error. The former callback route was 188.940 µs.
This removes that T127 reduced-order whole-call excess; the isolated fused
kernel remains 2.12x DUCC. Full all-feature and no-default-feature release
tests passed. The next serial lead is fixed-cost attribution in T1–T64.

**Plan 14 T127 table analysis (2026-10-21):** commit `0484a876` adds a
Pulp V3 two-complex-latitude table accumulator. On T127 full `mmax`, q1
whole-call latency fell 5.5% (398.496 to 376.600 µs); the candidate remains
1.281x DUCC (+82.6 µs), with `8.52e-15` relative L2 error. q8 showed no
measurable change and was noisier under scheduler interference. The table
selector remains unchanged because forced recurrence was slightly slower
in the route control. See the
[focused report](reports/plan14-t127-table-analysis-simd-2026-10-21.md) and
[raw data](reports/data/plan14-t127-table-analysis-simd-2026-10-21/).

**Plan 14 T64 reduced-order analysis (2026-10-21):** commit `7e51ae43`
lowers only the f64 checked fused-analysis route floor from T127 to T64.
T64 `mmax=16` q1 GL analysis improved 35–36% at `nphi=33/36`; T63 controls
moved less than 0.4%. Stage probes attribute about 14 µs of the gain to
recurrence/accumulation, with numerical relative L2 below `2.91e-15`. f32,
spin-1, table policy and safety fallback are unchanged. See the
[T64 report](reports/plan14-t64-fused-analysis-cutoff-2026-10-21.md) and
[raw data](reports/data/plan14-t64-fused-analysis-cutoff-2026-10-21/).

**Plan 14 progress (2026-10-21):** `fe0590ac` removes a separate
order-major-to-ring-major spectrum scatter from parallel scalar GL
synthesis. On clean parent/candidate builds, q8 T720 cases improved 16–35%;
T2047 synthesis (`ntheta=2049`, `nphi=4096`) improved from 75.24 ms
(1.307x DUCC) to 54.89 ms (0.950x), averaged over two candidate runs.
Maximum relative L2 error was `1.84e-13`. The output buffer is cleared
before jobs; any declined route clears partial output before fallback.
The measured change removes an intermediate spectrum pass and a parallel
scatter region; their individual shares are not isolated. The full
distributions and comparison data are in the
[direct-output checkpoint](reports/plan14-direct-spectrum-output-2026-10-21.md).

The T720 `mmax=180` q8 stage probe identifies about 0.13 ms excess in the
ordered recurrence and about 0.16 ms in the combined inverse-FFT/map-write
stage for odd-minimum `nphi=361`; the even-smooth `nphi=378` inverse stage is
shorter in Rust than DUCC's combined `leg2map`. These are diagnostic phase
samples, not calibrated Rust stage distributions. The odd case uses Rust's
full complex FFT path while DUCC uses a real FFT, making odd-length inverse
work the next source-grounded comparison. A scattered polar-zero initializer
was correct but 18% slower and removed. A row-contiguous clear reduction was
inconclusive under 12% scheduler wait and was also removed; production keeps
the original full spectrum clear. Details and limits are recorded in
[Plan 14](plans/14-small-serial-parallel-performance.md).

**Odd inverse component profile (2026-10-21):** three independent pinned q8
captures each used 30 batches × 16 T720 synthesis calls. Median inverse-stage
latencies were 0.423, 0.385 and 0.370 ms, with process spread recorded in
Plan 14. Test-only timers attributed about 0.300 ms on the slowest worker to
RustFFT's full-complex N=361 inverse, versus about 0.034 ms for Hermitian
packing and 0.008 ms for real output extraction. Profiling timers added
11–23 µs in the first three paired controls; a later short control reversed
the sign under wider timing spread, so no fixed timer correction is applied.
A one-thread screening run of the existing
`rustsht-fft` real inverse was 3.861 µs per transform versus 3.089 µs for the
RustFFT odd-complex adapter; it is not a production q8 A/B and was not
promoted. Production FFT code remains unchanged.

**T720 q8 analysis profile (2026-10-21):** three interleaved process pairs
collected 30 calibrated batches per backend for full `mmax=720` and reduced
`mmax=180`, both at `nphi=1442`. Full-order Rust/DUCC was 3.009/2.854 ms
(1.054x, +0.155 ms); reduced-order was 1.687/1.972 ms (0.856x, -0.284 ms).
Relative L2 errors were `4.66e-14` and `4.93e-14`. Rust's full-order forward
FFT plus geometry/order preparation was about 1.215 ms, close to DUCC
`map2leg` at 1.221 ms; Rust recurrence/initialization was about 1.831 ms
versus DUCC `leg2alm_internal` at 1.589 ms. For reduced order, Rust's
front-end group was about 0.886 ms versus DUCC 1.176 ms, while the recurrence
group was about 0.801 ms versus 0.757 ms. These test-timer stages have
different boundaries; the recurrence timer includes scheduling and worker
wait. DUCC stage hooks counted only timed calls after calibration (90 full,
130 reduced). No T720 q8 source change is justified. The machine-readable
data and method are in [the Plan 14 profile](plans/14-small-serial-parallel-performance.md)
and [summary CSV](reports/data/plan14-q8-analysis-t720-2026-10-21/summary.csv).

**Plan 14 current T720 findings (2026-10-21):** the q8 reduced odd synthesis
rebaseline is 1.682/1.252 ms Rust/DUCC (+0.430 ms), versus +0.141 ms for the
even-smooth control. The odd inverse stage remains a lead, but the tested
standalone real inverse was slower, so production FFT is unchanged. A measured
analysis candidate moves f64 paired-route coefficient initialization from one
serial full-output fill into order-owned worker slices. At T720 full-order q8,
Rust fell 2.578→2.388 ms (−7.4%; excess +0.450→+0.260 ms); reduced
`mmax=180` fell 1.236→1.195 ms (−3.3%, directional). q1 changed by less than
0.3% under elevated runqueue wait. Candidate output was bitwise identical to
baseline; relative L2 versus DUCC was at most `4.96e-14`. The compact-table
path and f32 initialization are unchanged. See the
[analysis initialization data](reports/data/plan14-analysis-order-local-init-2026-10-21/)
and [odd/even synthesis rebaseline](reports/data/plan14-q8-synthesis-odd-rebaseline-2026-10-21/).

**Plan 14 direct scalar-analysis packing (e51da649):** f64 GL recurrence now
loads ring-major FFT values while packing reflected pairs, avoiding the
separate T720 full-order `721 x 721` complex order-spectrum materialization.
The matched repeat screen improved T720 full-order q1/q8 by 3.9%/11.3% and
reduced-order q1/q8 by 2.3%/5.6%; candidate output was bitwise equal to the
previous route. The q8 benefit is directional because candidate scheduler
wait was lower than baseline in the repeat process pairs; the initial q8
screen also had a wider interval. Candidate/DUCC relative L2 was at most
`4.96e-14`. See the [Plan 14 source A/B data](reports/data/plan14-direct-analysis-ring-pack-2026-10-21/).

The matched T2047 q8 confirmation used `ntheta=2049`, `nphi=4096`, full and
quarter `mmax`, three interleaved process runs and 30 calibrated batches per
backend/case. At full order Rust fell `39.177→34.094 ms` (−13.0%, −5.082 ms)
and changed from `0.958x` to `0.830x` DUCC. At `mmax=511` it fell
`16.995→16.067 ms` (−5.3%, −0.929 ms), from `1.035x` to `0.977x` DUCC.
The candidate was bitwise identical to baseline; relative L2 error versus
DUCC was at most `1.27e-13`. Scheduler wait was low apart from one reduced
case pair; process medians consistently favored the candidate. The direct
route is retained. The T720 full-order q1 recheck and stage attribution below
now identify the recurrence kernel as the next measured lead. Full
distributions and method are in the
[direct-input A/B dataset](reports/data/plan14-direct-analysis-ring-pack-2026-10-21/).

**Plan 14 T720 q1 kernel recheck (2026-10-21):** the direct-input path reduced
full-order q1 Rust latency from `16.090` to `15.459 ms` (candidate/baseline
`0.964`); DUCC measured `14.676 ms`. Rust recurrence plus accumulation was
`12.435 ms` versus DUCC `leg2alm_internal` at `11.278 ms`, while Rust FFT
was `3.082 ms` versus DUCC `map2leg` at `3.615 ms`. These profile boundaries
differ, and q1 scheduler wait was elevated; use these as stage leads rather
than subtractable counters. The corrected direct `map2alm_kernel` benchmark
measures the warmed Rust/DUCC ratio at `1.171/1.107/1.083` for 64/128/256
lanes, with `1.324 us` excess at 256 lanes. Both executed loops use four
f64 AVX2 lanes and two-degree recurrence grouping; Rust has separate slice
bases where DUCC uses fixed offsets from one structure. The next test is a
single-base workspace dataflow candidate, judged by matched latency and
executed assembly. See [Plan 14](plans/14-small-serial-parallel-performance.md)
and its [kernel summary](reports/data/plan14-warmed-analysis-kernel-2026-10-21/summary.csv).


## Highest-confidence current finding — prefix discontinuity removed

The controlled change at `0bc5f153` removes only RustSHT's global
`latitude_count * 2^-60 <= 8*EPSILON` gate. The fixed DUCC threshold and
all finite-value, exponent/range and safe-fallback checks remain. Matched
f64 GL STANDARD analysis uses `mmax=2047`, `nphi=4096`, tile 256, Rust
1.99.1/Pulp V3 and DUCC 0.42.0 `42ce8ab5`, GCC 15.2 V3 with
`-O3 -ffast-math -ffp-contract=fast` for the SHT kernel.

| Geometry | q | Rust before → after | DUCC after | Rust/DUCC before → after |
| --- | ---: | ---: | ---: | ---: |
| T2047, 2048 latitudes | 1 | 251.5 → 254.6 ms | 246.5 ms | 1.011× → 1.033× |
| T2047, 2048 latitudes | 8 | 62.2 → 58.6 ms | 42.3 ms | 1.358× → 1.384× |
| T2047, 2049 latitudes | 1 | 828.4 → 254.6 ms | 245.7 ms | 3.321× → 1.036× |
| T2047, 2049 latitudes | 8 | 143.7 → 57.3 ms | 42.1 ms | 3.129× → 1.361× |
| T2048, 2049 latitudes | 1 | 832.4 → 255.4 ms | 248.3 ms | 3.329× → 1.029× |
| T2048, 2049 latitudes | 8 | 134.0 → 58.3 ms | 42.2 ms | 3.159× → 1.383× |

Three interleaved process rounds collected 15 long q1 calls per backend/tile
and 30 calibrated q8 batches. The pathological route is gone: the 2049-latitude
cases now use 2,434 prefix-scaled blocks and zero checked-scaled blocks.
The extra gate bounded a worst-case coherent sum of omitted terms; it was an
additional accuracy budget, not a finite/exponent-safety invariant. Numerical
and exceptional-value results and full timing distributions are in the
[follow-up report](reports/scalar-analysis-cumulative-2026-10-20.md)
and [data](reports/data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/).

T3072 full-order validation (`ntheta=3073`, `nphi=6174`, q1) also selected
the prefix route: 5,780 prefix-scaled blocks, zero checked-scaled blocks and
zero declines. Rust/DUCC analysis was 775.8/735.1 ms (1.056×); synthesis was
1121.4/1152.6 ms (0.973×). These are one-process five-call directional
measurements, not a large-resolution timing campaign. Random output relative
L2 errors were `1.96e-13` for analysis and `2.11e-13` for synthesis; extreme,
subnormal, polar/high-order, cancellation and adjoint checks passed.

**Current q8 result:** commit `dfed3ff7` parallelizes independent order-row
preparation for recurrence plans; `3b32f2de` bundles its shared read-only
state. The compact-table safety pass remains serial, and recurrence,
fallbacks, prefix threshold and 256-pair tile are unchanged. Order-prep phase
time fell from 23.5 to about 6.4 ms at T2047 and 40.5 to about 15.4 ms at
T3072. Matched whole-transform Rust q8 latency fell 25.6% at T2047 full
`mmax` (57.310 to 42.655 ms) and 14.9% at T3072 (144.337 to 122.835 ms);
T720 fell 14.4% (3.597 to 3.081 ms). Current median Rust/DUCC ratios are
1.003x, 1.042x and 1.067x, respectively, with distribution overlap at
T2047. Commit `41c7223f` then pruned unused real-FFT output bins for scalar
GL analysis without changing the FFT algorithm, finite-input checks,
workspace stride or other grid/spin paths. Against the preceding source,
reduced-order q8 latency improved 12.4% at T2047 `mmax=511` (21.399 to
18.739 ms) and 7.8% at T3072 `mmax=768` (64.306 to 59.266 ms); current
median ratios are 1.098x and 1.017x. Full-order changes stayed within the
measured distributions. The q8 Rust outputs were bitwise identical
before/after, and DUCC errors remained at most `2.08e-13` relative L2. The
pruning source is current commit `1fddce4a` (the source-equivalent pre-rebase
hash was `41c7223f`). The follow-up substage screen found no remaining
forward-FFT excess against DUCC `map2leg`; the safe input check remains fused
with full-row FFT packing, and no further frontend rewrite is justified by
the overlapping whole-call distributions. See the [candidate comparison and
frontend closeout](reports/scalar-analysis-cumulative-2026-10-20.md#plan-11-reduced-mmax-frontend-closeout-2026-10-21),
[q1/q8 summary](reports/data/scalar-analysis-cumulative-2026-10-20/truncated-output-candidate/summary.csv)
and [order-preparation results](reports/data/scalar-analysis-cumulative-2026-10-20/order-prep-q8-candidate/summary.csv).

## Tile decision — not an open recurring benchmark

**Retain 256 reflected-latitude pairs per scalar-analysis tile.**
After restoring the prefix route at 2049 latitudes, tile-256 vs tile-128 vs
tile-64:

| Geometry / q | 256 | 128 | 64 |
| --- | ---: | ---: | ---: |
| T2047 q1 | **254.6 ms** | 263.5 ms | 280.7 ms |
| T2047 q8 | **57.3 ms** | 59.2 ms | 61.9 ms |
| T2048 q1 | **255.4 ms** | 263.6 ms | 282.1 ms |
| T2048 q8 | **58.3 ms** | 58.6 ms | 60.9 ms |

Tiles 64/128 are diagnostic only. They lose to 256 after the prefix fix;
there is no reason to revert production or run more 64/128 matrices.
A tile contains up to 256 reflected north/south **latitude pairs**
(512 physical latitudes), not 256 maps, degrees or CPU threads.

A single **optional** 512-pair experiment remains deferred until a
reasoned cache/setup hypothesis exists; current fixed SIMD-array
scratch capacity is only 256, so it requires a coordinated and
validated capacity change.
See [Plan 11](plans/11-ducc-equivalent-scalar-analysis.md).

## T720 production/candidate screening — resolved dispositions

Matched 30-batch, three-process-pair Rust/DUCC full-transform A/S,
q1/q8, T720 `mmax=180/720`, odd-minimum/even-smooth
longitudes at source `b25c011d`. Maximum relative L2
error `5.20e-14`.

| Candidate | Measured result | Disposition |
| --- | --- | --- |
| Fixed inline aligned SIMD scratch | Initial q8 synthesis screen: 11.4–18.0%; current matched T720 q8 screens: 7 of 8 A/S cases improved 0.2–5.8%; q1 repeat is neutral | **Production default at `8e49f7d5`** after alignment ablation and equivalent-capacity/memory review; see current source evidence below |
| Packed complex reduction | Small isolated kernel gains, whole-analysis -2.0% to +0.4% alone, inconsistent across shapes | **Retired from production** |
| Direct coefficient accumulation | Earlier sizable T720 analysis regression; guarded variant not shown to beat production | **Retired** |
| Safe indexed SIMD traversal | 0.2–0.3% directional isolated changes within uncertainty; no full-call proof | **Retired** |
| Sectoral seed and scaled-prefix epoch/preflight reuse | Correctness passes; included in measured production, **not separately ablated** in current-source A/B | **Retained; independent win unquantified** |

The earlier cumulative build with all flags produced very large
T720 analysis regressions and is **not** the production recommendation.
Do not revive retired alternatives based solely on instruction counts,
isolated `m=1` results or a desire to assemble another candidate matrix.

[Current A/B data](reports/data/scalar-analysis-cumulative-2026-10-20/t720-ab-current/)
· [Historical full experiment narrative](reports/archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md).

## Established policy and remaining gaps

- **Precomputed table policy:** f64 scalar **synthesis** switches to
  recurrence at T64; scalar **analysis** and spin-1 use distinct
  selectors. Global T64 recurrence regressed q1 analysis at T64/T127/T128.
  [Retained dispatch study](reports/scalar-t64-recurrence-transition-2026-10-20.md).
  Multi-host cache-aware selection is **future Plan 12**, not current work.
- **T3072 current-source check:** after the prefix fix, full-order q1 GL
  measured 775.8 ms Rust vs 735.1 ms DUCC analysis (1.056×) and
  1121.4 vs 1152.6 ms synthesis (0.973×), with five whole-call samples per
  backend in one process. These timings are directional; errors, edge cases
  and adjoint checks passed. The older Rust 1.99.0 / DUCC `80388904` 214-case
  full sweep observed up to 8.39×/7.44× q1/q8 T3072 analysis and remains
  historical, not a current-source claim.
  [Historical baseline](reports/archive/experiments/ducc-full-transform-parity-2026-10-20.md).
- **Final Plan 11 current-source analysis checkpoint:** at T720 full
  `mmax`, q8 Rust/DUCC was 3.063/2.937 ms (1.043×); reduced `mmax=180`
  was 1.728/2.002 ms (0.863×). At T2047 full `mmax`, q8 was
  42.907/42.563 ms (1.008×), and reduced `mmax=511` was
  18.739/17.064 ms (1.098×). At T3072 full `mmax`, q8 was
  124.428/117.119 ms (1.062×), while reduced `mmax=768` was
  59.266/58.253 ms (1.017×). The final reduced frontend attribution
  found no material Rust forward-row excess over DUCC `map2leg`; do not
  repeat it. Plan 14 should remeasure selected cases with its interleaved
  protocol and investigate the remaining absolute excess, especially full
  T3072, separately from small-q1 fixed costs.
- **Compiler and inner kernel:** GCC/Clang DUCC results differ even
  with verified source flags; host contention limits attribution.
  The corrected warmed T720 `m=1` recurrence/accumulation kernel
  is competitive with DUCC in one GCC comparison. Do not
  optimize checked recurrence as if it were the warmed hot path;
  profile the actual residual at equivalent boundaries.
  [Historical toolchain report](reports/archive/experiments/ducc-upgrade-baseline-2026-10-20.md).

## Plan 14 — T32 checked fused analysis (2026-10-21)

Current source is `de4c6569`. The f64 scalar analysis checked fused route
now starts at T32; the f32 floor remains T255 and the safety proof, scaled
transition, late checks and fallback are unchanged. Three interleaved
q1 Rust/DUCC process pairs with 30 calibrated batches per backend showed
T31 controls stable within 0.4%, while T32 full and reduced-order cases
improved 18.1–22.1%. At T32 full-order even sampling, Rust fell from 31.011
to 25.176 µs; DUCC was 21.678/21.833 µs, reducing excess from 9.333 to
3.343 µs. The stage probe attributes 4.626 µs of the change to
recurrence/accumulation; FFT and preparation were stable. Maximum relative
L2 was `2.62e-15`. The residual low-band analysis lead is T31 full-order
even-smooth: 27.130/27.054 µs Rust versus 16.499/16.491 µs DUCC.
Runqueue wait was elevated, so further threshold work requires a phase
profile and matched T31/T32 control. Full distributions and data are in the
[T32 report](reports/plan14-t32-fused-analysis-cutoff-2026-10-21.md).

## Plan 14 — T16 checked fused analysis (2026-10-21)

Source commit `830ecc63` lowers only the f64 scalar-analysis fused-route
floor from T32 to T16. The matched q1 GL full-order screen used T15/T16/T31,
odd-minimum/even-smooth longitude sampling, three interleaved process pairs
and 30 calibrated batches per backend/case. T15 controls changed within
0.5%; T16 improved 8.8–9.4%, and T31 improved 20.3–22.3%. At T31
even-smooth, Rust moved 27.051→21.559 µs while DUCC stayed near 16.5 µs,
reducing excess from 10.531 to 5.234 µs. The T31 stage probe attributes
4.936 µs of the gain to recurrence/accumulation; FFT and preparation stayed
flat. Candidate relative L2 was at most `1.54e-15`; full and no-default
release tests, safety/adjoint checks, formatting and strict Clippy passed.

A 10-process-pair T15/T16 cold/warm follow-up measured T16 first-call
medians of 26.230 µs before and 30.898 µs after; their p10–p90 ranges
overlap, as do shifts in the T15 control. If the full 4.67 µs T16 shift is
real, the 0.775 µs warm saving repays it after about six more analyses.
Keep this caveat visible: the warmed reusable-plan path improves, but no
first-call speedup is established. Full samples, phase distributions and
build provenance are in the
[T16 report and data](reports/plan14-t16-analysis-cutoff-2026-10-21.md).

**Plan 14 inline aligned scratch (source commit `8e49f7d5`):** the former
opt-in layout is now the default. Seven 256-element recurrence/analysis
buffers are contiguous and 64-byte aligned with unchanged scratch payload;
the layout removes seven per-worker heap allocations. At T720 q8 it improved
7 of 8 full/reduced, odd/even A/S screens by 0.2–5.8%; the exception was a
0.2% analysis regression. Reduced even-smooth synthesis was repeated for 60
batches per source: Rust median fell 1.4127→1.3380 ms while DUCC stayed
1.2138/1.2110 ms; the Rust/DUCC ratio fell 1.164→1.105. A q1 repeat found
no median change, and the near-2× process outlier moved to the Vec baseline.
Rust outputs were bitwise identical between layouts; maximum relative L2
versus DUCC across the matrix was `5.20e-14`. V3 disassembly showed the same 2,435-instruction
sequence, so the gain is in scratch placement rather than recurrence codegen.
Release all-feature/no-default tests, format and strict Clippy passed. Full
per-case results and alignment ablation are in
[Plan 14](plans/14-small-serial-parallel-performance.md) and its
[summary CSV](reports/data/plan14-aligned-scratch-layout-2026-10-21/summary.csv).

**Plan 14 q8 rebaseline (source `42401f20`):** clean, interleaved T720
whole-call comparisons show the largest current selected excess at
full-order even-smooth analysis (`2.479/2.124 ms`, `+0.355 ms`) and
reduced-order odd-minimum synthesis (`1.638/1.263 ms`, `+0.375 ms`).
The full-analysis Rust phase probe measured `240 us` for order-spectrum
preparation and `82 us` to zero coefficients; Rust's 491 us forward FFT is
close to DUCC's 500–503 us `map2leg` in the two stable instrumented runs.
DUCC `leg2alm_internal` was 1.567–1.568 ms in those runs (a third slower,
retained as contended data). For odd synthesis, order-queue lock and pop
cost `6.04/6.67 us` across 181 jobs in two diagnostic calls, far below its
whole-call excess; no synthesis scheduler rewrite is indicated. A polar-
inactive spectrum skip in GL recurrence analysis saved about `17 us` in the
preparation phase but did not change whole-call latency beyond noise, so that
candidate was removed. The analysis order-queue and per-thread work profile
is summarized below.
Rebaseline distributions and candidate ablation are in
[Plan 14](plans/14-small-serial-parallel-performance.md) and
[machine-readable data](reports/data/plan14-q8-rebaseline-2026-10-21/) and
[the candidate ablation](reports/data/plan14-inactive-ring-preparation-2026-10-21/candidate-ablation.csv).

**Plan 14 T720 q8 analysis queue profile:** two test-only worker-local runs
counted 721 order jobs per call. Summed dequeue lock/wait medians were 41.7
and 39.0 us across 30 calls per run. Per-thread accumulated order time
stayed within 1.645–1.668 ms; estimated work spread was 3.0–7.5% max/min.
This is aggregate lock time, not a calibrated wall-stage subtraction, and
the probe's call timing shifted under host variability. Dynamic scheduling
appears balanced. Details and reproduction are in
[Plan 14](plans/14-small-serial-parallel-performance.md) and
[the compact queue data](reports/data/plan14-analysis-scheduler-2026-10-21/analysis-queue-profile.csv).

The T720 q8 analysis one-order queue was tested with two temporary dequeue
variants. Fixed pairs of orders gave a noisy 1.2% full-order improvement but
regressed the reduced-order control by 2.2% and serialized the two-job
`mmax=1` control by 40%. A guided variant restored the small case but changed
full/reduced q8 latency by only +0.2%/−0.2%, within variability. Neither
candidate is retained; the one-order scheduler remains production. Matched
Rust/DUCC results for q1/q8, full/reduced `mmax`, numerical error and host
observations are in the [batch ablation CSV](reports/data/plan14-analysis-scheduler-2026-10-21/batch-ablation.csv).

## Current next steps and deferred work

1. Test whether giving the warmed fused analysis kernel one workspace base
   with fixed field offsets improves executed AVX2 dataflow. The matched
   current kernel gap is 1.324 us per 256-lane block; the candidate must also
   improve T720 q1 whole-transform timing before adoption.
2. Recheck representative T720 q8 and reduced-order controls after that
   candidate; the T2047 q8 direct-input comparison is already complete.
3. The odd q8 synthesis inverse excess is quantified at +0.430 ms, but the
   tested alternative was slower. Do not revisit it without a new source-level
   mechanism or matched isolated real-FFT evidence.
4. **Future only:** [Plan 12](plans/12-cache-informed-legendre-dispatch.md)
   cache-topology queries plus optional offline pointer-chase diagnostic
   (no startup autotuning); [Plan 13](plans/13-fused-batched-sht.md)
   true basis-shared batches with measured internal blocking.

All prior measurements, full machine-readable CSV/JSON, rejected
experiments, historical plans and source versions are intentionally
**retained in the archive or reports/data**. Historical sections and
old forward-looking language do not override active Plan 14.
