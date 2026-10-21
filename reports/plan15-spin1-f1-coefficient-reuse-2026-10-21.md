# Plan 15: F1 q8 synthesis coefficient reuse

**Source A/B:** RustSHT baseline
`0b2e1336409ba0bbe9fd6fa4cff70db0e327cd52`; candidate
`7f47f06f875991a982caf0f4c396323a32c0d72b`. The candidate change is now in
production on `opt/sht-performance-parity`. Comparison DUCC is pinned to
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).

## Change and mechanism

On q8 T720 F1 synthesis, the existing ring partition produces 12 groups of
64/approximately 64 rings. Before this change, each group converted the same
E/B coefficient orders to scalar channels and rechecked the converted values
for finiteness. The coefficient transform is independent of latitude group.
The candidate extends the existing checked workspace cache to the already
selected high-work F1 group route: prepare each order once, retain its
per-order finite flag, then reuse the converted rows in all groups. The 17 MiB
cache limit, lmax bound, finite checks, conversion safety and general scaled
fallback remain unchanged. This is not a recurrence or FFT change.

Source-derived work counts at T720 (not hardware performance counters):

| Order range | Coefficient degree positions transformed: old → cached | Converted values scanned for finiteness: old → cached |
|---|---:|---:|
| Full, `mmax=720` | 3,123,372 → 260,281 | 12,493,488 → 1,041,124 |
| Reduced, `mmax=180` | 1,370,532 → 114,211 | 3,136,368 → 261,364 |

These counts assume the 12 ring groups used in the measured T720 q8 F1
geometry. The cache occupies about 15.9 MiB full-order and 4.0 MiB at
`mmax=180`; its vector storage is caller-workspace-owned and reused.

## Matched q8 results

Warm latency is the whole native spin-1 synthesis call, with plans/workspaces
reused and output reset semantics matched. Each backend had three interleaved
process runs and ten calibrated batches per process, targeting 25 ms. Ratios
below use the median of the three paired process medians. Absolute table
latencies are pooled batch medians; process-pair ratios are retained to show
the run-to-run spread.

| Case | Rust baseline ms (p10–p90) | Rust candidate ms (p10–p90) | DUCC ms (p10–p90) | Candidate/baseline paired ratios | Baseline → candidate Rust/DUCC paired median | Pooled-median excess vs DUCC |
|---|---:|---:|---:|---:|---:|---:|
| F1 T720 full, q8 | 48.373 (43.856–51.250) | 42.752 (39.161–48.395) | 9.442 (8.344–10.835) | 0.886, 0.896, 0.844; median **0.886** | 5.034× → 4.458× | 38.93 → 33.31 ms |
| F1 T720 `mmax=180`, q8 | 27.345 (19.021–31.032) | 23.744 (20.496–29.299) | 4.193 (3.684–5.488) | 0.761, 0.979, 1.001; median **0.979** | 6.47× → 4.92× | 23.15 → 19.55 ms |

The full-order candidate was faster in all three process pairs, saving 5.62 ms
at the aggregate medians (11.4% by the paired ratio median). Host runqueue
wait was elevated: medians were 12.8% baseline, 17.4% candidate and 19.4%
DUCC for q8 synthesis, with thousands of involuntary context switches for
the Rust runs. Since the candidate had more wait yet was faster in every
pair, the direction is credible, while its exact size remains host-sensitive.

Reduced-order pooled medians are lower for the candidate, but the paired
ratios span no change to a 24% improvement. Treat that result as
**inconclusive**, not as a second validated gain. The q8 analysis controls
showed no repeatable change (full paired candidate/baseline ratios 0.926,
1.064, 0.992; reduced 0.841, 0.915, 1.069). Cache preparation is only used
for synthesis; these controls quantify the noise floor.

The q1 cache-ineligible controls had multimodal process timings and crossed
candidate/baseline ratios. They do not support a q1 effect. First-call data
also do not show a cold-start improvement: for full-order q8 synthesis, the
three baseline first calls were 48.37–50.55 ms and candidate first calls
46.82–58.45 ms. Setup medians were 21.18 ms baseline and 18.35 ms candidate;
setup is outside warm-call latency and is reported separately.

## Numerical and regression checks

Across all q1/q8 full/reduced analysis/synthesis checks, candidate and
baseline outputs were bitwise identical (`max abs difference = 0`). All
outputs were finite and repeated-call/output-reset checks passed. Maximum
relative L2 error versus DUCC was `5.23e-13` for full-order synthesis and
`8.09e-13` for reduced-order synthesis. The source change also extends the
existing F1 workspace-reuse test to poisoned output buffers and changed E
coefficients. Full release tests passed with all features (248 unit tests,
plus integration suites); no-default-features release tests passed (201 unit
tests, plus integration suites). Formatting and strict Clippy passed.

## Reproduction and data

The sanitized machine-readable tables and reusable benchmark/summarizer
scripts are in
[`reports/data/plan15-spin1-f1-coefficient-reuse-2026-10-21/`](data/plan15-spin1-f1-coefficient-reuse-2026-10-21/README.md).
That directory records toolchains, compiler flags, CPU affinity, source and
binary hashes, fixture hashes, process-pair medians, batch distributions,
first-call/setup timing and summarized host observations. Raw per-process
captures remain local because they include host telemetry.

**Next Plan 15 step:** test whether the same grid-independent coefficient
reuse addresses CC q8 synthesis, where the current source still prepares
spin-1 scalar coefficients per ring group. Measure an exact T720 CC case
against an unchanged baseline before adjusting eligibility; retain the
current cache safety limits and use numerical/reset checks. Then return to
the remaining matched CC/F1 synthesis recurrence attribution.
