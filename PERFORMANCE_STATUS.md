# RustSHT performance checkpoint — current findings

**Owner priority update 2026-10-23:** follow the [ACTIVE f64 scalar CC/F1 parity implementation plan](plans/f64-scalar-cc-f1-ducc-parity-implementation-2026-10-23.md): production algorithm changes first, then small matched benchmarks. Next comes remaining scalar CC/F1 analysis work, then f64 spin-1. f32 remains paused. The GL table-policy study, unfinished exhaustive CC groups and repository housekeeping are preserved backlogs, not gates.

**Latest production results:** M1's [ordered/reflected scalar synthesis change](reports/cc-scalar-ordered-synthesis-2026-10-23.md) is pushed in `4d5dcf97941d2b4561a58affc88e67ba3edaa92f`. M2 now reuses the bounded spin-1 order-parallel resampler for f64 scalar exact analysis on CC/F1, with the same measured 65,536 order/ring work cutoff. At exact CC T720 full order, q8 analysis fell from the saved serial-path `34.557 ms` to `14.510 ms`; pinned DUCC measured `13.108 ms`. The ratio moved from `2.81x` to `1.09x`; this narrows the gap substantially but does not establish parity. The q1 full-order control stayed near its saved latency. At T720 quarter order, q8 was faster than DUCC at the tested geometry, while q1 remains slower. F1 performance is still unmeasured. See the [M2 report](reports/cc-scalar-parallel-analysis-2026-10-23.md) and [sanitized results](reports/data/cc-parallel-resampling-m2-2026-10-23/summary.csv).

