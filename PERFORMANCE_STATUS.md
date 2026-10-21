# RustSHT performance checkpoint — current findings

**Updated:** 2026-10-21 UTC. **Branch:** `opt/sht-performance-parity`.
**Plan 14:** COMPLETE at closeout commit
`f2d9ce47fdf04946ef5cbe478a30b0fc38956f8e`. **Active objective:**
[Plan 15 — spin-1 vector GL, CC and F1 performance](plans/15-spin1-vector-gl-cc-f1-performance.md),
authorized for the same session and branch. Scalar CC/F1 optimization
outside spin-1 is not included; Plans 12/13 are deferred.
**Owner scope decision (2026-10-21): defer odd-length FFT optimization**, including
paired-real-row FFTs, `realfft` substitution and custom `rustsht-fft`
integration. Keep odd/even longitude benchmarks and numerical correctness
controls, but do not initiate an odd-FFT performance campaign. Continue
Plan 15 Phase A native spin-1 baseline (GL → CC → F1) and its measured
non-FFT vector opportunities.
This is a **measured checkpoint, not a standalone instruction queue**.
The previous long status, including intermediate experiments and historical
next-step text, is preserved
[here](reports/archive/performance-status-2026-10-21-pre-refresh.md).
For reproducible raw benchmark tables see [reports/README](reports/README.md).

**Scalar GL T64–T256 diagnostic complete (2026-10-21):** matched controls
identified separate row-FFT and compact-table costs. The T63→T64 q8 analysis
jump was a selector change (T63 recurrence to T64 compact table); both
baseline FFT routes were serial. The T126→T127 full-grid change combined the
old row-FFT boundary with the table path. At T255→T256, row FFTs were already
active and the jump came from compact-table eligibility ending at T255. The
f64 scalar GL row-FFT gate now uses `ntheta*nphi >= 4096`; the q8 exact-grid
analysis route uses recurrence, and q1 uses recurrence above a measured 20
MiB table payload. q8 T124 full-order analysis moved from 2.52–2.75x to
0.68–0.95x DUCC; T248/T255 moved from 2.08–2.91x to 0.86–1.14x. The
remaining full-order T248–T256 q8 synthesis ratio is 1.15–1.45x and needs a
separate measured stage study. The source delta from the published sweep
baseline contains no earlier scalar production change, so these were
dispatch deficits rather than newly introduced source regressions. Numerical
comparisons and release checks passed. See the
[focused report and matched summary](reports/scalar-t64-256-performance-discontinuities-2026-10-21.md).

**Plan 15 Phase A baseline complete:** 170 GL/CC/F1 geometry cases produced
680 q1/q8 A/S timing rows and 2,040 numerical comparisons with no failures.
Maximum relative L2 was `1.63e-12`. The largest GL q8 synthesis excess was
T720 over-resolved odd (`+60.7 ms`); GL q1 T1023 synthesis was `+138.9 ms`.
CC/F1 retain larger grid-specific q8 synthesis excesses that need separate
stage attribution. The host had substantial scheduling wait during F1, so
small deltas remain provisional. See the
[matched baseline report and aggregate data](reports/plan15-spin1-native-baseline-2026-10-21.md).
Plan 15 Phase B now starts with GL stage attribution. Its two-channel adapter
was smoke-validated on T4 GL/CC/F1 for A/S q1/q8 (24 comparisons, max
relative L2 `6.19e-16`, stale-output checks passed). The adapter and
reproducible smoke command are in
[`reports/data/plan15-spin1-native-baseline-2026-10-21`](reports/data/plan15-spin1-native-baseline-2026-10-21/README.md).

**Plan 15 Phase B candidate:** q8 GL spin-1 synthesis now borrows the shared
scalar-conversion rows across ring groups instead of copying them into worker
scratch for each order/group. At T720's 23 groups, the removed copies account
for about 365 MiB of coefficient payload (roughly 730 MiB of source-read and
destination-write traffic before cache effects); checked polar/scaled fallbacks
still populate their worker-local rows when the paired route declines. The
T127 cached/uncached test now reuses a poisoned workspace with changed
coefficients; its maximum absolute channel difference is `2.17e-18`. The T720
q8 adjoint identity, general-channel and large-finite fallback tests pass. A
short q8 timing screen was stopped because
runqueue wait was 9–40% with thousands of involuntary switches, so it gives no
performance conclusion. Next: repeat the matched whole-transform A/B and
stage profile under a quiet host.

**Current-source full-transform sweep (2026-10-21):** the frozen Rust source
`e487b85b` was compared with pinned DUCC 0.42.0 `42ce8ab5` across every
`lmax=0…128` and 28 sampled degrees through T3072, full/reduced `mmax`,
analysis/synthesis and q1/q8. All 1,504 measurement groups completed; 752
numerical validation configurations passed. The result is workload-dependent:
large-degree full-order ratios are mostly near parity, while some T64–T256
full-order q8 analysis cases exceed 2x. Host frequency and scheduling varied,
so individual low-degree extremes need cautious interpretation. This is a
baseline for the current source, not an optimization A/B. See the [report,
four principal figures and raw data](reports/ducc-current-full-sweep-2026-10-21.md).

