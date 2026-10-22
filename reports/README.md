# RustSHT performance evidence — reading guide

**Current checkpoint:** owner-directed scalar Legendre dispatch and paired odd-FFT validation/sweep, then return to [Plan 17 — f32 and high-resolution performance](../plans/17-f32-atmospheric-performance.md). Plan 16 is complete; Plans 14 and 15 are complete.
**Current factual status and priorities:** [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md).
**Recent measured results:** [Plan 16 T1023 cache/analysis-resampling closeout](plan16-spin1-t1023-cache-extension-2026-10-22.md), [Plan 16 executed-path crosswalk](plan16-spin1-executed-path-crosswalk-2026-10-22.md), [Plan 16 parity-separated synthesis A/B](plan16-spin1-parity-separated-ab-2026-10-22.md), [Plan 16 paired-coefficient preparation reuse](plan16-spin1-paired-coefficient-preparation-2026-10-22.md), [Scalar-analysis cumulative-ablation closeout](scalar-analysis-cumulative-ablation-2026-10-21.md), [Plan 15 synthesis dataflow closeout](plan15-spin1-synthesis-dataflow-closeout-2026-10-21.md), [Plan 15 CC q8 surplus-group scheduling](plan15-spin1-cc-group-scheduling-2026-10-21.md), [Plan 15 CC q8 synthesis coefficient reuse](plan15-spin1-cc-coefficient-reuse-2026-10-21.md), [Plan 15 F1 q8 synthesis coefficient reuse](plan15-spin1-f1-coefficient-reuse-2026-10-21.md), [Plan 15 F1 order-parallel analysis resampling](plan15-spin1-f1-analysis-resampling-2026-10-21.md), [Plan 15 native spin-1 GL/CC/F1 baseline](plan15-spin1-native-baseline-2026-10-21.md), [Plan 15 GL spin-1 row-parallel FFT A/B](plan15-spin1-gl-rowfft-2026-10-21.md), [Plan 15 GL spin-1 dynamic ring-group scheduling A/B](plan15-spin1-gl-dynamic-scheduling-2026-10-21.md), [Plan 15 GL q1 T1023 synthesis stage profile](plan15-spin1-gl-q1-t1023-stage-profile-2026-10-21.md), [Plan 15 CC order-parallel resampling and synthesis profile](plan15-spin1-cc-resampling-2026-10-21.md), [Scalar GL T64–T256 dispatch diagnostic](scalar-t64-256-performance-discontinuities-2026-10-21.md), [Plan 14 sparse spectrum-tail validation and current synthesis profile](plan14-sparse-spectrum-tail-2026-10-21.md), [Plan 14 reflected-geometry preflight](plan14-reflected-geometry-preflight-2026-10-21.md), [Plan 14 T16 fused analysis route](plan14-t16-analysis-cutoff-2026-10-21.md), [Plan 14 T64 fused analysis route](plan14-t64-fused-analysis-cutoff-2026-10-21.md), [Plan 14 T127 compact-table SIMD analysis](plan14-t127-table-analysis-simd-2026-10-21.md), [Plan 14 small-q1 baseline](plan14-small-q1-baseline-2026-10-21.md), [Plan 14 direct synthesis output](plan14-direct-spectrum-output-2026-10-21.md) and [Plan 11 scalar-analysis closeout](scalar-analysis-cumulative-2026-10-20.md).

- [Exhaustive scalar Legendre table/recurrence sweep (2026-10-22)](scalar-legendre-table-recurrence-sweep-2026-10-22.md): forced both f64 exact-GL scalar paths for all T0–T256 at q1/q8, analysis/synthesis and odd/even longitude sampling. The data confirm q1 analysis table wins mainly below the 20 MiB footprint boundary; q8 analysis and synthesis favor recurrence throughout the currently eligible table region. A boundary-complete coverage manifest and 30-batch repeats are linked from the report.

- [Production paired odd-FFT validation (2026-10-22)](paired-odd-longitude-fft-production-validation-2026-10-22.md): same-source paired-versus-independent scalar f64 GL analysis/synthesis passed every T0–T256 at q1/q8 with odd/even longitude controls, plus serial/no-default checks. The report links both exhaustive CSVs and records late-subnormal, nonfinite, Fourier-mode-skew and poisoned-workspace fallback coverage.

