# Spin-1 direct recurrence and scalar conversion comparison

## Result

For f64 GL STANDARD transforms, the current spin-1 scalar-conversion path was faster than the general spin recurrence at every tested bandlimit. At T511 it was 19.8× faster for one-thread synthesis and 25.7× faster for one-thread analysis. At eight threads it was 6.8× faster for synthesis and 15.1× faster for analysis. At T720 the ratios were 11.9–28.8×. The direct general recurrence is not a viable replacement for the scalar-conversion path. No production code changed. The test-only forcing control and prototype were removed after measurement. The direct general recurrence was slower, and a fused direct recurrence-and-accumulation prototype was also slower across all three tested bands. Both results favor the scalar-conversion path for these GL workloads.

## Comparison method


```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo test --release -p rustsht --lib \
  benchmark_spin1_scalar_conversion_vs_direct_recurrence -- --ignored --nocapture
```

The T511 repeats used `RUSTSHT_SPIN1_BENCH_LMAX=511` with the same command. Seven-sample raw timings are in [`native-samples.txt`](data/spin1-direct-vs-scalar-conversion-2026-10-14/native-samples.txt). Times below are milliseconds as `scalar conversion / general recurrence`; the ratio is general recurrence divided by scalar conversion.

## Native results

| Bandlimit and grid | Threads | Synthesis ms (ratio) | Analysis ms (ratio) |
| --- | ---: | ---: | ---: |
| T63, `65x129` | 1 | 0.276 / 2.541 (9.20×) | 0.294 / 2.372 (8.07×) |
| T63, `65x129` | 8 | 0.216 / 0.789 (3.65×) | 0.201 / 0.558 (2.77×) |
| T72, `74x147` | 1 | 0.295 / 3.591 (12.18×) | 0.307 / 3.439 (11.19×) |
| T72, `74x147` | 8 | 0.183 / 0.933 (5.09×) | 0.143 / 0.581 (4.05×) |
| T127, `129x257` | 1 | 2.415 / 19.169 (7.94×) | 1.533 / 17.894 (11.67×) |
| T127, `129x257` | 8 | 0.924 / 3.398 (3.68×) | 0.733 / 2.916 (3.98×) |
| T255, `257x513` | 1 | 12.064 / 148.585 (12.32×) | 7.713 / 139.777 (18.12×) |
| T255, `257x513` | 8 | 4.526 / 23.160 (5.12×) | 2.892 / 24.350 (8.42×) |
| T511, `513x1025` | 1 | 58.275 / 1156.332 (19.84×) | 41.700 / 1070.976 (25.68×) |
| T511, `513x1025` | 8 | 24.506 / 167.693 (6.84×) | 13.149 / 198.434 (15.09×) |
| T720, `722x1443` | 1 | 137.192 / 3178.737 (23.17×) | 102.856 / 2963.587 (28.81×) |
| T720, `722x1443` | 8 | 49.722 / 589.056 (11.85×) | 35.555 / 562.180 (15.81×) |

Across the three T511 process medians, conversion-path synthesis ranged from `58.189–59.355 ms` at one thread and `24.076–24.981 ms` at eight threads. Direct synthesis ranged from `1152.291–1164.041 ms` and `166.526–167.887 ms`. Conversion-path analysis ranged from `41.625–42.938 ms` and `13.011–13.513 ms`; direct analysis ranged from `1064.855–1111.968 ms` and `197.401–200.228 ms`. The q=8 direct analysis was slow but more thread-scalable: it fell 5.4× from q=1, compared with 3.2× for scalar conversion. Its absolute latency remained 15.1× higher. The benchmark timed complete transforms, including the shared FFT and map/coefficient access. It did not isolate recurrence, coefficient conversion, setup, or individual worker time. It measures no new persistent memory cost: the control used separate plans and workspaces of the same types, and both plans were built before timing. The `ntheta=lmax+2` choice excludes the compact scalar table and its setup cost.

## Direct fused recurrence prototype

The prototype fused the two spin-1 recurrence sequences and E/B accumulation into one Pulp-dispatched latitude loop. It held four output sums in SIMD vectors while updating the plus and minus spin bases. The scalar-conversion control used the existing `spin1_to_scalar()` coefficient conversion and optimized paired scalar recurrence. Both plans used identical f64 GL inputs and disabled the compact table. The grid used `ntheta=lmax+1`, `nphi=2*lmax+3`, `mmax=lmax`, and `phi0=0.37`; tested bands were T127 (`128x257`), T255 (`256x513`) and T511 (`512x1025`). Three separate processes were pinned with `taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}"`. Each process warmed both methods three times, then collected nine paired samples per thread count with execution order alternating. Plans, workspaces, coefficients and outputs were allocated before timing. The reported values are medians across the three process medians. Raw sample vectors and per-process medians are in [`direct-fused-raw-samples.tsv`](data/spin1-direct-vs-scalar-conversion-2026-10-14/direct-fused-raw-samples.tsv). The command was:

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo test --release -p rustsht \
  benchmark_spin1_direct_fused_synthesis -- --ignored --nocapture --test-threads=1
