# Scaled-lane occupancy profile

## Method

The ignored `m6_fast_route_counts` release test measured nonzero recurrence
exponents after each prefix normalization in one T2047 q1 GL f64 scalar
synthesis. The workload used full `mmax`, `phi0=0.37`, and a `2048x4096` grid.
A one-worker Rayon pool ran on one allowed logical CPU. The test performed one
unprofiled synthesis and one instrumented synthesis, then compared the output
buffers bitwise. The test passed.

The source before the occupancy counters was the event-profile-v1 source,
SHA-256
`5eb7276d2c460baf962c78ff887f42a59aff1f081463db7cf63ba4afb031a99c`. The
occupancy patch is SHA-256
`6f21105c61df39b015ded2fdd040b7159584d12f97fd084db777c8afffdaab91`; the
instrumented source is SHA-256
`e877d0bfab25fe3c643337526eb1d918f00e5374748192ce32408bcc7c6d3613`. The
release test executable is SHA-256
`eec00b2fbc1fe87fe72b24e5930d6dfe5b9817888d99f3c9ad6ccdb65dd529c8`. The
prospective [protocol](scaled-lane-occupancy-v1-protocol.md) is SHA-256
`5bcd423142fdbc022bc49063965ce04148090951c7d7351eebdf67581ff72186`.

The counter recorded the exponent distribution in the same loop that compared
pre- and post-normalization exponents. It added no second per-lane scan and
changed no recurrence arithmetic or routing. The CPU, compiler, grid, thread,
and pressure protocol match the preceding
[event profile](scaled-prefix-transition-events-v1.md). Preflight and
postflight checks passed the 2.00% CPU-pressure gate; neither entered the
warning band. No concurrent CPU-intensive build or benchmark, cgroup
throttling, or clear host interference was observed. Raw host and process
records remain local. This profile did not measure transform memory or
candidate performance.

## Occupancy counts

The run made 1,034,964 prefix normalizations over 109,490,856 lane-visits. Of
these visits, 53,737,170 (49.08%) had nonzero recurrence exponents after
normalization. The mean was 51.92 nonzero-exponent lanes per normalization,
and one normalization reached all 128 lanes.

| Nonzero-exponent lanes after normalization | Calls |
|---|---:|
| 0 | 8,263 |
| 1 | 17,880 |
| 2–4 | 58,812 |
| 5–8 | 66,024 |
| 9–16 | 112,173 |
| 17–32 | 177,105 |
| 33–64 | 243,530 |
| 65+ | 351,177 |

At least 53,030,846 of the 53,737,170 nonzero-exponent visits (98.69%) had no
exponent change during that normalization. The run recorded 706,324 exponent
changes across all prefix lane-visits. These are counts for the synthetic
T2047 coefficient workload, not a universal latitude distribution.

## Interpretation

Scaled exponents were common within each active tile: 57.46% of normalization
calls had at least 33 nonzero-exponent lanes, and 33.93% had at least 65. This
distribution does not support testing a split that isolates only a few scaled
lanes from a large ordinary group.

The preceding source showed that `scalar::normalize_pair_lanes` calls
`normalize_pair` for every nonzero-exponent lane, even when the pair mantissas
are already inside the supported normal range. The occupancy counts show that
most such visits did not change the exponent. The next isolated synthesis
hypothesis is to skip this no-op normalization when both mantissas are in
range, while keeping the tile in the scaled route whenever any lane exponent
is nonzero and calling the existing normalization path for every out-of-range
pair. This change must match the existing scaled recurrence for finite
extremes, subnormals, and polar lanes before any performance screen.

The accumulator candidate remains isolated and unaccepted. Its T2047 q1 paired
median ratio was `0.407892` versus the clean baseline, but its T511 baseline
stability control failed and the full regression matrix did not run. This
occupancy profile makes no performance acceptance decision.
