# Scalar analysis stage profile

**Workload:** T720 GL, 721 latitude rings × 1442 even longitude samples, full `mmax=720`, f64, `phi0=0.37`, q1 and q8. Rust used a reusable plan/workspace. Each instrumented profile had one warmup and five timed calls. Temporary test-only timing probes stayed outside production. Native Rust/DUCC stage probes used release `-O3`-equivalent code, `x86-64-v3`, debug symbols and frame pointers; Rust 1.99.0 and GCC 15.2.0 on an AMD Ryzen 9 5950X (x86_64, AVX2/FMA). The Rust profile source was `ff6f3427` plus the analysis scratch no-fill change, now committed in `fbefc60d`; `7f450e8a` adds documentation only. Thus the source state used for the profile is reproducible from committed code. The native DUCC probe used pinned revision `80388904b8cfc32400ba84af8a6a45aabfe60f86`; installed ducc0 0.41.0 was used only for independent output comparison.

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

## Decline counters and DUCC execution

Temporary low-overhead counters around the fused normal-range route found no
recurrence declines and no completed recurrence work discarded for fallback.
Every decline occurred during sectoral initialization at `l=m`; the checked
callback then evaluated that order through its robust recurrence. No recurrence
degree was rejected. This separates a repeated seed attempt from degree-loop
replay: the latter was zero.

| Workload | Active blocks (fast / init decline / recurrence decline) | Active lanes (fast / declined) | Declined-block recurrence | Callback time q1 / q8 |
| --- | ---: | ---: | --- | ---: |
| T720, `mmax=720` | 3,384 (2,910 / 474 / 0) | 163,657 / 30,336 | 454 mixed-scale, 20 fully scaled | 11.82 ms / 12.65 ms worker sum |
| T300, `mmax=150` | 453 (369 / 84 / 0) | 17,037 / 5,376 | 64 mixed-scale, 20 fully scaled | 1.18 ms / 1.30 ms worker sum |

All declined blocks had 64 active lanes. The T720 callback covered 6,613,248
lane-degree pairs (103,332 degree-pair callbacks); T300 covered 542,912
lane-degree pairs (8,483 callbacks). The prior 11.1 ms estimate is therefore
mostly the required mixed/scaled recurrence of initialization-declined blocks,
not repeated work from a failed ordinary recurrence. The fast route retries
sectoral initialization before the callback seeds the same block, but does
not replay any degree loop.

DUCC's `fill_m2a` computes a stable cosine square per packed latitude, with
`(1-sth)*(1+sth)` near the pole, and preweights the opposite-parity spectrum
channels by cosine while packing active rings. `calc_map2alm` advances the
insignificant prefix, maintains lane scaling through its scaled loop, then
normalizes lanes into the ordinary SIMD kernel. `map2alm_kernel` accumulates
two same-parity recurrence pairs per inner iteration before reducing and
storing. Rust's fused route still forms cosine squares inside every recurrence
pair. Before this candidate it also multiplied the opposite channel by cosine
at every degree pair; its callback still handles mixed and scaled lanes safely.

## Bounded normal-route change

The candidate now cosine-weights opposite-parity real/imaginary channels once
per eligible active block, then uses those channels directly in the fused
recurrence/accumulator. It restores the raw channels before callback fallback.
The existing input-range preflight remains in place; the preweight pass checks
each product as it stores it and declines any newly subnormal or unintended
underflow value. The robust scaled recurrence and its safety checks remain
unchanged. The rejected two-pair prototype remains archived and was not
reintroduced. A direct SIMD-tail test compares the preweighted route against
the paired recurrence reference; an additional test checks subnormal decline.

An interleaved optimized-release screen used three process comparisons per
case, one warmup and five timed calls per process. The Rust baseline executable
was built from the preceding production source at `fbefc60d`; the candidate
includes the change in this report. Rust reused plan/workspace; DUCC used
installed `ducc0 0.41.0` (`analysis_2d`, source build revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`). Rust release runtime dispatch
selected portable AVX2/FMA on the same AMD Ryzen 9 5950X. These full-call
ratios provide context, not identical setup boundaries: the DUCC Python call
includes per-call setup while Rust reuses its plan and workspace. The T720
candidate/DUCC output relative L2 error was `6.039e-14`.

| Workload | q | Rust baseline | Rust candidate | Candidate change | DUCC | Candidate / DUCC |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T720 GL, full `mmax`, 1442 longitude | 1 | 42.232 ms | 41.182 ms | -2.5% | 19.215 ms | 2.14× |
| T720 GL, full `mmax`, 1442 longitude | 8 | 6.423 ms | 6.317 ms | -1.7% | 3.118 ms | 2.03× |
| T300 GL, `mmax=150`, 603 longitude | 1 | 4.330 ms | 4.282 ms | -1.1% | 3.112 ms | 1.38×* |
| T300 GL, `mmax=150`, 603 longitude | 8 | 0.646 ms | 0.652 ms | +0.9% | 2.120 ms | 0.31×* |

`*` T300 DUCC timings are inconclusive: q1 process medians ranged from 2.18 to
3.16 ms and q8 from 0.79 to 2.55 ms, while Rust remained near 0.65 ms at q8.
No host counters were available, so interference cannot be isolated from
thread-startup and call overhead. T720 q8 DUCC also varied from 2.94 to
3.30 ms. The candidate improved T720 q1/q8 and T300 q1 modestly; T300 q8's
0.9% difference is within screen noise. The remaining T720 gap is about
2.0–2.1× at these unequal call boundaries.

## Finding and next work

FFT, coefficient preparation and ring packing do not explain the gap. At q8 both implementations keep workers busy for roughly 98% of the recurrence wall interval, so dynamic scheduling is not the primary cause. The remaining excess is in recurrence/accumulation, including DUCC's scaled-to-ordinary lane transition and its two-pair kernel. This change removes repeated cosine multiplication without increasing recurrence register pressure. Further scalar work should target the lane-wise scaled transition only with a source-grounded design; the next operation is spin-1 analysis.

The first fused two-pair prototype was screened against the immediately preceding source with the same release flags and five-call `m1b_bench`: q1 medians were 43.789 ms versus 40.681 ms, and q8 medians were 7.772 ms versus 6.929 ms. It remains removed in the archive. This screen was bounded and is not a broad benchmark matrix.