**Plan 14 closeout (2026-10-21):** the fresh sweep reproduced the T720
`mmax=180`, `nphi=361` q8 synthesis excess at +0.228 ms; the prior matched
stage run measured +0.231 ms. Its two measured excess stages are recurrence
plus spectrum (+0.112 ms) and inverse/writeback (+0.121 ms). Sparse-tail
initialization is 1.62 µs, and the tested real-C2R alternative was slower.
No further production change has a supported mechanism. The q1 reduced odd
T720 analysis/synthesis excesses (+1.452/+2.080 ms) remain visible but
un-attributed. Plan 14 is complete; see the [concise closeout](reports/plan14-closeout-2026-10-21.md).
The next owner-authorized scope is Plan 15 spin-1 GL/CC/F1.

**Plan transition (2026-10-21):** Plan 11's reduced-`mmax` frontend study
is closed at `a37f74d68631ab5fd373a4097640895e9f0c0c23`. Plan 14's current
source sweep, four principal figures, numerical validation and q8 stage
closeout are complete at `f2d9ce47`. The remaining T720 reduced-odd q8
synthesis excess is +0.228 ms, split across recurrence/spectrum and inverse
rows; no tested replacement is faster. The authorized next plan, spin-1
GL → CC → F1, is now active on the same branch/session. Plans 12/13 remain
deferred.

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

**Plan 14 T127 reflected-spectrum reuse (2026-10-21):** the V3 compact-table
analysis kernel now forms north/south sums and differences once per order and
reuses them for each degree pair, using existing worker buffers. At full
`mmax`, T127 q1 improved 24.5–26.7% for odd/even longitude counts and moved
from 1.27–1.30x DUCC to 0.94–0.95x. q8 improved 8.0–8.9%, but remains
2.35–2.59x DUCC and the q8 result is directional under scheduling
interference. Relative L2 remained at most `8.53e-15`; full release and
no-default-feature tests passed. See the
[matched report](reports/plan14-t127-reflected-spectrum-reuse-2026-10-21.md)
and [comparison data](reports/data/plan14-t127-reflected-spectrum-reuse-2026-10-21/comparison.csv).
The remaining T127 q8 front-end lead is measured separately below.

**Plan 14 T127 q8 row FFT (2026-10-21):** lowering the row-parallelism
geometry floor from `nphi=257` to `255` reduced matched T127 q8 analysis and
synthesis latency by 26.7–51.5% across odd/even longitude counts. Rust/DUCC
ratios moved from 1.55–2.91x to 1.13–1.49x; the q1 path remains serial.
Phase timing shows the forward FFT fell from 79–107 µs to 19–24 µs. Relative
L2 stayed below `8.53e-15`; full release/no-default tests, Clippy and format
passed. See the [row-FFT report](reports/plan14-t127-row-fft-parallelism-2026-10-21.md)
and [comparison data](reports/data/plan14-row-fft-parallelism-2026-10-21/comparison.csv).
Next: resume the T720 q8 odd-synthesis stage investigation.

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
than subtractable counters. The first corrected direct `map2alm_kernel`
comparison used fragmented Rust `Vec` scratch and overstated the
production-layout gap. A same-buffer control now uses the current contiguous
`RecurrenceWorkspace` layout with the production slice dispatch. Its paired
Rust/DUCC ratios are `1.162/1.095/1.057` for 64/128/256 active lanes; the
256-lane excess is `0.91 us` per isolated kernel-plus-commit operation. The
one-base wrapper measured no consistent gain over slice dispatch (paired
current/one-base ratios `1.013/1.009/0.991`) and is not integrated. Both
paths execute the same four-lane AVX2/two-degree body; inspection found no
hot-loop YMM spills. The next Plan 14 task is the remaining small-q1
fixed-cost attribution across T1–T64. See the
[workspace-base report](reports/plan14-workspace-base-control-2026-10-21.md),
[machine-readable results](reports/data/plan14-workspace-base-control-2026-10-21/),
and earlier [kernel summary](reports/data/plan14-warmed-analysis-kernel-2026-10-21/summary.csv).


