# f32 spin recurrence normalization range-check screen

A q1 Callgrind profile of f32 GL T511 spin-1 STANDARD analysis attributed 27.46% of instruction references to lane normalization and 19.09% to recurrence updates. I tested a SIMD range check that would skip the scalar normalizer when every pair of recurrence lanes remained within the established scaling band. The prototype used Pulp reductions and kept the existing scalar normalizer for mixed or exceptional lanes.

The prototype was rejected after one five-pair alternating screen. Every paired result was slower: synthesis increased by 66–72% at q1/q8, and analysis increased by 67–82%. The candidate source and its focused tests were removed; only the profile, raw process data, and rejection record remain.

## Profile

Callgrind 3.26.0 measured the release test `f32_spin1_gl_t511_analysis_matches_ducc_0410` at q1, pinned to CPU 0, with `RAYON_NUM_THREADS=1`. The profiled workload used GL `512x1025`, T511, f32, spin 1, full `mmax`, and the accepted source at commit `82605f6`. The test includes plan/workspace and map setup plus one analysis transform. The host was an AMD Ryzen 9 5950X running Linux `7.0.14-20-pve`, with Rust `1.99.0`, LLVM `23.1.1`, and Pulp runtime AVX2/FMA dispatch.

The profile recorded 9.446 billion instruction references. `analyze_spin_order_group` accounted for 99.11% inclusive references. `normalize_pair_lanes` accounted for 2.594 billion references (27.46%); the two `recurrence_step` call paths accounted for 1.803 billion (19.09%). The accumulator dispatch had 3.898 billion exclusive references (41.26%). These are instruction counts, not cycles or wall-time fractions. The raw profile and function counts are in [`the profile data`](data/f32-spin1-analysis-profile-2026-10-15/).

## Prototype and timing

The baseline binary used commit `82605f6`; the candidate applied [`the saved source patch`](data/f32-spin1-normalization-simd-2026-10-15/candidate.patch). Both binaries came from `examples/m4_f32_bench.rs` and used the same release compiler and runtime CPU dispatch. Five alternating baseline/candidate process pairs ran under the benchmark lock. Each process warmed once and collected five timed calls for q1/q8 spin-1 synthesis and analysis, using GL `512x1025`, T511, full `mmax`, and `phi0=0.37`; processes were pinned to CPUs 0–7.

Times are medians across five process medians. Paired change is the median candidate/baseline percentage across pairs; ranges show all paired changes. Positive changes mean the candidate was slower.

| Operation | Baseline (ms) | Candidate (ms) | Paired change | Paired range |
| --- | ---: | ---: | ---: | ---: |
| Spin-1 synthesis q1 | 1026.758 | 1763.327 | +71.60% | +70.06% to +75.60% |
| Spin-1 synthesis q8 | 151.012 | 251.695 | +66.73% | +66.06% to +88.10% |
| Spin-1 analysis q1 | 871.221 | 1593.806 | +82.30% | +81.99% to +86.53% |
| Spin-1 analysis q8 | 163.523 | 272.108 | +66.53% | +65.53% to +66.62% |
| Synthesis + analysis q1 | 1896.919 | 3358.491 | +76.83% | +75.51% to +80.61% |
| Synthesis + analysis q8 | 315.043 | 524.017 | +66.66% | +65.78% to +76.80% |

The host load averages were 1.65/2.62/3.56 before and 1.08/2.10/3.13 after the screen. Median process peak RSS was 14,688 KiB for the baseline and 14,540 KiB for the candidate; estimated core numeric arrays remained 12.13 MiB. Because every transform/thread cell regressed substantially in all five pairs, I did not repeat the screen.

The focused normalization and recurrence tests passed with the prototype present, including scalar/runtime recurrence agreement across spin 1–4, multiple latitude blocks, polar lanes, and tails. The T511 f32 DUCC coefficient check also passed in debug mode. No candidate code or candidate-specific tests remain in the source tree. The accepted source and its full validation remain at commit `82605f6`.

Raw pair logs, process RSS records, binary hashes, environment details, runner, parser, and source patch are in [`the screen data`](data/f32-spin1-normalization-simd-2026-10-15/).
