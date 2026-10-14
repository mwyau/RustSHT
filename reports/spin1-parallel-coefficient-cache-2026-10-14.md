# Parallel spin-1 coefficient preparation

## Decision

Keep a bounded, lazy workspace cache for the two scalar coefficient streams used by the spin-1 STANDARD synthesis conversion path. It is selected only for f64 GL transforms with `127 <= lmax <= 720`, at least eight executing Rayon threads, an active latitude-group split, and a cache payload no larger than 17 MiB. The workspace allocation is deferred until the first eligible synthesis. Allocation failure or any ineligible case uses the existing per-group conversion. The implementation is commit `c6b238d`, based on `14358f1` (`Record latitude group scheduling experiment`). It does not change the public API, numerical recurrence, FFT, CC/F1, analysis, spin-2, GRAD_ONLY or DERIV1 paths. The previously rejected serial cache remains rejected; this candidate prepares independent orders in parallel and then reuses each prepared row across latitude groups.

## Mechanism and selection


The layout is a rectangular order-major scratch array with two complex rows per order. It uses `2 * (mmax + 1) * (lmax + 2)` complex values. The buffer belongs to `ShtWorkspace`, is empty when the workspace is created, and is rebuilt for every transform because callers may change coefficients between calls. It is never shared across workspaces. Checked length arithmetic, `try_reserve_exact`, and the size gate preserve the recurrence fallback on overflow, allocation failure, or excessive storage. The deterministic selector checks the actual Rayon pool during execution. It requires f64 paired-scalar support, spin 1, both STANDARD coefficient channels, non-derivative synthesis, GL geometry, the scalar-conversion fast path, `lmax` from 127 through 720, at least eight workers, and multiple latitude groups. The byte limit is 17 MiB. T1023 was excluded even when a reduced `mmax` would fit the byte limit because the measured T1023 half-truncation case did not show a repeatable gain. No calibration, global state or public strategy control was added.

## Benchmark method

- Baseline source: `14358f1`; candidate source: `c6b238d`.
- Workload: `examples/m1b_bench.rs`, native Rust f64 spin-1 STANDARD synthesis, GL, `phi0=0.37`, reused plans and workspaces, deterministic coefficients, one first-call measurement and five warm timed samples per process.
- Exact full-band grids: T127 `128x257`, T255 `256x513`, T511 `512x1025`, T720 `721x1443`, T1023 `1024x2049` (`ntheta x nphi`). Reduced-order controls retained the same grid and changed only `mmax`.
- Warm process order alternated between baseline and candidate. Each value below is the m1b process median; paired changes compare process `i` with process `i` from the other revision. Raw process medians are listed, and the benchmark command prints all five timed samples.
- Hardware: x86-64 CPU, x86-64, Linux; Rust `1.99.0 (b940084d7 2026-09-28)`, LLVM 23.1.1, Cargo release `opt-level=3`, 16 codegen units, LTO off, no `target-cpu=native`. Pulp selected AVX2/FMA. Runs used `taskset` on 8 pinned CPU cores for q8, one CPU core for q1, 4 pinned CPU cores for q4 and 16 pinned CPU cores for q16. The host had other users and variable CPU frequency; paired process medians and ranges are reported rather than treating small changes as exact.
- Reproduce a run with `cargo build --release --example m1b_bench`, then `taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 720 8 synthesis GL STANDARD 721 1443 720 requested-only`. The baseline executable was built from `14358f1` with the same Rust 1.99 release setup. These are reused-workspace whole-transform measurements. DUCC timing was not repeated for this cache-only change. Numerical comparison uses the existing DUCC 0.41.0 regression data from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

## Warm timing results

Times are milliseconds. `Base` and `Cache` columns list the raw process medians in run order. The paired median is the median percent reduction `(base - cache) / base`; positive values favor the cache.

