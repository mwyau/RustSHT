# RustSHT performance checkpoint

**Branch:** `opt/sht-performance-parity`. **Updated:** 2026-10-20. **Checked
production baseline:** Pulp post-group scan. **A/B source:** `4fcb1136`.
**Benchmark default:** production flags at `e2c2dc15`. For the
active scope and acceptance criteria, see [Plan 11](plans/11-ducc-equivalent-scalar-analysis.md).

- Production f64 scalar GL analysis keeps the checked two-pair AVX2/FMA route
  and scaled fallback. A compile-time unchecked specialization is available
  for controlled evaluation; it removes the ordinary post-group range scan.
- The refreshed `m=1, lmax=720` direct-kernel comparison records 30 calibrated
  batches in five independent interleaved process pairs at 64, 128 and 256
  lanes. Unchecked Rust/DUCC ratios are 1.142×, 1.085× and 1.101×; reset-
  inclusive ratios are 1.129×, 1.074× and 1.092×. These are Legendre
  recurrence/accumulation timings, excluding setup, FFT and scheduling. See
  the [calibrated checkpoint](reports/archive/experiments/scalar-analysis-pair-grouping-2026-10-19.md).
- A new five-process comparison of the production post-group scan against the
  unchecked specialization saves 2.321, 3.923 and 7.446 µs per kernel call at
  64/128/256 lanes. At 256 lanes, the scan is 30.0% of checked Rust time and
  accounts for 83.4% of the measured Rust/DUCC excess; unchecked Rust remains
  9.4% slower than DUCC. All 2,700 calibrated batches were 33.51–82.40 ms.
- A Pulp grouped four-accumulator reduction remains behind
  `rustsht_group_analysis_reduction` for cumulative evaluation. It reduced the
  final channels with AVX2 shuffles and packed adds but regressed unchecked
  kernel time by 13.8%, 7.0% and 4.9% at 64/128/256 lanes. The standard
  individual reductions remain the production path. The repeated-batch
  comparison measured DUCC at 4.128 µs (64 lanes) and 15.805 µs (256 lanes),
  with tightly grouped process medians; the old single-call timing regimes
  were not reproduced.
- The private DUCC-style AVX2 `vhaddpd`/`vperm2f128` reduction remains behind
  `rustsht_avx2_group_analysis_reduction`. It improved unchecked kernel time
  by 2.7%, 0.8% and 0.7% at 64/128/256 lanes, but made the checked route
  0.3%, 1.3% and 1.0% slower. Keep production on Pulp's original reductions;
  the AVX2 candidate applies the same summation order to checked and unchecked
  variants so their benchmark outputs remain exactly equal.
- Checked and unchecked direct outputs match exactly. Extreme finite,
  subnormal, zero/cancellation, SIMD-tail, T255/T511 DUCC, scalar adjoint and
  full T720 q1/q8 checks passed. Valid T720 GL q1 and q8 analyses had zero
  late ordinary-range declines. The checked route remains the default pending
  broader safety review.
- An in-loop SIMD state-mask candidate was correct but 21–33% slower than the
  checked post-group scan. Its executed AVX2 loop spills a YMM value to the
  stack while eight accumulators are live. It remains behind a custom cfg for
  ablation and is not selected in production.
- A scalar reduction per four-lane chunk avoids that spill but is 41–50% slower
  than the scan with Pulp reductions. An experimental AVX2 `vtestpd` mask
  conversion cut that in-loop candidate by 23.8/24.7/23.9%, but it remains
  7.7/12.0/15.3% slower than the post-group scan. Applying the helper to the
  production scan changed timings by less than 1.2%, within batch variation;
  keep it cfg-gated and leave production unchanged.
- Scalar analysis now uses a separate 256-pair tile by default, with compile-
  time 64/128 controls. Shared synthesis/spin tiles and f32 analysis remain at
  64. T720 and T300 active-block counts fell by about two thirds with unchanged
  SIMD-tail capacity and no increase in recurrence scratch.
- Pair packing now writes north/south parity spectra directly into the four
  real/imaginary SIMD arrays. It removes the intermediate complex-spectrum
  staging and copy; packed ring indices let the checked fallback reconstruct
  the unweighted opposite parity when needed. A three-process T720 full-analysis
  screen was neutral: q1 medians were 26.705 ms before and 26.851 ms after;
  q8 medians were 4.348 ms and 4.327 ms, with one candidate process outlier.
  Retain the layout change for cumulative evaluation, without claiming a speedup.
- On the same T720 GL full-order analysis, five interleaved q1 process pairs
  measured checked/unchecked medians of 26.932/23.541 ms (12.6% lower unchecked).
  q8 medians were 4.415/3.936 ms (10.8% lower), with one slow unchecked process.
  The checked T720 route recorded zero late declines at q1 and q8; the unchecked
  release suite also passed. Keep the robust checked default until the remaining
  high-order exceptional-value coverage justifies a production switch.
- The three-process full-analysis screen improved q1 by 4.0% at T720 and 5.4%
  at reduced-order T300; q8 T720 improved 4.4%. q8 T300 was too noisy to show a
  clear benefit. The candidate passed all-feature release tests at 64/128/256,
  no-default-feature tests, and all-target/all-feature Clippy.
- DUCC process medians were stable and the old single-call regimes were not
  reproduced. The checked experimental Rust route had several 3–8% process
  shifts that sampled frequency and scheduler data did not explain; the
  unchecked route remained stable. All batches are retained. See the report
  for process-level distributions and host diagnostics.

**Next:** reduce the repeated production post-group validation traffic without
carrying a lane mask through the eight-accumulator loop. The checked kernel
remains about 1.56× DUCC at 256 lanes; the AVX2 mask helper did not materially
change this. Keep the range check and scaled fallback in production.
