# 128-ring versus 256-ring route timing diagnostic

This test-only comparison isolates the active-ring tile-size effect in the
scalar synthesis prototype. It does not accept the prototype or replace the
matched whole-transform benchmark against a clean baseline.

## Frozen candidate and source variants

- Frozen 128-ring candidate source: SHA-256
  `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`.
- Frozen 128-ring Python extension: SHA-256
  `5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`.
- Candidate integration patch: SHA-256
  `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`.
- Clean baseline source: commit
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; extension SHA-256
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Earlier 256-ring polar-split implementation remains archived in
  `../polar-split-screen/candidate-source.patch`, SHA-256
  `1b2cb8c11cd11ed396cd766f5b31a502ebe6973270a922c035f9cc5708a87f0f`.
- Instrumented route-test source snapshots are retained here. They differ
  only in `ACTIVE_TILE_SIZE` (128 versus 256): 128 source SHA-256
  `ac4854d7b6c07a93a65e2a8db2f64f38d95c26fb2c1e7f3152989915f99360c0`; 256
  source SHA-256
  `d94bb0b9b861c1ad0debade62180b2ad35ddbdd3a0c848deb3cd47b390389a85`. These test-only source changes were not compiled into the frozen Python
  extension. The 128 test binary SHA-256 was
  `a0d7592e37b3e1fbd74e412c476afaaf872833a385ae33746066810bcb6e2ff4`; the
  256 test binary SHA-256 is
  `cb6eb68e1ef86ab41a4591d81b22d6b67105e7f04b09deb39effd2bd559514f8`.

## Method and validity

Each diagnostic used the ignored `m6_fast_route_counts` test in a release
binary, a one-thread Rayon pool pinned to one CPU core, and a GL f64 scalar plan with
`ntheta=L+1`, `nphi=2(L+1)`, full `mmax=L`, and `phi0=0.37`. The test performed
one unmeasured synthesis, then five timed complete Rust synthesis calls and
asserted bitwise equality with its reference map after each call. The test
also recorded route counters and nested timers. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. No sample was removed or retried. Structured test samples and aggregate route results are retained. Raw command, stdout, preflight, postflight, stability, process-placement, and frequency captures remain local and untracked. The 128-ring measurements used the worker-count affinity policy; full placement and frequency captures were not retained in the committed summary. This is
route-diagnostic evidence only. The later matched whole-transform q1 screen
used the frozen Python extensions; its results and T511 stability stop are in
the [paired q1 record](../paired-q1-128/summary.md). The route timers are
nested and inclusive: prefix and suffix time are inside
transition time; transition and scaled fallback are inside tile time. Do not
sum them. Full-call elapsed time includes the rest of `synthesis_into`,
including the inverse longitude FFT. Timer and counter code exists only under
`cfg(test)`. The candidate release extension is uninstrumented. AVX2/FMA
assembly inspection confirms four-lane YMM suffix code; hardware FMA event
counters were unavailable, so suffix throughput below is logical lane-pairs
per second, not a PMU instruction rate.

## Results

### Complete instrumented synthesis and active-lane occupancy

| L | Tile | Five elapsed samples (ms) | Median (ms) | CV | Drift | Active lanes / tile capacity | Tile occupancy |
|---:|---:|---|---:|---:|---:|---:|---:|
| 511 | 128 | 31.364, 31.548, 31.489, 31.351, 31.289 | 31.364 | 0.30% | 0.54% | 204,678 / 228,096 | 89.73% |
| 511 | 256 | 44.725, 44.283, 44.274, 45.213, 44.475 | 44.475 | 0.79% | 1.26% | 204,678 / 249,344 | 82.09% |
| 2,047 | 128 | 1,695.458, 1,703.905, 1,699.594, 1,697.339, 1,697.716 | 1,697.716 | 0.17% | 0.12% | 2,849,114 / 2,969,728 | 95.94% |
| 2,047 | 256 | 2,180.683, 2,172.977, 2,174.148, 2,174.153, 2,185.073 | 2,174.153 | 0.22% | 0.25% | 2,849,114 / 3,086,592 | 92.31% |

The instrumented 256-ring calls were 41.8% slower at T511 and 28.1% slower
at T2047 than the 128-ring calls. At T2047, both variants processed the same
2,847,778 ordinary lanes, the same 711,341 four-lane SIMD blocks, and the same
2,414 scalar tail lanes. The SIMD lane-pair rate was also effectively equal:
3.064 billion/s for tile 128 and 3.061 billion/s for tile 256. Thus the
measured difference is not higher SIMD throughput from the smaller tile; the
ordinary suffix runs at the same rate.

