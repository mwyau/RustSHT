# Scaled-prefix accumulator T2047 route profile

## Method

The profile used the isolated tile-128 accumulator source from q1 protocol v1
(source SHA-256 `edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`).
The production extension remained unchanged at SHA-256
`48db7ad3a6ba4c7fb0a8c50a303d9fbe948a7cac989c6f8a6e39b2f5e1b40184`.

Test-only `Instant` timers were added to a temporary source copy. The patch is
[archived here](accumulator-prefix-route-timing.patch), SHA-256
`9c172821fd93710c9c54f738f60d9366648d1f96ec3a20ed6da749fe3a62d067`; the
instrumented source was SHA-256
`a068ce30e67ff96349af25f13893a488c5b63c0dd35984022ea4dcbbdea1cff8`. The
release test binary was SHA-256
`cb8298183c88b1583267ff75ad8e2047f9352e34128843d24e2c1bd7cecf5bde`. After
profiling, the frozen source was restored and its original hash verified.

The ignored `m6_fast_route_counts` test ran a GL f64 scalar T2047 transform
(`2048x4096`, full `mmax`, `phi0=0.37`, q1) after one unprofiled synthesis.
The timed call produced a map bitwise equal to the warm-up map. The release
test used all features and a single pinned logical CPU. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Hardware performance counters
were unavailable to the process, so the profile used test-only wall timers.

Reproduction command template:

```sh
RUSTSHT_FAST_ROUTE_LMAX=2047 \
RUSTSHT_FAST_ROUTE_THREADS=1 \
RAYON_NUM_THREADS=1 \
taskset -c "${CPUSET:?set one allowed logical CPU}" \
  cargo test --release --all-features --lib \
  m6_fast_route_counts -- --ignored --nocapture
```

## Route times

| Measurement | Calls | Time (ms) | Fraction of full call |
|---|---:|---:|---:|
| Full `synthesis_into` call | 1 | 1379.113 | 100% |
| `process_tile` total, including its nested work | 23,201 | 1331.970 | 96.58% |
| Fast pre-suffix transition, excluding the SIMD suffix | 23,165 | 715.931 | 51.91% |
| Ordinary SIMD suffix | 23,165 | 543.517 | 39.41% |
| Scaled fallback calls | 262 | 41.953 | 3.04% |
| Coefficient-safety scans | 2,048 | 5.393 | 0.39% |

The transition and suffix times are separate portions of the successful fast
route. Fallback and coefficient-scan times are nested within `process_tile`;
they are not additional whole-call costs. `process_tile` accounts for
1,331.970 ms of the 1,379.113 ms full call. Its remaining 47.142 ms includes
order preparation, packing, longitude FFT work, and code outside the tile
timer; this profile does not split those costs further.

## Route work and sampled prefix components

The transform processed 23,201 tiles: 23,165 completed the fast transition
and 36 declined because no safe transition was found. There were no empty
tiles or unsafe coefficient sets. Coefficient safety was scanned 2,048 times.
The route counters recorded 110,444,000 prefix lane-pairs, 1,619,628,108
ordinary-suffix lane-pairs, and 5,439,548 fallback lane-pairs, of which
1,331,458 were polar lanes.

The test timed one prefix pair in every 64 iterations, with an offset selected
per tile. It sampled 16,196 pairs covering 1,712,770 lane-pairs:

| Sampled component | Time (ms) |
|---|---:|
| Factor checks, exponent rebasing, and complex accumulation | 7.379 |
| Recurrence updates and exponent normalization | 3.690 |
| Transition range check | 0.377 |
| Accumulator materialization | 0.053 |

These are measured totals for the selected samples. They are not extrapolated
to the full prefix because lane lengths and timing overhead vary across tiles.

## Interpretation

The pre-suffix transition is the largest timed route component at 715.931 ms;
the ordinary SIMD suffix costs 543.517 ms. Scaled fallback and coefficient
safety scans are smaller in this run. Within the sampled prefix work, factor
checks, rebasing, and accumulation took the most time. That sample does not
establish their share of the complete transition.

The next kernel investigation should separate exponent-change and
normalization costs inside the pre-suffix transition, while preserving every
scaled-prefix contribution. This is a route profile of one instrumented q1
call, not an acceptance benchmark. The T511 control stopped at a failed DUCC
stability series, so the accumulator remains isolated and q8 and spin-1
regression measurements remain gated.
