# Capacity-proportional analysis order splitting (E, 2026-10-15)

Parallel scalar and spin analysis now splits active order work according to the worker capacity assigned to each child branch. Equal-capacity branches retain the established midpoint. Unequal branches target the exact rational work share, including its fractional part, and retain the earlier split on an exact tie. The implementation uses the active-order work prefix added in the first E increment.

Each output order still has one owner and keeps its existing within-order summation. The change adds no recurrence scratch. The E1 prefix contains `mmax + 2` `u128` values and is built only when analysis selects the parallel order-group path.

## Split correctness

`weighted_order_midpoint_for_capacity` uses degree work when every ring excludes the interval. The prefix splitter uses active ring work when any order is active. Both preserve the old split when child capacities are equal. Capacity sums, work totals, and the target calculation use checked arithmetic.

The target comparison retains the fractional share. For work `20` and capacities `2:1`, the target is `13 + 1/3`; the regression chooses the work-15 boundary over work 11. The earlier floored comparison tied those boundaries and chose the worse one. An equivalent active-prefix regression covers the prefix path. Tests also check every candidate boundary against the closest capacity-weighted target, inactive cutoffs, asymmetric cutoffs, truncated `mmax`, and equal-capacity midpoint compatibility.

A read-only Sol High review found three issues across its first pass: the test-only T2047 Rayon call needed a `parallel` feature gate; the fractional part needed to participate in midpoint comparison; and one overflow assertion assumed 64-bit `usize`. The implementation and tests now address all three. The follow-up review found no remaining correctness findings. The reviewer made no file changes.

## Numerical validation

The cross-pool split test compares scalar and spin-1 results from q2, q3, q4, q5, q6, q8 and q16 against q1 for a truncated T511 GL analysis. It passed after the fractional-target correction.

Release validation with five Rayon threads passed:

- `m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form`: scalar synthesis and analysis at T511, T1023 and T2047 on GL, CC and F1, against stored DUCC 0.41.0 samples and an independent sectoral closed form.
- `m3_spin_t1023_exact_analysis_matches_ducc`: spin-1 and spin-2 exact analysis at T1023 on GL, CC and F1.
- `m3_spin1_t2047_low_order_analysis_matches_q1_and_adjoint`: spin-1 q1/q5 agreement and the real-field analysis-adjoint identity.

