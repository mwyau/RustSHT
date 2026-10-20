# RustSHT performance checkpoint

**Branch:** `opt/sht-performance-parity`. **Updated:** 2026-10-20. **Benchmark
checkpoint:** `6c0189cb`. For the
active scope and acceptance criteria, see [Plan 11](plans/11-ducc-equivalent-scalar-analysis.md).

- Production f64 scalar GL analysis keeps the checked two-pair AVX2/FMA route
  and scaled fallback. A compile-time unchecked specialization is available
  for controlled evaluation; it removes the ordinary post-group range scan.
- The calibrated `m=1, lmax=720` direct-kernel comparison now measures 64, 128
  and 256 lanes with 30 batches per process, three independent interleaved
  Rust/DUCC invocations and 20–100 ms batches. Checked Rust / DUCC ratios are
  1.71×, 1.56× and 1.56×; unchecked ratios are 1.17×, 1.09× and 1.10×.
  These are Legendre recurrence/accumulation timings, excluding setup, FFT and
  scheduling. See the [calibrated checkpoint](reports/archive/experiments/scalar-analysis-pair-grouping-2026-10-19.md).
- Removing the post-group scan saves 2.28, 3.83 and 7.39 µs per call at those
  lane counts; at 256 lanes, 7.39 µs is 27.8% of the historical 26.590 µs
  measurement. Reset-inclusive timings exceed kernel-only timings by 0.03–0.21
  µs per call across the measured variants.
- Checked and unchecked direct outputs match exactly. Extreme finite,
  subnormal, zero/cancellation, SIMD-tail, T255/T511 DUCC, scalar adjoint and
  full T720 q1/q8 checks passed. Valid T720 GL q1 and q8 analyses had zero
  late ordinary-range declines. The checked route remains the default pending
  broader safety review.
- DUCC and checked Rust process medians were stable. One 256-lane unchecked
  Rust process was about 5% slower than the other two; several batches had
  isolated high maxima, but no source was identified. All batches remain in
  the summaries. CPU-frequency and scheduling counters were available, with
  low sampled run-queue wait in this run. The prior inconsistent single-call
  DUCC regimes were not reproduced; their exact historical cause is unknown.

**Next:** determine whether a cheaper preflight can replace repeated group
validation without changing valid or exceptional outputs, then use the
unchecked specialization in cumulative comparisons. After scan removal, the
remaining ordinary-kernel gap is about 9–17% across these lane counts.
