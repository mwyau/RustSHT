# Isolated fast-route accounting

This is a test-only route-accounting diagnostic for the isolated
`ACTIVE_TILE_SIZE=128` fast-kernel worktree. It is not a latency benchmark and
does not establish production performance.

## Source and executable

- Worktree: `<local-home-path> HEAD
  `739275d20987fec35506916cb682f961e8a7ffaf`, plus the pre-existing candidate
  edits shown in `build-preflight.txt`.
- Candidate source fingerprints: `candidate-tracked.patch` has SHA-256
  `737c45bbc946a51e686010ab780968cdaa9c44fa543ee8f505cd4b20baccece0`;
  `candidate-synthesis_fast.rs` has SHA-256
  `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`.
- Compiler: rustc 1.99.0, LLVM 23.1.1; Cargo 1.99.0. The filtered release
  test binary was built once with
  `cargo test --release --lib --features parallel legendre::synthesis_fast::tests::m6_fast_route_counts --no-run`. Build output, elapsed time, and host pre/post state are in `build.*`.
- Test executable:
  `<local-home-path>
  SHA-256 `8567c2467e7473e333eb8ae5f4165844a10907207033a68ac3d529d104a1ccca`. Exact test name: `legendre::synthesis_fast::tests::m6_fast_route_counts`.
- The test synthesizes once without counters, resets the per-worker counters,
  synthesizes again with counters, and asserts bitwise map equality. This is a
  same-source repeatability check, not an independent DUCC comparison. The host was an x86-64 CPU with visible 32 pinned CPU cores and an unlimited
cgroup CPU quota. q1 cases requested one CPU core; q8 requested 8 pinned CPU cores. Each raw
case has pre/post PSI and process snapshots, requested command, observed
affinity, and worker-thread placement. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. No competing build or benchmark was found before any case. The T2047 q1 command requested one CPU core, but its captured `taskset -pc` result
reported worker-sized CPU affinity; sampled test threads were on one CPU core. The q1 T511 and
T1023 captures reported affinity=pinned, and the q8 capture reported 8–15. This
affinity mismatch is retained and not hidden; no case was rerun.

## Aggregate route counts

Pair-lane counts approximate route work across latitude lanes and recurrence
pairs. For the route-mix percentages, the denominator is
`prefix_pair_lanes + suffix_pair_lanes + fallback_pair_lanes`. Prefix work may
be repeated by scaled fallback when a tile declines, so these proportions are
not unique-lane shares or elapsed-time fractions. `polar_pair_lanes` is a
subset of fallback accounting and is shown separately.

| L | q | tiles (done/declined) | prefix pairs | suffix pairs | fallback pairs | polar pairs | route mix: prefix / suffix / fallback | polar / fallback |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 511 | 1 | 1,782 (1,778/4) | 1,839,684 | 27,953,772 | 150,912 | 47,534 | 6.144% / 93.352% / 0.504% | 31.498% |
| 1,023 | 1 | 6,307 (6,307/0) | 11,589,102 | 213,105,236 | 310,920 | 310,920 | 5.151% / 94.711% / 0.138% | 100.000% |
| 2,047 | 1 | 23,201 (23,201/0) | 110,608,856 | 1,622,437,750 | 1,331,458 | 1,331,458 | 6.377% / 93.546% / 0.077% | 100.000% |
| 2,047 | 8 | 24,114 (24,114/0) | 108,128,094 | 1,624,918,512 | 1,331,458 | 1,331,458 | 6.234% / 93.689% / 0.077% | 100.000% |

At T2047, q1 and q8 have the same aggregate route-proxy total:
1,734,378,064 pair lanes. The q8 distribution across workers is:

| worker | tiles | route-proxy pair lanes | share | prefix | suffix | fallback | polar |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 1,382 | 111,539,555 | 6.431% | 24,747,552 | 86,126,274 | 665,729 | 665,729 |
| 1 | 4,096 | 268,684,284 | 15.492% | 0 | 268,684,284 | 0 | 0 |
| 2 | 2,785 | 223,546,401 | 12.889% | 25,397,788 | 198,148,613 | 0 | 0 |
| 3 | 3,794 | 263,418,792 | 15.188% | 5,101,202 | 258,317,590 | 0 | 0 |
| 4 | 2,785 | 223,546,401 | 12.889% | 25,911,168 | 197,635,233 | 0 | 0 |
| 5 | 1,382 | 111,539,555 | 6.431% | 22,159,005 | 88,714,821 | 665,729 | 665,729 |
| 6 | 4,096 | 268,684,284 | 15.492% | 0 | 268,684,284 | 0 | 0 |
| 7 | 3,794 | 263,418,792 | 15.188% | 4,811,379 | 258,607,413 | 0 | 0 |

The work-proxy distribution varies across workers; it must not be read as a
wall-time or CPU-time distribution. Each raw stdout contains the full counter
struct and the test pass result.

## Preserved harness check

An initial exact-name invocation omitted `--ignored`, so Rust's test harness
reported zero tests run. Its command and snapshots are retained with the
`.ignored-selection-check` suffix. The four actual cases then used
`--ignored --exact`; each reported `running 1 test`, passed, and asserted exact
map equality. All four commands and raw outputs are retained. No route counts
were taken from the zero-test harness check.
