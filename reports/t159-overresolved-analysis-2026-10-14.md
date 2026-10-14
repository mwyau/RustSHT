# T159 over-resolved GL analysis table evaluation

## Investigation plan

The current automatic analysis-table ceiling is T127 because the prior over-resolved study found T191 regressions and did not measure T159. This experiment evaluates only f64 GL STANDARD analysis at T159 (`320x640`, full `mmax`), where the existing compact north-hemisphere scalar table remains within the 34 MiB cap. Scalar and spin-1 table and recurrence runs use the same release harness, inputs, plans, workspaces, CPU affinity and thread pool. Process order is paired and alternated at q1, q2, q4, q8 and q16. Record cold first-use cost and RSS separately from warm latency. Extend automatic selection only for thread counts with repeatable whole-transform analysis gains. Preserve recurrence for inconclusive cells and all other grids, operators, precisions and truncations. Validate scalar analysis and adjoint behavior against recurrence, spin-1 scalar-conversion analysis against the general recurrence and adjoint identities, then compare both selected paths with pinned DUCC 0.41.0. Keep the 34 MiB cap and measure table payload and reuse break-even. Run existing all-grid and high-bandlimit regressions after the change.

## Decision criteria

Accept the extension only if repeated paired process results show a stable warm improvement, numerical checks pass without changing tolerances, and first-call setup and retained memory are reported alongside the steady-state gain. Do not extend the synthesis policy from analysis measurements. If a thread-count cell is noisy or neutral, retain recurrence for that cell.

## Result

The source baseline was `7d88834`. The benchmark-only candidate changed the internal over-resolved analysis ceiling from T127 to T191 so the compact table could be selected at T159; it did not change recurrence arithmetic. Recurrence and table runs used the same `m1b_bench` release executable configuration, deterministic f64 inputs, reused plans/workspaces and exact GL `320x640`, `lmax=mmax=159`, `phi0=0.37`. The release benchmark was built with Rust `1.99.0 (b940084d7 2026-09-28)` / LLVM `23.1.1`, `opt-level=3`, 16 codegen units and debug symbols; runtime Pulp dispatch selected AVX2/FMA. No LTO or `target-cpu=native` was used. For each thread count, five process-paired comparisons were collected except q2, q4 and q16, which used three pairs. Each process warmed once, then measured five calls. Pair order was alternated. q1/q2/q4/q8/q16 were pinned to CPUs `0`, `0-1`, `0-3`, `0-7` and `0-15`, respectively. The shared host could not be made fully idle. The table figures below are medians across paired process medians; raw samples are in [`reports/data/overresolved-t159-analysis-2026-10-14`](data/overresolved-t159-analysis-2026-10-14/). The build and run form was:

```sh
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release --example m1b_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench <spin> 159 8 analysis GL STANDARD 320 640 159 requested-only
```

