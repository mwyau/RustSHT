# Spin-1 order-major queue tile screen

**Decision: reject tile sizes 2 and 4.** Neither produced a repeatable
whole-call gain across full/reduced GL T720 q1/q8. Tile 4 was clearly slower
for reduced q8 and remained above DUCC for both q8 cells. Tile 2's reduced-q8
median landed near DUCC, but its three paired ratios ranged from 0.791x to
1.166x and candidate/parent from 0.816x to 1.025x. That result is
inconclusive, not evidence of parity. No production source changed.

## Provenance and protocol

- Exact source parent: `b3b93f17fe4d404fe4c033ffc7420eb190dc00b0` (same as
  fetched `origin/dev` during the screen).
- Each candidate changes only `SPIN1_ORDER_MAJOR_TILE` in
  `src/transform/spin.rs`: `1 -> 4` or `1 -> 2`. Patch SHA-256 values are
  `097195978361dbc804f5b903becacee3161e6e9a092be419bde7798379f86a3d` and
  `82db671db7a21924704f8e95589451fd296fe39b1373098e8426f2da78874d08`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust 1.99.1 / LLVM 23.1.1, `--locked --release --features parallel`,
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. DUCC used GCC 15.2,
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver added
  `-fno-fast-math`.
- Public f32-I/O/f64-internal Rust calls were compared with the native-f32
  Rust parent and native-f32 DUCC. Every timed call included input promotion,
  transform, FFT, narrowing and map writeback; plan/workspace setup was
  amortized outside warm batches. Each cell used three interleaved pairs,
  a 25 ms warmed-call target, q1 pinned to one physical core and q8 pinned to eight distinct
  physical cores.
- The benchmark runner SHA-256 is
  `4c866cf095aba235a04118028c03350a94a889162c98b5cfbe0d40bb9ac6a4ef`.
  Its emitted `candidate_change` label says tile 4 in both raw manifests; for
  the tile-2 run this label is stale. The preserved tile-2 patch, source diff
  and candidate binary hash identify the actual `1 -> 2` candidate.
- The focused workspace-reuse unit test passed (1 test). Candidate and parent
  outputs were bitwise equal in all 12 comparisons; every worker passed finite,
  repeat and poisoned-workspace reset checks. Maximum candidate/DUCC relative
  L2 error was `3.91e-8`, maximum absolute error `1.20e-7`. This scheduling-only
  screen did not repeat the independent analytic/adjoint suite.

## Whole public-call results

Times are medians in milliseconds. `C/P` and `C/D` are paired candidate/parent
and candidate/DUCC medians; bracketed values are the observed three-pair
minimum and maximum, not confidence intervals.

| Tile | GL T720 spin-1 synthesis | Parent / candidate / DUCC ms | C/P median [range] | C/D median [range] | Screen status |
|---:|---|---:|---:|---:|---|
| 4 | Full M720 q1 | 37.887 / 38.585 / 40.364 | 1.018 [1.010, 1.039] | 0.956 [0.935, 0.983] | q1 route unchanged; no promotion evidence |
| 4 | Reduced M180 q1 | 27.700 / 29.253 / 17.695 | 1.056 [1.055, 1.062] | 1.653 [1.576, 1.664] | GAP |
| 4 | Full M720 q8 | 9.333 / 8.393 / 5.886 | 0.899 [0.880, 1.019] | 1.395 [1.272, 1.532] | GAP; no stable gain |
| 4 | Reduced M180 q8 | 3.351 / 3.937 / 2.622 | 1.175 [1.066, 1.223] | 1.324 [1.012, 1.631] | GAP; repeatable parent regression |
| 2 | Full M720 q1 | 37.845 / 38.795 / 40.106 | 1.020 [1.018, 1.025] | 0.971 [0.945, 0.981] | q1 route unchanged; no promotion evidence |
| 2 | Reduced M180 q1 | 27.553 / 27.821 / 17.541 | 1.010 [0.998, 1.018] | 1.588 [1.571, 1.591] | GAP |
| 2 | Full M720 q8 | 8.732 / 8.671 / 5.884 | 1.000 [0.890, 1.092] | 1.474 [1.437, 1.624] | GAP; inconclusive vs parent |
| 2 | Reduced M180 q8 | 4.673 / 3.974 / 4.808 | 1.012 [0.816, 1.025] | 1.005 [0.791, 1.166] | INCONCLUSIVE |

The q1 branch is not selected by the tile constant because the order-major
queue requires at least eight execution threads. The q1 timings are controls
for the builds, not a tile effect. q8 CPU-frequency and pressure snapshots
varied substantially across pairs, consistent with the wide ranges. Tile 4's
full-q8 apparent median gain did not repeat in its third pair or in reduced
q8. Tile 2 did not materially improve the full-q8 median and did not establish
reduced-q8 parity. Neither candidate was extended to 15 pairs.

The tile-2 q8-reduced route probe reported eight order-major output-scratch
workers, confirming that the intended queue was active. It also recorded 118
generic per-degree fallback blocks. This focused direct-f64 test probe is route
evidence only; its phase timings are not public f32 whole-call measurements.

## Exact artifacts

- `results-tile2.json.gz` and `results-tile4.json.gz` retain all pair times,
  fixture/output hashes, errors, route checks and worker validity checks.
- `candidate-tile2.patch` and `candidate-tile4.patch` preserve both source
  candidates. `screen4.py` and both `manifest-*.json` files preserve the runner
  and build provenance.
- `route-profile-tile2-q8-reduced.log.gz` preserves the focused route probe.
- `SHA256SUMS.txt` verifies the durable package files.

## Next

Return to the largest unresolved public f32 spin-1 synthesis gap with a
cell-specific profile. Start with reduced GL T720 q1 (the 15-pair f32-internal
baseline was about `1.65x` DUCC); separate setup, recurrence/accumulation, FFT
and complete adapter conversion costs, then select one bounded source-level
change from the measured dominant stage. Keep the full/reduced q1/q8 cells and
protected f64 T1023 q1/q8 controls separate. M6 remains open.
