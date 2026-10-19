# Scalar analysis stage profile

**Workload:** T720 GL, 721 latitude rings × 1442 even longitude samples, full `mmax=720`, f64, `phi0=0.37`, q1 and q8. Rust used a reusable plan/workspace. Each instrumented profile had one warmup and five timed calls. Temporary test-only timing probes stayed outside production. Native Rust/DUCC stage probes used release `-O3`-equivalent code, `x86-64-v3`, debug symbols and frame pointers; Rust 1.99.0 and GCC 15.2.0 on an AMD Ryzen 9 5950X (x86_64, AVX2/FMA). Rust source was `ff6f3427` plus the pending analysis scratch no-fill change. The native DUCC probe used pinned revision `80388904b8cfc32400ba84af8a6a45aabfe60f86`; installed ducc0 0.41.0 was used only for independent output comparison.

## Matched results

| Stage / boundary | Rust q1 | DUCC q1 | Rust q8 | DUCC q8 |
| --- | ---: | ---: | ---: | ---: |
| Reused-call FFT | 9.263 ms | 8.39 ms | 1.712 ms | 1.22 ms |
| Order coefficient preparation | 0.715 ms | 1.31 ms | 0.998 ms wall | 1.4 ms worker-CPU sum |
| Active reflected-ring packing | 0.782 ms | 0.80 ms | 0.98 ms worker-CPU sum | 1.9–2.0 ms worker-CPU sum |
| Sectoral initialization | 1.196 ms | 0.53 ms | 1.25 ms worker-CPU sum | 0.53 ms worker-CPU sum |
| Insignificant prefix | No separate skip stage | 0.050 ms | No separate skip stage | 0.055 ms worker-CPU sum |
| Normal recurrence + coefficient accumulation | 18.35 ms estimated* | 7.12 ms | 22.25 ms worker-CPU estimated* | 7.9 ms worker-CPU sum |
| Mixed/scaled fallback recurrence + accumulation | 11.1 ms | 0.022 ms scaled-loop only† | 12.2 ms worker-CPU sum | 0.024 ms scaled-loop only† |
| Rust coefficient commit / DUCC normalization-scatter | 0.263 ms | 0.69 ms | 0.70 ms worker-CPU sum | included in order stage |
| Recurrence + accumulation wall stage | 34.166 ms | 11.12 ms | 5.123 ms | 1.76 ms |

`*` Rust's fused-kernel timer includes sectoral initialization: 19.55 ms q1 and 23.5 ms q8 worker-CPU sum. Subtracting the separately measured initialization gives the estimates in the table. `†` These rows are not equivalent: Rust's callback path recomputes and accumulates every degree for declined mixed blocks; DUCC's scaled-loop timer measures only its explicit scaled transition. DUCC ordinary and scaled loops together take about 7.14 ms q1 / 7.92 ms q8 worker-CPU sum. q8 worker-CPU sums are aggregate CPU time across workers, not elapsed latency.

The uninstrumented interleaved high-level calls measured Rust/DUCC medians of 41.822/19.443 ms (2.15×) at q1 and 7.507/3.245 ms (2.31×) at q8. Relative L2 output error was `6.039e-14` for both. Rust plan and workspace construction measured separately at about 0.038/7.72 ms q1 and 0.038/8.65 ms q8; DUCC's high-level `analysis_2d` measured 19.8/3.0 ms and includes per-call setup. The native `leg2alm` stage includes ring/workspace setup. Therefore the reused-call stage rows, rather than the high-level totals, are the closer algorithm comparison.

Sparse pair instrumentation recorded Rust accumulation/recurrence work of 0.372/0.359 ms across 10,708 sampled pairs, and DUCC 0.248/0.165 ms across 2,432 sampled pairs. These totals are diagnostic only: the sample populations and loop shapes differ, so they are not per-operation speed comparisons. `perf stat` hardware counters were unavailable under the host's `perf_event_paranoid=4`; system permissions were not changed.

## Finding and next change

FFT, coefficient preparation and ring packing do not explain the gap. At q8 both implementations keep workers busy for roughly 98% of the recurrence wall interval, so dynamic scheduling is not the primary cause. The largest excess is the Rust degree recurrence/accumulation path: normal fused work is about 2.6× DUCC at q1 and 2.8× by aggregate worker time at q8, with an additional 11.1 ms q1 fallback for mixed blocks. DUCC's `map2alm_kernel` processes two recurrence pairs per inner loop and accumulates their coefficients before leaving the SIMD latitude batch; Rust's fused analysis currently reduces and stores one pair at a time. The next change targets that measured normal-path loop structure, retaining the checked fallback for exceptional ranges.

The first fused two-pair prototype was screened against the immediately preceding source with the same release flags and five-call `m1b_bench`: q1 medians were 43.789 ms versus 40.681 ms, and q8 medians were 7.772 ms versus 6.929 ms. The prototype was removed because it regressed; a lower-register-pressure form remains to be tested. q8 samples varied substantially, so this was a bounded rejection screen, not a broad benchmark matrix.
