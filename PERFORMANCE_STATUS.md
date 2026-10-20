# RustSHT performance checkpoint

**Branch:** `opt/sht-performance-parity`. **Updated:** 2026-10-20. **Current
production candidate:** `04ea6cf4` (fused scalar-analysis input preparation
enabled by default; rejected direct-output, custom reductions and promotion
variant removed).
**Earlier A/B source:** `9f7e8658`.
**Calibrated harness:** `1ee1a195`. **Benchmark default:** production flags at
`e2c2dc15`. For the
active scope and acceptance criteria, see [Plan 11](plans/11-ducc-equivalent-scalar-analysis.md).

- Full-transform checkpoint at source `5e290b7` compared production f64 scalar
  GL analysis and synthesis with DUCC `80388904b8cfc32400ba84af8a6a45aabfe60f86`
  using Rust 1.99.0/Pulp V3 and GCC DUCC x86-64-v3, at q1/q8. The exhaustive
  small-degree and cross-bandlimit sweep covered 214 configurations, 856 warm
  rows, 2,568 random-output comparisons and 428 additional numerical suites;
  maximum relative L2 output error was `2.536e-13`. Of 856 rows, paired
  intervals classified 292 faster, 552 slower and 12 inconclusive. Ratios vary
  by operation, size and longitude sampling: even-smooth full-order q1 was
  near parity at T1536 (A 0.99×, S 0.96×), but T3072 analysis reached 8.39×
  at q1 and 7.44× at q8, while T3072 synthesis was 0.99× at q1 and 1.50× at
  q8. End-to-end measurements do not isolate the stage responsible. Host
  samples show intermittent scheduling/frequency interference; no timings were
  removed and paired distributions are retained. See the
  [full-transform report](reports/ducc-full-transform-parity-2026-10-20.md)
  and machine-readable results. The historical benchmark remains pinned to
  its original compiler and DUCC source.
- Plan 11 Phase 2 uses Rust 1.99.1 (`rustc 58cf8f9c3`, LLVM 23.1.1) and the
  latest fetched DUCC 0.42.0 `ducc0` head,
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The five numerical SHT, SIMD
  and constants headers checked by the native benchmark are byte-identical to
  the previous `80388904` pin; checksums are recorded in
  `reports/data/ducc-reference-rebaseline-2026-10-20/SHA256SUMS`. Native T255
  GL validation covered six longitude/order configurations, A/S, q1/q8 and
  72 output comparisons; maximum relative L2 error was `8.976e-14`. Twelve
  additional basis, roundtrip, edge-input and adjoint suites passed. Workspace
  release tests and no-default-features release tests passed (242 passed in
  each; 24 ignored); formatting and strict Clippy passed with the repository's
  existing custom-cfg warning suppressed.
- DUCC 0.42.0 was built from its upstream CMake project with
  `DUCC0_OPTIMIZATION=multiarch`, using GCC 15.2 and Clang 21.1.8. Both
  extensions reported runtime dispatch to x86-64-v3 on this AVX2-only Ryzen
  5950X. Their SHT-specific `map2alm_kernel` symbols were identified and
  disassembled in the v3 and v4 objects; v3 uses four f64 lanes and v4 uses
  eight. The v4 objects were static-inspection only. A public T255 multiarch
  analysis smoke call completed, but its short timing screen was noisy and is
  not a performance comparison. Build commands and findings are in
  `reports/data/ducc-multiarch-2026-10-20/README.md`. Next: establish the
  calibrated Rust 1.99.1 / DUCC GCC and Clang v3 baseline, then isolate the
  remaining warmed-kernel costs.

- Production f64 scalar GL analysis keeps the checked two-pair AVX2/FMA route
  and scaled fallback. The active range-cache candidate reuses a checked
  ordinary-recurrence range result for each plan/order/tile/Pulp backend only
  after that block has exercised every checked group and completed successfully.
  Initialization, product preflights, scaled recurrence and robust fallback
  still run for every call. The compile-time unchecked specialization remains
  available for controlled evaluation.