| Workload | Base process medians | Cache process medians | Median paired reduction | Unpaired median change |
| --- | --- | --- | ---: | ---: |
| T127, q8, full mmax | `1.340, 0.904, 0.891, 1.220, 0.930` | `0.830, 0.856, 0.816, 0.815, 0.829` | `10.9%` | `0.930 -> 0.829 ms` |
| T255, q8, full mmax | `4.442, 6.048, 5.069, 4.923, 5.545` | `4.095, 4.139, 4.166, 4.052, 4.064` | `17.8%` | `5.069 -> 4.095 ms` |
| T511, q8, full mmax, eight pairs | `20.531, 23.027, 20.835, 21.044, 23.671, 20.104, 20.954, 20.212` | `18.824, 20.898, 20.777, 20.736, 20.278, 20.078, 18.540, 18.349` | `8.8%` | `20.895 -> 20.178 ms` |
| T511, q8, `mmax=255` | `19.691, 19.713, 20.142, 21.813, 20.255` | `18.513, 16.841, 17.683, 18.453, 18.574` | `12.2%` | `20.142 -> 18.453 ms` |
| T720, q8, full mmax | `45.791, 51.346, 53.120, 52.461, 51.140` | `46.441, 44.773, 47.184, 47.270, 44.722` | `11.2%` | `51.346 -> 46.441 ms` |

At T511, all eight candidate process medians were no slower than their paired baseline; three pairs were close to timing noise, while the others improved by 8–14%. At T720, four of five pairs improved by 9–13%; one pair was 1.4% slower. T127 and T255 warm gains were consistent, but their absolute savings are smaller. The final-source T511 q8 confirmation used three more pairs: baseline `22.848, 23.225, 23.157 ms`, candidate `18.387, 18.540, 20.813 ms`. The final-source T720 q8 confirmation used three pairs: baseline `51.461, 51.933, 47.004 ms`, candidate `47.008, 45.016, 45.420 ms`. These confirmations reproduce the direction and show the expected shared-host spread. The five within-process warm samples for these final-source confirmation pairs are retained here as raw timings:

| Band | Pair | Base samples (ms) | Cache samples (ms) |
| --- | ---: | --- | --- |
| T511 | 1 | `20.003, 22.846, 22.741, 19.877, 20.104` | `20.275, 20.078, 18.369, 20.233, 18.270` |
| T511 | 2 | `20.015, 20.954, 21.391, 22.981, 20.431` | `15.955, 20.191, 18.540, 18.071, 18.757` |
| T511 | 3 | `19.935, 20.212, 22.878, 20.147, 20.522` | `18.852, 18.349, 21.169, 18.300, 18.251` |
| T720 | 1 | `45.626, 51.553, 45.791, 51.697, 45.378` | `47.656, 47.482, 45.448, 46.441, 44.055` |
| T720 | 2 | `52.827, 51.346, 51.763, 48.496, 46.843` | `46.420, 47.391, 44.773, 44.616, 44.308` |
| T720 | 3 | `53.120, 52.100, 48.451, 53.439, 54.500` | `45.300, 50.203, 47.273, 47.184, 45.359` |

## Thread-count and truncation controls

The candidate leaves q1 on the existing path. Three paired T511 q1 process medians were baseline `57.321, 57.938, 57.592 ms` and candidate `57.802, 57.840, 58.510 ms`. At T720 q1 they were `132.846, 132.585, 134.456 ms` and `133.539, 133.018, 133.068 ms`. Differences were within measurement variability; no cache was allocated. At T720 q4, three warm process medians were `66.756, 66.789, 66.446 ms` versus `65.674, 65.501, 66.168 ms`, a small 1–2% warm change. Candidate first calls were `93.236, 76.492, 78.825 ms`, versus baseline `72.785, 74.224, 72.316 ms`. The warm gain did not justify retaining nearly 16 MiB at q4, so the selector starts at eight threads. At q16, warm medians were baseline `45.719, 50.683, 45.894 ms` and candidate `44.674, 44.137, 44.610 ms`; two pairs improved by about 2–3%, and one baseline sample was an outlier. q16 remains eligible through the `>=8` rule, with q8 as the strongest demonstrated case. The old 34 MiB cache prototype was also forced on at T1023. Full-mmax q8 baseline medians were `152.376, 159.281, 157.061, 166.627, 167.915 ms`; cache medians were `153.215, 154.795, 154.512, 154.698, 162.831 ms`. Paired warm changes ranged from a small regression to a few-percent gain, within the baseline spread, while the first call was several milliseconds slower and retained 32.03 MiB. With `mmax=511`, baseline medians were `150.277, 153.477, 149.595, 151.241, 151.213 ms`; cache medians were `150.080, 147.999, 150.419, 149.834, 151.216 ms`. This was also inconclusive. T1023 therefore uses recurrence, even when reduced truncation would fit the memory cap.

