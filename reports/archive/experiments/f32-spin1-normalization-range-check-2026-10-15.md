# f32 spin recurrence normalization range-check screen


The prototype was rejected after one five-pair alternating screen. Every paired result was slower: synthesis increased by 66–72% at q1/q8, and analysis increased by 67–82%. The candidate source and its focused tests were removed; only the profile, raw process data, and rejection record remain.

## Profile


The profile recorded 9.446 billion instruction references. `analyze_spin_order_group` accounted for 99.11% inclusive references. `normalize_pair_lanes` accounted for 2.594 billion references (27.46%); the two `recurrence_step` call paths accounted for 1.803 billion (19.09%). The accumulator dispatch had 3.898 billion exclusive references (41.26%). These are instruction counts, not cycles or wall-time fractions. Aggregate instruction counts are retained above; raw profile output remains local and untracked.

## Prototype and timing

The baseline binary used commit `82605f6`; the candidate applied [`the saved source patch`](../../data/f32-spin1-normalization-simd-2026-10-15/candidate.patch). Both binaries came from `examples/m4_f32_bench.rs` and used the same release compiler and runtime CPU dispatch. Five alternating baseline/candidate process pairs ran under the benchmark lock. Each process warmed once and collected five timed calls for q1/q8 spin-1 synthesis and analysis, using GL `512x1025`, T511, full `mmax`, and `phi0=0.37`; processes used affinity sized for eight workers. Times are medians across five process medians. Paired change is the median candidate/baseline percentage across pairs; ranges show all paired changes. Load-average telemetry was retained only in local diagnostics. Median process peak RSS was 14,688 KiB for the baseline and 14,540 KiB for the candidate; estimated core numeric arrays remained 12.13 MiB. Because every transform/thread cell regressed substantially in all five pairs, I did not repeat the screen. The focused normalization and recurrence tests passed with the prototype present, including scalar/runtime recurrence agreement across spin 1–4, multiple latitude blocks, polar lanes, and tails. The T511 f32 DUCC coefficient check also passed in debug mode. No candidate code or candidate-specific tests remain in the source tree. The accepted source and its full validation remain at commit `82605f6`. Paired timing summaries, binary hashes, the runner, parser, and source patch are retained; raw process and environment captures remain local and untracked.