- At `88716ca3`, the one-pass preparation route became the default: it checks
  packed spectra while applying the opposite-parity cosine weight. In three
  interleaved T720 GL full-transform pairs, candidate/base median ratios were
  0.989/0.979 (full-mmax q1/q8) and 0.991/0.989 (reduced-mmax q1/q8). The
  helper edge cases, T511 DUCC/analytic analysis, exceptional-input range-cache
  checks and q1/q8 schedule test passed. This is a modest Rust-only gain, not a
  DUCC parity claim. Final ablations found direct coefficient accumulation
  substantially slower end to end; unchecked ordinary recurrence was neutral
  after the checked range cache; the custom AVX2 reduction and single-pass
  scale-promotion variants had mixed or negligible whole-transform effects.
  Production retains checked recurrence, staged output, portable Pulp
  reductions and the existing robust fallback. At `3ad85539`, removed the
  direct-output experiment and its preflight/dispatch plumbing after completing
  the four-case ablation. Release T511 DUCC/analytic, exceptional-input cache,
  q1/q8 schedule, no-default-feature safety tests, formatting and all-target
  Clippy passed. At `dfa900ca`, removed grouped Pulp and private AVX2
  reductions: grouped reduction was 4.9–13.8% slower in unchecked kernel
  timing, while AVX2's sub-1% unchecked benefit did not improve checked or
  full-transform execution. T511 DUCC/analytic, formatting and all-target
  Clippy passed. At `04ea6cf4`, removed the single-pass scaled promotion
  alternative and its diagnostic-only counters/tests; the cumulative
  full/reduced q1/q8 comparison had mixed sub-percent results and all 474
  initially scaled T720 blocks were ready at the first promotion point. The
  checked scale correction remains the production transition. Release T511
  DUCC/analytic and cached exceptional-input tests, formatting and all-target
  Clippy pass.
- Final calibrated kernel profile ran at `d36b1a63` on the selected source:
  T720 GL `m=1`, q1, Rust 1.99/Pulp V3 and pinned DUCC, both x86-64-v3, on a
  Ryzen 9 5950X. Ten interleaved process pairs collected six 20–100 ms batches
  per process/backend/lane. Paired Rust kernel-plus-commit medians versus DUCC:

  | Lanes | Rust checked | Rust warmed route | DUCC | Checked ratio / excess | Warmed ratio / excess | Rust commit only |
  |---:|---:|---:|---:|---:|---:|---:|
  | 64 | 7.135 µs | 4.893 µs | 4.105 µs | 1.736× / 3.024 µs | 1.191× / 0.786 µs | 0.136 µs |
  | 128 | 12.832 µs | 8.919 µs | 8.045 µs | 1.595× / 4.772 µs | 1.109× / 0.876 µs | 0.136 µs |
  | 256 | 24.578 µs | 17.267 µs | 15.800 µs | 1.556× / 8.780 µs | 1.095× / 1.506 µs | 0.136 µs |

  The warmed route uses the same unchecked specialization production selects
  after its plan/order range proof; the checked route remains necessary to
  establish that proof. Final coefficient commit is only 0.136 µs, so it cannot
  explain the warmed excess. The fused recurrence and per-degree
  SIMD lane reductions remain jointly timed; this profile does not attribute
  their separate shares. All 1,800 kernel/reset batches were 28.83–77.32 ms;
  all 1,440 operation wall batches were 33.40–77.75 ms. Numerical relative
  L2 error was `5.928e-14..1.628e-13`. Process ratio spreads were stable; median
  CPU frequency was 4.592 GHz and median run-queue wait 0.012%, with one
  66.528% wait outlier retained. Next: isolate the horizontal reductions in a
  benchmark-only kernel variant before changing recurrence scheduling.
- The release regression test covers oversampled T511 GL, an odd SIMD-tail
  lane, zero/subnormal/extreme finite maps, repeated workspace reuse, and a
  compatible workspace reused across plans. The valid T720 fixture recorded
  zero late group-range declines at q1 and q8. A short Rust 1.99.0/Pulp V3,
  x86-64-v3 T720 GL STANDARD screen on Ryzen 9 5950X compared the cache with
  the same source compiled using `rustsht_disable_analysis_pair_range_cache`.
  At q1 the no-cache/cached medians were 26.405/23.291 ms (11.8% lower with
  cache). At q8, process medians were 5.138–5.287/4.556–4.772 ms; the candidate
  was lower in both pairs, but outliers and host variation make this a
  provisional Rust-only full-transform result. Earlier unpaired baseline
  screens are not used for attribution. The benefit applies after the checked
  workspace warmup and is not a DUCC comparison.
