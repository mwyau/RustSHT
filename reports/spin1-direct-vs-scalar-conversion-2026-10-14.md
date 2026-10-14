# Spin-1 direct recurrence and scalar conversion comparison

## Result

For f64 GL STANDARD transforms, the current spin-1 scalar-conversion path was faster than the general spin recurrence at every tested bandlimit. At T511 it was 19.8× faster for one-thread synthesis and 25.7× faster for one-thread analysis. At eight threads it was 6.8× faster for synthesis and 15.1× faster for analysis. At T720 the ratios were 11.9–28.8×. The direct general recurrence is not a viable replacement for the scalar-conversion path. No production code changed. The benchmark-only forcing control and test were removed after measurement. A new fused direct vector kernel was not prototyped: the measured general path performs far more work, so merging its callbacks would need to remove a large part of the recurrence and accumulation cost to catch the scalar-conversion path. This comparison gives no evidence that such a change would help.

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

## Numerical comparison and DUCC context

Relative-L2 differences between the two RustSHT paths were `7.12e-15` synthesis and `5.75e-15` analysis at T63; `6.56e-15` and `6.08e-15` at T72; `1.11e-14` and `1.26e-14` at T127; `1.96e-14` and `2.31e-14` at T255; `3.99e-14` and `4.25e-14` at T511; and `5.86e-14` and `9.55e-14` at T720. These are cross-path agreement checks, not DUCC comparisons or independent analytic validation. The existing matched native RustSHT/DUCC 0.41.0 baseline at T511 uses `512x1025`, not this over-resolved `513x1025` grid. It measured spin-1 STANDARD synthesis at `59.210/24.561 ms` for RustSHT/DUCC with one thread and `21.672/4.361 ms` with eight; analysis measured `39.114/13.300 ms` and `12.942/1.921 ms`. This separate workload places the optimized Rust path at 2.41×/4.97× DUCC for synthesis and 2.94×/6.74× for analysis. The general-path control is substantially slower than both. Those DUCC results are from the [`independent-field study`](batched-transform-study-2026-10-14.md) and are not combined with the timings above. Existing spin-1 analytic, DUCC, adjoint, GL/CC/F1 and mode tests were not changed. This experiment added no lasting tests and did not claim validation beyond the stated cross-path check. No FFT implementation changed.

## Decision

Keep the scalar-conversion path and the general recurrence fallback. Do not replace the fast path with the current general algorithm. The test did not measure a newly fused direct vector kernel, and this negative result does not rule out an algorithm that changes work or data movement. Such a kernel needs a concrete measured mechanism before implementation. The q=8 spin-1 STANDARD analysis gap is a better next target: the existing scalar-conversion path has weak scaling relative to its q=1 result and remains 6.74× slower than DUCC on the matched T511 baseline.
