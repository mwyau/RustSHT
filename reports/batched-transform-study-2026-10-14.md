# Independent-field transform study

## Scope and decision

This study evaluates repeated independent scalar and spin-1 STANDARD transforms on a shared immutable `ShtPlan`. It compares sequential workspace reuse with caller-level field concurrency and tests whether two scalar synthesis fields can share a recurrence. It does not add a batch API or change a numerical kernel. The measured concurrency benefit is available today through the existing plan/workspace API; callers give each concurrent transform its own workspace. For eight T511 fields on eight threads, one-thread workspaces run fields concurrently in 2.0–2.6 times less warm elapsed time than eight sequential transforms. Two fields are insufficient to benefit at T511; four fields generally help. The result depends on operation and bandlimit, so the evidence does not justify an implicit scheduler or a public batch API. Pool-sized workspaces for each concurrently running field use more memory and can perform worse, especially for spin-1 synthesis. A private experiment reused RustSHT's existing dual-channel paired scalar synthesis kernel to handle two independent scalar fields at once. It reproduced both ordinary output maps exactly on this T511 test, but took about 11% longer at one thread. At eight threads it was modestly faster in two of three process runs and approximately tied in the third, with substantial overlap in sample ranges. That evidence is not strong enough to retain the extra path. The prototype has been removed; raw samples are retained in [`shared-recurrence-prototype-samples.txt`](data/batched-transform-2026-10-14/shared-recurrence-prototype-samples.txt).

## Source audit

RustSHT source baseline: commit `5813be9f14bfc37422b48448201bcfeea88fcdaa`. The benchmark-only additions were the `m5_batch_bench` example, raw outputs and this report. No production transform source or public API changed. The read-only SHTns audit used revision `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`. Its CPU drivers traverse harmonic order and then batch field; comments identify recurrence data locality as a reason to process fields together. `shtns_set_many()` also configures batched FFT plans and a theta-contiguous field layout, and its current batching setup rejects odd `nlat`. This suggests two possible advantages: recurrence/coefficient locality and batched FFT execution. The first was prototyped with an existing RustSHT dual-channel synthesis kernel. The second was out of scope because RustSHT's public map storage is row-major, uses RustFFT, and this phase did not investigate FFT planning or layout changes. SHTns was not built or timed, so no numerical or timing comparison to it is claimed. RustSHT plans are immutable and reusable. A workspace owns its Fourier and recurrence scratch and is sized for the Rayon pool active at workspace creation. One workspace supports repeated sequential operations. Concurrent field operations require separate workspaces. Constructing those workspaces inside a one-thread pool makes each transform execute its own recurrence serially while an outer Rayon pool schedules fields. This avoids nested transform parallelism but retains one full workspace per active field. Constructing each workspace for the outer pool enables nested Rayon work; it is measured separately below.

## Benchmark method

### Hardware and builds

- x86-64 CPU, host topology omitted, AVX2 and FMA; no AVX-512.
- Linux; processes were pinned to configured worker affinity set..q-1` using `taskset` for a `q`-thread run. Load-average telemetry was retained only in local diagnostics. Pulp runtime dispatch selected the available AVX2/FMA path.
- DUCC reference: installed `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; explicit `nthreads` matched Rust's requested pool size.
- Python-facing comparisons used Python `3.11.15`, NumPy `2.4.6`, and the reusable Rust plan methods. Python data generation and plan benchmark commands are retained in the raw log.

### Workloads