### Route work and elapsed time at T2047

| Tile | Tiles | Prefix pair-lanes | Suffix pair-lanes | Fallback pair-lanes | Transition (ms) | Prefix (ms) | Suffix (ms) | Fallback (ms) | Tile total (ms) |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 128 | 23,201 | 110.61 M | 1,622.44 M | 1.33 M | 1,621.9 | 1,012.1 | 529.5 | 12.2 | 1,651.4 |
| 256 | 12,057 | 175.73 M | 1,557.32 M | 1.33 M | 2,100.7 | 1,549.0 | 508.8 | 12.2 | 2,128.2 |

The 256-ring batch performs 58.9% more scaled-prefix lane-pairs and spends
52.9% more time there. It saves 4.1% suffix lane-pairs and 3.9% suffix time,
but that does not offset the slower prefix. Tile occupancy is also lower by
3.63 percentage points. This is consistent with a wider tile grouping more
latitude states under one transition decision: a lane that needs a longer
scaled prefix keeps the whole tile on the costly scaled route. This is the
measured explanation for the 128-versus-256 difference; it does not establish
why 128 beats the clean production baseline yet. At T511, the 256-ring case also has 2.08 times as many scaled-prefix
lane-pairs (3.82 M versus 1.84 M), 69% more fallback lane-pairs (255,104
versus 150,912), and lower tile occupancy. The ordinary suffix lane-pair rate
remains similar (2.96–2.98 billion/s). The reusable per-worker arrays occupy
72 bytes per active slot (seven f64 arrays, one i64 exponent array, and one
`usize` ring-index array): 9 KiB at 128 slots and 18 KiB at 256 slots, before
allocator metadata. The tile-size scratch difference is therefore bounded to
9 KiB per worker; peak whole-process RSS is reported separately by the full
transform benchmark.

## Earlier tile-128 candidate

The 128-ring source and binary described above are an earlier candidate without
the scaled-prefix accumulator. Its instrumented calls explain the tile-size
difference but do not compare either tile against the clean baseline. Its
six-pair q1 whole-transform matrices passed at T1023 and T2047, with
candidate/baseline median ratios 0.679651 and 0.533182. The first T511 baseline
process then failed the fixed Rust timing-stability limits; no candidate mate,
retry, q8 cell or broader regression measurement followed. That candidate was
not accepted. Its numerical checks and timing summary are in the
[earlier paired q1 report](../paired-q1-128/summary.md).

The route timers point to the scaled prefix as the next measured kernel target:
at T2047 the 128-ring case spent 1,012.1 ms in the prefix and 529.5 ms in the
ordinary suffix. These nested test-only timers include instrumentation and do
not form an additive decomposition of whole-transform time.

## T2047 prefix component diagnostic

A follow-up test-only profile split coefficient validation from sampled work
inside the scaled prefix. It did not modify the frozen candidate mathematics.
The frozen source and extension hashes remain `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25` and
`5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`.
The diagnostic source was the frozen source plus
[`prefix-component-timing.patch`](prefix-component-timing.patch), with SHA-256
`58f0aa261940f6b3d3c0b9457fedde1a959c414b3a03330c343340305df4f385`; the
patch SHA-256 is `58250f994e5913f8a476328107d0e01d9550f73f448d79eeca667fd299d1350f`.

The ignored release test ran scalar f64 GL T2047, full `mmax`, `phi0=0.37`,
`2048x4096` (even longitude), one Rayon worker pinned to one allowed CPU. It
performed one unprofiled warm-up call and one profiled call; the latter was
bitwise equal to the warm-up output. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. This single diagnostic has no timing-stability interval and is
not an acceptance measurement. It used Rust 1.99.0, Cargo release settings,
and the AMD Ryzen 9 5950X x86-64 host with AVX2/FMA. The reproducible command
template is `RUSTSHT_FAST_ROUTE_LMAX=2047 RUSTSHT_FAST_ROUTE_THREADS=1
taskset -c "${RUSTSHT_CPUSET:?set one allowed CPU}" cargo test --release --lib
m6_fast_route_counts -- --ignored --nocapture`. The exact CPU ID, frequency,
context switches and peak RSS were not recorded. The accepted q1 matrix used
`2048x4097`; these route timings are a separate mechanism probe.

