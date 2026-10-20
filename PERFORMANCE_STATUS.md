# RustSHT performance checkpoint

**Branch:** `opt/sht-performance-parity`. **Updated:** 2026-10-20. **Source
baseline for the five-process benchmark refresh:** `205c20b6`. For the
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
- Removing the post-group scan saves 2.28, 3.83 and 7.39 µs per call at those
  lane counts; at 256 lanes, 7.39 µs is 27.8% of the historical 26.590 µs
  measurement. Reset-inclusive timings exceed kernel-only timings by 0.03–0.21
  µs per call across the measured variants.
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
  than the scan; keep both checked experiments out of the production path.
- DUCC process medians were stable and the old single-call regimes were not
  reproduced. The checked experimental Rust route had several 3–8% process
  shifts that sampled frequency and scheduler data did not explain; the
  unchecked route remained stable. All batches are retained. See the report
  for process-level distributions and host diagnostics.

**Next:** keep the checked post-group scan; reject both in-loop variants.
Continue from the unchecked direct kernel, 9–17% slower than DUCC in these
matched cases, and reduce recurrence instruction cost without weakening
fallback behavior.
