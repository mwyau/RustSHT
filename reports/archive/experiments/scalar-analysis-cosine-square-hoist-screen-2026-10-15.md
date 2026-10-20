# Reusing cosine squares did not improve q8 consistently

The candidate computed `cos(theta) * cos(theta)` once for each active
latitude block and reused those exact f64 values in the fused scalar analysis
recurrence. This removes one vector multiply per lane from each degree update. It used a 64-value stack temporary and added no workspace storage; the
recurrence and accumulation operations were otherwise unchanged.

## Paired source comparison

Five alternating release-process pairs compared the candidate with commit
`387ef37` using `examples/m1b_bench.rs`. Each process ran one warmup and five
timed f64 scalar GL STANDARD analysis calls, full `mmax`, at T511
(`512 x 1025`) and T1023 (`1024 x 2049`). q1 used one CPU core and q8 used 8 pinned CPU cores. An additional five q8 pairs repeated both sizes with the same affinities. Rust was `1.99.0`, LLVM `23.1.1`, with portable Pulp runtime dispatch (AVX2/FMA
on this x86-64 CPU host). No CPU quota applied. Release builds did not
set `target-cpu=native`. Negative paired changes favor the candidate. Each change is the percentage
difference within one baseline/candidate process pair. The median and range
summarize the five initial pairs; the q8 summary also reports the repeat and
the combined ten pairs.

| Grid | Threads | Baseline median | Candidate median | Paired change median (range) | Candidate faster |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | q1 | 18.856 ms | 18.597 ms | -1.04% (-3.45% to -0.54%) | 5/5 |
| T511 | q8 initial | 3.504 ms | 3.772 ms | +6.10% (-1.25% to +26.48%) | 1/5 |
| T1023 | q1 | 148.527 ms | 146.096 ms | -1.86% (-2.48% to -0.91%) | 5/5 |
| T1023 | q8 initial | 25.244 ms | 26.255 ms | +4.00% (-0.97% to +30.24%) | 1/5 |

The q1 measurements favored the candidate by a small amount in all five
pairs. The initial q8 runs favored the baseline at both sizes, but one slow
candidate process made the ranges wide. In the additional five q8 pairs,
T511 had a `-5.11%` median change (all five faster); T1023 had `-1.22%` (three
of five faster). Across all ten q8 pairs, the median paired change was
`-1.78%` at T511, with a `-21.31%` to `+26.48%` range, and `+0.45%` at T1023,
with a `-17.39%` to `+30.24%` range. The q8 screens disagree too much to
establish an improvement. The candidate was removed. The complete medians and paired ranges are in
[`paired-results.tsv`](../../data/scalar-analysis-cosine-square-hoist-screen-2026-10-15/paired-results.tsv)
and [`q8-results.tsv`](../../data/scalar-analysis-cosine-square-hoist-screen-2026-10-15/q8-results.tsv). Numerical samples, summarized RSS and run scripts are in the [benchmark data directory](../../data/scalar-analysis-cosine-square-hoist-screen-2026-10-15); its [paired-process provenance](../../data/scalar-analysis-cosine-square-hoist-screen-2026-10-15/paired-process-provenance.md) retains the binary hashes. Raw affinity and load captures remain local and untracked.

## Validation

The three focused fused scalar analysis block tests passed, including the
paired-reference and range-failure cases. Formatting passed. Full workspace
suites and Clippy were not run for this rejected candidate. The change did
not alter recurrence arithmetic or use new workspace storage.
