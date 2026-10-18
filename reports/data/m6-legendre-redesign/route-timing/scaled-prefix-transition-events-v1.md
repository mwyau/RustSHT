# Scaled-prefix transition event profile

## Method

The ignored `m6_fast_route_counts` release test profiled one T2047 q1 GL f64
scalar synthesis. The workload used `lmax=2047`, full `mmax`, `phi0=0.37`, and
a `2048x4096` grid. A one-worker Rayon pool ran on one allowed logical CPU.
The test performed one unprofiled synthesis and one instrumented synthesis,
then compared the output buffers bitwise. The test passed.

The event-counter patch changed no recurrence arithmetic or routing. It counted
normalization calls and lane visits, exponent changes, scale probes, and
accumulator rebases. The source reconstruction used base commit
`739275d20987fec35506916cb682f961e8a7ffaf`, the frozen tile-128 integration
patch (`bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`),
the per-order safety-cache patch
(`2ee6fc08997a3f373412b6407164b16a81b36402df75931a5348682ca47f03cd`), and
the scaled-prefix accumulator patch
(`3a75e88d6c6d0640005caaa486e29d8fcbb8ecdd08c5419da9d015f39d736873`). The
source before event counters was SHA-256
`a068ce30e67ff96349af25f13893a488c5b63c0dd35984022ea4dcbbdea1cff8`. The
instrumented source is SHA-256
`5eb7276d2c460baf962c78ff887f42a59aff1f081463db7cf63ba4afb031a99c`; its
[patch](scaled-prefix-transition-events-v1.patch) is SHA-256
`fcbd67e225027b96372310cd3c1e5ff9d978b32797836e93f7c9d294f2012a85`. The
release test executable is SHA-256
`acd0ebef9413e228de7acf8d70245f114ac72b6c6541ef3a69f27922f556807a`.
The prospective method is recorded in
[protocol v1](scaled-prefix-transition-events-v1-protocol.md), SHA-256
`7579f2f6a63852aabfeb429d505f43eab083580ad1656add7588b261d9eb5043`.

The build used Rust and Cargo 1.99.0 on Linux x86-64. The host CPU was an AMD
Ryzen 9 5950X with 16 physical cores, 32 logical CPUs, AVX2 and FMA, and 96 GiB
of installed RAM. The release test ran with all Cargo features. The command
template was:

```sh
RUSTSHT_FAST_ROUTE_LMAX=2047 \
RUSTSHT_FAST_ROUTE_THREADS=1 \
RAYON_NUM_THREADS=1 RUST_TEST_THREADS=1 \
taskset -c "$ONE_ALLOWED_CPU" \
  cargo test --release --all-features --lib \
  m6_fast_route_counts -- --ignored --nocapture
```

Preflight and immediate postflight CPU-pressure checks passed the 2.00% gate.
No competing build or benchmark, cgroup throttling, or clear host interference
was observed. Exact pressure, frequency, placement, process, and GNU-time
captures remain local. The profiling test compiled in the same timed command,
so its aggregate peak memory does not measure transform memory.

## Counts and route times

| Event | Calls or events | Lane visits or absolute exponent shift |
|---|---:|---:|
| Initial pair normalization | 23,201 calls | 2,847,778 lane visits |
| Initial exponent changes | 0 lanes | 0 |
| Prefix pair normalization | 1,034,964 calls | 109,490,856 lane visits |
| Prefix exponent changes | 706,324 lanes | 565,059,200 total exponent steps |
| `pair_lanes_need_scaling` | 8,263 probes; 0 triggers | — |
| Accumulator rebases | 706,324 events | 565,059,200 total exponent steps |

The transform processed 23,201 tiles. Of these, 23,165 completed the ordinary
suffix and 36 declined the fast transition. The route recorded 262 scaled
fallback calls and no unsafe coefficient sets. It accumulated 110,444,000
prefix pair-lanes, 1,619,628,108 ordinary-suffix pair-lanes, and 5,439,548
fallback pair-lanes. Polar work accounted for 1,331,458 pair-lanes.

| Instrumented route timer | Calls | Time (ms) |
|---|---:|---:|
| Full `synthesis_into` | 1 | 1,415.152 |
| `process_tile` including nested work | 23,201 | 1,368.076 |
| Pre-suffix transition | 23,165 | 744.962 |
| Ordinary SIMD suffix | 23,165 | 550.916 |
| Scaled fallback | 262 | 41.368 |
| Coefficient-safety scans | 2,048 | 5.686 |

These timers include the event instrumentation. In particular,
`record_normalization` adds an exponent-array scan for each normalization; its
1,034,964 prefix calls cover 109,490,856 lanes. These times therefore cannot
be compared directly with the earlier route profile or used to estimate
un-instrumented candidate performance. The test checks bitwise agreement with
its warm-up output; it does not compare against DUCC in this invocation.

## Interpretation

The event counts identify repeated scaled-prefix work as the next measured
bottleneck. The prefix covers 6.36% of recorded pair-lanes but its instrumented
transition timer is 744.962 ms, compared with 550.916 ms for the suffix over
1.62 billion pair-lanes. The prefix normalizes about once per degree-pair
iteration. Only 8,263 iterations used the scale-probe path, and none of those
probes triggered normalization. Most prefix iterations therefore entered the
existing unconditional normalization path while the tile remained outside the
ordinary range.

Exponent changes and accumulator rebases were one-to-one in this workload.
They affected 706,324 lane updates, compared with 109,490,856 normalization
lane visits. The count does not show how many lanes carried nonzero exponents
at each degree. A bounded next diagnostic should measure that occupancy in the
existing counter pass. If scaled lanes occupy a small fraction of a tile, test
whether separating them from exponent-zero lanes lets the latter use the
ordinary SIMD recurrence while preserving the scaled path for every other
lane. No production change or performance acceptance follows from this profile.

The paired T2047 q1 accumulator result remains a separate whole-transform
measurement: the candidate/baseline median ratio was `0.407892`, the candidate
was `3.666x` DUCC latency, and its relative-L2 error against DUCC was
`3.015e-13`. The candidate is unaccepted because the T511 timing-stability
control failed and the complete regression matrix did not run. See the
[T2047 q1 results](scaled-prefix-accumulator-t2047-q1.md), the
[T511 stop record](scaled-prefix-accumulator-t511-q1-stopped.md), and the
[cumulative M6 report](../../../../m6-cpu-performance-parity.md).
