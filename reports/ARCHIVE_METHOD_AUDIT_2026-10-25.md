# Historical method audit coverage — 2026-10-25

**Status:** repository-archive index pass completed; follow-up checks still required for raw per-process data and arbitrary commit history. **Branch scanned:** `dev` on October 25, 2026. [Backlog of recoverable candidates](PERFORMANCE_CANDIDATES.md) · [accepted current results](../PERFORMANCE_STATUS.md) · [negative/retired source mappings](NEGATIVE_EXPERIMENTS.md).

## What was actually inspected

The audit **individually fetched the full Markdown contents** of **84 archived experiment reports**, **74 retired report/milestone records** and **12 archived technical/future plan files** (**170 files total**) and programmatically inspected their headings, method summaries, trial dispositions, near-neutral/mixed timing passages and numerical validation notes. Particularly promising/misclassified entries were inspected more deeply, including exact results tables. The [2026-10-19 consolidation](archive/performance-findings-2026-10-19-consolidated.md) covering 80 historical reports and the [2026-10-20/21 cumulative experiments](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md) / [ablation](archive/retired-active-docs-2026-10-24/reports/scalar-analysis-cumulative-ablation-2026-10-21.md) were cross-checked.

The >500 KB [milestone history](archive/plans/05-validation-and-milestones-history.md) (2,653 lines, 220 headings), three [ASTRA SHT](archive/reviews/ASTRA_SHT_REVIEW.md), [FFT](archive/reviews/ASTRA_FFT_REVIEW.md), [integrated](archive/reviews/ASTRA_INTEGRATED_REVIEW.md) review documents, and older profiling/performance plans were fetched and indexed at heading/method level, **not manually audited line by line**.

The early [milestone-history record](archive/plans/05-validation-and-milestones-history.md) alone contains **220 indexed headings**, including Oct 12 experiments that are not separate items in the 84-file archived-experiment directory. Examples include Fourier-weight and spin-phase hoisting, scalar order-major spectra, inactive polar-block skipping, normal-range/scaled-prefix transitions, early SIMD eligibility tests, spin-factor ordering, and the later FFT radix/Rader work. The accepted paths are historical production examples; B27–B29 preserve the most relevant mixed-result alternatives from this heading audit.

**Important boundary:** This does not prove that every method in the repository's **3,931** tracked paths, every historical Git commit, all `reports/data/` raw CSV/JSON/patch files or local/untracked experiment captures has been examined. Nor does it validate old benchmarks under today's Rust compiler or accepted source. Missing raw files or external branches must be marked `source-not-recovered`, not silently reconstructed.

## Previously overlooked method families now represented

- f64 scalar cosine-square hoisting, fused degree-loop recurrence/accumulation, sequential Q/U SIMD dispatch and mixed compiler-profile variants — **B16–B19** in the backlog
- CC active-ring compaction and q1 paired coefficient preparation reuse — **B20–B21**; separate from already accepted ring-group scheduling and bounded checked preparation reuse
- scalar-only 16-ring scheduling, spin-1 coefficient direct reads, one-base dispatch and M6 isolated high-band scaled-prefix/epoch routes — **B22–B25**
- degree-pair-outer CC spin-1 synthesis as a design-negative with neutral q8 reduced control and substantial q1 regressions — **B26**
- additional **Oct 12–13** methods found only in the extensive 2,653-line milestone history: scalar polar-block skipping, lowered scalar synthesis continuation threshold, and spin-factor preparation reordering — **B27–B29**. These predate today's sources; their favorable cells coexist with material regressions and do not justify unrestricted restoration.
- f32 conditional normal-range skipping, incomplete T2047 controls, force-inline codegen, already accepted f32 SIMD prechecks and q8 task granularity — **F01–F06**, explicitly not current f64 optimization targets
- odd-length paired real FFT production route, the slower standalone N=361 C2R alternative, FFT prime/radix ideas, cache-informed Legendre choice and batched SHT — **F07–F09**, distinguishing accepted, negative, and merely proposed work
- [Plan 14 accepted cutoffs, CC/F1 resampling, GL/F1/CC dynamic scheduling and Legendre-table decisions](archive/retired-active-docs-2026-10-24/README.md) cross-checked to avoid falsely classifying production implementations as rejected.

## Reliability rules for future method archaeology

