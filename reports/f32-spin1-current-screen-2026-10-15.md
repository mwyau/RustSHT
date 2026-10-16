# f32 spin-1 T511 performance screen (M4, 2026-10-15)

The current RustSHT branch completed f32 spin-1 GL T511 synthesis and analysis in five release processes. The median q1-to-q8 speedup was 6.77x for synthesis and 5.67x for analysis. These runs characterize the branch; they were not paired with an earlier binary or with DUCC.

## Method

The benchmark used `examples/m4_f32_bench.rs` with a reused `ShtPlan<f32>` and workspace, full-mmax deterministic E/B coefficients, GL `512x1025`, and `phi0=0.37`. Each process warmed each operation once and collected five timed calls at q1 and q8. Processes were 8 pinned CPU cores and run under `target/rustsht-benchmark.lock`. The source was commit `8ccc0d5`; commit `8952ea9` adds only the order-owned synthesis screening report. The binary was built with `cargo build --release --example m4_f32_bench`, rustc `1.99.0`, and LLVM `23.1.1`. Binary SHA-256: `255966f3b306f5cd6a3b649b94f5fd2932c05e8ba128534b3bc4bbfcba4490d9`. The host was an x86-64 CPU with host topology omitted, running Linux. The benchmark used Pulp runtime dispatch; this executable did not record the selected ISA. Load-average telemetry was retained only in local diagnostics. Process snapshots showed no unrelated CPU-heavy job.

## Results

Values are medians across five process medians. Ranges show the five process medians. Process peak RSS is the median and range of the five process-level maximum resident set size measurements.

| Operation | q1 median (ms) | q1 range (ms) | q8 median (ms) | q8 range (ms) | q1/q8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 993.391 | 987.991–1007.420 | 146.682 | 145.053–149.518 | 6.77x |
| Spin-1 analysis | 1014.694 | 1009.091–1016.913 | 179.030 | 178.196–180.697 | 5.67x |

The benchmark estimated 12.13 MiB for core numeric arrays. Median process peak RSS was 15,312 KiB (15,052–15,576 KiB). The 2026-10-12 f32 SIMD measurements recorded medians of 1168.563/178.572 ms for q1/q8 synthesis and 1273.005/297.207 ms for q1/q8 analysis. The current medians are lower by 15.0%, 17.9%, 20.3%, and 39.8%, respectively. Those earlier values averaged two run medians and were not paired with this screen. Synthesis also changed across source revisions, so the difference cannot be attributed to analysis row scheduling. The screen did not rerun DUCC; the `ducc0==0.41.0` comparison in [the M4 record](../plans/05-validation-and-milestones.md#f32-latitude-recurrence-simd-m4-2026-10-12) remains the available reference. This screen adds a current f32 GL T511 measurement. It does not establish a source-level speedup or close the measured performance gap with DUCC. The raw process output and runner are in [`data/f32-rowfft-check-2026-10-15/`](data/f32-rowfft-check-2026-10-15/).
