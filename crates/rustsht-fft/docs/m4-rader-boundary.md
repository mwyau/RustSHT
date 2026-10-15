# Direct real boundary for Rader passes

Date: 2026-10-15. The baseline is M3 commit
`64f60e81410b0b884fe5ece6b1fefdb11d2aebbe`. The candidate removes the
per-group complex input and output staging around Rader passes in the real
R2C and C2R paths at N=257 and N=723. Candidate binary SHA256:
`ff92ab25ab31d4a0f9612c3256aac9f75a4cb86bc1368bd5271669e5b4cf021d`. Baseline binary SHA256:
`d61ce536311469f0ccd7f4888bd35367db36d9f45d8da645f65a409e46f00b95`.

## Method

The real forward pass gathers its prime-radix samples into the existing Rader
convolution workspace and writes the packed nonnegative spectrum from that
workspace. The real inverse gathers the Hermitian spectrum there and scatters
the real samples directly to the output. Both directions use the existing
convolution and its normalization. The inverse applies its outer real-transform
scale once. No plan or workspace buffers were added. The six-pair benchmark measures public f64 R2C and C2R operations at N=241,
257, and 723. N=241 is the control; N=257 and N=723 are Rader targets. Each
length uses ordinary, sparse, cancellation-heavy, and broad-dynamic-range
inputs, with one row and 32 sequential rows. The sparse forward input has one
nonzero sample; the sparse inverse spectrum has three nonzero bins. The
cancellation inputs alternate signs and include `1e-12` perturbations. The
broad-dynamic inputs place a `1e-300` term among ordinary values. Inputs are
constructed before timing. Each public operation reuses its plan and
workspace, and each process reports the median of seven timed samples after a
warm-up. The one-row and 32-row timings are normalized per transform. Six process pairs alternate candidate and baseline order. The host was an x86-64 CPU, with each process pinned to logical one CPU core. Host-load variation was reviewed locally; exact transient load measurements remain untracked. The compiler was rustc 1.99.0 (`b940084d7 2026-09-28`) using the optimized
`cargo bench` profile, without custom `RUSTFLAGS` or `target-cpu` settings. The M3 baseline used the same benchmark harness as the candidate. Each cell's displayed timing is the median of its six process medians. Its
speedup is the median of the six paired ratios, `M3 time / M4 time`; values
above 1 favor M4. The cell interval is a 95% percentile bootstrap interval over
those six paired ratios. The primary statistic first takes the median target
speedup within each process pair, then the median of those six pair-level
values. Its interval resamples the six pair-level values. Both intervals use
5,000 deterministic bootstrap samples.

## Predeclared gate and result

The gate was recorded before the six-pair run. The median of the six pair-level
primary speedups must exceed 1, its 95% interval must be above 1, no target,
control, or fallback cell may have a median slowdown above 5%, and workspace
bytes must match. Route and numerical checks must pass. The 24 target cells have a primary median speedup of `1.0479`, with a 95%
interval of `[1.0332, 1.0585]`. The largest target-cell median slowdown is
0.69%; the largest control or fallback slowdown is 0.90%. Workspace byte
counts match. The gate passes. One target cell, C2R at N=723 with 32 rows and
ordinary input, is 0.69% slower by its median; the other 23 target-cell medians
favor M4. The table reports median public-operation time in microseconds as M3 → M4. Ratios and intervals use paired process results.

| N | Direction | Input | Rows | Time (µs) | Speedup [95% interval] |
|---:|---|---|---:|---:|---:|
| 257 | R2C | ordinary | 1 | 2.340 → 2.174 | 1.067 [1.038, 1.105] |
| 257 | R2C | ordinary | 32 | 2.296 → 2.129 | 1.077 [1.055, 1.104] |
| 257 | R2C | sparse | 1 | 2.279 → 2.104 | 1.081 [1.039, 1.115] |
| 257 | R2C | sparse | 32 | 2.245 → 2.074 | 1.084 [1.048, 1.112] |
| 257 | R2C | cancellation | 1 | 2.325 → 2.149 | 1.075 [1.050, 1.103] |
| 257 | R2C | cancellation | 32 | 2.288 → 2.120 | 1.073 [1.050, 1.107] |
| 723 | R2C | ordinary | 1 | 11.367 → 11.016 | 1.032 [1.019, 1.047] |
| 723 | R2C | ordinary | 32 | 11.346 → 11.001 | 1.032 [1.015, 1.049] |
| 723 | R2C | sparse | 1 | 11.171 → 10.845 | 1.031 [1.016, 1.046] |
| 723 | R2C | sparse | 32 | 11.175 → 10.828 | 1.033 [1.021, 1.050] |
| 723 | R2C | cancellation | 1 | 11.316 → 11.025 | 1.031 [1.018, 1.041] |
| 723 | R2C | cancellation | 32 | 11.344 → 10.999 | 1.033 [1.019, 1.044] |
| 257 | C2R | ordinary | 1 | 2.565 → 2.415 | 1.056 [1.047, 1.085] |
| 257 | C2R | ordinary | 32 | 2.536 → 2.377 | 1.068 [1.052, 1.087] |
| 257 | C2R | sparse | 1 | 2.455 → 2.284 | 1.070 [1.039, 1.092] |
| 257 | C2R | sparse | 32 | 2.405 → 2.270 | 1.061 [1.046, 1.079] |
| 257 | C2R | cancellation | 1 | 2.555 → 2.399 | 1.069 [1.037, 1.086] |
| 257 | C2R | cancellation | 32 | 2.550 → 2.393 | 1.068 [1.049, 1.076] |
| 723 | C2R | ordinary | 1 | 12.022 → 11.852 | 1.011 [0.925, 1.022] |
| 723 | C2R | ordinary | 32 | 12.046 → 12.129 | 0.993 [0.990, 1.007] |
| 723 | C2R | sparse | 1 | 11.672 → 11.422 | 1.020 [0.978, 1.026] |
| 723 | C2R | sparse | 32 | 11.643 → 11.456 | 1.017 [1.013, 1.022] |
| 723 | C2R | cancellation | 1 | 12.008 → 11.762 | 1.018 [0.978, 1.023] |
| 723 | C2R | cancellation | 32 | 12.020 → 12.003 | 1.000 [0.986, 1.017] |