1. Inspect source and *exact* parent/candidate/revert commits when available; old report headings that say “rejected” may mean only “not deployed.” A complete runtime/correctness failure is different from a near-neutral result.
2. Extract exact workload, numerator/denominator of timing ratios, q1/q8/other q, warm/cold boundaries, CPU/contention telemetry, statistical uncertainty, bitwise/numerical behavior, and **matched whole-call Ir**, if measured. Static disassembly improvement is not equal to measured Callgrind reduction.
3. Place **validated and deployed** methods in `PERFORMANCE_STATUS.md`; **correct but noisy or conditional** in `PERFORMANCE_CANDIDATES.md`; **strong regressions or correctness failures** in `NEGATIVE_EXPERIMENTS.md` (with candidate ledger cross-links if design insight remains). A method may appear in both negative and candidate records for different variants and workloads.
4. If a candidate reduces matched Callgrind instructions with numerically identical outputs and timing is indistinguishable on every affected cell, favor lower Ir as a selection tie-breaker. If a regression appears, isolate the dispatch before universal deployment.
5. Further audit work should inspect **raw data/provenance and Git commits**, not repeat the already-indexed document catalog without a specific missing method, and should append to this register when verification scope expands.

## Files individually fetched and indexed

### Archived individual experiments (84)

- [adaptive-legendre-latest-head-recheck-2026-10-14.md](archive/experiments/adaptive-legendre-latest-head-recheck-2026-10-14.md)
- [adaptive-legendre-latest-head-results-2026-10-14.md](archive/experiments/adaptive-legendre-latest-head-results-2026-10-14.md)
- [adaptive-legendre-refresh-2026-10-14.md](archive/experiments/adaptive-legendre-refresh-2026-10-14.md)
- [adaptive-legendre-strategy-2026-10-14.md](archive/experiments/adaptive-legendre-strategy-2026-10-14.md)
- [analysis-capacity-split-2026-10-15.md](archive/experiments/analysis-capacity-split-2026-10-15.md)
- [analysis-leaf-imbalance-2026-10-15.md](archive/experiments/analysis-leaf-imbalance-2026-10-15.md)
- [analysis-phase-profile-2026-10-15.md](archive/experiments/analysis-phase-profile-2026-10-15.md)
- [analysis-sectoral-simd-2026-10-18.md](archive/experiments/analysis-sectoral-simd-2026-10-18.md)
- [atmospheric-resolution-performance-ectrans-2026-10-16.md](archive/experiments/atmospheric-resolution-performance-ectrans-2026-10-16.md)
- [batched-transform-study-2026-10-14.md](archive/experiments/batched-transform-study-2026-10-14.md)
- [current-analysis-phase-profile-2026-10-15.md](archive/experiments/current-analysis-phase-profile-2026-10-15.md)
- [current-scalar-analysis-kernel-profile-2026-10-14.md](archive/experiments/current-scalar-analysis-kernel-profile-2026-10-14.md)
- [ducc-full-transform-parity-2026-10-20.md](archive/experiments/ducc-full-transform-parity-2026-10-20.md)
- [ducc-source-build-screen-2026-10-14.md](archive/experiments/ducc-source-build-screen-2026-10-14.md)
- [ducc-upgrade-baseline-2026-10-20.md](archive/experiments/ducc-upgrade-baseline-2026-10-20.md)
- [ectrans-legendre-strategy-study-2026-10-16.md](archive/experiments/ectrans-legendre-strategy-study-2026-10-16.md)
- [f32-scalar-normalization-skip-2026-10-16.md](archive/experiments/f32-scalar-normalization-skip-2026-10-16.md)
- [f32-scalar-normalization-skip-T1279-2026-10-16.md](archive/experiments/f32-scalar-normalization-skip-T1279-2026-10-16.md)
- [f32-scalar-normalization-skip-T2047-2026-10-16.md](archive/experiments/f32-scalar-normalization-skip-T2047-2026-10-16.md)
- [f32-spin1-analysis-simd-2026-10-15.md](archive/experiments/f32-spin1-analysis-simd-2026-10-15.md)
- [f32-spin1-current-screen-2026-10-15.md](archive/experiments/f32-spin1-current-screen-2026-10-15.md)
- [f32-spin1-normalization-range-check-2026-10-15.md](archive/experiments/f32-spin1-normalization-range-check-2026-10-15.md)
- [f32-spin1-normalize-interleave-screen-2026-10-15.md](archive/experiments/f32-spin1-normalize-interleave-screen-2026-10-15.md)
- [f32-spin1-simd-overflow-fallback-2026-10-15.md](archive/experiments/f32-spin1-simd-overflow-fallback-2026-10-15.md)
- [f32-spin1-zero-lane-simd-2026-10-15.md](archive/experiments/f32-spin1-zero-lane-simd-2026-10-15.md)
- [f32-t1023-ducc-validation-2026-10-16.md](archive/experiments/f32-t1023-ducc-validation-2026-10-16.md)
- [fft-friendly-longitude-selection-2026-10-14.md](archive/experiments/fft-friendly-longitude-selection-2026-10-14.md)
- [fine-synthesis-latitude-groups-2026-10-14.md](archive/experiments/fine-synthesis-latitude-groups-2026-10-14.md)
- [fused-scalar-analysis-2026-10-15.md](archive/experiments/fused-scalar-analysis-2026-10-15.md)
- [fused-spin1-analysis-2026-10-15.md](archive/experiments/fused-spin1-analysis-2026-10-15.md)
- [fused-spin2-analysis-2026-10-15.md](archive/experiments/fused-spin2-analysis-2026-10-15.md)
- [fused-synthesis-register-hoist-2026-10-14.md](archive/experiments/fused-synthesis-register-hoist-2026-10-14.md)
- [hemisphere-paired-synthesis-2026-10-14.md](archive/experiments/hemisphere-paired-synthesis-2026-10-14.md)
- [latitude-group-size-16-2026-10-14.md](archive/experiments/latitude-group-size-16-2026-10-14.md)
- [legendre-precomputation-2026-10-13.md](archive/experiments/legendre-precomputation-2026-10-13.md)
- [legendre-reuse-estimate-2026-10-14.md](archive/experiments/legendre-reuse-estimate-2026-10-14.md)
- [m6-cpu-performance-parity.md](archive/experiments/m6-cpu-performance-parity.md)
- [matched-rust-ducc-stage-profile-2026-10-18-41b309a5.md](archive/experiments/matched-rust-ducc-stage-profile-2026-10-18-41b309a5.md)
- [matched-rust-ducc-stage-profile-2026-10-18.md](archive/experiments/matched-rust-ducc-stage-profile-2026-10-18.md)
- [opposite-cosine-hoist-2026-10-14.md](archive/experiments/opposite-cosine-hoist-2026-10-14.md)
- [order-owned-synthesis-2026-10-15.md](archive/experiments/order-owned-synthesis-2026-10-15.md)
- [overresolved-gl-legendre-table-2026-10-14.md](archive/experiments/overresolved-gl-legendre-table-2026-10-14.md)
- [paired-analysis-three-chain-2026-10-14.md](archive/experiments/paired-analysis-three-chain-2026-10-14.md)
- [paired-analysis-two-way-unroll-2026-10-14.md](archive/experiments/paired-analysis-two-way-unroll-2026-10-14.md)
- [paired-recurrence-range-check-2026-10-14.md](archive/experiments/paired-recurrence-range-check-2026-10-14.md)
- [performance-status-pre-ducc-port-2026-10-18.md](archive/experiments/performance-status-pre-ducc-port-2026-10-18.md)
- [release-profile-screen-2026-10-14.md](archive/experiments/release-profile-screen-2026-10-14.md)
- [row-fft-parallelism-2026-10-15.md](archive/experiments/row-fft-parallelism-2026-10-15.md)
- [scalar-analysis-avx2-codegen-2026-10-19.md](archive/experiments/scalar-analysis-avx2-codegen-2026-10-19.md)
- [scalar-analysis-basis-cosine-screen-2026-10-15.md](archive/experiments/scalar-analysis-basis-cosine-screen-2026-10-15.md)
- [scalar-analysis-codegen-followup-2026-10-19.md](archive/experiments/scalar-analysis-codegen-followup-2026-10-19.md)
- [scalar-analysis-cosine-square-hoist-screen-2026-10-15.md](archive/experiments/scalar-analysis-cosine-square-hoist-screen-2026-10-15.md)
- [scalar-analysis-cumulative-2026-10-20-snapshot.md](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md)
- [scalar-analysis-ducc-refresh-2026-10-15.md](archive/experiments/scalar-analysis-ducc-refresh-2026-10-15.md)
- [scalar-analysis-loop-fusion-screen-2026-10-15.md](archive/experiments/scalar-analysis-loop-fusion-screen-2026-10-15.md)
- [scalar-analysis-pair-grouping-2026-10-19.md](archive/experiments/scalar-analysis-pair-grouping-2026-10-19.md)
- [scalar-analysis-range-check-fusion-2026-10-15.md](archive/experiments/scalar-analysis-range-check-fusion-2026-10-15.md)
- [scalar-analysis-range-check-min-aggregation-screen-2026-10-15.md](archive/experiments/scalar-analysis-range-check-min-aggregation-screen-2026-10-15.md)
- [scalar-analysis-range-mask-reduction-2026-10-15.md](archive/experiments/scalar-analysis-range-mask-reduction-2026-10-15.md)
- [scalar-analysis-stage-profile-2026-10-18.md](archive/experiments/scalar-analysis-stage-profile-2026-10-18.md)
- [scalar-ducc-kernel-screen-2026-10-18.md](archive/experiments/scalar-ducc-kernel-screen-2026-10-18.md)
- [scalar-paired-zip-iteration-2026-10-14.md](archive/experiments/scalar-paired-zip-iteration-2026-10-14.md)
- [scaled-lane-normalization-2026-10-14.md](archive/experiments/scaled-lane-normalization-2026-10-14.md)
- [simd-transition-range-check-screen-2026-10-18.md](archive/experiments/simd-transition-range-check-screen-2026-10-18.md)
- [spin-analysis-ring-scale-hoist-2026-10-14.md](archive/experiments/spin-analysis-ring-scale-hoist-2026-10-14.md)
- [spin1-analysis-channel-dispatch-2026-10-14.md](archive/experiments/spin1-analysis-channel-dispatch-2026-10-14.md)
- [spin1-analysis-ducc-refresh-2026-10-15.md](archive/experiments/spin1-analysis-ducc-refresh-2026-10-15.md)
- [spin1-analysis-leaf-overpartition-2026-10-14.md](archive/experiments/spin1-analysis-leaf-overpartition-2026-10-14.md)
- [spin1-analysis-range-check-fusion-2026-10-15.md](archive/experiments/spin1-analysis-range-check-fusion-2026-10-15.md)
- [spin1-analysis-ring-scale-screen-2026-10-18.md](archive/experiments/spin1-analysis-ring-scale-screen-2026-10-18.md)
- [spin1-analysis-thread-scaling-2026-10-14.md](archive/experiments/spin1-analysis-thread-scaling-2026-10-14.md)
- [spin1-direct-vs-scalar-conversion-2026-10-14.md](archive/experiments/spin1-direct-vs-scalar-conversion-2026-10-14.md)
- [spin1-dual-channel-analysis-2026-10-14.md](archive/experiments/spin1-dual-channel-analysis-2026-10-14.md)
- [spin1-hemisphere-pair-synthesis-2026-10-14.md](archive/experiments/spin1-hemisphere-pair-synthesis-2026-10-14.md)
- [spin1-paired-coefficient-preparation-2026-10-14.md](archive/experiments/spin1-paired-coefficient-preparation-2026-10-14.md)
- [spin1-paired-zip-iteration-2026-10-14.md](archive/experiments/spin1-paired-zip-iteration-2026-10-14.md)
- [spin1-parallel-coefficient-cache-2026-10-14.md](archive/experiments/spin1-parallel-coefficient-cache-2026-10-14.md)
- [spin1-shared-coefficient-cache-2026-10-14.md](archive/experiments/spin1-shared-coefficient-cache-2026-10-14.md)
- [spin1-staged-synthesis-factorization-2026-10-14.md](archive/experiments/spin1-staged-synthesis-factorization-2026-10-14.md)
- [t1023-analysis-ducc-refresh-2026-10-15.md](archive/experiments/t1023-analysis-ducc-refresh-2026-10-15.md)
- [t1023-scalar-ducc-followup-2026-10-15.md](archive/experiments/t1023-scalar-ducc-followup-2026-10-15.md)
- [t159-overresolved-analysis-2026-10-14.md](archive/experiments/t159-overresolved-analysis-2026-10-14.md)
- [workspace-capacity-control-2026-10-15.md](archive/experiments/workspace-capacity-control-2026-10-15.md)
- [workspace-capacity-paired-2026-10-15.md](archive/experiments/workspace-capacity-paired-2026-10-15.md)

