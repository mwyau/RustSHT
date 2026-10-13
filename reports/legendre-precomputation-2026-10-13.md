# Legendre precomputation and recurrence at T72–T720

Date: 2026-10-13. Branch: `main`. The recurrence baseline is commit `7c7e963884add727a02b0a76e297f9fc04681271`. The bounded scalar GL table was added in `d251af6`; its T72 gate expansion is `d7f3dea`. DUCC comparisons use `ducc0==0.41.0` and source revision [`c1ed01b8fda69d2a378190f0915f1a19762dddbf`](https://github.com/mreineck/ducc/tree/c1ed01b8fda69d2a378190f0915f1a19762dddbf).

## Findings

The precomputed Legendre method is useful for repeated scalar GL transforms from T72 through T255. RustSHT stores only the triangular harmonic support on the northern hemisphere, uses contiguous latitude lanes, and caps the plan-owned table at 34 MiB. At T72, the 0.762 MiB table reduced native synthesis by 12%/33% and analysis by 42%/41% at one/eight threads; its 0.616 ms setup increment repays after about two paired S/A iterations at eight threads. At T127 and T255, synthesis gains range from 37–58% and analysis gains from 24–38%. Plan construction increases from about 0.02 ms to 2.4 ms at T127 and 33 ms at T255, so plan reuse determines whether the execution savings repay setup. Torch-harmonics PR #322 stores complete order-degree-latitude tensors. Its folded CC/F1 projection is assembled in bounded blocks, but the final cached tensor is still full-sized. With identical full GL dimensions, the Rust layout uses one quarter of a dense scalar table's float64 payload. The memory advantage is an exact shape calculation; Torch-harmonics was not installed here, so no direct Torch execution timing was measured. Temporary compact-table runs at T360, T511 and T720 reduced synthesis time. Analysis gains were smaller or reversed at eight threads. At T720 the table occupied 716.9 MiB, added about 679 ms of plan construction, improved one-thread synthesis by 25%, and slowed eight-thread analysis by 37%. The production code therefore uses recurrence above T255. The precomputed method does not apply to spin 1, f32, CC or F1. The table improves RustSHT relative to its recurrence baseline but does not establish general DUCC parity. At T72, Python-plan scalar synthesis/analysis measured `0.348/0.301 ms` versus DUCC `0.268/0.344 ms` at one thread; at T127 the respective timings were `0.921/0.629 ms` versus `0.728/0.670 ms`. At T255 they were `6.304/4.446 ms` versus `1.915/1.329 ms`. Reused native workspaces are faster than Python-plan methods, which allocate a workspace on each call. T720 scalar and spin-1 operations continue to use recurrence and remain several times slower than DUCC in these Python-plan measurements.

## Software and benchmark conditions

The CPU was an x86-64 CPU, host topology omitted, with AVX2 and FMA and no AVX-512. The host ran Linux. Processes were 8 pinned CPU cores. Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0`, LLVM `23.1.1`; C++ was GCC `15.2.0`. Rust release used `opt-level=3`, the default 16 codegen units, no LTO, no `target-cpu=native`, and Pulp runtime dispatch. All timing values are milliseconds. Native measurements through T720 used `examples/m1b_bench.rs` with f64, full `mmax`, GL dimensions `(lmax+1) × (2*lmax+3)`, `phi0=0.37`, one warm-up and five timed samples per process. The harness reports separate one-thread and eight-thread Rayon executions with reused native plans and workspaces. Three process runs were collected for T127/T255 and the temporary T360/T511/T720 table screen. Those recurrence baselines came from `7c7e963`; production-table data came from `d251af6` before report-only edits. The T72 gate expansion was measured separately in five alternating baseline/candidate process pairs with 25 timed samples per process. Its recurrence baseline was `e2a8bd0`; the candidate changed the minimum table bandlimit and added the T72 regression checks. The paired methodology and conditions are detailed below. Python comparisons used `examples/python_perf_baseline.py`, Python `3.11.15`, NumPy `2.4.6`, the rebuilt release Rust extension and installed `ducc0==0.41.0`. Inputs, `phi0`, precision, grid and thread counts matched. Each run group warmed the operation and timed five calls; two or three run groups were collected. The benchmark's reusable Python plan still allocates a Rust workspace on every method call. Procedural functions also construct a plan on every call. These results are not native reused-workspace timings. Representative commands (repeat each invocation in separate processes to obtain process medians) are:

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo run --release --example m1b_bench -- 0 255 1 synthesis GL STANDARD
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo run --release --example m1b_bench -- 0 255 8 analysis GL STANDARD
RAYON_NUM_THREADS=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 255 0 1 GL 2 plan both both STANDARD
RAYON_NUM_THREADS=8 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 255 1 8 GL 2 plan both both STANDARD
```

For other cases, substitute the bandlimit, spin, thread count, and `GL`, `CC` or `F1`. Native `m1b_bench` takes spin, bandlimit, thread count, operation, grid and mode. The Python harness takes bandlimit, spin, thread count, grid, process-run count, scope, operation, backend and mode. `/usr/bin/time -v` was used for peak-RSS measurements. The local DUCC source build came from the pinned source revision. The source CMake defaults select C++17, `-O3 -ffast-math -march=native`, interprocedural optimization and stripped symbols. The built extension reported version `0.41.1`; the installed wheel reported `0.41.0` and its wheel build flags were not recorded. At T511 and T720, source-build and wheel timings were similar. The source build measured:

| Grid | Spin | Synthesis 1T / 8T | Analysis 1T / 8T |
| --- | ---: | ---: | ---: |
| GL T511 | 0 | `10.127 / 3.091` | `6.599 / 1.153` |
| GL T511 | 1 | `20.608 / 4.966` | `12.822 / 1.942` |
| GL T720 | 0 | `23.931 / 6.584` | `15.076 / 2.299` |
| GL T720 | 1 | `48.967 / 9.133` | `31.293 / 4.465` |

These runs used two process medians of five calls and were 8 pinned CPU cores. T127 source-build eight-thread measurements varied sharply between processes and are not used for a ratio claim. The source build is a compiler-configuration check; the installed DUCC wheel remains the primary Python comparison. The machine's RTX 5070 has 12,227 MiB of memory, but neither PyTorch nor torch-harmonics was installed. No GPU table construction, device memory measurement or Torch execution benchmark was performed.

## Torch-harmonics storage and Rust table layout

The Torch-harmonics implementation inspected for PR [#322](https://github.com/NVIDIA/torch-harmonics/pull/322) uses `_precompute_legpoly` for standard GL transforms and `_precompute_folded_projection` for extended CC/F1 analysis. The PR was open at the inspected head `37751d3e90b8f12fe9439c52847f4c5d70d5a627`, based on `97f7d2756d2d2d3b974d8ee31ca04e032fdf1d05`. Both tables use order-degree-latitude storage and retain rectangular spectral support with unsupported triangular entries masked to zero. The folded projection combines native quadrature, complementary-grid quadrature, interpolation and its adjoint into the analysis weights; it does not replace the synthesis recurrence. Its order/degree blocks bound the real basis construction input to roughly 8 MiB, but the returned `(mmax,lmax,nlat)` projection remains full-sized. Vector projection data has two rows. The source uses a two-entry cache, so bounded construction blocks do not bound all persistent host or module-buffer memory. These layouts target batched tensor contractions, including GPU execution. The byte counts below compare table payloads only; they exclude Fourier data, FFT workspaces, framework caches, model activations and other live tensors. At the matching full GL dimensions `(721,721,721)`, the Torch scalar table is 2.79 GiB and the two-row vector table is 5.59 GiB. A second same-sized cached projection can double those host-cache payloads; device copies and actual peak GPU residency depend on which modules and cache entries are alive. This is consistent with a multi-gigabyte device-memory cost, but this machine had no PyTorch installation, so no direct Torch allocation, OOM reproduction or runtime comparison was measured. For full triangular support and a GL grid with `ntheta=lmax+1`, Rust's table payload is

```text
8 * ceil(ntheta / 2) * sum(m=0..lmax, lmax - m + 1) bytes
```

The dense scalar Torch payload is `8*(lmax+1)^3` bytes; the vector payload is twice that. The table sizes are:

| Bandlimit | Rust compact table | Torch scalar table | Torch two-row vector table |
| --- | ---: | ---: | ---: |
| T72 | 0.762 MiB (eligible) | 2.97 MiB | 5.93 MiB |
| T127 | 4.03 MiB | 16 MiB | 32 MiB |
| T255 | 32.13 MiB | 128 MiB | 256 MiB |
| T360 | 90.23 MiB (temporary) | 358.9 MiB | 717.9 MiB |
| T511 | 256.5 MiB (temporary) | 1 GiB | 2 GiB |
| T720 | 716.87 MiB (temporary) | 2.79 GiB | 5.59 GiB |

Rust stores only `m <= l <= lmax` and northern latitude rows. Within each `(m,l)` row, latitude is contiguous. The reflected southern basis is `(-1)^(l+m)` times the northern value. Scalar synthesis and analysis use the table for GL; `S`, `A`, `S*` and `A*` retain their separate FFT and quadrature conventions. The table uses existing order-major workspace scratch and allocates no second full map or spectrum per transform. The current plan gate requires f64 scalar GL, exact GL sampling, `72 <= lmax <= 255`, `mmax >= ceil(3*lmax/4)`, and a table no larger than 34 MiB. The CC/F1 folded projection in Torch-harmonics combines grid resampling and analysis weights. It is not the same operation as storing a GL Legendre basis, so this experiment did not construct a second Rust CC/F1 table or substitute one for the existing exact resampling path.

## Native recurrence and table results

Each cell gives one-thread / eight-thread medians. Baseline plan construction did not build a table. Candidate plan construction includes the table. Candidate timing ranges are shown in the text rather than hiding setup behind execution throughput.

| Bandlimit | Plan setup, baseline → table | Scalar synthesis, recurrence → table | Scalar analysis, recurrence → table | Table payload |
| --- | ---: | ---: | ---: | ---: |
| T72 | `0.037 → 0.653 ms` | `0.401/0.522 → 0.353/0.352` | `0.462/0.450 → 0.266/0.266` | 0.762 MiB |
| T127 | `0.018 → 2.402 ms` | `1.884/0.542 → 0.962/0.343` | `1.114/0.579 → 0.702/0.347` | 4.03 MiB |
| T255 | `0.017 → 33.238 ms` | `13.579/4.018 → 6.574/1.681` | `6.553/2.244 → 5.011/1.565` | 32.13 MiB |

At T127, three candidate process runs ranged `0.961–1.007 ms` / `0.343–0.358 ms` for synthesis and `0.701–0.710` / `0.336–0.389 ms` for analysis. T255 ranges were `6.248–6.781` / `1.653–1.760 ms` for synthesis and `4.854–5.494` / `1.549–1.647 ms` for analysis. Table setup medians were `2.402 ms` and `33.238 ms`; workspace allocation was about `0.04 ms` and `0.97 ms`. `/usr/bin/time -v` measured 41,496 KiB peak process RSS for the T255 table transform, which contains a 32.13 MiB table payload. The T72 table reduced synthesis by 12%/33% and analysis by 42%/41% at one/eight threads. Its 0.616 ms setup increment amortizes after about 13/4 synthesis calls or 4/4 analysis calls at one/eight threads. A repeated pair of one synthesis and one analysis repays setup after about three one-thread iterations or two eight-thread iterations. The measured setup break-even for one operation is approximately:

| Bandlimit | Synthesis 1T / 8T calls | Analysis 1T / 8T calls |
| --- | ---: | ---: |
| T72 | 13 / 4 | 4 / 4 |
| T127 | 3 / 12 | 6 / 11 |
| T255 | 5 / 15 | 22 / 49 |

These counts divide measured table setup by the difference between the recurrence and table medians. They omit allocation differences and variation between processes. A paired synthesis-plus-analysis workload amortizes setup sooner because both operations use the table. One-shot calls do not: the Python procedural T127 scalar synthesis/analysis medians were `3.261/2.908 ms`, versus `0.925/0.622 ms` through the Python plan methods; at T255 they were `38.154/35.990 ms`, versus `6.196/4.473 ms`. The T255 procedural cost includes rebuilding a roughly 32 ms table.

### Temporary higher-bandlimit table screen

The table gates were temporarily raised to include T360, T511 and T720. Plan construction confirmed table activation at all three sizes. The gates were then restored to T255 and the large-table changes were not committed. Three native process runs gave:

| Bandlimit | Table setup | Scalar synthesis recurrence → table | Scalar analysis recurrence → table | Table payload |
| --- | ---: | ---: | ---: | ---: |
| T360 | `0.034 → 89.6 ms` | `26.145/6.918 → 16.184/5.132` | `15.558/5.712 → 11.997/5.368` | 90.23 MiB |
| T511 | `0.035 → 251.2 ms` | `60.979/15.962 → 43.239/12.873` | `36.171/10.142 → 32.731/14.435` | 256.5 MiB |
| T720 | `0.091 → 678.9 ms` | `153.981/43.642 → 115.706/34.477` | `97.507/27.644 → 89.412/37.918` | 716.87 MiB |

At T360, the eight-thread analysis medians overlapped their process variation. At T511, table analysis was about 9% slower at one thread and 42% slower at eight. At T720, it was about 8% faster at one thread and 37% slower at eight. The T720 table process reached 786,656 KiB peak RSS. This memory and eight-thread analysis behavior do not justify raising the production cap for an atmospheric workload that uses both analysis and synthesis. A synthesis-only table policy could be measured separately, but it would require a separate selection rule and is not part of this implementation. The temporary high-band Python comparison against the installed DUCC wheel measured these relative-L2 errors for table-backed scalar `S/A`: T360 `2.652e-14/3.561e-14`, T511 `4.690e-14/4.707e-14`, and T720 `5.349e-14/7.719e-14`. The Python method also allocates a workspace per call; native reusable-workspace timings are the performance acceptance data.

## GL Python-plan comparison against DUCC

The table entries below report `Rust / DUCC` milliseconds. Each row separates synthesis and analysis, one and eight threads. At T72/T127/T255 the Rust GL scalar rows use the compact table. Spin 1 and the other bandlimits use recurrence. The newly measured T72 Python run is a correctness cross-check; process-level Rust timings at eight threads had substantial scheduling variation and are not used to accept the native kernel.

| Bandlimit | Spin | S 1T R/D | A 1T R/D | S 8T R/D | A 8T R/D | Max relative-L2 S/A | Max 8T process range |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T72 | 0 | `0.348/0.268` | `0.301/0.344` | `0.392/0.520` | `0.297/0.596` | `5.409e-15/7.052e-15` | 0.479 ms |
| T72 | 1 | `0.300/0.276` | `0.483/0.290` | `0.631/0.469` | `0.258/0.190` | `9.137e-15/7.670e-15` | 2.734 ms |
| T127 | 0 | `0.921/0.728` | `0.629/0.670` | `4.998/0.174` | `0.332/0.147` | `1.070e-14/1.114e-14` | 6.622 ms |
| T127 | 1 | `3.182/1.564` | `3.005/1.357` | `3.315/0.251` | `4.866/0.292` | `1.487e-14/1.364e-14` | 1.675 ms |
| T255 | 0 | `6.304/1.915` | `4.446/1.329` | `1.746/0.697` | `6.491/0.248` | `2.001e-14/2.277e-14` | 10.973 ms |
| T255 | 1 | `14.189/4.073` | `17.841/3.016` | `4.730/1.317` | `5.978/0.506` | `3.017e-14/2.267e-14` | 7.276 ms |
| T360 | 0 | `24.524/9.768` | `15.468/7.965` | `7.621/2.085` | `6.325/1.379` | `4.216e-14/3.436e-14` | 6.999 ms |
| T360 | 1 | `25.456/22.552` | `38.066/17.284` | `10.170/4.704` | `12.673/2.896` | `4.530e-14/3.187e-14` | 2.263 ms |
| T511 | 0 | `59.290/10.197` | `35.308/6.434` | `17.126/3.648` | `11.276/1.126` | `6.632e-14/3.484e-14` | 2.184 ms |
| T511 | 1 | `60.566/20.679` | `91.003/13.081` | `24.695/4.691` | `29.241/2.303` | `6.697e-14/5.128e-14` | 5.599 ms |
| T720 | 0 | `149.967/24.223` | `93.233/14.870` | `47.198/6.306` | `36.372/2.365` | `8.545e-14/6.020e-14` | 3.869 ms |
| T720 | 1 | `145.424/48.525` | `222.770/31.584` | `60.576/9.280` | `67.339/5.145` | `9.123e-14/6.920e-14` | 1.809 ms |

Input copies were small relative to these operations: at T720 the coefficient copy took about `0.1–0.3 ms`, the map copy about `0.8–1.4 ms`, and output allocation about `0.001 ms`. The Python API's per-call workspace allocation explains some of the difference between these measurements and native reuse, but input copying does not explain the high-band transform gap.

## CC and F1 Python-plan comparisons

The scalar table is GL-only. CC and F1 continue to use their existing resampling and recurrence kernels. These rows are current Python-plan medians in milliseconds, `Rust / DUCC`, with errors from the same inputs. They use two process medians of five calls; several T127 eight-thread runs had scheduling outliers.

| Grid / bandlimit | Spin | Synthesis 1T R/D | Analysis 1T R/D | Synthesis 8T R/D | Analysis 8T R/D | Max relative-L2 S/A |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| CC T127 | 0 | `1.647/0.738` | `1.626/1.356` | `1.455/0.884` | `2.992/0.486` | `1.268e-14/6.466e-15` |
| CC T127 | 1 | `3.848/1.886` | `4.578/2.824` | `2.798/1.974` | `6.485/0.958` | `1.785e-14/1.104e-14` |
| CC T511 | 0 | `58.562/10.578` | `43.765/19.085` | `23.636/3.220` | `17.968/2.887` | `5.367e-14/2.979e-14` |
| CC T511 | 1 | `64.844/21.597` | `120.495/40.655` | `30.254/4.924` | `45.611/7.103` | `5.235e-14/4.181e-14` |
| CC T720 | 0 | `150.013/20.753` | `122.882/60.545` | `49.290/2.904` | `56.984/10.982` | `9.625e-14/5.620e-14` |
| CC T720 | 1 | `151.987/50.062` | `337.554/130.188` | `58.504/9.923` | `123.864/22.321` | `8.997e-14/6.519e-14` |
| F1 T127 | 0 | `2.155/0.751` | `1.624/1.374` | `2.809/0.884` | `1.390/0.506` | `1.781e-14/6.429e-15` |
| F1 T127 | 1 | `3.461/1.815` | `4.610/2.843` | `2.558/1.505` | `6.573/1.007` | `1.511e-14/1.105e-14` |
| F1 T511 | 0 | `62.859/10.347` | `47.367/19.648` | `16.386/3.498` | `18.348/2.820` | `6.742e-14/2.989e-14` |
| F1 T511 | 1 | `56.860/20.770` | `118.849/40.339` | `23.877/4.826` | `45.071/6.223` | `8.722e-14/4.172e-14` |
| F1 T720 | 0 | `148.343/20.582` | `121.955/58.587` | `46.427/2.849` | `57.703/10.846` | `9.180e-14/5.635e-14` |
| F1 T720 | 1 | `146.182/48.895` | `314.895/122.412` | `57.954/9.426` | `122.814/23.142` | `9.751e-14/6.505e-14` |

The all-grid comparisons confirm that the GL table selection does not affect CC/F1. Spin-1 timing gaps remain larger in analysis than synthesis at the high bands. No FFT code or CC/F1 numerical path changed in this work.

## Numerical checks and limitations

The table-versus-recurrence test covers scalar synthesis `S`, exact analysis `A`, synthesis adjoint `S*` and analysis adjoint `A*` at rectangular and full truncations, odd and even longitude counts, nonzero `phi0`, and an equatorial ring. Relative L2 differences are below `2e-12`; the DUCC comparisons above are substantially tighter. A focused asymmetric-cutoff case sets one member of a reflected ring pair inactive and the other active, then checks phase and parity in the order spectrum. The table builder rejects any active exact-zero basis value except an exact equatorial parity zero. Coefficients or Fourier inputs with magnitude above `1e100`, non-finite inputs, and the tested `1e300` case use the scaled recurrence; the extreme-input output matched the recurrence exactly. The table is f64-only. Its high-bandlimit screen passed DUCC comparisons for scalar synthesis and analysis with relative-L2 errors below `7.8e-14`, but its timing does not justify a production gate above T255. Spin 1 uses recurrence throughout. High-band release checks covered scalar T511/T1023/T2047 on GL/CC/F1, spin 1/2 T511 on GL/CC/F1, and spin 1/2 T1023 exact analysis on all three grids. Existing spin-3/4 regression tests passed in the routine all-feature suite. Validation on source commit `d251af6` passed:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features` (125 Rust tests and 10 f32 tests passed; 8 high-band tests were ignored)
- `cargo test -p rustsht --no-default-features` (115 Rust tests and 10 f32 tests passed; 7 high-band tests were ignored)
- `uv run pytest python/tests` (8 passed)
- Release scalar GL/CC/F1 T511/T1023/T2047 DUCC and analytic checks
- Release spin-1/2 GL/CC/F1 T511 checks and spin-1/2 T1023 exact-analysis checks

The ignored high-spin T1023/T2047 tests were not run in this comparison. No numerical tolerances were changed. No new dependencies were added. The source table carries the DUCC `sht_inner_loop.h` license notice and the pinned source revision.

### Available phase attribution

Hardware counters remained unavailable. This experiment measured complete reusable-workspace table versus recurrence operations and did not add invasive phase timers to the current source. The most recent earlier-revision T511 timers in `plans/05-validation-and-milestones.md` split the Legendre/accumulation work from longitude FFT and row movement. They are historical attribution, not timings for `d251af6`:

| Operation / threads | Legendre, recurrence and accumulation Rust / DUCC | FFT and row movement Rust / DUCC | Whole timed phases Rust / DUCC | Absolute excess in Legendre phase |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis 1T | `75.27 / 6.66 ms` | `3.48 / 1.79 ms` | `78.75 / 8.45 ms` | `68.61 ms` |
| Scalar synthesis 8T | `25.78 / 1.04 ms` | `3.52 / 0.38 ms` | `29.30 / 1.42 ms` | `24.74 ms` |
| Scalar analysis 1T | `154.90 / 4.49 ms` | `3.40 / 1.85 ms` | `158.30 / 6.34 ms` | `150.41 ms` |
| Scalar analysis 8T | `27.10 / 0.71 ms` | `3.40 / 0.30 ms` | `30.50 / 1.01 ms` | `26.39 ms` |
| Spin-1 synthesis 1T | `469.40 / 14.80 ms` | `7.80 / 3.74 ms` | `477.20 / 18.54 ms` | `454.60 ms` |
| Spin-1 synthesis 8T | `75.90 / 2.00 ms` | `7.90 / 0.58 ms` | `83.80 / 2.58 ms` | `73.90 ms` |
| Spin-1 analysis 1T | `684.30 / 8.88 ms` | `6.70 / 3.83 ms` | `691.00 / 12.71 ms` | `675.42 ms` |
| Spin-1 analysis 8T | `173.90 / 1.40 ms` | `7.10 / 0.62 ms` | `181.00 / 2.02 ms` | `172.50 ms` |

The phase sums approximately reconcile with the scoped operations. In that earlier revision, recurrence and accumulation were about 88–99% of RustSHT's phase total or 11 CPU cores% of DUCC's, depending on operation and thread count. Their absolute excess dominates the FFT/row phase excess in every row. This supports prioritizing Legendre and accumulation work, not an FFT change. The current table comparison shows that basis precomputation reduces recurrence cost at selected small bands; the temporary large tables do not give a robust all-operation, eight-thread benefit.

## Review and further experiments

The read-only Sol review examined parity, cutoffs, equatorial handling, all four scalar operators, exceptional-value fallback, slice ownership and thread safety. It found no correctness blocker. The review asked for an underflow guarantee narrower than the initial code comment and a regression for asymmetric mirrored cutoffs; both are covered in commit `d251af6`. The reviewer also noted that the table's 2.4/33 ms plan setup requires repeated plan use. That setup cost is included above. A separate review of the T72 gate change found no numerical or bounds issue. It flagged the native sample-count statement, which had generalized the earlier three-process runs to the T72 measurements; the methods section now separates T72's five paired process runs with 25 timed samples from the earlier comparisons. The next performance work should continue with scalar and spin-1 recurrence and accumulation at T255–T1023. The precomputed table is not a substitute for reducing the high-band recurrence cost. A separate experiment could test whether a synthesis-only table at T360/T720 is worth its memory when synthesis dominates, but it should keep analysis on recurrence and include setup amortization. FFT optimization remains deferred.

### Reusable Python workspace screen

A temporary Python-plan change cached one `ShtWorkspace` behind a plan-local mutex and reused it across calls. It preserved separate workspaces across plan instances but serialized concurrent calls on the same plan. The candidate passed formatting, Clippy, the all-feature Rust suite and all nine Python tests, including a same-plan concurrent-call check. Load-average telemetry was retained only in local diagnostics. For T511 GL spin-1, three Python-plan run-group medians were `132.955 → 128.867 ms` for one-thread synthesis, `290.418 → 204.866 ms` for one-thread analysis, `164.283 → 170.243 ms` for eight-thread synthesis and `194.699 → 196.993 ms` for eight-thread analysis. The one-thread analysis ranges overlapped near 200 ms (`202.807–325.046` baseline; `196.628–208.499` candidate), and neither eight-thread operation improved. A separate native workspace-allocation sample was 15.369 ms, but it was not a matched before/after whole-transform comparison. No repeatable end-to-end gain was established, so the candidate and its same-plan serialization behavior were reverted. Python plan calls continue to allocate per-call workspaces until a quieter matched benchmark demonstrates a net benefit.