Across the 16 N=241 control cells, median ratios range from 1.001 to 1.012. Across the eight broad-dynamic fallback cells at N=257 and N=723, median
ratios range from 0.991 to 1.016. The largest fallback slowdown is 0.90%. All 24 control and fallback cells satisfy the 5% limit. Their individual
medians, paired ranges, and intervals are in the six-pair summary CSV.

## Route and numerical checks

Route diagnostics cover both directions at N=257 and N=723. Ordinary, sparse,
and cancellation inputs select Rader in all 12 cases. Broad-dynamic inputs
select the existing Bluestein fallback in all four cases. The fallback trace
checks C2C convolution lengths 525 at N=257 and 484 at N=723. N=241 provides
control timing but is not included in that route count. The focused integration test compares complete R2C and C2R outputs against
independent direct DFT calculations and RustFFT 6.4.1 at N=241, 257, and 723. It covers ordinary, sparse, cancellation-heavy, and broad-dynamic inputs in
both directions, plus finite-extreme, subnormal, and nonfinite fallback
behavior. The added route test checks the selected plan path for all 16
N=257/723 direction and input-class combinations. Review R39 found no
indexing, Hermitian reconstruction, normalization, bounds, or workspace
aliasing defect; it requested explicit inverse sparse and cancellation
references, which are now tested against both references.

## Setup and first-call measurements

Plan and workspace construction are outside the timed public operation. The
following setup medians pool the input classes, directions, and row counts;
workspace bytes are unchanged.

| N | Plan M3 → M4 (ns) | Workspace M3 → M4 (ns) | Workspace bytes |
|---:|---:|---:|---:|
| 241 | 18,786 → 18,893 | 836 → 831 | 35,120 |
| 257 | 29,194 → 29,247 | 1,045 → 1,040 | 46,032 |
| 723 | 37,534 → 37,704 | 1,098 → 1,101 | 56,576 |

A separate six-pair ordinary-input run measured the first operation after plan,
workspace, and input construction, then the median of seven warm calls. The
table gives baseline → candidate medians in microseconds and paired speedup
intervals. These are one-row measurements and were not part of the throughput
gate.

| N | Direction | First call (µs) | First speedup [95% interval] | Warm call (µs) | Warm speedup [95% interval] |
|---:|---|---:|---:|---:|---:|
| 241 | R2C | 16.907 → 16.551 | 1.087 [0.679, 1.305] | 12.805 → 11.081 | 1.050 [0.803, 1.267] |
| 241 | C2R | 13.356 → 12.043 | 1.025 [0.811, 1.472] | 11.597 → 11.212 | 1.020 [0.724, 1.169] |
| 257 | R2C | 3.101 → 3.457 | 0.888 [0.709, 1.043] | 2.525 → 2.250 | 1.121 [0.811, 1.300] |
| 257 | C2R | 3.021 → 3.577 | 0.845 [0.623, 0.870] | 2.726 → 2.525 | 1.076 [0.764, 1.110] |
| 723 | R2C | 12.995 → 13.030 | 0.991 [0.807, 1.023] | 11.943 → 11.146 | 1.051 [0.840, 1.092] |
| 723 | C2R | 12.855 → 13.164 | 0.982 [0.878, 1.062] | 12.444 → 12.694 | 0.969 [0.759, 1.041] |

The first-call N=257 C2R median favors M3, while its warm median favors M4. The first-call interval lies below 1; most other first-call and all warm
intervals cross 1. These one-row measurements do not establish a general
first-use effect. The N=257 C2R result should be revisited if first-call
latency is a target workload.

## Recorded data

- [`m4-rader-boundary-six-pairs.jsonl`](../benches/results/m4-rader-boundary-six-pairs.jsonl): six alternating process pairs, raw timing output, executable hashes, and environment records.
- [`m4-rader-boundary-six-pairs-summary.csv`](../benches/results/m4-rader-boundary-six-pairs-summary.csv): per-cell medians, paired ratios, intervals, setup values, and workspace sizes.
- [`m4-rader-boundary-acceptance.csv`](../benches/results/m4-rader-boundary-acceptance.csv): predeclared aggregate acceptance calculation.
- [`m4-rader-boundary-route-diagnostics.txt`](../benches/results/m4-rader-boundary-route-diagnostics.txt): runtime route trace for the four input classes.
- [`m4-rader-boundary-screen.jsonl`](../benches/results/m4-rader-boundary-screen.jsonl): exploratory one-pair screen; it is not used for acceptance.
- [`m4-rader-boundary-first-call-six-pairs.jsonl`](../benches/results/m4-rader-boundary-first-call-six-pairs.jsonl) and [`m4-rader-boundary-first-call-summary.csv`](../benches/results/m4-rader-boundary-first-call-summary.csv): separate first-call and warm-call observations.
- [`summarize_rader_boundary.py`](../benches/summarize_rader_boundary.py): deterministic primary-gate calculation from the per-cell summary.