**Plan 14 n=361 f64 inverse improvement (`a6cfdc49`):** at T720,
`mmax=180`, the measured `nphi=361=19²` synthesis path now uses two
existing RustFFT radix-19 butterflies for the ordinary f64 inverse series;
f32, other lengths and checked inverses retain their previous planner.
Matched inverse-row medians fell 15.9% q1 and 22.3% q8. Full synthesis
improved 2.3% q1 and 2.7% q8; the q8 Rust/DUCC ratio moved 1.294→1.264 and
excess fell about 39 µs. Candidate relative L2 was `5.19e-14` end-to-end.
The full-call p10/p90 intervals overlapped in two of three q8 process pairs,
and q8 runqueue wait had occasional 5.8–6.7% p90 spikes. Treat this as a
directional, case-specific gain; the remaining q8 excess is about 0.332 ms.
See [Plan 14](plans/14-small-serial-parallel-performance.md) and the
[comparison CSV](reports/data/plan14-mixed-radix-n361-2026-10-21/comparison.csv).
**Plan 14 current-source T3072 q8 analysis recheck (`140b4bff`):** with
`lmax=mmax=3072`, `ntheta=3073`, `nphi=6174`, Rust measured 106.014 ms and
DUCC 115.653 ms (0.917x; Rust faster by 9.639 ms), with relative L2
`1.96e-13`. This confirms the current rerank: T3072 full-order analysis is
not a remaining excess. Each of three interleaved process pairs recorded
five individually timed calls; the Rust runs had 240–453 involuntary context
switches versus DUCC's 63–83, so the size of the win is directional. The
current largest selected q8 excess remains T720 reduced-order odd synthesis
at about 0.332 ms after the n=361 inverse change. See the
[recheck data](reports/data/plan14-t3072-current-q8-2026-10-21/).

Next: compare the current T720 q8 `nphi=361` synthesis recurrence and
inverse-row costs on matched boundaries, then pursue only a source-level
change supported by the measured residual. The rejected standalone C2R
engine is not a candidate without a new mechanism.

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

**Plan 14 small-q1 fixed-cost attribution (current source `f3f222b7`):**
the matched T1/T2/T4/T16/T31/T32/T63/T64 full-order q1 sweep found Rust
faster at T1–T4. The largest sampled positive excesses were T63 synthesis
(+9.982 µs), T64 synthesis (+7.703 µs), and T31 analysis (+5.208 µs).
Three process-level Rust phase captures put setup below 1.9 µs for T63/T64
synthesis and non-FFT/non-Legendre analysis phases below 0.8 µs at T31;
recurrence and FFT work dominate the Rust calls. Runqueue wait was elevated
(10.5% median, 31.6% p90), and no small-case DUCC stage hooks were enabled,
so this does not support a source edit or assign the full excess to one
stage. Phase A is closed; see the
[attribution report](reports/plan14-small-q1-fixed-cost-attribution-2026-10-21.md)
and [timing/phase data](reports/data/plan14-small-q1-current-2026-10-21/).

## Plan 14 — current-source q8 rerank and FFT screen

At Rust source `1d39a288`, a matched q8 rebaseline found T3072 full-order
analysis faster than DUCC (105.052 vs 115.697 ms, 0.908x), so its old
positive excess is no longer current. The largest selected current q8 excess
is T720 reduced odd synthesis: +0.407 ms (1.315x); T720 full and reduced
analysis excesses were +0.122 and +0.036 ms. Relative L2 errors across these
cases were at most `1.96e-13`. T720 used three interleaved process pairs and
30 calibrated batches; T3072 used 15 individual warm calls/backend because
calls exceeded 100 ms. Runqueue wait and full distributions are recorded in
the [rerank report](reports/plan14-q8-current-rerank-2026-10-21.md).

A separate q8, n=361, 721-row screen compared the production-style RustFFT
Hermitian-pack/C2C inverse with the existing RustSHT-FFT real C2R engine.
Across three processes and 30 calibrated batches per process, the real engine
was 8.8–10.0% slower. Relative L2 was `5.53e-16`; it is rejected as a
production replacement. This was not a direct DUCC FFT comparison and does
not identify the complete SHT excess. The next measurement is a matched
DUCC pocketfft versus Rust inverse-row comparison with packing, scaling and
q8 scheduling included. No production kernel changed in this checkpoint.

## Current next steps and deferred work

The q8 `nphi=361` inverse candidate and current-source T3072 q8 analysis
rebaseline are complete. Commit `93ba59fb` hoists reflected-geometry and
polar-order validation out of each T720 q8 synthesis order job. At reduced
`mmax=180`, `nphi=361`, the matched 60-batch median fell 1.6033→1.5379 ms;
DUCC stayed at 1.2607/1.2619 ms and Rust/DUCC moved 1.273×→1.224×. The
whole-call excess fell 66.7 µs, but the optimized dispatcher also changed
code generation, so that amount is not assigned solely to the removed scan.
The exact-source next step is to reprofile the remaining q8 recurrence and
inverse-row costs, then return to the unresolved small-q1 stage difference.
The odd inverse change remains limited to f64 `nphi=361`; do not revive the
rejected standalone C2R engine or generalize that route without matched
evidence.
**Future only:** [Plan 12](plans/12-cache-informed-legendre-dispatch.md)
cache-topology queries and [Plan 13](plans/13-fused-batched-sht.md)
true basis-shared batches.

