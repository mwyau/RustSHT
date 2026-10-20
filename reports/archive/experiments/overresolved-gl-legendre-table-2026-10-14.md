# Over-resolved GL Legendre strategy follow-up

> Follow-up: the original T127 analysis ceiling described below was extended to T159 for measured thread-count regions. The additional process-paired comparison, numerical validation and retained-memory decision are recorded in [the T159 analysis report](t159-overresolved-analysis-2026-10-14.md). The synthesis policy was not extended by that analysis result.

## Scope and decision

This follow-up measures the compact north-hemisphere table against the current DUCC-derived recurrence on atmospheric over-resolved Gaussian grids. It follows the initial selector audit in [adaptive-legendre-strategy-2026-10-14.md](adaptive-legendre-strategy-2026-10-14.md) and the earlier exact-grid table study in [legendre-precomputation-2026-10-13.md](legendre-precomputation-2026-10-13.md). The source baseline is RustSHT `7a93bdb`; the recurrence arithmetic did not change during this selector experiment. The benchmark-only force control was removed before production changes. The implementation and focused tests are committed on `main` as `d88629dfb5a26f02c6a7df0efbc44a40e3f27d6a`. This report and its raw benchmark data are committed separately. The measurements justify a bounded, deterministic extension of the internal selector. Tables may cover GL grids from the minimum exact latitude count through twice that count, but only within the existing 34 MiB payload cap. Over-resolved automatic use requires full `mmax`; scalar and spin-1 STANDARD analysis are limited to T127. Scalar T63 analysis uses recurrence at q4 and above; q1/q2 use the table when eligible. Scalar synthesis uses a table on over-resolved grids only with a multi-thread pool. The exact-grid policy remains operation- and executing-pool-specific. Spin-1 synthesis remains on its separately measured exact-grid path. This is lazy and per operation; there is no public tuning API, call-count autotuner, global state, or change to transform semantics. The selector change does not claim a universal table advantage. First-use setup can dominate tens to hundreds of short transforms, and the T191 analysis table regresses even after setup. A caller-specific reuse estimate is not available in the immutable plan API, so the deterministic selector targets repeat execution only in the measured regions and the cold cost is reported explicitly.

## Algorithm and implementation audit

`ScalarLegendreTable` stores only northern latitude lanes in triangular `(m,l)` order. The south side is reconstructed using scalar degree parity. At an odd latitude count, the equatorial ring is stored once and its odd-parity values are zero. Table construction still uses the scaled scalar recurrence and stops at the existing mirrored-ring polar cutoff. Spin 1 reuses scalar bases with a one-degree extension; spin-1 STANDARD analysis initializes its table through `OnceLock`, while GRAD_ONLY, DERIV1, other grids and f32 retain their existing paths. Scalar plans also initialize an eligible table lazily on first selected use. The table is immutable; each call retains caller-owned workspace. Current table layout eligibility is f64-supported precision, GL, `63 <= lmax <= 255` (with the existing spin-1 degree extension), `lmax+1 <= ntheta <= 2*(lmax+1)`, near-full truncation, valid longitude sampling, and at most 34 MiB of table payload. The automatic over-resolved selector is narrower than layout eligibility: full `mmax`, analysis through T127, and scalar synthesis only in a multi-thread pool. Spin-1 analysis applies the same over-resolved limit. A grid beyond twice minimum sampling, such as T63 `129x129`, is rejected before table construction. The payload limit still rejects over-resolved T255 (`512` latitudes). The initial research compared the policy with SHTns source revision `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`. `sht_gauss` and `sht_gauss_fly` choose grid families, `sht_quick_init` avoids setup search, and `choose_best_sht` benchmarks operation-specific on-the-fly variants during initialization when that source enables the search. Operation-specific deterministic choices and numerical checks transfer well. Hidden calibration tied to plan construction and its thread pool does not: RustSHT plans are immutable, execution pools can differ at each call, and the strategy under study also trades persistent memory and first-use construction against reuse. The historical `sht_bench` repository was read at `1328f83feb5dc95b3e6708b51228d954820ff15a`; its atmospheric GL dimensions are not the minimum exact grids used by the earlier table report.