The native harness is [`examples/m5_batch_bench.rs`](../examples/m5_batch_bench.rs), selected by a `parallel`-feature-gated example target in `Cargo.toml`. It measures f64 GL scalar and spin-1 STANDARD synthesis and analysis at T127 (`128 x 257`) and T511 (`512 x 1025`), with `mmax=lmax` and `phi0=0.37`. T127 and T511 grids use `ntheta=lmax+1`, `nphi=2*lmax+3`; coefficients and field-0 input maps match `examples/python_perf_baseline.py`. Other fields use deterministic coefficient shifts and independently shifted maps. There are 152 configurations, three process replicates per configuration, and five warm samples per process. The matrix covers 1, 2, 4, 8 and 16 threads; field counts 1 and 8 for sequential execution; additional field counts 2 and 4 at eight threads; and, where scheduled, field-parallel execution using one-thread workspaces or pool-sized workspaces. The 16-thread field-parallel runs have at most eight active field tasks for eight fields. The reusable benchmark accepts explicit `ntheta`, `nphi` and `mmax` arguments for follow-up exact-grid comparisons. For every process the harness reports plan construction, workspace allocation, first-batch time, warm samples, workspace RSS delta, peak RSS and a checksum. The warm batch excludes setup and represents the complete requested field batch. The native matrix is in [`native-batch-summary.csv`](data/batched-transform-2026-10-14/native-batch-summary.csv) and [`native-batch-process-samples.txt`](data/batched-transform-2026-10-14/native-batch-process-samples.txt). DUCC-only samples are in [`ducc-process-samples.txt`](data/batched-transform-2026-10-14/ducc-process-samples.txt); matched Python comparisons, including numerical errors, are in [`python-ducc-process-samples.txt`](data/batched-transform-2026-10-14/python-ducc-process-samples.txt). The single-field native-vs-DUCC summary is [`native-ducc-single-summary.csv`](data/batched-transform-2026-10-14/native-ducc-single-summary.csv). Representative commands:

```sh
cargo build --release --example m5_batch_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m5_batch_bench 0 511 8 synthesis 8 parallel-one 512 1025 511 5
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m5_batch_bench 1 511 8 analysis 8 sequential 512 1025 511 5
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env RAYON_NUM_THREADS=1 uv run python examples/python_perf_baseline.py 511 0 1 GL 3 plan synthesis both STANDARD 1025
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env RAYON_NUM_THREADS=8 uv run python examples/python_perf_baseline.py 511 1 8 GL 3 plan analysis both STANDARD 1025
```

DUCC's timings are from the installed pinned package. An optimized source build was not produced for this batch study. Python-facing Rust thread-8 timings in `python-ducc-process-samples.txt` had outlier process runs and are not used for the native performance conclusions. Native Rust and DUCC results below use separately measured reusable native plans/workspaces.

### Phase profiling limits


## Results

All times are milliseconds. Native single-field Rust/DUCC pairs are medians of process medians; the DUCC process spread can be large for sub-millisecond workloads. Ratios should be read with that variability in mind.

### Single-field native execution and DUCC comparison

| Grid / operation | Rust 1T | DUCC 1T | Ratio | Rust 8T | DUCC 8T | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T127 scalar synthesis | 0.883 | 0.742 | 1.19x | 0.323 | 0.202 | 1.60x |
| T127 scalar analysis | 0.625 | 0.662 | 0.94x | 0.319 | 0.150 | 2.13x |
| T127 spin-1 synthesis | 2.415 | 1.703 | 1.42x | 0.976 | 0.565 | 1.73x |
| T127 spin-1 analysis | 1.132 | 1.343 | 0.84x | 0.503 | 0.261 | 1.93x |
| T511 scalar synthesis | 43.916 | 9.807 | 4.48x | 12.383 | 2.798 | 4.43x |
| T511 scalar analysis | 23.783 | 6.431 | 3.70x | 7.479 | 1.017 | 7.35x |
| T511 spin-1 synthesis | 59.210 | 24.561 | 2.41x | 21.672 | 4.361 | 4.97x |
| T511 spin-1 analysis | 39.114 | 13.300 | 2.94x | 12.942 | 1.921 | 6.74x |

At T511, the largest absolute single-field excess is spin-1 synthesis at one thread (`34.65 ms`), followed closely by scalar synthesis (`34.11 ms`). At eight threads, spin-1 synthesis has the largest excess (`17.31 ms`), then spin-1 analysis (`11.02 ms`), scalar synthesis (`9.59 ms`) and scalar analysis (`6.46 ms`). These are current single-transform measurements, not multi-field speedups. Python-facing numerical comparisons against installed DUCC 0.41.0 were within the existing accuracy range. At T127, scalar synthesis/analysis had maximum absolute errors `5.412e-16 / 6.101e-15` and relative-L2 errors `1.070e-14 / 1.114e-14`; spin-1 synthesis/analysis had maximum absolute errors `1.757e-15 / 6.999e-15` and relative-L2 errors `4.344e-14 / 1.052e-14`. At T511, scalar synthesis/analysis had maximum absolute errors `3.495e-14 / 1.238e-14` and relative-L2 errors `6.632e-14 / 3.484e-14`; spin-1 synthesis/analysis had maximum absolute errors `3.257e-14 / 1.401e-14` and relative-L2 errors `6.697e-14 / 3.740e-14`. These measurements compare field 0; they are numerical checks, not evidence that the batch schedule itself changes the transform.

