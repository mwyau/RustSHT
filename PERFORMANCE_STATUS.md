# RustSHT performance checkpoint

**Branch:** `opt/sht-performance-parity`. **Updated:** 2026-10-20. **Checked
production baseline:** Pulp post-group scan. **A/B source:** `20992eaf`.
**Calibrated harness:** `19771bb7`. **Benchmark default:** production flags at
`e2c2dc15`. For the
active scope and acceptance criteria, see [Plan 11](plans/11-ducc-equivalent-scalar-analysis.md).

- Production f64 scalar GL analysis keeps the checked two-pair AVX2/FMA route
  and scaled fallback. A compile-time unchecked specialization is available
  for controlled evaluation; it removes the ordinary post-group range scan.
- **Work-boundary correction:** the prior direct-kernel comparison stopped
  Rust timing after writing `staged_coefficients`, while DUCC's kernel also
  accumulated into `alm`. Its Rust/DUCC ratios omitted Rust's final coefficient
  commit and are superseded below. The calibrated runner now reports Rust
  recurrence/staging, commit-only, kernel-plus-commit and reset-inclusive work.
- The matched `m=1, lmax=720` runner uses identical fixture data and reset
  semantics, 20–100 ms calibrated batches, ten interleaved Rust/DUCC process
  pairs and six batches per process/backend (60 per lane/backend). It reports
  kernel-only, kernel-plus-commit and reset-inclusive phases, validates outputs
  outside timing, and prints paired process ratios and excess latency. The
  timed processes are pinned to the same physical core; frequency/scheduling
  aggregates exclude build and calibration processes. DUCC revision is
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`.
- Fresh matched run: AMD Ryzen 9 5950X, Rust 1.99.0/Pulp V3, GCC 15.2.0,
  x86-64-v3 AVX2/FMA. All 1,800 timed batches were 29.32–60.16 ms. Median
  µs/op, Rust checked/unchecked versus DUCC:

  | Lanes | Kernel + commit | Reset inclusive |
  |---:|---:|---:|
  | 64 | 7.146 / 4.836 / 4.126 | 7.218 / 4.923 / 4.231 |
  | 128 | 12.728 / 8.869 / 8.066 | 12.830 / 8.983 / 8.219 |
  | 256 | 24.532 / 17.088 / 15.859 | 24.687 / 17.240 / 16.062 |

  Paired process kernel-plus-commit checked/DUCC ratios were 1.734× (p10–p90
  1.727–1.737), 1.579× (1.572–1.583) and 1.547× (1.542–1.549) at
  64/128/256 lanes; median excess was 3.026/4.664/8.677 µs. DUCC process
  medians stayed within 4.102–4.144 µs at 64 lanes and 15.828–15.912 µs at
  256 lanes; the old 9.599/5.621 and 25.318–39.165 µs regimes did not recur.
  Timed-process frequency medians were 4.542–4.642 GHz; median/max run-queue
  wait was 0.005/0.113%, with 1.5 median involuntary switches (11 max).
  Numerical errors remained `1.970e-16` coefficient-scaled, `5.928e-14` to
  `1.628e-13` relative output L2, and at most `2.559e-11` maximum absolute.
  See the [reproducible runner](scripts/benchmark_scalar_analysis_kernel.py).
- Follow-up `rustsht_rematerialize_analysis_coefficients` candidate at
  `20992eaf` used the same calibrated 10-process-pair run and the same
  four-lane V3/DUCC AVX2 work. Across 64/128/256 lanes, kernel-plus-commit
  medians (checked candidate / unchecked candidate / DUCC, µs) were
  `7.152/4.928/4.127`, `12.725/8.926/8.030`, and
  `24.582/17.253/15.868`; paired checked ratios were `1.732/1.585/1.550`.
  The previous checked implementation measured `7.146/12.728/24.532` µs, so
  the candidate did not improve the checked kernel; its unchecked path was
  0.6–1.9% slower. Focused release tests pass with the default and candidate
  cfgs. Keep the candidate disabled by default. The 1,800 timed batches took
  29.30–60.90 ms; host frequency median was 4.617 GHz. Median run-queue wait
  was 0.009%, with one 66.45% process outlier; all samples remain included.
- T720 Rust-only full-transform A/B for that candidate found paired q1 medians
  about 0.9% lower at full `mmax` and 1.1% lower at reduced `mmax`. Three
  additional q8 pairs were mixed: full `mmax` was 1.9% slower by paired
  median, reduced `mmax` 0.4% faster. These small results do not justify
  enabling the candidate, and they do not change the direct-kernel conclusion.
- Sectoral initialization now follows DUCC's per-SIMD-vector `mypow` choice:
  vectors above the `2^-400/m` underflow limit use direct exponentiation, while
  vectors containing smaller lanes retain scaled exponent management. This
  avoids scaling safe vectors merely because another vector in the block needs
  it. The scalar seed reference test covers orders 0, 1, 250 and 720, mixed
  vectors and a SIMD tail. The all-feature and no-default-feature release test
  suites pass.
- The scaled-prefix candidate carries DUCC's below-one epoch frontier out of
  the two-pair recurrence pass, checking only vectors that rescaled instead of
  rereading the full epoch array after each group. Scale correction,
  insignificant-term handling and fallback checks are unchanged. Tests cover
  threshold crossings, sparse rescaled vectors and SIMD tails. A calibrated
  A/B/A screen found no consistent speed change from the further conditional
  epoch-mask reduction; retest that source delta with the timer-corrected
  harness before claiming a speed effect.
- The calibrated prefix runner dispatches Pulp once per batch and separates
  kernel-only from reset-inclusive timing. Kernel-only batches exclude state
  copies and subtract same-binary per-call clock overhead; reset-inclusive
  batches use one outer timer around state restoration and recurrence. Each
  phase targets 32 ms within an enforced 20–100 ms window, with 30 batches per
  case across three interleaved process pairs. The comparison used Rust
  1.99.0/Pulp V3, GCC 15.2, AVX2/FMA, x86-64-v3, Ryzen 9 5950X and pinned DUCC
  revision `80388904b8cfc32400ba84af8a6a45aabfe60f86`. Median µs/op:

  | Seed | Lanes | Rust/DUCC kernel | Ratio | Rust/DUCC reset-inclusive | Ratio |
  |---|---:|---:|---:|---:|---:|
  | Synthetic `m=1` | 64 | 3.901 / 2.644 | 1.48× | 3.922 / 2.663 | 1.47× |
  | Synthetic `m=1` | 128 | 6.412 / 5.162 | 1.24× | 6.447 / 5.185 | 1.24× |
  | Synthetic `m=1` | 256 | 13.173 / 10.765 | 1.22× | 13.197 / 10.795 | 1.22× |
  | T720 GL sectoral `m=500` | 64 | 0.572 / 0.288 | 1.98× | 0.602 / 0.304 | 1.98× |
  | T720 GL sectoral `m=500` | 128 | 0.0842 / 0.0477 | 1.76× | 0.1204 / 0.0732 | 1.65× |

  Timer overhead was about 20 ns/op. State relative L2 error was
  `2.013e-13..4.279e-14` on synthetic seeds and `0..1.471e-16` on GL seeds;
  scale epochs matched exactly. Rust scale/correction preflight was
  0.240/0.371 µs at 64/128 lanes and is outside kernel timing. GL kernel
  p10–p90 was 0.570–0.577/0.287–0.290 µs (Rust/DUCC) at 64 lanes and
  0.0837–0.0849/0.0474–0.0484 µs at 128 lanes. GL process medians were
  grouped. One synthetic 64-lane Rust process median was 4.37 µs versus 3.87 µs
  in the other two without a matching scheduler-wait signal; retain that
  variability in synthetic 64-lane conclusions. DUCC process medians were
  grouped. The 256-lane GL case is a no-prefix control and was not timed.
  Reset-inclusive timing also includes Rust's correction-scratch copy, which
  DUCC does not need. This prefix result does not explain the full scalar
  analysis gap. See [the calibrated runner](scripts/benchmark_scalar_analysis_prefix.py).
- A short full-transform Rust screen compared `fc9f44a4` with this change using
  five interleaved process pairs and five samples per process, GL STANDARD A,
  odd `nphi=2T+3`, and q1/q8. Median baseline→candidate times were T300 q1
  3.396→3.244 ms, q8 0.623→0.603 ms; T720 q1 26.669→26.284 ms, q8
  4.525→4.420 ms. Both builds used Rust 1.99/Pulp V3, x86-64-v3, and the same
  Ryzen 9 5950X host; q1 used one pinned physical core and q8 the same set of
  eight physical cores. One T300 q8 sample was an outlier, while process
  medians remained grouped. This is a Rust-only screen, not a DUCC parity
  measurement; it does not change the matched-kernel ratios above.
- A Pulp grouped four-accumulator reduction remains behind
  `rustsht_group_analysis_reduction` for cumulative evaluation. It reduced the
  final channels with AVX2 shuffles and packed adds but regressed unchecked
  kernel time by 13.8%, 7.0% and 4.9% at 64/128/256 lanes. The standard
  individual reductions remain the production path. DUCC's kernel-plus-commit
  process medians spanned 4.130–4.163, 8.054–8.076 and 15.838–15.927 µs at
  64/128/256 lanes. Pooled p10–p90 ranges were 4.117–4.347, 8.020–8.415 and
  15.818–16.565 µs; the old 64-lane 9.599/5.621 µs and 256-lane
  25.318–39.165 µs process regimes did not recur. Some individual batches
  remain high outliers. Full batch ranges for kernel-plus-commit/reset-inclusive
  were 4.081–6.888/4.183–6.596, 7.981–13.391/8.090–12.311 and
  15.720–22.505/15.840–20.934 µs at 64/128/256 lanes; the cause is not
  established.
- The private DUCC-style AVX2 `vhaddpd`/`vperm2f128` reduction remains behind
  `rustsht_avx2_group_analysis_reduction`. It improved unchecked kernel time
  by 2.7%, 0.8% and 0.7% at 64/128/256 lanes, but made the checked route
  0.3%, 1.3% and 1.0% slower. Keep production on Pulp's original reductions;
  the AVX2 candidate applies the same summation order to checked and unchecked
  variants so their benchmark outputs remain exactly equal.
- Cumulative interaction screen at source `35179326` combined the fused
  in-loop range check with that AVX2 reduction, retaining the 256-pair
  production tile. Ten interleaved process pairs collected 60 batches per
  lane/backend; all 1,800 timed batches were 29.21–75.90 ms. Kernel-plus-commit
  medians, Rust checked/unchecked versus pinned DUCC, were 6.995/4.709/4.148 µs
  at 64 lanes, 12.766/8.803/8.090 µs at 128, and 24.587/17.510/15.890 µs at
  256. Relative to the earlier checked default, this combination was 3.4%
  faster at 64, 0.6% at 128 and neutral at 256; unchecked 256 was 1.4% slower.
  Coefficient errors were `1.970e-16`; output relative L2 error was
  `5.928e-14..1.628e-13`. In the exact AVX2 build, the hot fused-check loop
  keeps its vector range mask in a YMM register; the Pulp-reduction fused
  build stores and reloads that mask from the stack each vector iteration.
  Recurrence coefficient vectors still use stack-backed memory operands.
  This interaction removes the earlier fused-check regression in the kernel
  screen but does not materially close the checked 256-lane gap. Keep it as a
  cumulative candidate pending full-transform measurement and ablation.
- Full-transform screen at source `e8675966` compared production with that
  cumulative candidate, using identical `-C target-cpu=x86-64-v3` builds and
  three alternating process pairs per cell. The reusable-plan `m1b_bench`
  measured f64 scalar GL analysis on `721x1443`, full and half `mmax`, q1 and
  q8; q1 used the same physical core, q8 the same eight distinct cores. The
  median process times (production/candidate, ms) and median paired candidate
  changes were: full q1 `26.371/27.648` (`+3.30%`), full q8 `4.295/4.406`
  (`+2.21%`), reduced q1 `21.903/22.767` (`+3.94%`), reduced q8
  `3.436/3.481` (`+1.39%`). Each process used one warmup and five timed calls.
  A first generic-target build was discarded after it ran about four times
  slower than the established x86-64-v3 baseline. The candidate's focused
  fused-analysis reference/tail test passed. Retain the cfg candidate for
  cumulative ablation, but do not select it for production based on this
  full-transform screen.
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
- Host samples gave a 4.592 GHz median reported clock rate (3.593–4.692 GHz),
  zero median run-queue wait fraction (maximum 0.701%), and 28–39 involuntary
  switches per DUCC process versus zero or one for Rust. DUCC process medians
  stayed tightly grouped; long batch tails remain in the reported distributions.
  No favorable or unfavorable samples were removed.

**Next:** reduce ordinary-loop coefficient stack traffic without restoring
the rejected wide unroll; preserve the checked post-group path and keep fused
range-check/AVX2 reduction for final cumulative ablation. The checked kernel
remains 1.55× DUCC at 256 lanes after the Rust output commit; the unchecked
A/B route is about 1.10×.