- [Current-source native scalar GL sweep through T3072 (2026-10-22)](ducc-full-transform-parity-current-2026-10-22.md): native RustSHT versus pinned DUCC 0.42.0, all T0–T256 plus 24 selected high degrees, full/reduced order, analysis/synthesis and q1/q8. All 2,520 groups completed; maximum random-output relative L2 error was `2.536e-13`. The report links absolute timings, distributions, coverage audit and the four principal [analysis full](data/ducc-full-transform-parity-current-2026-10-22/ratio-analysis-full-mmax.svg), [analysis reduced](data/ducc-full-transform-parity-current-2026-10-22/ratio-analysis-reduced-mmax.svg), [synthesis full](data/ducc-full-transform-parity-current-2026-10-22/ratio-synthesis-full-mmax.svg) and [synthesis reduced](data/ducc-full-transform-parity-current-2026-10-22/ratio-synthesis-reduced-mmax.svg) ratio figures. The report records marked q8 process-level timing variability; no observations were filtered.

- [Owner-directed cumulative scalar-analysis closeout (2026-10-21)](scalar-analysis-cumulative-ablation-2026-10-21.md): compares the original/current sources, packed reduction, scratch order, direct output, range-cache control and cumulative 64/128/256 tiles. Production keeps the staged path, range-proof cache and 256-pair tile; measured candidates that lacked whole-call gains were removed. Sanitized per-case results and variant manifests are linked from the report.

- [Plan 16 parity-separated spin-1 synthesis A/B (2026-10-22)](plan16-spin1-parity-separated-ab-2026-10-22.md): a completed, non-production Milestone B experiment. The two-channel candidate spilled YMM accumulators; the one-channel variant removed those spills but regressed q1 T720 CC synthesis by about 23% and was neutral to slower at q8. Both were removed. Paired batches, distributions and numerical/reset checks are preserved in [the data directory](data/plan16-parity-separated-t720-2026-10-22/README.md). Plan 16 proceeds to order-major dataflow.

- [Plan 16 paired-coefficient preparation reuse (2026-10-22)](plan16-spin1-paired-coefficient-preparation-2026-10-22.md): q1 preparation calls fell across 64-latitude blocks, but T720 CC whole-transform changes stayed within ±0.92% and were not repeatable. q8 critical-worker preparation was at most about 1.24 ms full order; a transform-wide cache was not justified. The temporary source path was removed; data and patch are in [the data directory](data/plan16-spin1-paired-coefficient-block-reuse-2026-10-22/README.md). Plan 16 proceeds to order-major dataflow.

Do not follow forward-looking instructions embedded in historical reports.

## Current and accepted evidence

- [Plan 15 synthesis dataflow closeout (2026-10-21)](plan15-spin1-synthesis-dataflow-closeout-2026-10-21.md): after the accepted GL/CC/F1 changes, active-ring compaction and a degree-pair-outer SIMD candidate were checked against exact-CC T720 q1/q8 A/S workloads. The first reduced fallback routes without a whole-call gain; the second replaced repeated coefficient broadcasts with per-pair scratch traffic and regressed q1 and full-order q8 synthesis. Both candidates were removed. The remaining CC/F1 synthesis excess is in the recurrence/spectrum stage, not output FFTs.

