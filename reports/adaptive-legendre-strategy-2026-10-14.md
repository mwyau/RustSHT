# Adaptive Legendre strategy investigation

## Initial audit and experiment plan

This plan was recorded at RustSHT `bf47cd1` before adding benchmark-only strategy controls or changing production selection. The investigation compares the current DUCC-derived recurrence with RustSHT's compact, north-hemisphere `ScalarLegendreTable`; it does not introduce a dense Torch-harmonics-style representation.

### Current implementation

`ScalarLegendreTable::build_if_beneficial` currently accepts only supported precision, exact GL geometry (`ntheta = lmax + 1`), `lmax >= 72`, near-full truncation (`mmax >= lmax - lmax/4`), and a payload no larger than 34 MiB. It stores contiguous northern latitude lanes for triangular `(m,l)` support, reflects southern values by parity, and rejects unsafe table values. The degree extension is `lmax` for spin 0 and `lmax+1` for spin 1. The spin-0 ceiling is T255; spin 1 is eligible through the degree-extended T255 table. Scalar spin-0 plans build an eligible table eagerly in `ShtPlan::new`, even if a later call uses only one operator or recurrence would be preferable. Synthesis and analysis separately check input safety and cutoffs before accepting the table path. Spin-1 plans begin without a table; eligible STANDARD GL analysis and a restricted single-thread STANDARD synthesis initialize the same table lazily through `OnceLock`. Spin-1 GRAD_ONLY/DERIV1, other grids, f32, and unsupported table values use recurrence. The current execution thread count is available only at transform time; the plan and workspace remain separate and reusable. The last table measurements predate the paired spin-1 analysis at `1102dbe` and mixed normal/scaled scalar synthesis at `bf47cd1`. Their thresholds are therefore hypotheses, not current policy evidence. The committed table-versus-recurrence results and Torch-harmonics storage comparison are in [the precomputation report](legendre-precomputation-2026-10-13.md).

### External algorithm-selection references