## Measurement method

The host was an x86-64 CPU (host topology omitted), AVX2/FMA, without AVX-512, on Linux. Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0`, LLVM `23.1.1`. Release builds used Cargo's normal optimized profile, runtime Pulp dispatch, and no `target-cpu=native` or LTO. The same native release harness, compiler settings, deterministic coefficient formulas, f64 STANDARD operation, `phi0=0.37`, reused plan and workspace were used for both strategies. The recurrence and table runs were process-isolated and alternated in three paired repetitions. Each process warmed once and recorded five timed transforms. The summary below uses the median of process-level medians; raw per-call samples remain in [`reports/data/overresolved-gl-legendre-table-2026-10-14/`](../../data/overresolved-gl-legendre-table-2026-10-14). The eight-thread process affinity was 8 pinned CPU cores and one-thread runs to one CPU core. The host was shared; changes near the observed timing spread are marked as inconclusive rather than accepted. The main command form was:

```sh
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release --example m1b_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench <spin> <lmax> <threads> <synthesis|analysis> GL STANDARD <ntheta> <nphi> <mmax> requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench <spin> <lmax> <threads> <synthesis|analysis> GL STANDARD <ntheta> <nphi> <mmax> requested-only
```


## Paired over-resolved warm results

Cells show `recurrence/table` milliseconds. Every row used three paired processes, each with one warmup and five measured calls. `q1` and `q8` denote the executing Rayon pool. “Minimum” grids use the exact analysis sampling; “atmospheric” dimensions are the documented larger GL dimensions. All values are full `mmax` unless the row says otherwise.

| GL grid (`ntheta x nphi`) | Operation | q1 recurrence/table (ms) | q8 recurrence/table (ms) |
| --- | --- | ---: | ---: |
| T63 minimum `64x128` | scalar synthesis | `0.106 / 0.110` | `0.075 / 0.052` |
|  | scalar analysis | `0.123 / 0.091` | `0.064 / 0.066` |
|  | spin-1 synthesis | `0.169 / 0.306` | `0.115 / 0.163` |
|  | spin-1 analysis | `0.232 / 0.199` | `0.110 / 0.096` |
| T63 atmospheric `96x192` | scalar synthesis | `0.161 / 0.167` | `0.103 / 0.080` |
|  | scalar analysis | `0.178 / 0.145` | `0.115 / 0.105` |
|  | spin-1 synthesis | `0.262 / 0.476` | `0.150 / 0.207` |
|  | spin-1 analysis | `0.334 / 0.283` | `0.187 / 0.170` |
| T72 minimum `73x147` | scalar synthesis | `0.159 / 0.168` | `0.101 / 0.079` |
|  | scalar analysis | `0.169 / 0.128` | `0.096 / 0.088` |
|  | spin-1 synthesis | `0.271 / 0.483` | `0.157 / 0.215` |
|  | spin-1 analysis | `0.326 / 0.275` | `0.162 / 0.140` |
| T72 over-resolved `96x150` | scalar synthesis | `0.183 / 0.206` | `0.107 / 0.088` |
|  | scalar analysis | `0.204 / 0.167` | `0.119 / 0.112` |
|  | spin-1 synthesis | `0.301 / 0.590` | `0.171 / 0.241` |
|  | spin-1 analysis | `0.378 / 0.328` | `0.207 / 0.174` |
| T85 minimum `86x173` | scalar synthesis | `0.300 / 0.325` | `0.205 / 0.173` |
|  | scalar analysis | `0.320 / 0.268` | `0.212 / 0.208` |
|  | spin-1 synthesis | `0.541 / 0.873` | `0.350 / 0.432` |
|  | spin-1 analysis | `0.572 / 0.514` | `0.364 / 0.340` |
| T85 atmospheric `128x256` | scalar synthesis | `0.314 / 0.396` | `0.144 / 0.136` |
|  | scalar analysis | `0.324 / 0.287` | `0.197 / 0.169` |
|  | spin-1 synthesis | `0.641 / 1.080` | `0.265 / 0.340` |
|  | spin-1 analysis | `0.624 / 0.528` | `0.300 / 0.250` |

T63 and T85 scalar over-resolved synthesis at q8 ranges are small in absolute time; the T85 q8 advantage is only about `0.008 ms` and its process ranges cross. Spin-1 scalar-conversion synthesis is not a table candidate: its table values regress strongly at these bands. Analysis generally benefits more consistently, but q8 T63 scalar analysis is within noise. Results for a reduced order count also changed the tradeoff: T63 atmospheric `mmax=48` gave scalar-analysis `0.155/0.133 ms` at q1 but was effectively tied at q8; T127 atmospheric `mmax=96` gave scalar-analysis `1.126/1.122 ms` at q1 and `0.557/0.459 ms` at q8. The selector therefore keeps reduced-mmax over-resolved calls on recurrence. Additional full-band atmospheric controls:

| Grid / operation | q1 recurrence/table (ms) | q8 recurrence/table (ms) |
| --- | ---: | ---: |
| T127 `256x512`, scalar synthesis | `1.326 / 1.553` | `0.530 / 0.464` |
| T127, scalar analysis | `1.293 / 1.183` | `0.610 / 0.533` |
| T127, spin-1 synthesis | `4.154 / 4.157` | `1.507 / 1.462` |
| T127, spin-1 analysis | `2.298 / 1.995` | `1.059 / 0.860` |
| T191 `384x768`, scalar analysis | `3.49 / 3.86` | `1.46 / 1.52` |
| T191, spin-1 analysis | `6.10 / 8.81` | `2.37 / 2.53` |
| T191, scalar synthesis | not collected | `1.71 / 1.32` |

The T127 scalar and spin-1 analysis table gains are repeatable at q8; scalar synthesis at q8 is near the noise floor. At T191 both analyses regress with a table and the q8 scalar-synthesis result favors the table. The T191 synthesis gain is not grounds to route analysis through the table. No T191 spin-1 synthesis comparison was collected in this phase. The previous exact-grid q1/q2/q4/q8/q16 measurements for T72–T255 are preserved in the initial strategy report and its raw files. They are historical calibration for exact-minimum grids, not a claim that all those cells were repeated after the current recurrence work. This follow-up adds process-paired q1/q8 measurements for all four operations at T63/T72/T85 minimum and selected atmospheric grids, plus all four operations at T127 atmospheric dimensions. T191 q1/q8 controls cover scalar and spin-1 analysis and scalar synthesis. The q2/q4/q16 sweep covers representative scalar synthesis/analysis and spin-1 analysis at T63/T85/T127/T191. T159/T255 over-resolved grids were not run through the whole operator matrix: the table cap rules out T255 at `ntheta=512`, and T159 was not needed to choose the new q1/q8 over-resolved gates. This explicit coverage boundary prevents interpolation of unmeasured cases. The q2/q4/q16 sweep produced the following paired median table-versus-recurrence changes. Each number is the median percentage change over three process pairs; a negative value favors the table. T63 scalar synthesis improved at q2/q4/q16 by `27.6%/22.8%/26.6%` (all three pairs favored the table). T63 scalar analysis improved `24.1%` at q2 in all pairs, but q4 was `6.5%` slower at the median and q16 was `3.2%` slower; only one of three pairs favored a table at each of those pools. The selector therefore uses recurrence for over-resolved T63 scalar analysis at q4+. T63 spin-1 analysis improved `6.6%` at q4 and `18.9%` at q16 in all pairs; q2 had an outlier and favored the table in two of three pairs. At T85, scalar synthesis improved by `47.7%/31.3%/16.1%` at q2/q4/q16 (all pairs favored the table). Scalar analysis improved `25.3%/7.8%/8.0%`; q16 had one pair slightly slower. Spin-1 analysis improved `18.3%/14.1%/16.5%`, with all pairs favoring the table. At T127, scalar synthesis improved `27.1%/19.8%/25.7%`; q2 and q4 were mixed (two of three pairs favored the table), while q16 was consistent. Scalar analysis improved `19.0%/23.4%/9.6%`, with q2 and q16 mixed and q4 consistent. Spin-1 analysis improved `21.9%/28.2%/10.8%`, with all pairs favoring the table. At T191, scalar synthesis improved `11.6%/36.0%/29.6%`, while scalar analysis regressed `34.5%/6.2%/17.5%` at q2/q4/q16 medians and spin-1 analysis regressed `7.8%/5.3%/13.3%`. T191 q2 scalar-analysis pairs were noisy; q4/q16 all favored recurrence. These data support the T127 analysis ceiling and keep T191 analysis on recurrence. Per-process samples and all pair medians are in `mid-thread-samples.txt` and `mid-thread-medians.csv`.

## Cold cost, memory, and reuse

Plan construction does not build either table. Workspace time is the existing per-plan scratch allocation. A selected table is built on first use, and its cost appears in that transform. Table payload estimates use the triangular north-hemisphere layout; measured process RSS differences also include allocator/workspace variation.

| Grid | Approx. table payload | Measured RSS increase over recurrence |
| --- | ---: | ---: |
| T63 atmospheric `96x192` | `0.79 MiB` | within `0–0.5 MiB` noise |
| T85 atmospheric `128x256` | `1.87 MiB` | `1.5–2.0 MiB` |
| T127 atmospheric `256x512` | `8.19 MiB` | `8.5–9.0 MiB` |
| T191 atmospheric `384x768` | `27.4 MiB` | `27–28 MiB` |
| T255 over-resolved `512x1024` | above `34 MiB` budget | rejected before allocation |

For N repeated transforms, totals below are calculated as `first_call + (N-1)*warm_median`. First-call samples are medians of three cold processes; warm medians are from three paired steady-state processes. Values in each cell are `recurrence/table` milliseconds.

| Workload | 1 | 5 | 10 | 25 | 50 | 100 | 1,000 | Approx. break-even |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T85 atmospheric scalar synthesis q8 | `0.160/2.001` | `0.736/2.545` | `1.456/3.225` | `3.616/5.265` | `7.216/8.665` | `14.416/15.465` | `144.016/137.865` | 232 calls |
| T63 atmospheric spin-1 analysis q1 | `0.350/1.068` | `1.686/2.200` | `3.356/3.615` | `8.366/7.860` | `16.716/14.935` | `33.416/29.085` | `334.016/283.785` | 16 calls |
| T85 atmospheric spin-1 analysis q8 | `0.372/2.282` | `1.572/3.282` | `3.072/4.532` | `7.572/8.282` | `15.072/14.532` | `30.072/27.032` | `300.072/252.032` | 40 calls |
| T127 atmospheric scalar analysis q8 | `0.868/8.937` | `3.308/11.069` | `6.358/13.734` | `15.508/21.729` | `30.758/35.054` | `61.258/61.704` | `610.258/541.404` | 106 calls |
| T191 atmospheric scalar synthesis q8 | `2.319/28.175` | `9.139/33.439` | `17.664/40.019` | `43.239/59.759` | `85.864/92.659` | `171.114/158.459` | `1705.614/1342.859` | 68 calls |
| T191 atmospheric scalar analysis q8 | `1.706/28.265` | `7.526/34.353` | `14.801/41.963` | `36.626/64.793` | `73.001/102.843` | `145.751/178.943` | `1455.251/1548.743` | no break-even |

For a mixed plan workload, one T127 q8 scalar synthesis followed by one scalar analysis is treated as a cycle. The table plan pays construction during the first synthesis and reuses that same table for analysis. The recurrence plan has no table setup. The medians give `0.772 ms` first recurrence synthesis plus `0.913 ms` first recurrence analysis; the table plan uses `8.500 ms` first synthesis plus `0.533 ms` warm analysis. Warm cycles are `1.140 ms` for recurrence and `0.997 ms` for the table. Total cycle times are:

| Cycles | 1 | 5 | 10 | 25 | 50 | 100 | 1,000 | Approx. break-even |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T127 scalar synthesis + analysis q8, recurrence/table (ms) | `1.685/9.033` | `6.245/13.021` | `11.945/18.006` | `29.045/32.961` | `57.545/57.886` | `114.545/107.736` | `1140.545/1005.036` | 53 cycles |

These estimates show why a q8 steady-state win is not automatically a useful default for a short plan lifetime. In particular, the current full-mmax over-resolved scalar-synthesis rule selects a T85 q8 table, but its measured warm advantage is only about `0.008 ms` per call, its paired timing ranges overlap, and the median requires 232 calls to repay first-use overhead. This matches the Sol review's conclusion that the measured favorable `0.009 ms` gain needs roughly 200 reuses. The selector cannot infer future reuse, so this borderline steady-state choice remains a documented low-confidence edge. The small table footprint at T63/T85 makes this a setup-cost decision rather than a memory-capacity problem.

## Numerical validation and controls

The new Python regression compares over-resolved T63 GL `96x192`, f64, `phi0=0.37`, spin-0 and spin-1 STANDARD synthesis, analysis, and both adjoints against installed DUCC 0.41.0. It uses `numpy.testing.assert_allclose` with `rtol=atol=2e-12`. An explicit error measurement on the same arrays found max-absolute / relative-L2 differences of `1.901e-12 / 7.521e-15` for scalar synthesis, `2.831e-14 / 3.264e-15` for scalar analysis, `4.668e-12 / 8.772e-15` for scalar synthesis adjoint, and `7.780e-16 / 3.453e-15` for scalar analysis adjoint. Spin-1 values were `3.403e-12 / 1.305e-14`, `5.688e-14 / 5.291e-15`, `8.479e-12 / 1.945e-14`, and `1.123e-15 / 5.868e-15`, respectively. The native table-versus-recurrence test exercises `S`, `A`, `S*`, and `A*` for T63/T72/T85/T127 over-resolved and exact grids, including an odd-latitude/equatorial grid, rectangular truncation, nonzero phase, and adjoint inner products. Spin-1 analysis is separately compared with the general recurrence and adjoint for GL, CC and F1, including the new T63/T85/T127 atmospheric cases. Existing high-band DUCC, analytic, polar cutoff, extreme finite fallback, and spin-0–4 regression tests remain enabled. The Python test uses allclose assertions; maximum-error statistics above were measured separately on those same inputs. No table-selection threshold changes arithmetic or accuracy tolerance. f32, CC/F1, GRAD_ONLY, DERIV1, and unsupported spin paths remain controls on their existing recurrence/specialized implementations.

## Reviewer findings and disposition

The requested Sol High review was strictly read-only and examined the selector, diff, tests, and measurement evidence. It found no numerical, cutoff/parity, workspace ownership, concurrency, or safety blocker. Its low-severity concern was that payload eligibility alone allowed unmeasured latitude oversampling ratios, and T85 q8 synthesis had a weak warm gain against a large first-call cost. The implementation now limits `ntheta` to at most twice the minimum exact GL count and the report records the T85 setup amortization (about 200 calls under its favorable warm delta). No reviewer files were changed.

## Validation and continuation

Validation completed on the final policy source:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: 151 Rust tests and 10 f32 tests passed; 9 opt-in tests were ignored.
- `cargo test -p rustsht --no-default-features`: 129 Rust tests and 10 f32 tests passed; 7 opt-in tests were ignored.
- `uv run --with maturin maturin develop --release --manifest-path python/Cargo.toml`: passed.
- `uv run pytest python/tests`: 18 passed.
- Focused table-versus-recurrence tests for all four scalar operators, q1/q2/q4/q8/q16 selector boundaries, spin-1 GL/CC/F1 analysis and adjoints, and over-resolved Python DUCC comparisons: passed. The full high-band opt-in suite was not rerun for this selector-only change; existing release T511/T1023/T2047 scalar/spin checks remain documented in their individual reports. New benchmark artifacts are limited to the reproducible raw samples and medians in the linked data directory; temporary selector hooks and experimental binaries are not part of the repository. The next strategy milestone should use the same paired methodology for the remaining selected exact-grid cases and for expected-reuse sensitivity, without adding a calibration API. After this selector milestone, continue the existing spin-0/1 performance objective; FFT optimization remains deferred.