The profiled call completed 23,201 tiles. It reported 110,608,856 prefix
lane-pairs, 1,622,437,750 suffix lane-pairs and 1,331,458 fallback lane-pairs;
all fallback lanes were polar. There were no declined tiles. The coefficient
safety scan ran once per completed tile: 23,201 scans consumed 69.235 ms in
total. For one deterministic sample in every 64 prefix degree-pair iterations,
16,215 samples covered 1,715,048 lane-pairs. The sampled scalar accumulation
took 12.384 ms, recurrence updates took 0.618 ms, and range checks plus
normalization took 3.365 ms. These sampled component times are not scaled to
whole-prefix estimates; sampling overhead and variable tile lengths prevent
an additive interpretation.

The sample indicates that scalar complex accumulation is the largest of the
three sampled prefix components. The separately measured coefficient scan is
also repeated for every tile even though its inputs are shared across tiles of
one order. The later accumulator candidate addresses repeated scaled complex
accumulation; its new route costs have not yet been measured. The earlier
per-order safety-cache hypothesis was evaluated as a separate isolated
candidate and did not pass its paired screen.

## Scaled-prefix accumulator q1 matrices

A separate source keeps prefix sums in the recurrence's binary scale, rebases
them when a lane exponent changes, and materializes each sum at the ordinary
SIMD transition. The source and extension hashes are listed in the frozen
[q1 protocol](scaled-prefix-accumulator-q1-v1-protocol.md).

Six-pair q1 matrices passed at T1023 and T2047. Candidate/baseline median
ratios were `0.568982` (95% interval `0.563779–0.571237`) and `0.407892`
(`0.407274–0.408108`). The T2047 candidate median was 1370.613 ms versus
3360.075 ms for the baseline; candidate latency was 3.666x DUCC and 5.040x
SHTns. Relative-L2 error against DUCC was `3.015e-13`, with maximum absolute
error `1.110e-12`. Full samples and CPU-pressure validity decisions are in the
[T1023](scaled-prefix-accumulator-t1023-q1.md) and
[T2047](scaled-prefix-accumulator-t2047-q1.md) result reports.

The paired matrices measure complete transforms. A separate test-only T2047
profile of this accumulator source measured 715.931 ms in the pre-suffix
transition, 543.517 ms in the ordinary SIMD suffix, 41.953 ms in scaled
fallback calls, and 5.393 ms in coefficient-safety scans. The exact source and
test patch hashes, route counts, sampled prefix components, and timer scopes
are in the [T2047 profile report](scaled-prefix-accumulator-route-profile-t2047.md).
The earlier 1,012.1 ms prefix timing belongs to a different binary. The
accumulator's six-pair T511 control stopped
at baseline pair 03 because the DUCC series failed the fixed CV and drift
limits; no candidate mate or retry ran. The complete samples and CPU-pressure validity decisions are in the [stopped T511 record](scaled-prefix-accumulator-t511-q1-stopped.md).
The T511 control is unresolved, and no production route, q8 matrix, or spin-1
regression result is accepted. The next kernel investigation should separate
exponent rebasing and normalization costs within the pre-suffix transition.

## SIMD prefix accumulation screen

A tile-128 SIMD-prefix variant vectorizes factor formation and accumulation
across active latitude lanes. Its [v1.2 T2047 q1 screen](simd-prefix-screen-t2047-q1.md)
was inconclusive: the RustSHT series passed timing stability, but the DUCC
series failed (CV `19.812%`, drift `25.775%`). The RustSHT process median was
1143.564 ms versus 1367.195 ms for the scalar-prefix process, but this single
comparison is descriptive only. No retry or six-pair screen ran. The source
patch is preserved and remains isolated.

## Scaled-prefix transition event profile

A test-only event profile counted 1,034,964 prefix normalizations across
109,490,856 lane visits. It recorded 706,324 exponent changes and matching
accumulator rebases. Only 8,263 scale probes ran, with no probe-triggered
normalizations. The instrumented route timer measured 744.962 ms in the
pre-suffix transition and 550.916 ms in the ordinary SIMD suffix. The event
counter adds an exponent-array pass, so these times are diagnostic and are not
performance comparisons. The event counts support measuring scaled-lane
occupancy before testing a lane-class split. Full counts, source hashes, test
configuration, and interpretation are in the
[event profile report](scaled-prefix-transition-events-v1.md); its
[test-only source patch](scaled-prefix-transition-events-v1.patch) is
archived alongside the report. The accumulator candidate remains unaccepted
while the T511 stability control and full regression matrix are unresolved.
