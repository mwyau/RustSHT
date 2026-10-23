# RustSHT performance checkpoint — current findings

**Owner priority update 2026-10-23:** follow the [ACTIVE f64 scalar CC/F1 parity implementation plan](plans/f64-scalar-cc-f1-ducc-parity-implementation-2026-10-23.md): production algorithm changes first, then small matched benchmarks. Next comes remaining scalar CC/F1 analysis work, then f64 spin-1. f32 remains paused. The GL table-policy study, unfinished exhaustive CC groups and repository housekeeping are preserved backlogs, not gates.

**2026-10-23 spin-1 checked pair preparation reuse:** synthesis now prepares the order-dependent two-channel paired coefficients once across each contiguous run of multiple safe 64-latitude blocks. In the exact CC T720 full-order q8 matched run, latency fell from `15.316 ms` to `14.938 ms` (paired candidate/parent ratios `0.978`, `0.976`, `0.971` across three process pairs); DUCC measured `6.123 ms`, so a substantial gap remains. All four tested q1/q8 full/quarter synthesis rows improved in every pair. Candidate outputs matched the parent exactly; reset/repeat checks passed and maximum relative L2 versus DUCC was `3.64e-13`. Full release, no-default-features, formatting and strict Clippy checks passed. See the [checked pair reuse result](reports/spin1-checked-pair-preparation-reuse-2026-10-23.md).

**2026-10-23 spin-1 paired coefficient preparation:** synthesis now prepares both converted scalar channels in one degree-pair pass. In the focused exact CC T720 full-order q8 A/B, candidate/parent median process-pair ratios were `0.962`, `0.959` and `0.968`; the candidate median was `15.449 ms` versus `16.015 ms` for the parent. The candidate remained `2.45x` DUCC (`6.289 ms`), so the spin-1 gap remains substantial and this single case does not establish broader parity. Outputs matched the parent exactly and DUCC relative L2 was `2.29e-13`. See the [paired coefficient preparation result](reports/spin1-paired-coefficient-preparation-2026-10-23.md).

**2026-10-23 spin-1 reflected-row trial rejected:** a q8 adapter that ran each converted scalar row through the reflected recurrence on central GL/CC/F1 latitudes passed the focused T720 numerical checks, but measured no repeatable whole-transform improvement over the parent source. Candidate/parent median pair ratios were `1.004x` GL, `0.998x` CC, and `1.008x` F1; all process distributions overlapped. The adapter was removed; see the [negative result](reports/spin1-reflected-scalar-row-adapter-negative-2026-10-23.md). The next spin-1 candidate needs to share recurrence work across both converted channels before benchmarking.

**2026-10-23 f64 scalar latitude geometry cache:** f64 scalar plans now cache latitude sine/cosine and polar order limits, including the prepared CC/F1 grid. In the selected T720 full-order comparison, CC q1 synthesis moved from `20.619 ms` (`1.073x` DUCC) to `19.964 ms` (`1.043x`); F1 q8 synthesis moved from `2.836 ms` (`1.051x`) to `2.770 ms` (`1.028x`). The CC q1 ratio ranges did not overlap; most other differences are small or overlap. These separate-source runs are directional, not broad parity evidence. CC q8 and F1 q1 synthesis remain about 3.0% and 3.4% slower than DUCC at this size. See the [geometry-cache comparison](reports/scalar-f64-latitude-geometry-cache-2026-10-23.md).

**2026-10-23 ordered-synthesis geometry packing:** the reflected recurrence now reads sine/cosine directly from workspace geometry and derives ring indices in place, removing repeated per-order staging. In a focused T720 full-order A/B against the immediate geometry-cache source, the clearest results were F1 q1 (`20.030 → 19.248 ms`, ratio `1.044x → 1.011x`) and CC q8 (`2.779 → 2.714 ms`, `1.028x → 1.014x`); the GL q8 comparison also improved directionally (`2.799 → 2.724 ms`, `1.036x → 1.011x`). F1 q8 was unchanged and CC q1 remains `1.043x`. Each row had 30 qualified batches across three process pairs, but source runs were separate. The focused runner marks its broad validation sidecar incomplete; direct numerical and reset checks passed. See the [direct-input report](reports/scalar-ordered-direct-latitude-input-2026-10-23.md).