- A calibrated direct-kernel run at `aeb123d6` used T720 GL `m=1`, Rust 1.99.0,
  Pulp V3, x86-64-v3, GCC 15.2.0 and pinned DUCC revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`. Ten interleaved process pairs
  collected 60 batches per backend/variant; batch durations were 28.94–82.12 ms.
  Equivalent kernel-plus-commit medians (checked/cached-hot-path/DUCC, µs) were
  `7.079/4.869/4.119`, `12.632/8.902/8.050`, and `24.356/17.199/15.814` at
  64/128/256 lanes. The ordinary post-group validation cost was respectively
  2.210/3.730/7.157 µs. At 256 lanes, it accounts for 83.5% of checked Rust's
  paired excess over DUCC (8.574 µs); the cached hot path retains 1.368 µs
  excess, or 1.088× DUCC. The 7.157 µs saving is about 27% of the earlier
  single-call 26.590 µs figure, but the calibrated checked median is 24.356 µs.
  Output relative L2 errors were `6.670e-14`, `1.628e-13`, and `5.928e-14`.
  Median measured frequency was 4.617 GHz; median/max involuntary switches were
  2/11. This is an ordinary `m=1` kernel result, not a whole-transform claim.
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
  timed processes are pinned to the same physical core. Rust warms its checked
  and unchecked routes three times each; each DUCC timed process now performs
  and reports three reset/kernel/checksum warmups, which the runner validates.
  Frequency/scheduling aggregates exclude build and calibration processes.
  DUCC revision is `80388904b8cfc32400ba84af8a6a45aabfe60f86`.
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
- Batch-timing methodology checkpoint on clean production source `3c86e40d`:
  the runner now brackets repeated operations with batch-level clocks, derives
  reset-excluded time by subtracting a separate reset-only batch, and retains
  the unadjusted reset-inclusive result. It checks each DUCC checksum and the
  Rust/DUCC coefficient/output error outside timing, and flags process ranges
  above 15% of the process-median value. Ten interleaved process pairs produced
  60 batches per backend/lane; all 1,800 measured intervals were 27.74–77.49 ms
  and all 1,440 operation wall batches were 34.05–77.82 ms. Process-level
  checked/unchecked/DUCC kernel-plus-commit medians were `7.016/4.902/4.053`,
  `12.633/8.903/7.981`, and `24.297/17.164/15.691` µs for 64/128/256 lanes.
  No process spread exceeded the gate; numerical relative L2 errors were at
  most `1.628e-13`. The earlier 9.599/5.621 µs and 25.318–39.165 µs DUCC
  regimes did not recur. Frequency telemetry was available; median/max
  run-queue wait was 0.004/0.080%, with 2 median and 11 maximum involuntary
  switches. This remains an isolated ordinary `m=1` kernel comparison.
- Experimental direct coefficient accumulation, enabled only by
  `rustsht_scalar_analysis_direct_accumulation`, passes the T511 cached-versus-
  checked safety test (relative L2 `2.60e-16`) and the T255/T1023 q1/q8
  analysis consistency test. A matched Rust-only T720 GL screen used the same
  reused plan/workspace and input at `721x1443`, with three alternating
  baseline/candidate process pairs and five timed samples per process. For full
  and reduced `mmax=255`, direct/staged median ratios were respectively
  `2.142/1.896` at q1 and `1.531/1.902` at q8. The q1 processes were pinned;
  q8 process ratios were more variable but direct was slower in every pair.
  Keep this correct candidate disabled and retain it for final cumulative
  ablation. Its regression source remains unattributed.
- Candidate `rustsht_single_pass_analysis_promotion` compares a SIMD scale-epoch
  check followed by checked in-place conversion against the current validation
  and conversion passes. The T720 GL q1 instrumentation counted 474 initially
  scaled fused blocks; all 474 were ready at the first promotion point, with no
  later mixed-to-ordinary transitions. The candidate exercised all 474. Tests
  matched checked conversion for ordinary and scaled values with a SIMD tail,
  passed T511 DUCC/analytic checks, T255/T1023 q1/q8 consistency, cached safety,
  no-default-feature coverage and Clippy. A Rust-only T720 GL full-transform
  screen used Rust 1.99.0, x86-64-v3, AMD Ryzen 9 5950X, three alternating
  process pairs, one warmup and five timed calls per process, with q1 pinned to
  one physical core and q8 pinned to eight physical cores. Candidate/base
  process-median ratios were `0.994/0.992` for full-mmax q1/q8
  and `0.945/1.014` for reduced `mmax=255` q1/q8. These small, noisy changes do
  not establish a general speedup; keep the candidate opt-in for final
  cumulative ablation.
- Candidate `rustsht_fused_analysis_input_preparation` at `0b423400` combines
  the packed-spectrum range check and opposite-parity cosine weighting in one
  lane pass, retaining the separate production path and restoring inputs before
  scaled fallback on failure. The helper matches the previous checks for normal,
  subnormal, zero, extreme and SIMD-tail cases; candidate T511 DUCC/analytic,
  cached-safety and q1/q8 consistency tests passed. A same-source Rust 1.99,
  x86-64-v3 T720 GL screen used three alternating process pairs, one warmup and
  five samples, with q1 on one physical core and q8 on eight. Candidate/base
  medians were `23.382/23.485 ms` full-mmax q1, `3.897/3.978 ms` full-mmax q8,
  `17.774/17.899 ms` reduced-mmax q1 and `2.643/2.584 ms` reduced-mmax q8.
  The mixed result, including a 2.3% reduced-mmax q8 regression, is not a
  general speedup; retain it disabled for cumulative ablation. All-target,
  all-feature Clippy and the no-default-feature helper test passed.
- Final coefficient conversion at `2cebdd2a` now carries the previous alpha
  with the previous even coefficient, matching DUCC's `inner_loop_m2a` data
  flow without the indexed prior-alpha reload. The helper matches the prior
  indexed formula exactly across odd/even `m`, degree tails and short orders;
  T511 DUCC/analytic and exceptional-input tests passed. In three alternating
  Rust 1.99 x86-64-v3 T720 GL process pairs, candidate/parent median ratios were
  `0.992/0.987` full-mmax q1/q8 and `0.996/1.007` reduced-mmax q1/q8. These
  small mixed changes do not establish a speedup. Formatting, no-default
  conversion tests and all-target/all-feature Clippy passed; strict Clippy
  requires allowing the repository's pre-existing custom-cfg warnings. Steps
  1–10 are implemented; next run the required cumulative 64/128/256-lane and
  DUCC comparison, then ablate the interacting candidates.
- Warmup-controlled rerun at `9967ad19` kept the same work and flags after the
  DUCC timed-process warmup fix. Across 64/128/256 lanes, kernel-plus-commit
  medians (checked/unchecked/DUCC, µs) were `7.151/4.848/4.115`,
  `12.679/8.846/8.021`, and `24.491/17.026/15.813`; paired checked/DUCC
  ratios were `1.736` (p10–p90 `1.727–1.748`), `1.582` (`1.576–1.597`) and
  `1.549` (`1.542–1.558`). Reset-inclusive medians were
  `7.242/4.947/4.217`, `12.787/8.956/8.159` and
  `24.643/17.204/16.000` µs. All 1,800 batches were 29.42–79.12 ms and passed
  the 20–100 ms gate. Output errors were unchanged. DUCC medians were within
  0.6% of the earlier run without timed-process warmups, so cold startup does
  not explain the previously reported 9.599/5.621 µs and 25.318–39.165 µs
  regimes; calibrated runs did not reproduce them. Median frequency was
  4.617 GHz, median run-queue wait 0.012%, and one 66.38% wait outlier was
  retained. See the runner and the [benchmark method](reports/data/scalar-analysis-pair-grouping-2026-10-19/README.md).
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
- Split-mask scan candidate `rustsht_split_analysis_range_masks` at `9f7e8658`
  emitted two independent AVX2 validity-mask chains in the post-group scan,
  but retained the same state loads and lane comparisons. Relative to the
  rematerialization candidate, checked kernel-plus-commit medians were
  `7.262/13.082/24.644` versus `7.152/12.725/24.582` µs at 64/128/256 lanes
  (`+1.5%/+2.8%/+0.3%`). The candidate did not reduce the scan cost; leave it
  disabled. All 1,800 calibrated batches were 29.12–71.25 ms; a single process
  had 66.44% run-queue wait while the median was 0.011%, so retain all samples.
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
- At the pre-cache source, the same T720 GL full-order analysis with five
  interleaved q1 process pairs
  measured checked/unchecked medians of 26.932/23.541 ms (12.6% lower unchecked).
  q8 medians were 4.415/3.936 ms (10.8% lower), with one slow unchecked process.
  The checked T720 route recorded zero late declines at q1 and q8; the unchecked
  release suite also passed. This was the earlier pre-cache default; the current
  path uses the cached unchecked specialization only after checked validation.
- The three-process full-analysis screen improved q1 by 4.0% at T720 and 5.4%
  at reduced-order T300; q8 T720 improved 4.4%. q8 T300 was too noisy to show a
  clear benefit. The candidate passed all-feature release tests at 64/128/256,
  no-default-feature tests, and all-target/all-feature Clippy.
- Host samples gave a 4.592 GHz median reported clock rate (3.593–4.692 GHz),
  zero median run-queue wait fraction (maximum 0.701%), and 28–39 involuntary
  switches per DUCC process versus zero or one for Rust. DUCC process medians
  stayed tightly grouped; long batch tails remain in the reported distributions.
  No favorable or unfavorable samples were removed.

**Next:** inspect the exact AVX2 ordinary loop from the calibrated run and
compare its remaining 1.368 µs excess at 256 lanes with DUCC's `map2alm_kernel`.
Attribute recurrence dependency chains, loads and horizontal reductions before
changing the loop. Keep the validated checked-first cache and robust fallback.