## Cold cost, memory and repeated-use total

The cache does not change plan construction or workspace allocation; the vector reserves storage only at the first eligible transform. A matched T720 q8 `time -v` run measured plan construction at `0.081/0.086 ms` and workspace allocation at `7.541/7.402 ms` for baseline/cache. Peak RSS was `70,768/87,396 KiB`, a `16,628 KiB` increase. The logical cache payload at T720 is `16,657,984 bytes` (`15.886 MiB`). Across five T720 q8 process runs, the median first-call time was `57.853 ms` for baseline and `59.920 ms` for the candidate. The medians of subsequent warm calls were `51.346` and `46.441 ms`. The following total-time estimate uses those values and makes the first-use cost explicit:

| Repeated transforms | Recurrence total (ms) | Cache total (ms) |
| ---: | ---: | ---: |
| 1 | `57.853` | `59.920` |
| 5 | `263.237` | `245.684` |
| 10 | `519.967` | `477.889` |
| 25 | `1,290.157` | `1,174.504` |
| 50 | `2,573.827` | `2,335.529` |
| 100 | `5,141.107` | `4,657.579` |
| 1,000 | `51,352.207` | `46,454.479` |

The measured medians cross after the first call: two or more repeated transforms amortize T720 preparation. A separate single-process run showed a larger first-call penalty, so a conservative user-facing expectation is two to three calls. Cache payloads are `0.504 MiB` at T127, `2.008 MiB` at T255, `8.016 MiB` at T511 and `15.886 MiB` at T720 for full mmax. The cap is per workspace; concurrent workspaces multiply retained memory.

## Numerical validation

The focused q8 T127 test compares the cached grouped transform with the uncached transform executed in the same pool. Maximum absolute differences were `5.463e-19` and `9.216e-19` for the two output components. The cache stores the output of the existing coefficient conversion without changing its arithmetic. Release DUCC-reference checks passed:

- `m3_spin_t511_all_grids_matches_ducc_and_roundtrips`: spin 1 and 2 on GL, CC and F1. Spin-1 synthesis maximum absolute sample errors were `4.885e-15`, `4.441e-15` and `4.219e-15`; selected analysis coefficient errors were `4.970e-18`, `4.216e-18` and `4.318e-18`.
- `m3_spin_t1023_sparse_synthesis_matches_ducc`: spin 1 GL/CC/F1 maximum absolute sample errors were `3.893e-15`, `3.150e-15` and `3.567e-15`.
- `m3_spin_t2047_sparse_synthesis_matches_ducc_and_sectoral_formula`: spins 1–4 passed on GL, CC and F1. Spin-1 DUCC middle-row error was `2.137e-13`; the independent analytic map maximum error was `1.344e-11`. The cache selector excludes T2047. All standard adjoint, GRAD_ONLY/DERIV1, f32, CC/F1, and spin-3/4 tests also passed. Full checks: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features` (144 Rust tests and 10 f32 tests passed, 8 ignored); `cargo test -p rustsht --no-default-features` (128 Rust tests and 10 f32 tests passed, 7 ignored). The ignored high-band checks relevant to this path were run separately in release mode. Python tests were not run because this change does not modify the binding.

## Rejected ranges and next work

Do not extend the cache through T1023 based on the small/noisy warm differences: the memory and first-use costs are much larger than the measured gain. Do not enable it for q4: the warm change is small and first-use costs several milliseconds. Preserve the per-workspace cap and the f64/GL/STANDARD gates. The benchmark does not measure a new DUCC wall-time ratio; it establishes an internal RustSHT improvement and keeps DUCC 0.41.0 as the numerical reference. The next useful comparison is a fresh q8/q16 spin-1 synthesis profile after this change, followed by the largest remaining spin-0/1 analysis gap. FFT work remains deferred.