**2026-10-23 reduced-order analysis follow-up:** CC/F1 scalar analysis now asks the row FFT to produce only `mmax+1` modes and normalizes that prefix, avoiding writes and scaling of unused modes. At T720 quarter-order CC q1, Rust fell from the saved `26.277 ms` to `24.810 ms` (5.6%); DUCC was `18.833 ms`, so Rust remains `1.32x` slower. q8 measured `5.988/6.275 ms` Rust/DUCC with median paired ratio `0.924x`; treat this as directional because 11.7% of q8 timed batches fell outside the target window and host runqueue wait reached 15.1% p90. No overall scalar parity claim is made. See the [reduced-order follow-up](reports/cc-scalar-parallel-analysis-2026-10-23.md#reduced-order-fourier-prefix-follow-up).

**Remaining measured gaps:** M1's T720 full q8 CC synthesis remains `1.32x` DUCC. M2's full q8 CC analysis is `1.09x`; quarter-order q1 analysis improved from `1.39x` to `1.32x` but remains slower. F1 parity and the full scalar CC/F1 gap remain open. Keep the complete historical CC sweep and GL crossover work nonblocking.

**Pre-M1 f64 scalar CC synthesis baseline:** full-order T720 q1 Rust/DUCC `90.315/20.435 ms` (4.42×); T720 q8 `47.916/2.951 ms` (16.24×); T1023 q8 `136.614/8.156 ms` (16.75×); T2048 q8 `1414.172/65.689 ms` (21.53×). The same T720 full-order GL q8 synthesis was `3.397/3.070 ms`. M1 later extends the ordered/reflected scalar engine from GL to CC/F1.

**2026-10-23 measured task-grain follow-up:** a CC scalar q8 task-grain change lowered Rust latency 12–43% in the focused T255/T511/T720 full/quarter A/B against the saved baseline; candidate Rust/DUCC ratios remained 3.60–10.36×. This is preserved as historical evidence, not a parity claim. See the [task-grain report](reports/cc-scalar-synthesis-grain-2026-10-23.md) and [summary](reports/data/cc-scalar-synthesis-grain-2026-10-23/summary.csv).

**Benchmark backlog (do not delete):** independent audit historically counted 2,456 matching saved / 2,528 required CC groups, 72 missing and 16 extra; the final high shard was lost. Later benchmark audit fixes do not by themselves prove measured closeout. Hold the 20 MiB GL policy provisionally and preserve raw shards. New source work should use small original/candidate/pinned-DUCC matched A/Bs and GL regression controls. Do not misrepresent old ratios as measured after new optimization.

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

Plan 16 Milestone E is now complete. The q8 spin-1 synthesis coefficient
cache gate extends to T1023 and 34 MiB. On exact T1023 GL/CC, this activates the
existing order-major queue and cuts warmed q8 synthesis by 37–67% across full
and quarter order; F1 remains on its previous path. q1 and analysis controls
did not move materially. The direct T1023 CC/F1 analysis profiles measured
Rust's resampling/scatter at 16.2/16.4 ms versus 17.1/18.0 ms for DUCC's
`resample_to_prepared_CC`; Rust recurrence/accumulation remains about 52 ms.
See the [Milestone E closeout](reports/plan16-spin1-t1023-cache-extension-2026-10-22.md)
and its sanitized [results](reports/data/plan16-spin1-t1023-cache-extension-2026-10-22/results.csv).

## Other measured context

### Owner-directed scalar table/recurrence dispatch sweep (2026-10-22)

The forced-route screen covers all 257 degrees with verified table and
recurrence execution, q1/q8, scalar f64 GL analysis/synthesis, and odd/even
longitude shapes. Earlier focused 30-batch repeats were initially reported as supporting the q1
20 MiB table-footprint threshold, but later independent dispatch analysis establishes that this is not an optimized crossover; they show recurrence faster for q8 analysis
and for q1/q8 synthesis across the production-eligible region. The only exact
grid scalar production route at T63 that contradicts this evidence is q8 table
synthesis. The q1 T255 and q8 T63 analysis exceptions are redundant with the
existing byte/thread rules. The table minimum and maximum are retained because
the low-T warm results do not establish a safe eager-build/reuse policy.
Commit `6ab40dec` applies that rule, removing the q8 T63 exact-grid synthesis
table exception; forced whole-transform batches measured the table at 1.56–1.58×
the recurrence route. Boundary tests cover odd/even `nphi` and verify spin-1's
existing selector. See the [forced-route report](reports/scalar-legendre-table-recurrence-sweep-2026-10-22.md)
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

**Paired odd-length FFT validation (2026-10-22):** the guarded scalar f64
paired-row route is already production code. Exhaustive same-source comparison
against its independent-row route passed T0–T256 for q1/q8, analysis/synthesis
and odd/even longitude shapes (2,056 rows; maximum relative L2 `1.91e-15`).
Serial/no-default coverage also passed all 257 degrees. Exceptional, subnormal,
poisoned-workspace and late partial-write fallback cases passed. The 8:1 row
scale guard remains an empirical filter, not a per-mode error bound. In the
focused same-source boundary screen, paired/independent geometric mean latency
was 0.963/0.943× for full-order analysis q1/q8 and 0.905/0.945× for synthesis;
reduced-order small-`nphi` analysis had some q1 regressions. See the
[validation report](reports/paired-odd-longitude-fft-production-validation-2026-10-22.md).

### Owner-directed full scalar GL baseline (2026-10-22)

The frozen production source at `819d1f44` was compared with DUCC 0.42.0
(`42ce8ab5`) across all T0–T256 and 24 selected degrees through T3072. The
matrix contains 630 geometries and 2,520 complete operation/thread groups
(analysis/synthesis, q1/q8), with full/reduced order and even-smooth sampling
throughout; odd-minimum cases are retained at representative sizes. Maximum
random-output relative L2 was `2.536e-13`; basis, roundtrip, adjoint, exceptional
input and output-reset checks passed. Performance varies by order and thread
count: q1 full-order analysis in T129–256 had 1.37× geometric-mean latency,
while T1537–3072 full-order analysis and synthesis were near or below parity
in the sampled cases. Some q8 high-degree process pairs were strongly
interfered with (for example, T2047 reduced-order analysis), so distributions
and telemetry are retained and no samples were removed. The four ratio plots,
two absolute-latency plots, table/recurrence crossover and paired-FFT A/B now
have both SVG and PNG figures; plotting changes only declutter x-axis labels
and preserve every point. The figures, CSV/JSON results and coverage audit are in the
[full-sweep report](reports/ducc-full-transform-parity-current-2026-10-22.md).
The scalar sweep itself changed no scalar production code. Its source was
`819d1f44`; current HEAD `32bb8b4b` adds only the spin-1 T1023 cache gate and
test in `src/spin_transform.rs` and `src/spin_transform/tests.rs`, with no
changes to scalar Legendre or FFT paths. On current HEAD, the exhaustive
paired-versus-independent correctness test again passed every T0–T256 case at
q1/q8 and in serial/no-default-features mode. A same-current-source T63 q8
odd-synthesis refresh measured 36.405 µs paired versus 38.072 µs independent
(0.956×), with Rust/DUCC ratios 0.481/0.502 and maximum relative L2 below
`1.5e-14`; this single cell is supplemental, not a new broad FFT claim.

### Plan 17 Milestone A — native f32 baseline (2026-10-22)

The current-source native f32 comparison completed 112/112 cases for scalar
spin-0 and vector spin-1, analysis/synthesis, q1/q8, GL T127–T1023 plus T255 CC
and F1 controls. It pins DUCC 0.42.0 and RustSHT `dcb38366`; every output was
finite and output-reset/repeat checks passed. Full-order GL scalar warmed gaps
range from 8.04× to 178.66×, with the T720 phase probe assigning roughly
98.8–99.9% of wall time to recurrence and accumulation. DUCC's public f32 path
uses double intermediates in the scalar recurrence, while Rust Pulp V3 uses
eight f32 lanes. The largest observed cross-library relative L2 was `2.981e-4`
(T1023 scalar synthesis q8); this is not an internal correctness failure.
The next Plan 17 task is a source/assembly-grounded recurrence analysis, not
FFT optimization. See the [baseline report](reports/plan17-f32-native-baseline-2026-10-22.md)
and its [CSV](reports/data/plan17-f32-native-baseline-2026-10-22/results/results.csv),
[JSON distributions](reports/data/plan17-f32-native-baseline-2026-10-22/results/results.json),
and [coverage audit](reports/data/plan17-f32-native-baseline-2026-10-22/results/coverage-audit.json).

### Plan 17 Milestone B — inline spin-f32 recurrence step (2026-10-22)

Inlining `spin_f32::recurrence_step` removed an outlined Pulp wrapper from the
executed V3 degree loop. On matched GL spin-1 T255/T720 A/S q1/q8 cases,
warmed latency fell 26.6–44.8% versus the prior Rust build; outputs were
bitwise identical. The candidate remains 19.9–82.4× DUCC, so the next task is
to attribute lane normalization, recurrence, coefficient accumulation and
scheduling costs before another kernel change. See the [report](reports/plan17-f32-spin-recurrence-inline-2026-10-22.md)
and [timing distributions](reports/data/plan17-f32-spin-recurrence-inline-2026-10-22/results.csv).
The follow-up inlined `normalize_pair_lanes`; it removed its two per-degree
calls but expanded the V3 caller about 28% and regressed seven of eight
matched cases by 0.2–5.9%. Production keeps the out-of-line normalizer. The
[negative A/B](reports/plan17-f32-spin-normalizer-inline-2026-10-22.md)
records all timings. A test-only T720 counter profile found 15.9–20.3% of
normalizer calls have at least one scaled lane, but only 0.39–0.55% of lanes
scale (one exponent step each). Counting overhead invalidates the profile's
latencies; the counters were frequency-only. The accepted Pulp SIMD range
precheck skips the scalar lane walk when every recurrence pair is already
inside the normalizer's exact no-op range; unsafe lanes still use the original
checked scaler. On matched GL spin-1 f32 T255/T720, q1/q8 A/S full transforms
it reduced warmed Rust latency in all eight cases by 5.6–17.4%, with
bitwise-identical Rust outputs. The candidate remains 17.1–73.8× DUCC. See the
[SIMD precheck report](reports/plan17-f32-spin-normalizer-simd-precheck-2026-10-22.md)
and its raw [JSON](reports/data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/results.json),
[summary](reports/data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/summary.csv)
and [timing samples](reports/data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/samples.csv).
The T1023 q8 check is complete. The SIMD precheck retained a paired
candidate/baseline improvement (0.822 analysis, 0.889 synthesis) with
bitwise-identical Rust outputs. Rust remains 39.18×/47.95× DUCC; the synthesis
baseline had a contended process with two >2 s calls, so its A/B improvement
is directional. The next task is to isolate recurrence arithmetic from the
visitor/accumulation and scheduling costs before changing the kernel. Results
are in the [precheck report](reports/plan17-f32-spin-normalizer-simd-precheck-2026-10-22.md)
and [T1023 timing data](reports/data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/t1023-q8/results.json).

A source-grounded synthesis group-granularity A/B found that coarse static
ring-group chunks leave q8 work-stealing opportunity when there are more
groups than workers. The f32 spin-1 synthesis route now selects one-group
tasks only under that geometry/work condition; q1 and cases already at one
group per worker retain their previous policy. On T720 q8, current-source
paired process-median ratios were 0.582–0.740 (median 0.691) versus the prior
build, with bitwise-identical Rust results. T1023 q8 ratios were 0.890–0.929;
the T511 q8 control was effectively unchanged because its old grain was
already one. The remaining Rust/DUCC ratios are roughly 47–55 at T720/T1023.
Test-only component sampling continues to rank visitor/spectral accumulation
well above recurrence arithmetic, but is too noisy for timing attribution.
See the [group-granularity report](reports/plan17-f32-spin1-group-granularity-2026-10-22.md)
and its [sanitized distributions](reports/data/plan17-f32-spin1-group-granularity-2026-10-22/).

The first Plan 17 spin-f32 normal-range skip candidate passed f32, adjoint,
extreme and reset checks and improved selected T720 workloads by 2.6–8.8%.
It also regressed T255/T511 by 2.3–4.6%, including T511 with the scan disabled
below the work cutoff, so the candidate was removed from production. The
matched evidence is in the [range-skip experiment report](reports/plan17-f32-spin-normal-range-2026-10-22.md).

**Constraints:** preserve strict Rust floating-point semantics, checked
exceptional-value fallbacks, portable Pulp SIMD, the stable API, and the
existing guarded paired odd-row scope. Keep DUCC 0.42.0 pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` for current comparisons.