### Retired milestone and integration reports (74)

- [archive/README.md](archive/retired-active-docs-2026-10-24/reports/archive/README.md)
- [cc-scalar-ordered-synthesis-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/cc-scalar-ordered-synthesis-2026-10-23.md)
- [cc-scalar-ordered-synthesis-atomic-scheduling-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/cc-scalar-ordered-synthesis-atomic-scheduling-2026-10-23.md)
- [cc-scalar-parallel-analysis-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/cc-scalar-parallel-analysis-2026-10-23.md)
- [cc-scalar-synthesis-grain-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/cc-scalar-synthesis-grain-2026-10-23.md)
- [ducc-current-full-sweep-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/ducc-current-full-sweep-2026-10-21.md)
- [ducc-full-transform-parity-2026-10-20.md](archive/retired-active-docs-2026-10-24/reports/ducc-full-transform-parity-2026-10-20.md)
- [ducc-full-transform-parity-current-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/ducc-full-transform-parity-current-2026-10-22.md)
- [ducc-scalar-f64-f1-performance-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/ducc-scalar-f64-f1-performance-2026-10-23.md)
- [ducc-upgrade-baseline-2026-10-20.md](archive/retired-active-docs-2026-10-24/reports/ducc-upgrade-baseline-2026-10-20.md)
- [f64-ducc-independent-review-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/f64-ducc-independent-review-2026-10-22.md)
- [handoff-f64-independent-review-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/handoff-f64-independent-review-2026-10-22.md)
- [paired-odd-longitude-fft-experiment-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/paired-odd-longitude-fft-experiment-2026-10-21.md)
- [paired-odd-longitude-fft-production-validation-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/paired-odd-longitude-fft-production-validation-2026-10-22.md)
- [PERFORMANCE_FINDINGS.md](archive/retired-active-docs-2026-10-24/reports/PERFORMANCE_FINDINGS.md)
- [plan14-closeout-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-closeout-2026-10-21.md)
- [plan14-direct-spectrum-output-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-direct-spectrum-output-2026-10-21.md)
- [plan14-ducc-inverse-rows-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-ducc-inverse-rows-2026-10-21.md)
- [plan14-q8-current-rerank-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-q8-current-rerank-2026-10-21.md)
- [plan14-reflected-geometry-preflight-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-reflected-geometry-preflight-2026-10-21.md)
- [plan14-small-q1-baseline-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-small-q1-baseline-2026-10-21.md)
- [plan14-small-q1-fixed-cost-attribution-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-small-q1-fixed-cost-attribution-2026-10-21.md)
- [plan14-sparse-spectrum-tail-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-sparse-spectrum-tail-2026-10-21.md)
- [plan14-t127-fused-route-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-t127-fused-route-2026-10-21.md)
- [plan14-t127-reflected-spectrum-reuse-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-t127-reflected-spectrum-reuse-2026-10-21.md)
- [plan14-t127-row-fft-parallelism-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-t127-row-fft-parallelism-2026-10-21.md)
- [plan14-t127-table-analysis-simd-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-t127-table-analysis-simd-2026-10-21.md)
- [plan14-t16-analysis-cutoff-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-t16-analysis-cutoff-2026-10-21.md)
- [plan14-t32-fused-analysis-cutoff-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-t32-fused-analysis-cutoff-2026-10-21.md)
- [plan14-t64-fused-analysis-cutoff-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-t64-fused-analysis-cutoff-2026-10-21.md)
- [plan14-workspace-base-control-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan14-workspace-base-control-2026-10-21.md)
- [plan15-spin1-cc-coefficient-reuse-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-cc-coefficient-reuse-2026-10-21.md)
- [plan15-spin1-cc-group-scheduling-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-cc-group-scheduling-2026-10-21.md)
- [plan15-spin1-cc-resampling-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-cc-resampling-2026-10-21.md)
- [plan15-spin1-f1-analysis-resampling-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-f1-analysis-resampling-2026-10-21.md)
- [plan15-spin1-f1-coefficient-reuse-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-f1-coefficient-reuse-2026-10-21.md)
- [plan15-spin1-f1-synthesis-scheduling-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-f1-synthesis-scheduling-2026-10-21.md)
- [plan15-spin1-gl-dynamic-scheduling-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-gl-dynamic-scheduling-2026-10-21.md)
- [plan15-spin1-gl-q1-t1023-stage-profile-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-gl-q1-t1023-stage-profile-2026-10-21.md)
- [plan15-spin1-gl-rowfft-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-gl-rowfft-2026-10-21.md)
- [plan15-spin1-integrated-t720-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-integrated-t720-2026-10-21.md)
- [plan15-spin1-native-baseline-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-native-baseline-2026-10-21.md)
- [plan15-spin1-synthesis-dataflow-closeout-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan15-spin1-synthesis-dataflow-closeout-2026-10-21.md)
- [plan16-spin1-executed-path-crosswalk-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan16-spin1-executed-path-crosswalk-2026-10-22.md)
- [plan16-spin1-order-major-dataflow-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/plan16-spin1-order-major-dataflow-2026-10-21.md)
- [plan16-spin1-paired-coefficient-preparation-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan16-spin1-paired-coefficient-preparation-2026-10-22.md)
- [plan16-spin1-parity-separated-ab-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan16-spin1-parity-separated-ab-2026-10-22.md)
- [plan16-spin1-t1023-cache-extension-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan16-spin1-t1023-cache-extension-2026-10-22.md)
- [plan17-f32-native-baseline-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan17-f32-native-baseline-2026-10-22.md)
- [plan17-f32-spin-normal-range-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan17-f32-spin-normal-range-2026-10-22.md)
- [plan17-f32-spin-normalizer-inline-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan17-f32-spin-normalizer-inline-2026-10-22.md)
- [plan17-f32-spin-normalizer-simd-precheck-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan17-f32-spin-normalizer-simd-precheck-2026-10-22.md)
- [plan17-f32-spin-recurrence-inline-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan17-f32-spin-recurrence-inline-2026-10-22.md)
- [plan17-f32-spin1-group-granularity-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/plan17-f32-spin1-group-granularity-2026-10-22.md)
- [README.md](archive/retired-active-docs-2026-10-24/reports/README.md)
- [repo-state-review-summary-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/repo-state-review-summary-2026-10-22.md)
- [scalar-analysis-cumulative-2026-10-20.md](archive/retired-active-docs-2026-10-24/reports/scalar-analysis-cumulative-2026-10-20.md)
- [scalar-analysis-cumulative-ablation-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/scalar-analysis-cumulative-ablation-2026-10-21.md)
- [scalar-f64-latitude-geometry-cache-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/scalar-f64-latitude-geometry-cache-2026-10-23.md)
- [scalar-legendre-table-recurrence-sweep-2026-10-22.md](archive/retired-active-docs-2026-10-24/reports/scalar-legendre-table-recurrence-sweep-2026-10-22.md)
- [scalar-ordered-direct-latitude-input-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/scalar-ordered-direct-latitude-input-2026-10-23.md)
- [scalar-t64-256-performance-discontinuities-2026-10-21.md](archive/retired-active-docs-2026-10-24/reports/scalar-t64-256-performance-discontinuities-2026-10-21.md)
- [scalar-t64-recurrence-transition-2026-10-20.md](archive/retired-active-docs-2026-10-24/reports/scalar-t64-recurrence-transition-2026-10-20.md)
- [spin1-checked-pair-preparation-reuse-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-checked-pair-preparation-reuse-2026-10-23.md)
- [spin1-hemisphere-pair-degree-tile-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-hemisphere-pair-degree-tile-2026-10-23.md)
- [spin1-order-major-reflected-pair-negative-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-order-major-reflected-pair-negative-2026-10-23.md)
- [spin1-paired-coefficient-preparation-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-paired-coefficient-preparation-2026-10-23.md)
- [spin1-reflected-latitude-normalization-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-reflected-latitude-normalization-2026-10-23.md)
- [spin1-reflected-order-major-local-fallback-negative-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-reflected-order-major-local-fallback-negative-2026-10-23.md)
- [spin1-reflected-pair-category-lane-negative-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-reflected-pair-category-lane-negative-2026-10-23.md)
- [spin1-reflected-pair-register-pressure-negative-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-reflected-pair-register-pressure-negative-2026-10-23.md)
- [spin1-reflected-pair-stream-register-negative-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-reflected-pair-stream-register-negative-2026-10-23.md)
- [spin1-reflected-parity-stream-negative-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-reflected-parity-stream-negative-2026-10-23.md)
- [spin1-reflected-scalar-row-adapter-negative-2026-10-23.md](archive/retired-active-docs-2026-10-24/reports/spin1-reflected-scalar-row-adapter-negative-2026-10-23.md)