- [Plan 15 CC order-parallel resampling and synthesis profile (2026-10-21)](plan15-spin1-cc-resampling-2026-10-21.md): CC spin-1 q8 analysis resampling by order reduced matched Rust latency 30–45% at T720 and 19–24% at T1023; T255 full-order q8 improved 36%, while its reduced-order control stayed below the work gate. CC T1023 q8 synthesis remains 6.89x DUCC and is dominated on the Rust side by the general order/recurrence stage; the corrected phase probe prepares q8 workspace capacity. The report links paired data and records the F1 next step.
- [Plan 15 F1 order-parallel analysis resampling (2026-10-21)](plan15-spin1-f1-analysis-resampling-2026-10-21.md): extending the guarded order-parallel resampler to F1 preserved its half-grid phase and reduced matched T720/T1023 q8 analysis medians 22–45% across full/reduced order. T1023 full-order resampling phase fell from 91.0 to 38.7 ms; synthesis was an unchanged control. Full raw distributions and fixture checksums are linked.
- [Plan 15 F1 synthesis dynamic scheduling (2026-10-21)](plan15-spin1-f1-synthesis-scheduling-2026-10-21.md): scheduling surplus existing ring groups reduced T720 q8 Rust synthesis latency by 25.9% for reduced order and 41.2% for full order. Remaining Rust/DUCC ratios are 7.07x and 5.70x; recurrence and spectrum accumulation dominate. A finer T1023 split regressed reduced-order latency and was rejected. Raw paired timing distributions are retained.
- [Plan 15 integrated current-source T720 screen (2026-10-21)](plan15-spin1-integrated-t720-2026-10-21.md): GL/CC/F1 exact-grid full/reduced A/S q1/q8 workloads passed all 72 numerical/reset comparisons. The remaining largest q8 gaps are synthesis recurrence/spectrum costs; the runqueue wait and distributions are included with the matched raw data.
- [Plan 15 F1 q8 synthesis coefficient reuse (2026-10-21)](plan15-spin1-f1-coefficient-reuse-2026-10-21.md): caching each converted scalar order across the 12 existing T720 F1 ring groups improved full-order q8 Rust latency by 11.4% in all three interleaved process pairs; paired Rust/DUCC moved 5.034×→4.458×. The reduced-order result is inconclusive. Outputs are bitwise equal to the pre-change Rust build and DUCC relative L2 error is at most `8.09e-13`. The next bounded check is whether the same reuse applies to CC q8 synthesis.
- [Plan 15 CC q8 synthesis coefficient reuse (2026-10-21)](plan15-spin1-cc-coefficient-reuse-2026-10-21.md): with CC scheduling held static, caching each converted order across the existing 12 exact-T720 ring groups reduced q8 synthesis latency 15.5% full and 18.1% at `mmax=180` in all paired runs. Candidate output was bitwise equal to baseline; DUCC relative L2 was at most `3.64e-13`. q1 and q8 analysis controls were neutral. The remaining 6.67×/7.98× Rust/DUCC synthesis ratios need stage attribution.
- [Plan 15 CC q8 surplus-group scheduling (2026-10-21)](plan15-spin1-cc-group-scheduling-2026-10-21.md): the follow-up used the existing bounded dynamic queue only when cached work had more ring groups than workers. It reduced T720 CC q8 synthesis 40.9% full and 19.4% at `mmax=180`; a T511 control supports the work gate. Outputs were bitwise equal, and phase timing leaves recurrence/spectrum accumulation as the dominant residual. Current-source F1 controls and sanitized timing distributions are included.
- [Plan 15 native spin-1 GL/CC/F1 baseline (2026-10-21)](plan15-spin1-native-baseline-2026-10-21.md): 170 geometry cases, 680 q1/q8 analysis/synthesis timing rows and 2,040 comparisons against pinned native DUCC, with no failed case. The report separates exact/over-resolved GL/F1 and CC resampling-active/skipped geometries and points to the sanitized per-case CSV/JSON. Phase B begins with GL spin-1 stage attribution; raw worker and host logs remain local.
- [Plan 15 GL spin-1 row-parallel FFT A/B (2026-10-21)](plan15-spin1-gl-rowfft-2026-10-21.md): the existing q8 inverse-row helper reduced the T720 two-channel output phase from 7.97 to 2.30 ms. Whole-call q8 changes were positive across three workloads, with host-contention caveats; recurrence/scheduling remains the dominant GL stage.
- [Plan 15 GL spin-1 dynamic ring-group scheduling (2026-10-21)](plan15-spin1-gl-dynamic-scheduling-2026-10-21.md): a bounded one-group queue lowered exact-source T720 q8 GL synthesis latency 19–39% across full/reduced-order odd/even controls. The remaining Rust/DUCC ratio is 3.12–6.53x; matched timings, raw distributions and phase attribution are linked in the report.
- [Plan 15 GL q1 T1023 synthesis stage profile (2026-10-21)](plan15-spin1-gl-q1-t1023-stage-profile-2026-10-21.md): current-source matched q1 whole-call latency is 261.848 vs 120.307 ms (2.18x, +141.541 ms). Temporary route counters show the recurrence/order phase dominates; the broad fallback timer is mostly successful staged recurrence, not repeated completed degree callbacks. No production change was justified by this attribution.
- [Current-source dense RustSHT/DUCC scalar GL sweep (2026-10-21)](ducc-current-full-sweep-2026-10-21.md): all `lmax=0…128` plus 28 sampled degrees through T3072, full/reduced order, analysis/synthesis, q1/q8, even-smooth coverage throughout and selected odd-minimum cases. Pinned Rust source `e487b85b`, DUCC 0.42.0 `42ce8ab5`; 1,504 complete measurement groups, 752 numerical validation configurations and no failed cases. The broad result is mixed by degree/order/thread count: large-degree full-order ratios cluster near parity, while some T64–T256 full-order q8 analysis cases are substantially slower. Raw samples, coverage audit and figures are linked from the report. This is a current-source baseline, not a Plan 14 optimization A/B.
- [Scalar GL T64–T256 dispatch diagnostic (2026-10-21)](scalar-t64-256-performance-discontinuities-2026-10-21.md): isolates the q8 row-FFT geometry gate and compact-table analysis route. The scalar GL q8 candidate reduced T124 full-order analysis from 2.52–2.75x to 0.68–0.95x DUCC, and T248/T255 from 2.08–2.91x to 0.86–1.14x. Small reduced grids below 4096 samples retain serial FFTs; the remaining T248–T256 full-order synthesis excess is 1.15–1.45x and was not changed.
- [Plan 14 scalar GL closeout (2026-10-21)](plan14-closeout-2026-10-21.md): the full sweep remeasured the T720 reduced odd q8 synthesis residual at +0.228 ms, matching the prior stage profile's +0.231 ms within variation. That residual splits between recurrence/spectrum (+0.112 ms) and inverse/writeback (+0.121 ms); the 1.62 µs sparse-tail initialization is not material. The checked real-C2R alternative was slower, so no further production change is supported. The q1 reduced odd T720 excess remains documented and un-attributed.
- [T2047/T2048 cutoff and previous T720 A/B findings](scalar-analysis-cumulative-2026-10-20.md):
  q1 nearly at DUCC parity at T2047/2048 latitudes with 256-pair
  tiles. The extra latitude-count prefix gate was removed after matched
  T2047/T2048/T3072 validation; the 3.1–3.3x discontinuity is closed.
  Fixed SIMD scratch remains opt-in; packed reduction, indexed traversal
  and direct coefficient accumulation are retired.
