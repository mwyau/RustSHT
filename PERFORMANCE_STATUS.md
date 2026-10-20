# RustSHT performance checkpoint

**Branch:** `opt/sht-performance-parity`. **Updated:** 2026-10-20. For the
active scope and acceptance criteria, see [Plan 11](plans/11-ducc-equivalent-scalar-analysis.md).

- Production f64 scalar GL analysis keeps the checked two-pair AVX2/FMA route
  and scaled fallback. A compile-time unchecked specialization is available
  for controlled evaluation; it removes the ordinary post-group range scan.
- The calibrated `m=1, lmax=720` direct-kernel comparison now measures 64, 128
  and 256 lanes with 30 batches per process, three independent interleaved
  Rust/DUCC invocations and 20–100 ms batches. Checked Rust / DUCC ratios are
  1.71×, 1.56× and 1.56×; unchecked ratios are 1.16×, 1.09× and 1.10×.
  These are Legendre recurrence/accumulation timings, excluding setup, FFT and
  scheduling. See the [calibrated checkpoint](reports/archive/experiments/scalar-analysis-pair-grouping-2026-10-19.md).
- Removing the post-group scan saves 2.26, 3.84 and 7.41 µs per call at those
  lane counts; at 256 lanes, 7.41 µs is 27.9% of the historical 26.590 µs
  measurement. Reset-inclusive timings exceed kernel-only timings by 0.06–0.42
  µs per call across the measured variants.
- Checked and unchecked direct outputs match exactly. Extreme finite,
  subnormal, zero/cancellation, SIMD-tail, T255/T511 DUCC, scalar adjoint and
  full T720 q1/q8 checks passed. Valid T720 GL q1 and q8 analyses had zero
  late ordinary-range declines. The checked route remains the default pending
  broader safety review.
- DUCC process medians were stable. One 64-lane Rust process showed elevated
  sampled run-queue wait without a latency shift; one 256-lane Rust process was
  about 5% slower than the other two, with no identified cause. All batches
  remain in the summaries. CPU-frequency and scheduling counters were
  available. The prior inconsistent single-call DUCC regimes were not
  reproduced; their exact historical cause is unknown.

**Next:** determine whether a cheaper preflight can replace repeated group
validation without changing valid or exceptional outputs, then use the
unchecked specialization in cumulative comparisons. After scan removal, the
remaining ordinary-kernel gap is about 9–16% across these lane counts.