### Archived technical and future plans (12)

- [11-ducc-equivalent-scalar-analysis.md](archive/context-2026-10-23/plans/11-ducc-equivalent-scalar-analysis.md)
- [12-cache-informed-legendre-dispatch.md](archive/context-2026-10-23/plans/12-cache-informed-legendre-dispatch.md)
- [13-fused-batched-sht.md](archive/context-2026-10-23/plans/13-fused-batched-sht.md)
- [14-small-serial-parallel-performance.md](archive/context-2026-10-23/plans/14-small-serial-parallel-performance.md)
- [15-spin1-vector-gl-cc-f1-performance.md](archive/context-2026-10-23/plans/15-spin1-vector-gl-cc-f1-performance.md)
- [16-ducc-spin1-recurrence-parity.md](archive/context-2026-10-23/plans/16-ducc-spin1-recurrence-parity.md)
- [17-f32-atmospheric-performance.md](archive/context-2026-10-23/plans/17-f32-atmospheric-performance.md)
- [f64-cc-gl-kernel-port-and-spin1-parity-2026-10-22.md](archive/context-2026-10-23/plans/f64-cc-gl-kernel-port-and-spin1-parity-2026-10-22.md)
- [f64-scalar-cc-f1-ducc-parity-implementation-2026-10-23.md](archive/context-2026-10-23/plans/f64-scalar-cc-f1-ducc-parity-implementation-2026-10-23.md)
- [f64-spin1-ducc-reflected-fused-parity-2026-10-23.md](archive/context-2026-10-23/plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md)
- [README.md](archive/context-2026-10-23/plans/README.md)
- [scalar-f64-cc-full-band-and-gl-table-crossover-2026-10-22.md](archive/context-2026-10-23/plans/scalar-f64-cc-full-band-and-gl-table-crossover-2026-10-22.md)