- [Plan 14 direct q8 synthesis output](plan14-direct-spectrum-output-2026-10-21.md):
  a new order-owned ring-major output path removes the intermediate
  order-major spectrum and parallel scatter. This is distinct from the
  retired analysis direct-coefficient-accumulation experiment.
- [Plan 14 reflected-geometry preflight](plan14-reflected-geometry-preflight-2026-10-21.md):
  hoisting the unchanged GL reflection/polar-order validation out of each
  T720 q8 synthesis order job reduced matched Rust median latency by 4.1%;
  generated-code inlining also changed, so the full gain is not assigned to
  the scan alone.
- [Plan 14 sparse spectrum-tail validation and current synthesis profile](plan14-sparse-spectrum-tail-2026-10-21.md):
  the current T720 synthesis matrix confirms the reduced odd q8 gap is
  +0.210 ms; matched stage timers rank recurrence/spectrum and odd inverse
  rows at roughly +0.112/+0.121 ms. No new recurrence or FFT implementation
  is justified; Plan 14 is in closeout.
- [Plan 14 T127 table analysis SIMD](plan14-t127-table-analysis-simd-2026-10-21.md):
  a V3 two-complex-lane table accumulator improved matched q1 whole-call
  latency by 5.5%; q8 was inconclusive and did not show a gain.
- [Plan 14 T64 fused analysis route](plan14-t64-fused-analysis-cutoff-2026-10-21.md):
  lowering the checked f64 fused-route floor from T127 to T64 improved
  matched q1 T64 m16 analysis by 35–36%; the T63 controls were stable.
- [T64 Legendre table/recurrence dispatch decision](scalar-t64-recurrence-transition-2026-10-20.md):
  retained synthesis-only recurrence cutoff; analysis and spin-1
  keep their own policies. This is a **completed** dispatch study,
  not an instruction to repeat it.
- [Raw benchmark datasets and reproduction evidence](data/):
  remain in their original paths. Dataset source revisions and
  compiler/ISA flags vary. Preserve exact attribution.

## Historical evidence — archived, not live directions

- [2026-10-20 full cumulative scalar-analysis experiments](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md):
  complete rejected/opt-in candidate history before consolidation.
- [DUCC 0.42.0 GCC/Clang post-upgrade baseline](archive/experiments/ducc-upgrade-baseline-2026-10-20.md):
  compiler/source contract and kernel evidence; high-contention
  differences cannot be treated as clean current conclusions.
- [Older full-transform 214-configuration sweep](archive/experiments/ducc-full-transform-parity-2026-10-20.md):
  Rust 1.99.0 and its original pinned DUCC source; T3072 historical
  ratios are not current-source benchmarks.
- [2026-10-19 consolidated review of 80 older reports](archive/performance-findings-2026-10-19-consolidated.md):
  source catalog and rejected prototypes, not a new task queue.
- [Prior Plan 11 and status snapshots](archive/README.md),
  [earlier dated experiments](archive/experiments/) and
  [past reviews/plans](archive/README.md).

**Method:** Do not combine different code revisions, compiler flags,
ISA dispatch, timing boundaries, truncation/order/longitude work,
q1/q8 thread placement or cold/warm states into a trend.
Matched whole-transform and reproducible numerical checks govern
production decisions. **Keep 256 reflected-latitude-pair tiles as
the current default; do not repeat the retired 64/128 comparison.**

**Deferred work only:** [cache-informed Legendre choice](../plans/12-cache-informed-legendre-dispatch.md)
and [fused batches](../plans/13-fused-batched-sht.md).
