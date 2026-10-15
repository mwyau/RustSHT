# Scalar and spin-1 analysis leaf profile

## Method

A test-only release profile measured scalar and spin-1 GL STANDARD analysis at T511 and T1023 with full `mmax`, `ntheta=lmax+1`, `nphi=2*lmax+3`, and `phi0=0.37`. Explicit Rayon pools used q3, q5, q6, q8, and q16. Each case ran five successive transforms after plan and workspace construction; the first call was not discarded as a warm-up. The profile recorded each recursive order leaf's `m` range, order count, sum of active ring/order pairs, estimated work, elapsed time, start and end offsets from the whole-call timer, Rayon worker index, and normal/scaled/mixed recurrence-block counts. The existing work estimate is

\[
W(m_0,m_1)=\sum_{m=m_0}^{m_1-1} n_{\mathrm{active}}(m)(l_{\max}-m+1).
\]

The block counters classify the recurrence handling observed by the instrumented scalar or spin-1 paired path. They are diagnostic counts; they do not assign a measured cost to each block class. Start and end timestamps exclude the profile's active-ring count scan. The test ran with rustc `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, and `cargo test --release`. The host was an x86-64 CPU with host topology omitted, running Linux. The process inherited 32 pinned CPU cores and had no explicit affinity. The CPU exposes AVX2 and FMA; Pulp runtime dispatch was enabled, but the diagnostic did not log the selected ISA. Load-average telemetry was retained only in local diagnostics. These measurements diagnose scheduling and are not paired performance acceptance results. The raw 100 samples are in [`profile.log`](data/analysis-leaf-profile-2026-10-15/profile.log). The test command was:

```text
cargo test --release -p rustsht --all-features m5_analysis_leaf_imbalance_profile -- --ignored --test-threads=1 --nocapture
```

## Results

The table reports the median across five samples of the slowest-to-fastest leaf elapsed-time ratio and the maximum-to-minimum estimated-work ratio within a transform.

| Operation | Degree | q3 time/work | q5 time/work | q6 time/work | q8 time/work | q16 time/work |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar | T511 | `1.45 / 1.0074` | `1.68 / 1.0173` | `1.89 / 1.0230` | `2.00 / 1.0253` | `3.59 / 1.0666` |
| Scalar | T1023 | `1.28 / 1.0053` | `1.38 / 1.0099` | `1.45 / 1.0071` | `1.55 / 1.0152` | `2.09 / 1.0378` |
| Spin 1 | T511 | `1.42 / 1.0085` | `1.56 / 1.0160` | `1.66 / 1.0223` | `1.76 / 1.0397` | `2.76 / 1.0675` |
| Spin 1 | T1023 | `1.31 / 1.0057` | `1.38 / 1.0104` | `1.46 / 1.0124` | `1.77 / 1.0129` | `2.18 / 1.0224` |

For q3 through q8, every sample reached the full pool size in concurrent leaf intervals and recorded all worker IDs. At q16, 99 of 100 samples recorded all 16 worker IDs. One scalar T511 q16 sample used 15 worker IDs and reached 14 concurrent leaf intervals. Rayon does not expose steal-event counts through the observed worker index, so the profile records worker assignment and overlap rather than claiming a steal count. The first scalar T1023 leaf began about 40 ms after the whole-call timer started; the first spin-1 T1023 leaf began about 74–76 ms after it started. The profile does not split that interval among Fourier transforms, weighting, or other analysis preparation. It shows that a substantial part of whole-call time precedes the measured order leaves. At q6, spin-1 T1023 leaves carried nearly equal estimated work while their elapsed times differed. Median times below use five samples; the path counts are constant across those samples.

| `m` range | Orders | Active ring/order pairs | Estimated work | Leaf time, median (range) | Normal / scaled / mixed blocks |
| --- | ---: | ---: | ---: | ---: | ---: |
| `0..74` | 74 | 75,776 | 74,828,800 | `27.73 ms (27.59–31.42)` | 564 / 0 / 28 |
| `74..156` | 82 | 83,066 | 75,569,552 | `34.85 ms (34.66–36.49)` | 574 / 22 / 60 |
| `156..251` | 95 | 91,168 | 74,894,756 | `33.76 ms (32.88–39.04)` | 585 / 0 / 175 |
| `251..368` | 117 | 104,272 | 74,641,208 | `36.17 ms (36.06–37.14)` | 625 / 0 / 244 |
| `368..531` | 163 | 130,098 | 75,051,370 | `40.58 ms (39.83–44.50)` | 759 / 0 / 344 |
| `531..1024` | 493 | 267,542 | 74,921,902 | `40.89 ms (40.40–41.80)` | 1,872 / 0 / 458 |

Across all q values, scalar T511 recorded 1,526 normal, 20 scaled, and 236 mixed blocks per transform; spin 1 recorded 1,523, 22, and 238. At T1023 the corresponding counts were 4,980 / 20 / 1,307 for scalar and 4,979 / 22 / 1,309 for spin 1. The q6 spin-1 T1023 example associates more mixed blocks and more orders with longer leaves. These counts correlate with elapsed time; this profile does not isolate their individual costs.

## Scheduling decision

The active-ring work estimate balances the measured leaf work to within 2.3% at q6, while the q6 leaf-time ratio reaches 1.9 at T511 and 1.5 at T1023. The q16 ranges are less even, in part because a finite number of orders cannot be split at every target boundary. The path counts indicate that the active-ring/degree product omits per-order and mixed-block costs. The profile does not justify enabling more leaves. A previous paired spin-1 experiment doubled leaves at q8 and regressed T511 by 3.19% and T720 by 11.55%; see the [overpartition report](spin1-analysis-leaf-overpartition-2026-10-14.md). The E2 change preserves the q8 equal-capacity split. No dynamic queue candidate was tested here. Any later queue experiment needs a fresh paired comparison against the current scheduler and those rejected overpartition results.

## Validation

`cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test --workspace --all-features`, and `cargo test -p rustsht --no-default-features` passed. The all-feature suite passed 181 Rust tests and 10 f32 tests, with 11 tests ignored. The no-default suite passed 144 Rust tests and 10 f32 tests, with 7 ignored. The release profile test passed and recorded 100 transform samples. Python tests were not run because bindings did not change. No numerical kernel or production scheduling path changed.