## Additional indexed large sources

- [2026-10-19 80-report consolidation](archive/performance-findings-2026-10-19-consolidated.md)
- [Long validation and milestone history](archive/plans/05-validation-and-milestones-history.md)
- [Historical performance status](archive/performance-status-2026-10-21-pre-refresh.md)
- [Earlier RustSHT/DUCC gap plan](archive/plans/07-ducc-performance-gap.md)
- [Matched stage-profiling plan](archive/plans/09-matched-ducc-rust-profiling.md)
- [ASTRA SHT independent review](archive/reviews/ASTRA_SHT_REVIEW.md)
- [ASTRA FFT independent review](archive/reviews/ASTRA_FFT_REVIEW.md)
- [ASTRA integrated review](archive/reviews/ASTRA_INTEGRATED_REVIEW.md)
- [Historical M6 parity report](archive/experiments/m6-cpu-performance-parity.md) (also listed in the experiment files)
- [Historical cumulative scalar-analysis ablation](archive/retired-active-docs-2026-10-24/reports/scalar-analysis-cumulative-ablation-2026-10-21.md) (also listed in retired reports)

## Unverified surfaces

- Full per-sample CSV/JSON statistics and build logs under `reports/data/`, especially old experiments with only local untracked captures.
- Every Git commit diff/revert and all historical branches: only specific candidate commits and source-provenance sections were cross-checked here.
- All code from external DUCC/SHTns/FFTW branches or notebooks: outside this repository scan.
- Actual retention of old source variants in current `dev`: old report “production” applies to its contemporaneous revision, not necessarily today.
- Reproducibility of any archived timing on the accepted October 25 source and modern compiler.

**Conclusion:** We identified and corrected material gaps in the earlier 19-row backlog, but a categorical claim of “no other old methods exist” would be unsupported. This register makes the document-index coverage explicit and provides a bounded route to completing commit-level and raw-data archaeology.