**T255 synthesis holdout:** against the same immediate geometry-cache source, the direct-input candidate measured `1.024x` CC q1 and `1.030x` F1 q1; q8 medians were `0.972x` CC and `0.964x` F1. The q1 paired ranges do not overlap. All T255 runs had runqueue-wait p90 near `20.8%`, and baseline CC q8 had some batches outside the target window, so these are directional results. The selected T255/T720 rows place CC/F1 full-order synthesis within about 3% of DUCC or faster, with T720 CC q1 the clearest remaining gap (`1.043x`).

**2026-10-23 focused F1 baseline:** after the ordered/reflected synthesis and parallel prepared-grid analysis changes, exact F1 T255/T720 full-order cases were measured at q1/q8 against pinned DUCC. F1 analysis measured `0.57–0.75x` Rust/DUCC; synthesis measured `1.022–1.148x`. The largest relative excess was T255 q1 (`1.148x`, `0.210 ms`); T720 synthesis was `1.038x` q1 and `1.022x` q8. All 8 rows had 30 qualified batches per backend across 3 independent process pairs; all numerical/reset checks were accepted, with maximum relative L2 `5.98e-14`. F1 performance is now measured for these selected workloads, not broadly established. See the [focused F1 comparison](reports/ducc-scalar-f64-f1-performance-2026-10-23.md).

