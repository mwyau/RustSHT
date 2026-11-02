# RustSHT M6 performance status

**Session status: PAUSED.** No further experiment is authorized automatically. This is a source-specific status summary; see the [M6 ledger](reports/DUCC_PARITY_M6.md) for per-cell data and the [active plan](plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md) for the paused record.

## Accepted B3 f64 improvement

The accepted sectoral-seed handoff at source `f4344353` improved B3 spin-1 analysis whole-transform time by approximately 10–14% against its exact parent in 15 paired GL T720 full/reduced q1/q8 measurements. Against pinned DUCC f64, its ratios were 1.113, 1.124, 1.098 and 1.088 respectively: an 8.8–12.4% gap for that accepted source. A separate exact-source three-pair screen at `85f74d6d` measured 1.095–1.139× DUCC (9.5–13.9% slower); those preliminary results are not 15-pair certification. Across the source-specific results, the remaining gap is approximately 9–14%. The ordinary fused recurrence and two-channel accumulation remain the dominant areas of interest, but no further experiment is scheduled. Evidence: [seed handoff](reports/archive/experiments/m6-b3-seed-materialize-2026-11-02/README.md), [current-source screen](reports/archive/experiments/m6-b3-current-source-85f74d6d-2026-11-02/README.md).

Several alternatives were rejected, including recurrence fusion variants, range-check changes, channel splitting, ring-phase changes and code outlining. See the [B3 evidence index](reports/archive/M6_EXPERIMENT_INDEX.md) and the source-specific packages linked from the [ledger](reports/DUCC_PARITY_M6.md).

Callgrind found excess simulated instructions and differences in simulated branch and cache behavior. It does not establish their actual cycle costs. Linux hardware performance counters were unavailable on the tested host (`perf_event_paranoid=4`). A separately archived RDPRU stage probe is explicitly exploratory and does not change that limitation: [probe report](reports/archive/experiments/m6-b3-rdpru-stage-probe-2026-11-02/README.md).

## B1 f64 status

The source-specific 15-pair results against DUCC f64 at parent `792c8f36` were 1.059× full q1, 1.131× full q8, 1.047× reduced q1 and 1.032× reduced q8. Full cells remain GAP; reduced cells were INCONCLUSIVE. B1's bounded candidate attempts did not establish a material improvement. These values do not certify a newer source.

## Correctness and validation limits

The intermittent native-f32 nonfinite SIMD-tail issue remains unresolved; no independently validated fix is recorded. Existing tests and references pass only for their documented coverage, which does not establish complete numerical correctness or DUCC parity. The latest CI run checked at source `41caa81ab01f32fd28630d4a1e1ebe748454e6f8` passed: [Rust CI run 41455708541](https://github.com/mwyau/RustSHT/actions/runs/41455708541). Comprehensive tests and benchmarks were not rerun for this wind-down.

Future research, if separately authorized, can return to measured B1/B3 cycle attribution and the ordinary fused B3 recurrence/channel work. No automatic NEXT is set.
