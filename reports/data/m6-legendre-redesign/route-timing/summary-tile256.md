# ACTIVE_TILE_SIZE=256 route-timing diagnostic

The prebuilt test-only route-timing binary was run sequentially for GL scalar
f64 at T511 q1 and T2047 q1. No build or Python extension was changed. Each
test did one unmeasured synthesis to establish its reference map, then five
timed synthesis calls; it asserted bitwise map equality after every call. These are instrumented test timings, not full-transform acceptance results.

## Provenance and gates

- Worktree: `<local-home-path> HEAD
  `739275d20987fec35506916cb682f961e8a7ffaf`.
- `src/legendre/synthesis_fast.rs` SHA-256:
  `d94bb0b9b861c1ad0debade62180b2ad35ddbdd3a0c848deb3cd47b390389a85`. It matches `route-timing-instrumented-tile256.rs`; a diff against the
  archived tile-128 instrumentation shows only `ACTIVE_TILE_SIZE: 128 -> 256`.
- Test binary:
  `<local-home-path>
  SHA-256 `cb6eb68e1ef86ab41a4591d81b22d6b67105e7f04b09deb39effd2bd559514f8`.
- Exact test:
  `legendre::synthesis_fast::tests::m6_fast_route_counts`, invoked with
  `--ignored --exact --nocapture`, `RUSTSHT_FAST_ROUTE_THREADS=1`, and
  `RUSTSHT_FAST_ROUTE_LMAX=511` then `2047`; both commands used `taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}"`.
- The test's GL grid is `(L+1) x 2(L+1)`, full `mmax=L`, `phi0=0.37`.
- Host architecture: x86-64 with portable runtime dispatch. Test affinity matched the requested worker count. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Selected-core frequency readings remain in local, untracked captures.
- Five-sample stability gates passed for both cases: CV and first-three versus
  final-two median drift each remained below 5%. No samples were retried or
  removed.

## Results

| L | raw elapsed samples (ns) | median (ms) | CV | first-3/final-2 drift | gate |
|---:|---|---:|---:|---:|---|
| 511 | 44,724,646; 44,282,719; 44,274,484; 45,212,558; 44,475,253 | 44.475 | 0.785% | 1.262% | pass |
| 2,047 | 2,180,682,592; 2,172,977,164; 2,174,148,260; 2,174,152,668; 2,185,073,309 | 2,174.153 | 0.216% | 0.251% | pass |

Median aggregate route timers were:

| L | transition (ms) | prefix (ms) | suffix (ms) | fallback (ms) | tile (ms) |
|---:|---:|---:|---:|---:|---:|
| 511 | 39.040 | 29.227 | 8.767 | 2.256 | 41.755 |
| 2,047 | 2,100.737 | 1,549.009 | 508.819 | 12.247 | 2,128.151 |

Route timers are inclusive and nested: prefix/suffix are within transition,
and transition/fallback are within tile. They must not be added together or
read as exclusive CPU time. `elapsed_ns` measures the complete instrumented
synthesis call. Raw stdout, exact commands, actual affinity, process lists,
PSI and frequency readings are retained in `t511-tile256.*` and
`t2047-tile256.*`.