The read-only `sht_bench` checkout was `1328f83feb5dc95b3e6708b51228d954820ff15a`. Its [README](https://github.com/mwyau/sht_bench/blob/1328f83feb5dc95b3e6708b51228d954820ff15a/README.md), [matrix specification](https://github.com/mwyau/sht_bench/blob/1328f83feb5dc95b3e6708b51228d954820ff15a/docs/MATRIX.md), [backend setup](https://github.com/mwyau/sht_bench/blob/1328f83feb5dc95b3e6708b51228d954820ff15a/src/sht_bench/backends.py), and published GL/CC result plots describe per-process thread-count runs, setup outside steady transform timing, and exact grid metadata. Its full atmospheric GL grid is `2F x 4F`: e.g. T63 is `96x192`, T85 is `128x256`, and T159 is `160x320`. Its CC points use `nlat=2L+1`, `nlon=4L`; these differ from RustSHT's minimum exact grids and must not be merged as matched samples. The plotted reference is useful for CPU backend context, not a numerical comparison or a recurrence/table selector benchmark. SHTns source revision `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e` maps `sht_auto` to its Gaussian grid; `sht_gauss_fly` also selects the Gaussian grid, while `sht_quick_init` avoids its timing search. `choose_best_sht` measures available on-the-fly kernel variants during grid initialization and selects function pointers separately by operation type, with an internal accuracy check. In this source, `on_the_fly_only` is enabled, so this selector is not a direct precedent for choosing a precomputed Legendre matrix. Useful transfer: keep operation-specific choices and validate accuracy. Inappropriate transfer: hidden plan-time benchmarking tied to construction-time thread count, long calibration, and implicit mutable global tuning. RustSHT should use a deterministic policy unless measurements show that a small explicit calibration is worth its total cost.

### Measurement plan

1. Add temporary, benchmark-only controls to force recurrence or the currently eligible compact table. Keep them out of the normal public Rust/Python API and remove them unless they prove useful as a private reproducibility facility.
2. Compare both strategies from the same source and release configuration on exact matching f64 GL grids, using scalar and spin-1 STANDARD synthesis and analysis. Start with T72/T127/T191/T255, which span current table gates; add T63/T85 and full atmospheric dimensions as relevant controls. Exercise full and reduced `mmax`, exact and even/odd longitude cases, and the existing minimum-grid versus atmospheric-grid definitions.
3. Run thread counts 1, 2, 4, 8, and 16 in separate, CPU-pinned processes. Retain raw per-call samples. Measure plan construction, table build/first use, workspace creation, warm transforms, Python plan methods, and peak RSS separately. Include GRAD_ONLY, DERIV1, CC/F1, and f32 as recurrence controls where table eligibility is intentionally absent.
4. Compare outputs with DUCC 0.41.0 and preserve independent analytic, adjoint, extreme-input, equatorial, cutoff, and high-band tests. Timings alone do not establish numerical validity.
5. Calculate cold-to-warm total time for 1, 5, 10, 25, 50, 100, and 1,000 transforms, and state break-even reuse separately for synthesis, analysis, and mixed use. Keep each table under the existing 34 MiB cap.
6. Change production policy only if paired measurements on the latest recurrence show a repeatable whole-operation benefit after first-call setup, memory, and actual execution thread count are included. Do not infer reuse count or pin a plan to its construction-time thread count. If the static policy remains best, keep it and report that result. No production selection or numerical code changed for this initial plan.

## Results and selected policy

Measurements completed 2026-10-13 on the `main` branch. The source baseline was `7fba08c` (`Plan adaptive Legendre strategy measurements`), following `bf47cd1` (mixed normal/scaled scalar synthesis) and `1102dbe` (mirrored-ring spin-1 GL analysis). The accepted implementation is commit `db3d230d186388d64cc94bd6584571b0864322ff` on `main`; report/data changes are committed separately. No other branch or repository was used. DUCC reference data use installed `ducc0==0.41.0`, with source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The compact table remains useful, but the previous eligibility thresholds treated plan construction and operation/thread differences too coarsely. The selected internal policy makes scalar table construction lazy, evaluates the executing Rayon pool at transform time, and chooses recurrence for measured regressions. The table remains f64-only, exact GL only, full/near-full truncation, `72 <= lmax <= 255`, with the existing 34 MiB per-table limit. Spin-1 uses the existing degree-extended form and never exceeds that limit. No table is built when the selected operation uses recurrence or when the input fails the existing finite/range guard. There is no public strategy API and no hidden timing calibration. The selector is deterministic and operation-specific:

- Scalar synthesis uses recurrence for one-thread `lmax < 127`. T72 and T85 tables are slower at one thread; T86 was effectively tied and its first-call table cost was not repaid in ordinary short reuse. Other eligible scalar synthesis uses the table.
- Scalar analysis uses recurrence for one-thread T255 with `mmax >= 192`, and for 16-or-more threads at full-mmax T255. The T255 analysis table is useful at two through eight threads in the measured cases; the one-thread loss and the weak/noisy 16-thread result do not justify a 32 MiB allocation.
- Spin-1 STANDARD analysis uses the table at one thread for `85 <= lmax <= 224`. T72 one-thread results did not show a repeatable benefit after mirrored-ring recurrence was added. For multithread pools it remains table-eligible except full-mmax T255 at eight or more threads, where the table has no stable advantage. Reduced-mmax T255 analysis retains the table where measurements show gains.
- The existing spin-1 synthesis specialization remains table-backed only in its measured single-thread T127–T191 range. Other thread counts and bands use recurrence.
- f32, CC, F1, over-resolved GL grids, GRAD_ONLY, DERIV1, and spins other than 0/1 use their existing recurrence or specialized paths. A caller-specific reuse count is not available through the immutable reusable-plan API. Adding an implicit counter, global calibration, or a public tuning option was not justified for this milestone. Auto therefore optimizes measured steady-state cases for reusable plans and initializes a table only on first eligible use. The cold/warm tables below make the first-call tradeoff explicit; a caller that performs fewer than the listed break-even calls can be slower than recurrence. This limitation is documented rather than hidden.

### Native recurrence/table timing matrix

All values are f64, exact-minimum GL, full `mmax`, `phi0=0.37`, reused native plan/workspace, one warm-up and five timed calls per process, with three process repetitions. Each cell is `recurrence / pre-selector Auto candidate` in milliseconds, with Auto using a table only where its then-current eligibility selected one. `q1` and `q8` are separately executed Rayon pools. Raw per-process rows and timed samples are in [`strategy-comparison-t72-t255.csv`](data/adaptive-legendre-2026-10-14/strategy-comparison-t72-t255.csv) and [`t159-comparison.csv`](data/adaptive-legendre-2026-10-14/t159-comparison.csv). The selector uses these current-source comparisons; it does not assume that every Auto row represents a table. Grid dimensions are `T72 73x147`, `T85 86x173`, `T127 128x257`, `T159 160x321`, `T191 192x385`, and `T255 256x513` (`ntheta x nphi`).

| Operation | Band | q1 recurrence / Auto | q8 recurrence / Auto |
| --- | ---: | ---: | ---: |
| Scalar synthesis | T72 | 0.174 / 0.216 | 0.126 / 0.094 |
|  | T85 | 0.332 / 0.407 | 0.265 / 0.210 |
|  | T127 | 0.967 / 0.911 | 0.434 / 0.350 |
|  | T159 | 1.985 / 1.742 | 0.936 / 0.648 |
|  | T191 | 3.947 / 2.513 | 1.018 / 0.651 |
|  | T255 | 7.870 / 6.033 | 2.410 / 1.656 |
| Scalar analysis | T72 | 0.218 / 0.146 | 0.115 / 0.093 |
|  | T85 | 0.339 / 0.266 | 0.263 / 0.227 |
|  | T127 | 0.776 / 0.626 | 0.477 / 0.362 |
|  | T159 | 1.533 / 1.326 | 0.852 / 0.673 |
|  | T191 | 1.985 / 1.633 | 0.763 / 0.571 |
|  | T255 | 4.180 / 4.364 | 1.689 / 1.452 |
| Spin-1 synthesis | T72 | 0.326 / 0.348 | 0.262 / 0.233 |
|  | T85 | 0.740 / 0.667 | 0.483 / 0.466 |
|  | T127 | 2.954 / 2.520 | 1.014 / 0.986 |
|  | T159 | 5.263 / 5.001 | 2.224 / 2.210 |
|  | T191 | 9.956 / 7.488 | 2.848 / 2.830 |
|  | T255 | 12.268 / 12.270 | 4.653 / 4.663 |
| Spin-1 analysis | T72 | 0.334 / 0.382 | 0.206 / 0.178 |
|  | T85 | 0.574 / 0.510 | 0.421 / 0.396 |
|  | T127 | 1.371 / 1.114 | 0.812 / 0.564 |
|  | T159 | 2.685 / 2.282 | 1.500 / 1.310 |
|  | T191 | 3.427 / 2.955 | 1.335 / 1.059 |
|  | T255 | 7.261 / 7.972 | 2.950 / 2.919 |

The strongest repeatable warm gains are scalar synthesis at T127–T255 and spin-1 analysis at T127–T191. The T255 spin-1 one-thread table loses by about 9–12%; the eight-thread full-mmax result is within noise and later paired confirmation favored recurrence slightly. The T255 spin-1 table is still useful at q4 (five-process confirmation: table 3.653 ms, range 3.57–3.83; recurrence 3.982 ms, range 3.903–4.053), but its 32.4 MiB payload and roughly 32 ms extra first-use time require about 100 repeated analyses to amortize. At q8 the confirmation did not reproduce a table win; at q16 it was neutral. The q8+ full-mmax gate is therefore recurrence. For full-mmax scalar T255 analysis, q1 recurrence was 4.180 ms versus table 4.364 ms. q16 table was slower in two of three paired process repetitions and effectively tied in one; the final policy selects recurrence. q8 table remains faster warm (1.452 versus 1.689 ms), but its cold setup requires well over 100 repeated operations to repay. Table thresholds are deliberately bounded by the existing memory budget and observed operation/thread behavior; high-band synthesis speed alone did not expand eligibility.

### Higher bands, grid controls, modes, and truncation

Tables are ineligible above T255. The high-band recurrence screening measured q1/q8 medians below; the source CSV also retains q2/q4/q16 samples for every listed operation and band.

| Band | Scalar S q1/q8 | Scalar A q1/q8 | Spin-1 S q1/q8 | Spin-1 A q1/q8 |
| --- | ---: | ---: | ---: | ---: |
| T319 | 14.061 / 4.717 | 7.461 / 2.912 | 20.623 / 9.294 | 12.555 / 5.205 |
| T360 | 17.388 / 4.779 | 10.074 / 4.143 | 24.546 / 9.389 | 17.860 / 6.746 |
| T511 | 43.370 / 12.539 | 23.626 / 8.013 | 58.875 / 21.161 | 38.821 / 12.761 |
| T720 | 104.353 / 34.244 | 63.134 / 20.050 | 134.689 / 52.698 | 102.646 / 34.468 |
| T1023 | 337.832 / 113.404 | 179.413 / 63.982 | 434.244 / 160.339 | 304.481 / 111.311 |

These are recurrence latency screens, not DUCC parity claims. They confirm that the table policy does not affect high-band memory or numerical paths. The control matrix includes atmospheric GL T63 `96x192` and T85 `128x256`, exact-minimum GL, CC and F1 T127, spin-1 GRAD_ONLY, and scalar DERIV1, at q1/q8. The table requires exact GL `ntheta=lmax+1`, so the full atmospheric T63/T85 grids are recurrence-only. This matters for applications using over-resolved Gaussian grids: the table gains above apply only when the user's GL latitude count exactly matches the Legendre degree count. CC/F1 remain on their exact resampling paths. The raw control samples and exact dimensions are in [`grid-mode-controls.csv`](data/adaptive-legendre-2026-10-14/grid-mode-controls.csv). Reduced-truncation sweeps cover T127 `mmax={0,63,95,96,127}` and T255 `mmax={0,127,191,192,255}` at q1/q8, plus a spin-1 T255 analysis sweep at `mmax={192,208,224,240,255}` and q1/q2/q4/q8/q16. The scalar-table gate begins at `mmax >= lmax - floor(lmax/4)` (T127 threshold 96; T255 threshold 192). Below that threshold, work savings and table access vary enough that the recurrence is retained. Spin-1 reduced-mmax T255 q8 analysis shows a measured table gain, unlike full-mmax q8. Raw data are in [`truncation-comparison.csv`](data/adaptive-legendre-2026-10-14/truncation-comparison.csv) and [`spin1-t255-truncation-threads.csv`](data/adaptive-legendre-2026-10-14/spin1-t255-truncation-threads.csv).

### Cold cost, reuse break-even, and memory

The scalar table is now lazy. Eligible plans construct in roughly `0.02 ms` rather than paying table setup at `ShtPlan::new`; recurrence-only operations retain only the plan's normal metadata. On first selected use, table construction is included in the transform. The fresh-plan T127 scalar q8 screen measured table first synthesis about `4.50 ms` and recurrence first synthesis about `0.48 ms`; warm synthesis was `0.349` versus `0.430 ms`. For analysis, first calls were `4.45` versus `0.52 ms`; warm calls were `0.342` versus `0.482 ms`. These imply approximately 50 repeated synthesis calls and 29 analysis calls to break even at T127 q8. The separate [`lazy-setup.csv`](data/adaptive-legendre-2026-10-14/lazy-setup.csv) preserves all three fresh-plan process repetitions. The scalar T72/T85 boundary screen records the q1 T85 table regression and near-tied T86 result; the q1 selector waits until T127 to use scalar synthesis tables because the T86 cold-start cost is not supported by a repeatable warm gain. The following table shows total operation time for N calls, including the first-call setup, in milliseconds. It uses measured fresh-plan medians and illustrates two workloads rather than claiming one reuse count for every band. `Rec` is recurrence-first-call plus `(N-1)` recurrence-warm calls; `Table` is table-first-call plus `(N-1)` table-warm calls.

| Workload | 1 | 5 | 10 | 25 | 50 | 100 | 1,000 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T127 scalar S q8 Rec | 0.477 | 2.197 | 4.347 | 10.797 | 21.547 | 43.047 | 430.047 |
| T127 scalar S q8 Table | 4.499 | 5.895 | 7.640 | 12.875 | 21.600 | 39.050 | 353.150 |
| T127 scalar A q8 Rec | 0.519 | 2.447 | 4.857 | 12.087 | 24.137 | 48.237 | 482.037 |
| T127 scalar A q8 Table | 4.499 | 5.867 | 7.577 | 12.707 | 21.257 | 38.357 | 346.157 |
| T255 spin-1 A q4 Rec | 3.878 | 19.806 | 39.716 | 99.446 | 198.996 | 398.096 | 3,981.896 |
| T255 spin-1 A q4 Table | 36.465 | 50.917 | 68.982 | 123.177 | 213.502 | 394.152 | 3,645.852 |

Break-even reuse counts are about 50 for T127 scalar synthesis q8, 29 for T127 scalar analysis q8, and around 100 for the confirmed T255 spin-1 analysis q4 comparison. Table payloads are 0.763 MiB at T72, 1.227 MiB at T85, 4.031 MiB at T127, 7.862 MiB at T159, 13.57 MiB at T191, and 32.13 MiB at scalar T255; the spin-1 degree extension uses 32.38 MiB at T255. The per-table 34 MiB cap is unchanged. With lazy initialization, these bytes are retained only after the plan actually takes an eligible table path. RSS rows, plan/workspace times, first calls and warm calls are preserved in the raw files.

### Python, DUCC accuracy, and benchmark limits

The rebuilt Python extension and installed DUCC 0.41.0 were compared at T127 GL `128x257`, f64, nonzero `phi0`, scalar and spin-1 STANDARD operations. The two-process Python plan run was noisy at eight threads and is not used to set thresholds. At one thread, the Python output comparison reported:

| Operation | Max absolute error | RMS error | Relative-L2 error |
| --- | ---: | ---: | ---: |
| Scalar synthesis | `5.412e-16` | `3.170e-17` | `1.070e-14` |
| Scalar analysis | `6.101e-15` | `2.333e-16` | `1.114e-14` |
| Spin-1 synthesis | `1.757e-15` | `1.323e-16` | `4.344e-14` |
| Spin-1 analysis | `6.999e-15` | `2.202e-16` | `1.052e-14` |


```sh
cargo build --release --example m1b_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 127 1 synthesis GL STANDARD 128 257 127 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 255 8 analysis GL STANDARD 256 513 255 requested-only
```

The forced strategy matrix was collected with the temporary benchmark-only `RUSTSHT_LEGENDRE_STRATEGY=auto|recurrence` control in the same release benchmark harness. That control was removed after capture; no public API, environment variable, Cargo feature, or hidden runtime override remains in the shipped source. The measurement commands used `taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}"` for q1 or `taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}"<threads-1>` for q2/q4/q8/q16, followed by `RUSTSHT_LEGENDRE_STRATEGY=<auto|recurrence> target/release/examples/m1b_bench <spin> <lmax> <threads> <synthesis|analysis> GL STANDARD <ntheta> <nphi> <mmax> requested-only`. Raw measured samples, not only summary medians, are committed under [`data/adaptive-legendre-2026-10-14/`](data/adaptive-legendre-2026-10-14/).

### Research, review, acceptance, and remaining limits

The read-only `sht_bench` reference was `1328f83feb5dc95b3e6708b51228d954820ff15a`; full atmospheric GL dimensions differ from the exact-minimum benchmark grids and remain table-ineligible. The read-only SHTns source was `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`. Its automatic path times on-the-fly kernel variants at initialization and selects callbacks by operation. This supports operation-specific strategy selection, but plan-time hidden calibration tied to construction threads is a poor fit for RustSHT's immutable reusable plans and execution-time Rayon pools. No SHTns code was copied. Sol High performed a strictly read-only review after the selector architecture and threshold measurements. It found no numerical or memory-safety blocker. It identified that earlier comparison tests could route both sides through recurrence after the first policy change and that a cold spin-1 analysis benchmark reused a plan already initialized by synthesis. The tests now assert the selected path in an explicit Rayon pool, and the cold analysis harness uses a fresh plan/workspace. The review also led to the T85/T86 and spin-1 T191/T192/T224/T254/T255 boundary screens. Final spin-1 T255 q4/q8/q16 confirmation informed the q8+ full-mmax recurrence gate. Accepted changes are lazy table construction, actual-pool-aware deterministic strategy selection, conservative recurrence exceptions at measured small/high-band boundaries, and tests for cross-pool use and fallback behavior. Rejected choices are a larger table budget, dense or blocked new basis representations, plan-time calibration, and selecting a T255 spin-1 table at q8+ full mmax. No FFT backend, grid, numerical convention, public API, tolerance, or memory cap changed. Checks on the final code before the report commit:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: 141 Rust unit tests and 10 f32 integration tests passed; 8 opt-in high-band tests are marked ignored by the default suite.
- `cargo test -p rustsht --no-default-features`: 126 Rust unit tests and 10 f32 integration tests passed; 7 opt-in high-band tests are marked ignored by that suite.
- `uv run --with maturin maturin develop --release --manifest-path python/Cargo.toml`: passed.
- `uv run pytest python/tests`: 16 passed.
- Focused selector, spin-1 policy-boundary and concurrent plan/table tests: passed in release mode. No numerical tolerances changed. Three opt-in release regressions were also run successfully: scalar T511/T1023/T2047 across GL/CC/F1 with DUCC and closed-form comparisons, spin T511 across all grids, and spin T1023 exact analysis. The selector sends these bands to recurrence. Other high-spin T1023/T2047 sparse tests remain unrun in this milestone.