The host was an x86-64 CPU (host topology omitted), Linux; Rust `1.99.0` / LLVM `23.1.1`; optimized release build with runtime AVX2/FMA dispatch, no `target-cpu=native` and no LTO. Pinned-DUCC validation used `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The table is the existing north-hemisphere triangular layout; this change adds no workspace or dependency.

| Operation | Threads | Recurrence (ms) | Table (ms) | Table change | Paired change range |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar analysis | 1 | 2.360 | 2.225 | -5.57% | -6.94% to -5.07% |
| Scalar analysis | 2 | 1.726 | 1.424 | -17.23% | -17.47% to -15.99% |
| Scalar analysis | 4 | 1.269 | 1.062 | -16.68% | -18.08% to -15.56% |
| Scalar analysis | 8 | 0.944 | 0.851 | -10.01% | -15.03% to +1.28% |
| Scalar analysis | 16 | 0.945 | 0.903 | -4.44% | -5.99% to +24.39% |
| Spin-1 analysis | 1 | 4.153 | 4.193 | +0.95% | -4.88% to +9.02% |
| Spin-1 analysis | 2 | 3.228 | 2.723 | -15.37% | -17.58% to -14.27% |
| Spin-1 analysis | 4 | 2.444 | 1.921 | -19.75% | -23.87% to -16.74% |
| Spin-1 analysis | 8 | 1.738 | 1.498 | -13.30% | -14.80% to -11.60% |
| Spin-1 analysis | 16 | 1.668 | 1.543 | -7.52% | -13.24% to -1.46% |

The scalar q16 pair set is too small and variable to retain the table. Spin-1 q1 does not have a repeatable benefit. The final deterministic selector therefore uses the scalar analysis table at q1/q2/q4/q8 and recurrence at q16; spin-1 analysis uses recurrence at q1 and the table at q2/q4/q8/q16. Other over-resolved bands remain unchanged: T191 analysis remains recurrence, and no synthesis threshold was extended from these analysis measurements. The maximum over-resolved analysis bandlimit is now T159. The exact scalar table payload is `16,486,400 bytes` (`15.72 MiB`); spin-1's extended table is `16,691,200 bytes` (`15.92 MiB`). One fresh q8 process per variant measured maximum RSS of `14,504 -> 30,848 KiB` for scalar and `14,912 -> 31,148 KiB` for spin 1; allocator and worker variation affect these single-process values. Across the q8 cold-call samples, first-use medians were `1.010/15.533 ms` (scalar recurrence/table) and `1.755/16.787 ms` (spin-1 recurrence/table). Approximate warm-call break-even counts from the measured savings are scalar q1/q2/q4/q8: `108/49/71/156`; spin-1 q2/q4/q8/q16: `30/29/63/124`. These are steady-state reuse estimates, not a guarantee for short-lived plans. Plan construction remains about `0.016–0.021 ms`; workspace construction is about `1.1–1.8 ms` in this harness and is paid by both strategies. Total elapsed over N calls is `first_call + (N-1)*warm_median`; the break-even counts identify where those totals cross. For q8, where both table analyses are selected, the measured first-call plus warm totals are below. Values are `recurrence/table` milliseconds; they use first-call observations above and warm medians from the paired runs.

| Repeated analyses | Scalar | Spin 1 |
| ---: | ---: | ---: |
| 1 | `1.010 / 15.533` | `1.755 / 16.787` |
| 5 | `4.786 / 18.937` | `8.707 / 22.779` |
| 10 | `9.506 / 23.192` | `17.397 / 30.269` |
| 25 | `23.666 / 35.957` | `43.467 / 52.739` |
| 50 | `47.286 / 57.232` | `86.917 / 90.189` |
| 100 | `94.466 / 99.782` | `173.817 / 165.089` |
| 1,000 | `944.066 / 865.682` | `1,738.017 / 1,513.289` |

## Correctness and disposition

The scalar native regression compares selected table and forced recurrence execution for synthesis, analysis and both adjoints at T159, including the existing inner-product checks. Relative-L2 differences remain below `2e-12`. Spin-1 q8 analysis was run against the general spin recurrence at the same grid; E/B coefficient differences remain within `3e-10 * max(1, |reference|)`, and the normalized adjoint identity remains within `3e-9`. The test also confirms table construction only on the selected q8 path and selector fallback at q1. Installed DUCC comparisons use deterministic f64 STANDARD analysis inputs, exact `320x640`, full `mmax`, nonzero `phi0=0.37`, and unchanged `rtol=atol=2e-12`. At q8, scalar analysis had maximum absolute / relative-L2 coefficient errors `6.11e-16 / 1.04e-14`; spin-1 analysis had `4.00e-16 / 1.12e-14`. The analysis adjoints had maximum absolute / relative-L2 map errors `5.05e-16 / 1.00e-14` (scalar) and `8.24e-16 / 1.58e-14` (spin 1). Both q1 and q8 Python tests pass. The all-grid native spin-1 comparison retains GL, CC and F1 coverage. An initial attempt to reuse the full T63 Python all-operator test at T159 was not retained: the existing T159 synthesis path does not satisfy that test's pointwise `2e-12` tolerance for either the one-thread recurrence or q8 table-selected scalar synthesis. The respective maximum absolute errors were `4.55e-11` and `6.49e-11`, while relative-L2 errors were `3.59e-14` and `4.34e-14`; spin-1 synthesis recurrence error was `5.04e-14` relative L2. This investigation changes no synthesis policy or tolerance. The accepted T159 Python regression therefore targets the analysis operator and its adjoint, for which both matched DUCC checks pass.

### Python plan-method comparison

The existing `examples/python_perf_baseline.py` gained an optional final `ntheta` argument so its plan-method benchmark can use the exact `320x640` T159 grid. The command form was:

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env RAYON_NUM_THREADS=1 uv run python examples/python_perf_baseline.py 159 <spin> 1 GL 5 plan analysis both STANDARD 640 320
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env RAYON_NUM_THREADS=8 uv run python examples/python_perf_baseline.py 159 <spin> 8 GL 5 plan analysis both STANDARD 640 320
```

Three process runs were collected per cell; each process reported five medians of five timed calls after warmup. Python plan methods allocate a workspace per invocation; the native table above reuses a workspace, so these boundaries are intentionally separate.

| Spin | Threads | Rust plan median (process range, ms) | DUCC median (process range, ms) | Relative-L2 error |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 1 | `2.225 [2.223–2.227]` | `0.973 [0.963–0.977]` | `1.451e-14` |
| 0 | 8 | `2.428 [2.233–4.752]` | `0.213 [0.201–0.223]` | `1.451e-14` |
| 1 | 1 | `4.145 [4.136–4.193]` | `2.216 [2.216–2.234]` | `1.212e-14` |
| 1 | 8 | `1.620 [1.548–2.557]` | `0.394 [0.392–0.395]` | `7.869e-15` |

The q8 Rust plan results varied substantially within individual processes (up to `6.53 ms` spread among five reported run medians), while DUCC's process ranges were narrower. Do not use these Python q8 medians to rank the native strategies or claim a stable ratio; the native reused-workspace runs set the selector thresholds. At q1, the Rust plan method was `2.29x` scalar and `1.87x` spin-1 DUCC latency. Analysis copies and plan construction are shown in the raw logs and remain small against transform time. The q8 scalar warm gain is about `0.093 ms` per analysis, so its table setup requires roughly 156 analyses to amortize; q2/q4 scalar and q2/q4 spin-1 amortize sooner. The selector cannot know caller reuse in advance, so its existing deterministic policy uses the measured steady-state regions and reports cold cost instead of calibrating. No hidden autotuning, public strategy API, FFT change or SHTns-derived implementation was added. The source acceptance commit is [`b7401fa`](https://github.com/mwyau/RustSHT/commit/b7401fa), pushed to `main`. Raw benchmark commands, process medians and first-use data are in `reports/data/overresolved-t159-analysis-2026-10-14/`; q8 RSS and cold-call samples are in [`rss-process-samples.txt`](data/overresolved-t159-analysis-2026-10-14/rss-process-samples.txt) and [`cold-first-call-process-samples.txt`](data/overresolved-t159-analysis-2026-10-14/cold-first-call-process-samples.txt). Full-suite validation results are recorded in `plans/05-validation-and-milestones.md`.