### Independent-field throughput

The table compares eight fields in a q=8 pool. “Sequential” reuses one q=8 workspace and runs fields in sequence. “Field parallel” runs concurrent transforms with a separate one-thread workspace per field. The final column compares warm batch latency. Workspace setup and RSS are reported below.

| Grid / operation | Sequential 8-field batch | Field-parallel batch | Throughput gain |
| --- | ---: | ---: | ---: |
| T127 scalar synthesis | 2.525 | 1.045 | 2.42x |
| T127 scalar analysis | 2.282 | 0.818 | 2.79x |
| T127 spin-1 synthesis | 7.734 | 3.612 | 2.14x |
| T127 spin-1 analysis | 4.069 | 1.205 | 3.38x |
| T511 scalar synthesis | 98.433 | 48.827 | 2.02x |
| T511 scalar analysis | 64.172 | 29.529 | 2.17x |
| T511 spin-1 synthesis | 175.026 | 68.666 | 2.55x |
| T511 spin-1 analysis | 107.363 | 44.976 | 2.39x |

Field-count and thread-count sweeps show the limit of this strategy. At T511 with eight threads, two-field parallel batches were slower than sequential batches for all four operations: scalar synthesis `45.18 vs 25.05 ms`, scalar analysis `24.38 vs 15.43 ms`, spin-1 synthesis `59.95 vs 44.69 ms`, and spin-1 analysis `40.67 vs 25.51 ms`. Four-field parallel batches were faster in this matrix: `46.31 vs 49.69`, `26.10 vs 31.64`, `63.01 vs 89.17`, and `42.48 vs 52.22 ms`, respectively. This threshold is workload-specific; the data do not support a universal field-count cutoff. At q=16 with only eight fields, there are no more than eight independent field tasks. T511 scalar analysis was essentially unchanged versus q=8 (`29.34 vs 29.53 ms`); scalar and spin-1 synthesis were slower (`51.14 vs 48.83 ms`, `72.22 vs 68.67 ms`). More worker threads do not help once each field has its own serial workspace. Across the tested cases, one-thread workspaces were more predictable than pool-sized workspaces. At T511/q=8/B=8, pool-sized workspaces increased total workspace memory from `65.2 MiB` to `74.2 MiB`; spin-1 synthesis took `95.74 ms` rather than `68.67 ms`. Scalar synthesis was nearly tied (`48.08 vs 48.83 ms`), and analysis was also similar. There is no consistent reason to use nested parallel workspaces for independent fields.

### Setup, memory and amortization

The common plan construction cost at T511 was about `0.10–0.13 ms`. It is negligible next to the execution and workspace costs. T127 plan construction was about `0.08–0.11 ms`.

| Grid / setup | One sequential workspace | Eight one-thread workspaces | Eight pool-sized workspaces |
| --- | ---: | ---: | ---: |
| T127 allocation / RSS increase | 0.39–0.47 ms / 0.73–0.77 MiB | 2.13–2.29 ms / 4.38–4.43 MiB | 3.06–3.45 ms / 5.74–6.00 MiB |
| T511 allocation / RSS increase | 3.99–4.26 ms / 9.04–9.30 MiB | 26.46–27.49 ms / 65.09–65.23 MiB | 30.79–31.64 ms / 72.21–74.21 MiB |