## Plan 14 — matched Rust/DUCC inverse rows

At current production source `41dc652`, q1 T720 inverse rows (`721` rows,
`mmax=180`) were measured against pinned DUCC `42ce8ab5` with exact Rust
`src/fft.rs` and DUCC `pocketfft_r<double>` paths. At odd `nphi=361`, Rust
was 2.189 ms versus 0.864 ms DUCC (2.540x, +1.328 ms); at even `nphi=378`,
Rust was 0.391 ms versus 0.570 ms DUCC (0.687x). Relative L2 error was at
most `5.40e-16`.

Two q8 episodes repeated the odd gap (Rust/DUCC 0.458/0.163 ms and
0.453/0.165 ms; paired ratios 2.811x and 2.771x), but PSI rose during both
and absolute q8 latency is host-sensitive. The even q8 controls were near
parity or slightly faster for Rust in these sessions. Rust expands odd
Hermitian spectra and runs a complex inverse; DUCC uses packed real inverse.
This stage includes row packing, scheduling and output, so it does not
isolate FFT arithmetic alone. The existing RustSHT-FFT real C2R alternative
was 8.8–10.0% slower than the current Rust path and remains rejected.
See the [focused report](reports/plan14-ducc-inverse-rows-2026-10-21.md),
[main data](reports/data/plan14-ducc-fft-rows-2026-10-21/) and
[q8 repeat data](reports/data/plan14-ducc-fft-rows-q8-recheck-2026-10-21/).

## Plan 14 — T720 q8 reflected-geometry preflight (`93ba59fb`)

The ordered scalar synthesis path now validates reflected GL geometry once
before launching per-order q8 jobs, rather than repeating the scan 181 times
at T720 `mmax=180`. In matched q8 odd-synthesis captures, 60-batch Rust
median latency fell from 1.6033 ms to 1.5379 ms against 1.2607/1.2619 ms
DUCC; relative L2 stayed `5.195e-14`. The q1 control was inconclusive under
7% median runqueue wait. Assembly confirms that the candidate removes the
per-order check but also changes inlining and dispatcher frame size, so the
full 66.7 µs excess reduction is not attributed solely to validation. See
the [focused report](reports/plan14-reflected-geometry-preflight-2026-10-21.md)
and [batch data](reports/data/plan14-reflected-geometry-preflight-2026-10-21/summary.json).

The later source-`1951e833` matched stage profile and current-source full
sweep completed that reprofile: T720 reduced odd q8 synthesis retains about
0.23 ms excess, split between recurrence/spectrum and inverse/writeback. The
tested real-C2R candidate was slower. Plan 14 is complete; Plan 15 is active.
Plans 12/13 remain deferred.

All prior measurements, full machine-readable CSV/JSON, rejected
experiments, historical plans and source versions are intentionally
**retained in the archive or reports/data**. The Plan 14 stage entries below
are historical evidence and do not override the current Plan 15 objective.

## Plan 14 current status — T720 q8 synthesis tail validation (2026-10-21)

The sparse inactive-spectrum-tail initialization in `1951e833` passed
repeated poisoned-workspace reuse, odd/even latitude counts, polar limits,
subnormal and extreme-finite decline/fallback checks. A test-only observer
confirmed the subnormal q8 route had already written nonzero output before
the fallback clear. Full release suites passed with and without `parallel`.

The integrated T720 whole-transform matrix covers full/reduced `mmax`, q1/q8
and odd/even `nphi` against DUCC 0.42.0. Reduced odd q8 is 1.477/1.267 ms
(1.166x, +0.210 ms); reduced even q8 is 1.263/1.220 ms (1.035x).
Full-order odd q8 is near parity at 1.015x; full-order even q8 is 1.047x.
Maximum relative L2 error is `5.20e-14`. On a matched q8 stage fixture,
Rust recurrence/spectrum costs 1.185 ms versus DUCC `alm2leg` 1.072 ms,
and Rust inverse/writeback costs 0.286 ms versus DUCC `leg2map` 0.165 ms.
Tail initialization itself is 1.62 µs, so it does not explain the residual.
The existing real C2R alternative remains rejected; no additional source
change is supported by this profile. See the
[expanded validation and stage report](reports/plan14-sparse-spectrum-tail-2026-10-21.md),
[whole-transform matrix](reports/data/plan14-sparse-spectrum-current-2026-10-21/results.json)
and [paired stage batches](reports/data/plan14-sparse-spectrum-current-2026-10-21/stage_profile.json).

Plan 14 moves into closeout: retain the validated tail initializer, summarize
the measured residual, and do not add speculative recurrence or FFT variants.