**2026-10-23 serial ordered-spectrum initialization:** the q1 CC/F1 scalar synthesis route now clears only polar-excluded and unused Fourier tails; the checked recurrence overwrites every active cell. Poisoned-workspace CC/F1 tests pass at q1/q8, and the no-default-features fallback test passes. At CC T720 full order q1, Rust/DUCC moved from `20.040/19.171 ms` (`1.042x`) to `19.938/19.174 ms` (`1.041x`); paired ratio ranges were `1.039–1.080x` before and `1.036–1.048x` after. q8 was effectively unchanged (`2.800` to `2.796 ms`). Each backend collected 30 calibrated batches across three process pairs, all within the target window. The q1 difference is small relative to run spread and is directional only; this does not close the scalar parity gap. The focused random-output comparison was `6.45e-14` relative L2 with accepted reset checks; the runner marked its broader numerical sidecar incomplete, while the targeted CC/F1 analytic and general-recurrence test passed. See the [serial initialization follow-up](reports/cc-scalar-ordered-synthesis-2026-10-23.md#serial-ordered-spectrum-initialization).

**2026-10-23 ordered-synthesis scheduling:** parallel scalar synthesis now claims the existing four-order jobs with an atomic index, removing a per-transform queue allocation and a mutex lock per job. In a counterbalanced exact CC T720 full-order q8 comparison, the median of two Rust run medians moved from `2.819 ms` to `2.777 ms` (1.5%); the median Rust/DUCC ratio moved from `1.038x` to `1.016x`. Each run had 30 qualified batches per backend over three process pairs, and numerical/reset checks passed (`6.81e-14` relative L2). The exact GL T720 q8 control measured `1.004x` versus `1.018x` on the previous source; process intervals touch, showing no regression but not establishing a speedup. A held-out exact CC T1023 full-order q8 check measured `6.921/6.801 ms` Rust/DUCC (`1.018x`) over 30 qualified batches. These selected full-order q8 cases are near parity; q1, reduced order and F1 synthesis, and CC/F1 analysis are not established by this change. See the [atomic scheduling result](reports/cc-scalar-ordered-synthesis-atomic-scheduling-2026-10-23.md).

**2026-10-23 ordered-synthesis block follow-up:** production q8 scalar synthesis now schedules four contiguous orders per worker job to reduce adjacent ring-major spectrum writes across workers. At exact CC T720 full-order q8, matched Rust latency fell from `3.154 ms` to `2.784 ms`; DUCC was `2.703 ms`. Rust/DUCC moved from `1.174x` to `1.034x`, with the process-pair ratio interval `1.022–1.040x`; the remaining measured excess is `0.081 ms`. Timing quality and reset checks were accepted; relative L2 was `6.45e-14`. The focused benchmark sidecar marks full numerical status incomplete, while ordered CC/F1 reference and analytic tests pass. This single case does not establish parity at q1, other degrees or F1. See the [order-block follow-up](reports/cc-scalar-ordered-synthesis-2026-10-23.md#contiguous-order-block-follow-up).

**Latest production results:** M1's [ordered/reflected scalar synthesis change](reports/cc-scalar-ordered-synthesis-2026-10-23.md) is pushed in `4d5dcf97941d2b4561a58affc88e67ba3edaa92f`, with the q8 order-block follow-up above. M2 reuses the bounded spin-1 order-parallel resampler for f64 scalar exact analysis on CC/F1, with the same measured 65,536 order/ring work cutoff. At exact CC T720 full order, q8 analysis fell from the saved serial-path `34.557 ms` to `14.510 ms`; pinned DUCC measured `13.108 ms`. The ratio moved from `2.81x` to `1.09x`; this narrows the gap substantially but does not establish parity. The q1 full-order control stayed near its saved latency. At T720 quarter order, q8 was faster than DUCC at the tested geometry, while q1 remains slower. Focused F1 baseline timings are recorded in the [F1 comparison](reports/ducc-scalar-f64-f1-performance-2026-10-23.md). See the [M2 report](reports/cc-scalar-parallel-analysis-2026-10-23.md) and [sanitized results](reports/data/cc-parallel-resampling-m2-2026-10-23/summary.csv).

**2026-10-23 prepared-output scatter follow-up:** the resampler now clears and scatters its ring-major prepared output over disjoint worker row ranges. In a current-source exact CC T720 full-order q8 A/B, Rust fell from `7.124 ms` to `6.811 ms` (4.4%); DUCC was `10.311/10.407 ms` in the two runs. Both Rust versions beat DUCC in this case; the q8 process-pair ratio interval improved from `0.676–0.726x` to `0.629–0.688x`, with slight overlap. Numerical/reset checks passed (`6.01e-14` relative L2). This current-source row supersedes the older `1.09x` q8 observation for this single case; it does not cover q1, other degrees or F1. See the [scatter follow-up](reports/cc-scalar-parallel-analysis-2026-10-23.md#parallel-prepared-output-initialization-and-scatter-follow-up).

**2026-10-23 reduced-order analysis follow-up:** CC/F1 scalar analysis now asks the row FFT to produce only `mmax+1` modes and normalizes that prefix, avoiding writes and scaling of unused modes. At T720 quarter-order CC q1, Rust fell from the saved `26.277 ms` to `24.810 ms` (5.6%); DUCC was `18.833 ms`, so Rust remains `1.32x` slower. q8 measured `5.988/6.275 ms` Rust/DUCC with median paired ratio `0.924x`; treat this as directional because 11.7% of q8 timed batches fell outside the target window and host runqueue wait reached 15.1% p90. No overall scalar parity claim is made. See the [reduced-order follow-up](reports/cc-scalar-parallel-analysis-2026-10-23.md#reduced-order-fourier-prefix-follow-up).

**2026-10-23 direct prepared-spectrum follow-up:** after CC/F1 resampling, recurrence analysis now consumes the normalized ring-major spectra directly and skips order-major transpose/copy. At T720 quarter-order CC, Rust/DUCC was `24.273/18.671 ms` q1 (`1.304x`) and `3.905/5.253 ms` q8 (`0.743x`). The q1 gap remains; q8 is faster on this case, while the saved q8 comparison was noisy and is only directional as a before/after claim. A GL quarter-order control measured `0.911x` at q1/q8 with numerical and timing checks accepted. This does not establish overall CC/F1 parity; F1 remains unmeasured. See the [direct prepared-spectrum follow-up](reports/cc-scalar-parallel-analysis-2026-10-23.md#direct-prepared-spectrum-follow-up).

**2026-10-23 reflected-profile follow-up:** CC/F1 resampling no longer clears its full reusable FFT profile before writing the active reflected span. NaN-poisoned serial and parallel scratch tests pass. In a separate-run CC T720 quarter-order comparison, q1 was effectively unchanged (`24.523/18.609 ms`, `1.315x` DUCC); q8 was `3.826/5.219 ms` (`0.734x`), about 2% lower Rust time than the saved prior run. Process-pair distributions overlap and q8 host runqueue wait p90 was 9.5%, so report this as directional only. The largest measured analysis excess remains about `5.9 ms` at quarter-order q1. See the [reflected-profile follow-up](reports/cc-scalar-parallel-analysis-2026-10-23.md#reflected-profile-clear-follow-up).

**2026-10-23 symmetric-cutoff preparation (`64f4e745`):** when actual CC/F1 scalar ring cutoffs are mirrored and monotone, analysis now prepares the all-active reflected suffix without compact-index or latitude scratch writes. Two independent CC T720 quarter-order q1 phase profiles were `23.855/23.806 ms` overall versus the saved pre-change `24.101 ms`; recurrence phase medians were `15.767/15.747 ms` versus `16.048 ms`. This is a small directional phase-profile gain, not an updated full-transform Rust/DUCC comparison. The last matched q1 row remains `1.315x`; the gap is open. See the [pair-preparation follow-up](reports/cc-scalar-parallel-analysis-2026-10-23.md#symmetric-cutoff-pair-preparation-follow-up).

**2026-10-23 SIMD-tail screen (`f0ce2aae`):** the last short reflected-latitude block now pads only the fused f64 analysis route to the runtime Pulp lane width, using finite equatorial dummy geometry and zero spectra. CC/F1/GL DUCC reference, q1/q8, compaction, range-cache, and no-default tests pass. A focused T720 CC q1 run measured `57.552/57.000 ms` Rust/DUCC full order (`1.008x`) and `24.357/18.554 ms` at quarter order (`1.317x`, `5.803 ms` excess), with 30 timed batches across three process pairs and accepted numerical/reset checks. The quarter-order result is close to the saved pre-change `24.523/18.609 ms`; those process-pair sets were separate, so no speedup is attributed to padding. Full scalar CC/F1 parity remains open. See the [SIMD-tail result](reports/cc-scalar-parallel-analysis-2026-10-23.md#runtime-simd-tail-padding).

**2026-10-23 prepared-spectrum prefix follow-up:** exact CC/F1 analysis now uses the existing safeguarded f64 DUCC `2^-60` prefix route on prepared spectra, retaining all range checks and the checked scaled fallback. The T191 all-grid DUCC-reference/adjoint test confirms exact CC and F1 enter the prefix route. At T720 quarter-order CC q1, the current matched candidate measured `13.513/18.518 ms` Rust/DUCC (`0.732x`, `-5.005 ms` excess) over 30 batches and three process pairs; relative L2 was `6.47e-14`. The saved pre-change candidate was `24.357/18.554 ms`, but source runs were separate, so the 44.5% lower Rust time is directional. This closes the gap on that one case only; full-order/q8 CC and F1 parity remain open. See the [prepared-spectrum prefix result](reports/cc-scalar-parallel-analysis-2026-10-23.md#safeguarded-prefix-recurrence-for-prepared-ccf1-analysis).

**Remaining measured gaps:** the latest T720 and T255 full-order synthesis comparisons cover CC/F1 at q1/q8, with a GL check at T720. T720 ratios are `1.011–1.043x`; directional T255 medians are `1.024–1.030x` at q1 and below DUCC at q8. The clearest remaining excess is T720 CC q1 (`0.893 ms`). Selected full-order analysis cases are faster than DUCC. These measurements do not cover reduced-order or high-degree behavior and do not establish broad scalar CC/F1 parity. Keep the complete historical CC sweep and GL crossover work nonblocking.

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
