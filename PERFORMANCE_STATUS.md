# RustSHT performance checkpoint

**Branch:** `opt/sht-performance-parity`. **Updated:** 2026-10-20. **Checked
production baseline:** Pulp post-group scan. **A/B source:** `4fcb1136`.
**Calibrated harness:** `e4c90133`. **Benchmark default:** production flags at
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
- A matched `m=1, lmax=720` rerun used 60 calibrated batches in ten
  interleaved process pairs per lane count (six batches per process). All
  1,800 measured batches were 29.31–70.75 ms. Median µs/op; kernel-plus-commit
  includes Rust's staged coefficient addition and matches DUCC's direct output
  accumulation:

  The run used an AMD Ryzen 9 5950X, Rust 1.99.0/Pulp V3 and GCC 15.2.0,
  both targeting x86-64-v3 AVX2/FMA. Each single-thread process used the same
  physical core; DUCC was pinned to revision `80388904b8cfc32400ba84af8a6a45aabfe60f86`.

  | Lanes | Kernel + commit: Rust checked / unchecked / DUCC | Reset inclusive: Rust checked / unchecked / DUCC |
  |---:|---:|---:|
  | 64 | 7.239 / 4.914 / 4.166 | 7.307 / 5.010 / 4.278 |
  | 128 | 12.840 / 8.920 / 8.089 | 12.885 / 9.038 / 8.230 |
  | 256 | 24.596 / 17.274 / 15.966 | 24.737 / 17.433 / 16.164 |

  Kernel-plus-commit checked/unchecked ratios were 1.738×/1.180×, 1.587×/1.103×
  and 1.540×/1.082× at 64/128/256 lanes. Reset-inclusive ratios were
  1.708×/1.171×, 1.566×/1.098× and 1.530×/1.079×. The isolated Rust commit
  loop measured 0.10–0.14 µs/op, under 1% of the 256-lane unchecked kernel
  time. The staging writes remain inside recurrence timing, so these data do
  not measure the full cost of changing the transaction design; they provide
  no basis for weakening fallback safety. Rust's staged recurrence output is
  not compared directly with DUCC. Outputs were
  validated outside timing; coefficient scaled difference was `1.970e-16`,
  output relative L2 error was `5.928e-14` to `1.628e-13`, and max absolute
  error was at most `2.559e-11`. DUCC kernel-plus-commit process medians were
  tightly grouped at 4.145–4.330, 8.030–8.222 and 15.852–16.577 µs. The old
  64-lane 9.599/5.621 µs and 256-lane 25.318–39.165 µs regimes did not recur.
  Host samples showed 4.592 GHz median frequency (3.593–4.692), 0.000% median
  run-queue wait (0.702% max), and zero median involuntary switches (13 max).
  These diagnostics do not identify the exact cause of the earlier regimes.
  See the [reproducible runner](scripts/benchmark_scalar_analysis_kernel.py).
- Sectoral initialization now follows DUCC's per-SIMD-vector `mypow` choice:
  vectors above the `2^-400/m` underflow limit use direct exponentiation, while
  vectors containing smaller lanes retain scaled exponent management. This
  avoids scaling safe vectors merely because another vector in the block needs
  it. The scalar seed reference test covers orders 0, 1, 250 and 720, mixed
  vectors and a SIMD tail. The all-feature and no-default-feature release test
  suites pass.
- The scaled-prefix candidate now carries DUCC's below-one epoch frontier out
  of the two-pair recurrence pass, checking only vectors that rescaled instead
  of rereading the full epoch array after each group. Scale correction,
  insignificant-term handling and fallback checks are unchanged. A focused
  test covers threshold crossings, sparse rescaled vectors and SIMD tails; the
  all-feature release library suite passes. The prefix harness now compares
  synthetic seeds and T720 GL sectoral seeds (`m=500`) against DUCC's
  `iter_to_ieee` state. Rust and DUCC agree on all lane states and scale epochs:
  the GL seed advances to pair 36 at 64 lanes, pair 2 at 128 lanes, and skips
  the prefix at 256 lanes because that block includes significant seeds. Its
  five-call samples remain exploratory; calibrated prefix timings remain next.
- The prefix frontier update now reduces the updated epoch mask only when a
  SIMD vector actually rescales, matching DUCC's conditional `rescale` check.
  Vectors with no scale change retain the entry invariant that every epoch is
  below one. Release tests and DUCC state checks pass; the five-call screen is
  too short and variable to establish a speed change, so keep this as an
  experimental candidate pending calibrated paired measurements.
- The calibrated prefix runner measured the production Rust frontier loop
  against pinned DUCC `iter_to_ieee` on x86-64-v3, Pulp V3/GCC 15.2 AVX2/FMA,
  Ryzen 9 5950X. Each case has 30 batches across three interleaved process
  pairs, with ~35 ms target batch walls; states matched at relative L2
  `0..1.47e-16` for GL seeds, with exact scale epochs. Median kernel µs/op:

  | Seed | Lanes | Rust | DUCC | Ratio | Reset-inclusive ratio |
  |---|---:|---:|---:|---:|---:|
  | Synthetic `m=1` | 64 | 3.813 | 2.676 | 1.43× | 1.41× |
  | Synthetic `m=1` | 128 | 6.476 | 5.171 | 1.25× | 1.24× |
  | Synthetic `m=1` | 256 | 13.093 | 10.761 | 1.22× | 1.20× |
  | T720 GL sectoral `m=500` | 64 | 0.570 | 0.306 | 1.87× | 1.67× |
  | T720 GL sectoral `m=500` | 128 | 0.099 | 0.067 | 1.48× | 0.87× |

  The GL seed advances 36 pairs at 64 lanes and 2 at 128; the 256-lane seed
  has significant values and does no prefix recurrence. Rust's scale/correction
  preflight is separate (0.250/0.381 µs at 64/128) and is not included in the
  DUCC kernel ratio. The 64-lane kernel excess is 0.264 µs per tested block;
  these measurements do not attribute the full analysis gap to prefix work.
  Reset-inclusive timing also resets Rust's correction scratch, which DUCC does
  not have. Process-median frequency samples were 4.6–4.7 GHz (individual
  samples fell to about 3.6 GHz); runqueue wait stayed below 0.09%.
  GL kernel p10–p90 ranges were 0.568–0.573/0.305–0.308 µs (Rust/DUCC) at 64
  lanes and 0.0985–0.1002/0.0668–0.0677 µs at 128 lanes; their process medians
  were similarly grouped. One synthetic 128-lane Rust process was slower
  without a matching scheduler-wait signal. The frontier
  conditional-check change itself still needs a calibrated before/after
  ablation. See [the calibrated runner](scripts/benchmark_scalar_analysis_prefix.py).
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

**Next:** reduce the checked kernel's repeated post-group state traffic without
carrying masks through the eight-accumulator recurrence or weakening its
scaled fallback. The checked kernel remains 1.55× DUCC at 256 lanes after the
Rust output commit is included; the unchecked A/B route is 1.08×.