The DUCC fixtures use `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

## Whole-transform measurements

The benchmark used the release `m1b_bench` example for f64 GL STANDARD analysis, full `mmax`, `ntheta=lmax+1`, and `nphi=2*lmax+3`. These cases use recurrence. Each process warmed the plan and workspace, then reported the median of five analysis samples. Five alternating baseline/candidate process pairs were collected per cell. The baseline source is commit `4aff01c63416ed0427e76add0baf7ce15efefd76`; candidate source commit is `0675b675b8a8ca189068250d0586b3dd139f2543`. Times are milliseconds; negative changes mean the candidate was faster.

| Operation | Degree | Threads | Baseline | Candidate | Paired change | Paired range | Faster pairs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar | T511 | 3 | 12.271 | 10.195 | -16.92% | -18.18% to -16.16% | 5/5 |
| Scalar | T1023 | 3 | 104.737 | 83.867 | -20.11% | -21.21% to -15.11% | 5/5 |
| Scalar | T511 | 5 | 9.176 | 7.973 | -11.57% | -18.55% to +8.33% | 8/10 |
| Scalar | T1023 | 5 | 73.666 | 66.004 | -10.30% | -13.07% to -9.83% | 5/5 |
| Spin 1 | T511 | 3 | 23.038 | 18.016 | -20.87% | -23.42% to -9.83% | 5/5 |
| Spin 1 | T1023 | 3 | 189.780 | 151.649 | -20.39% | -22.83% to -17.85% | 5/5 |
| Spin 1 | T511 | 5 | 15.584 | 13.832 | -12.03% | -12.64% to -9.54% | 5/5 |
| Spin 1 | T1023 | 5 | 132.909 | 122.375 | -8.06% | -9.22% to -6.74% | 5/5 |
| Scalar | T511 | 6 | 9.177 | 7.559 | -17.62% | -20.08% to -11.81% | 5/5 |
| Scalar | T1023 | 6 | 73.209 | 61.966 | -15.36% | -16.81% to -14.19% | 5/5 |
| Spin 1 | T511 | 6 | 15.689 | 13.063 | -16.82% | -17.02% to -13.84% | 5/5 |
| Spin 1 | T1023 | 6 | 131.428 | 112.343 | -14.47% | -15.26% to -14.12% | 5/5 |

The scalar q5 T511 row combines two five-pair sessions; its range crosses zero, so that case remains inconclusive despite the lower ten-pair median. The scalar q5 T1023 repeat improved in all five pairs; its earlier screen had one slower pair. The spin-1 q5 T511 repeat improved in all five pairs; its first screen contained a large outlier. Other q3 and q6 cells improved in every recorded pair. q6 keeps a 3:3 top-level split and exercises the unequal 2:1 splits below it.

The equal-capacity recursion used by q2, q4, q8 and q16 remains identical by test. q1 does not use the parallel order-group path. The q3/q5/q6 results demonstrate end-to-end analysis gains for those uneven worker trees; they do not establish DUCC parity or gains for every pool size.

Median process peak RSS changed by only a few hundred KiB in the measured cells. E2 adds no workspace data; the small differences are within run-to-run process variation. Raw samples retain each process's RSS and five inner timings.

The machine was an AMD Ryzen 9 5950X, 16 cores and 32 logical CPUs, running Linux `7.0.14-20-pve`. The compiler was rustc `1.99.0 (b940084d7 2026-09-28)`, LLVM 23.1.1, release `opt-level=3`. The build used portable Pulp runtime dispatch without `target-cpu=native`; the CPU exposes AVX2 and FMA, and Pulp's runtime detector selects its x86-v3 path when x86-v4 is unavailable. q3, q5 and q6 used CPU sets 0–2, 0–4 and 0–5. Host load averages ranged from 2.58 to 3.88 during the paired sessions; the ranges and repeated runs are reported rather than treating the host as isolated.

## Reproduction and checks

The candidate harness was built with `cargo build --release --example m1b_bench`; the baseline harness was built from the source commit above in a separate target directory. The benchmark scripts and raw logs are in [`the data directory`](data/analysis-capacity-split-2026-10-15/). The `paired-exact-*` logs and `run-*-exact-*` scripts use the corrected fractional-target implementation. Earlier logs are retained as preliminary runs from before the review correction.

The benchmark commands were:

```text
git worktree add --detach target/e-baseline-src 4aff01c63416ed0427e76add0baf7ce15efefd76
CARGO_TARGET_DIR=target/e-baseline-target cargo build --release --example m1b_bench --manifest-path target/e-baseline-src/Cargo.toml
cp target/e-baseline-target/release/examples/m1b_bench target/release/examples/m1b_bench-before-scheduler
cargo build --release --example m1b_bench
cp target/release/examples/m1b_bench target/release/examples/m1b_bench-after-exact-capacity-split
flock -n target/rustsht-benchmark.lock reports/data/analysis-capacity-split-2026-10-15/run-paired-exact-capacity-split.sh
flock -n target/rustsht-benchmark.lock reports/data/analysis-capacity-split-2026-10-15/run-q6-exact-capacity-split.sh
flock -n target/rustsht-benchmark.lock reports/data/analysis-capacity-split-2026-10-15/run-scalar-t1023-exact-repeat.sh
flock -n target/rustsht-benchmark.lock reports/data/analysis-capacity-split-2026-10-15/run-t511-q5-exact-repeat.sh
```

The focused commands were:

```text
cargo test -p rustsht --all-features capacity_target_work_uses_checked_quotient_and_remainder
cargo test -p rustsht --all-features active_order_capacity_splits_follow_work_and_preserve_equal_splits
cargo test -p rustsht --all-features analysis_capacity_splits_match_q1_across_pool_sizes
RAYON_NUM_THREADS=5 cargo test --release -p rustsht --all-features m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form -- --ignored --test-threads=1 --nocapture
RAYON_NUM_THREADS=5 cargo test --release -p rustsht --all-features m3_spin_t1023_exact_analysis_matches_ducc -- --ignored --test-threads=1
RAYON_NUM_THREADS=5 cargo test --release -p rustsht --all-features m3_spin1_t2047_low_order_analysis_matches_q1_and_adjoint -- --ignored --test-threads=1 --nocapture
```

Final checks passed:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features                 # 181 Rust + 10 f32 passed; 10 ignored
cargo test -p rustsht --no-default-features           # 144 Rust + 10 f32 passed; 7 ignored
```

The workspace contains the root `rustsht` and `python` packages; `crates/rustsht-fft` is not a workspace member. Python tests were not run because this increment changes neither bindings nor Python behavior.