```

The host was an x86-64 CPU running Linux. Rust was `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`; Cargo's release profile used `opt-level=3`, 16 codegen units, no LTO and no `target-cpu=native`. Pulp selected CPU features at runtime; the CPU supports AVX2 and FMA. The same binary and flags ran both methods. Plan and workspace setup were outside the timed region. Transform timings include longitude FFTs and map/coefficient access. The prototype added four 64-lane `f64` stack arrays (2 KiB per active call) and no persistent plan or workspace storage. Peak RSS was not measured.

| Bandlimit and grid | Threads | Scalar conversion (ms) | Direct fused (ms) | Fused / conversion |
| --- | ---: | ---: | ---: | ---: |
| T127, `128x257` | 1 | 2.774 | 25.027 | 9.02x |
| T127, `128x257` | 8 | 0.916 | 4.685 | 5.12x |
| T255, `256x513` | 1 | 12.115 | 102.907 | 8.49x |
| T255, `256x513` | 8 | 4.530 | 35.524 | 7.84x |
| T511, `512x1025` | 1 | 59.297 | 402.586 | 6.79x |
| T511, `512x1025` | 8 | 21.311 | 254.590 | 11.95x |

Each process produced the same direction of change. At q=1, process-median times ranged from `2.762–2.784 ms` versus `24.767–25.249 ms` at T127, `12.112–12.310 ms` versus `102.905–106.224 ms` at T255, and `58.530–59.369 ms` versus `400.864–402.924 ms` at T511. At q=8, they ranged from `0.905–0.925 ms` versus `4.683–4.724 ms`, `4.439–4.535 ms` versus `35.443–35.529 ms`, and `21.284–24.242 ms` versus `252.530–267.243 ms`, respectively. The forced path preserved the tested map values and synthesis-adjoint identities. Against scalar conversion, relative-L2 map differences were `7.655556e-14` and `7.666404e-14` at T127 for q=1 and q=8, and `4.061990e-13` and `4.071504e-13` at T511. Maximum absolute errors were `4.772573e-10` and `6.415939e-8`. Relative synthesis-adjoint errors ranged from `6.40e-15` to `3.14e-14`. The T255 DUCC 0.41.0 sample test passed with the direct spin-1 path forced. These checks compare tested implementations; they do not establish independent analytic accuracy.


## Numerical comparison and DUCC context

Relative-L2 differences between the two RustSHT paths were `7.12e-15` synthesis and `5.75e-15` analysis at T63; `6.56e-15` and `6.08e-15` at T72; `1.11e-14` and `1.26e-14` at T127; `1.96e-14` and `2.31e-14` at T255; `3.99e-14` and `4.25e-14` at T511; and `5.86e-14` and `9.55e-14` at T720. These are cross-path agreement checks, not DUCC comparisons or independent analytic validation. The existing matched native RustSHT/DUCC 0.41.0 baseline at T511 uses `512x1025`, not this over-resolved `513x1025` grid. It measured spin-1 STANDARD synthesis at `59.210/24.561 ms` for RustSHT/DUCC with one thread and `21.672/4.361 ms` with eight; analysis measured `39.114/13.300 ms` and `12.942/1.921 ms`. This separate workload places the optimized Rust path at 2.41×/4.97× DUCC for synthesis and 2.94×/6.74× for analysis. The general-path control is substantially slower than both. Those DUCC results are from the [`independent-field study`](batched-transform-study-2026-10-14.md) and are not combined with the timings above. Existing spin-1 analytic, DUCC, adjoint, GL/CC/F1 and mode tests were not changed. Temporary tests exercised the prototype on GL T127, T255 and T511, then were removed after collecting the accuracy and timing results. No FFT implementation changed.

## Decision

Keep the scalar-conversion path and the general recurrence fallback. Neither the direct general recurrence nor this direct fused prototype improved whole-transform time. A different direct formulation would need to reduce recurrence work or avoid per-degree safety checks before another implementation is justified. The q=8 spin-1 STANDARD analysis gap remains a higher-value target: the scalar-conversion path scales weakly relative to q=1 and measured 6.74x the DUCC time on the matched T511 baseline.