Peak process RSS for the T511 eight-field q=8 one-thread-workspace batches was `197–262 MiB`, depending on operation. It includes all input and output arrays as well as the workspaces. The reported workspace RSS increase was measured before allocating the field arrays. Each concurrent field therefore adds a material per-workspace cost; using multiple fields is appropriate when memory is available and throughput matters, not a free substitution for a single transform. For eight T511 fields, a simple amortization model is `workspace allocation + N * warm batch latency`, with common plan construction omitted. For one batch, sequential vs field-parallel totals are: scalar synthesis `102.42 vs 75.28 ms`, scalar analysis `68.30 vs 56.83 ms`, spin-1 synthesis `179.29 vs 95.93 ms`, spin-1 analysis `111.42 vs 72.47 ms`. The field-parallel strategy already repays workspace setup in one eight-field batch at T511. At T127 the same model gives `2.94 vs 3.21 ms` for scalar synthesis and `2.67 vs 3.04 ms` for scalar analysis: the extra workspaces are not repaid on the first batch for those operations. Spin-1 T127 synthesis and analysis are repaid within one batch (`8.16 vs 5.74 ms`, `4.54 vs 3.50 ms`). These totals exclude the small, shared plan construction cost and any application-specific input/output allocation. For repeated batches, at T511 the field-parallel strategy continues to save about `49.6`, `34.6`, `106.4` and `62.4 ms` per eight-field scalar-synthesis, scalar-analysis, spin-synthesis and spin-analysis batch, respectively. T127 scalar wins by only `1.48 ms` and `1.46 ms` per batch, so workspace construction can dominate for one or few batches. The raw process output retains first-batch measurements; T127's first batch is appreciably slower than warm execution, and includes any first-use lazy initialization. This harness does not separate lazy table construction from the first transform, so that cost is not presented as an independently measured phase.

### Shared scalar recurrence prototype

The temporary ignored test used the existing two-channel paired recurrence kernel for two independent scalar synthesis inputs, on blocks meeting the kernel's normal-range and cutoff eligibility conditions. Other blocks fell back to the existing synthesis code. It reused `workspace.spectra` and `workspace.spin_spectra`, adding no full map or workspace allocation. Full map outputs were exactly equal to two ordinary Rust synthesis calls for both fields at one and eight threads in every run (`relative-L2 = 0`). Across three saved process runs, q=1 reference/paired medians were approximately `85–87 / 94–95 ms`, a consistent 9–11% regression. At q=8 reference/paired medians were `24.8 / 23.4 ms` across process medians, about 6% faster overall; individual process results ranged from roughly a tie to an 8% gain, with overlapping sample variability. The code also adds dual coefficient preparation, dispatch and fallback logic. The result does not establish an adequate, robust gain across thread counts, so this prototype was removed and no dual-scalar production path was added.

## Checks and limitations

The ignored release prototype test checked full-map equality against the ordinary scalar path and passed for q=1 and q=8. The benchmark harness uses the existing scalar and spin transform public methods; the committed harness does not modify numerical implementations. Final checks passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features` (142 Rust tests and 10 f32 tests passed, 8 high-bandlimit tests ignored); `cargo test -p rustsht --no-default-features` (127 Rust tests and 10 f32 tests passed, 7 high-bandlimit tests ignored); `uv run pytest python/tests` (16 passed); and `cargo build --release --example m5_batch_bench`. This study tests GL f64 only at T127/T511. It does not establish batching behavior for f32, CC/F1, GRAD_ONLY, DERIV1, adjoints, reduced `mmax`, T63/T72/T85/T159/T191/T255/T319/T360/T720/T1023 or T2047. It does not measure SHTns, an optimized DUCC source build, an isolated lazy-table setup phase, or batch FFT execution. Those omissions are intentional: the next optimization study concerns hemisphere-paired synthesis, and this batch result did not justify changing the plan/workspace architecture.

## Recommended use and next work

Applications processing multiple independent fields can reuse one immutable plan and run each field concurrently with a distinct workspace. At high bandlimit, one-thread workspaces per active field avoid nested Rayon oversubscription and gave the best measured throughput. Applications should cap concurrent fields to their memory budget and benchmark the field count on their own grid. Sequential reuse remains preferable for small batches, constrained memory, or low-bandlimit scalar workloads. No changes were made to the RustSHT-FFT implementation. The next measured kernel study is hemisphere-paired scalar synthesis; only a repeatable whole-transform gain with existing numerical and high-bandlimit validation would justify integrating that path.
