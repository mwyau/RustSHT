# Interleaved f32 spin normalization showed no T511 whole-transform gain

The interleaved plus/minus recurrence-normalization loop did not improve complete f32 spin-1 transforms at T511. In the five-pair screen, one-thread analysis changed by -0.72% and eight-thread analysis by -0.21%; both paired ranges crossed zero. Summed synthesis-plus-analysis time increased by 0.39% at q1 and 0.98% at q8. I removed the kernel change. The `m4_f32_bench` example now records each of its five inner timings as well as the median, so later screens retain the raw samples.

## Method

The benchmark measured spin-1 synthesis and analysis on GL `512x1025`, f32, T511, full `mmax`, and `phi0=0.37`, using a reusable plan and workspace. Each process warmed each operation once, measured five calls, and reported all five times plus their median. Five baseline/candidate process pairs ran in alternating order, pinned to CPUs 0–7, under `target/rustsht-benchmark.lock`.

The baseline was commit `ea22382b4d84a664b6a7436a1ed07bc6833a5470`. The candidate interleaved normalization of the plus and minus recurrence lanes by latitude and used the same scalar binary-scaling rules. Both binaries used the instrumented benchmark example saved in `measurement-harness.patch`; their SHA-256 hashes are in `build.txt`. The host was a shared AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`, using Rust 1.99.0, LLVM 23.1.1, and portable Pulp runtime dispatch. Load averages before and after the screen were 2.60/4.73/5.34 and 6.48/4.97/5.29.

## Results

Times are medians of five per-process call medians. Paired change is the median candidate/baseline percentage across five process pairs; ranges show the full paired spread. Negative values favor the candidate. Combined values sum the synthesis and analysis medians from each process.

| Operation | Baseline (ms) | Candidate (ms) | Paired change | Paired range |
| --- | ---: | ---: | ---: | ---: |
| Synthesis q1 | 1028.812 | 1043.025 | +1.42% | -0.21% to +1.77% |
| Synthesis q8 | 151.906 | 155.762 | +1.09% | -7.95% to +11.68% |
| Analysis q1 | 857.531 | 863.071 | -0.72% | -1.17% to +0.88% |
| Analysis q8 | 161.932 | 162.423 | -0.21% | -4.55% to +1.40% |
| Synthesis + analysis q1 | 1887.762 | 1895.639 | +0.39% | -0.20% to +1.17% |
| Synthesis + analysis q8 | 316.398 | 317.350 | +0.98% | -6.27% to +7.33% |

The q8 raw calls include outliers above 200 ms in both builds; the per-call samples and process RSS records are in the data directory. A preliminary five-pair screen used the older median-only benchmark binary and also showed no q1 improvement. Those logs are retained with a separate summary but excluded from the table. The change adds no workspace storage. It passed the focused bitwise comparison against the two-pass scalar normalizer and the release T511 DUCC analysis check. The full workspace suite was not run with the rejected candidate.

`cargo fmt --all -- --check`, workspace Clippy with warnings denied, and release builds of both benchmark binaries passed after the raw-sample output change. The benchmark did not include a DUCC wall-time comparison.

The source candidate, measurement-harness diff, raw inner samples, preliminary screen, process timings, environment snapshots, and paired summary are in [`reports/data/f32-spin1-normalize-interleave-2026-10-15`](data/f32-spin1-normalize-interleave-2026-10-15/).
