# GL T720 f32 scalar-synthesis 32-ring scheduling screen

This screen validates the 32-ring q8 split against exact production parent `9dc4516080eb0b25711ae74825c634ca71166ffa`, candidate `335ce4c87b416a5af89399bc0004032a6684adf6`, and pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. It changes only task grain for native-f32 GL T720 scalar synthesis. The recurrence and polar/scaled fallback are unchanged.

## Whole-transform timings

Warm whole-transform runs used randomized/interleaved fresh processes, x86-64-v3 native Rust and DUCC builds, one thread for q1 and eight threads for q8. There were three paired runs per q1 cell and fifteen per q8 cell. The paired candidate/parent intervals shown for q8 are percentile bootstrap 95% intervals over paired ratios.

| mmax | q | Parent ms | Candidate ms | DUCC ms | Candidate / parent | Candidate / DUCC | Numerical result |
|---:|---:|---:|---:|---:|---:|---:|---|
| 720 | 1 | 775.134 | 761.968 | 20.592 | 0.9826 | 37.75× | bitwise parent match |
| 720 | 8 | 160.606 | 125.180 | 3.262 | 0.7808 `[0.7331, 0.8321]` | 39.35× | parent rel-L2 `9.66e-8`, max abs `2.09e-7` |
| 180 | 1 | 235.473 | 231.698 | 8.808 | 0.9840 | 26.31× | bitwise parent match |
| 180 | 8 | 97.650 | 62.294 | 1.327 | 0.6420 `[0.6216, 0.6607]` | 46.63× | parent rel-L2 `8.14e-8`, max abs `1.79e-7` |

All 30 q8 paired comparisons favored the candidate. q1 medians are slightly faster and within the three-pair screen; q1 does not execute the parallel task path. Rust/DUCC remains far from parity in all four cells.

## Scheduling, allocation and route evidence

The isolated counter build showed that all eight workers ran jobs under both policies. The change did not increase the active-worker count; it made more jobs available and reduced load skew. Parent/candidate jobs per transform were 12/23. Median per-worker max/min group-time ratios changed from 3.24 to 1.70 at full mmax and from 6.56 to 3.49 at reduced mmax. Ring-indexed costs in [ring-costs.csv](ring-costs.csv) show why: full-mmax 64-ring jobs at the north/south ends take about 140–160 ms, while central jobs take 2–5 ms; splitting them limits the hottest 32-ring jobs to about 64–72 ms. Reduced mmax still has expensive polar-end groups, but the maximum measured group fell from about 107–124 ms to about 62–63 ms.

The residual `parallel_phase_wall - slowest_worker_group_sum` was 61–73 μs for parent full-mmax and 45–63 μs for candidate; for reduced mmax it was 20–26 μs and 18–38 μs. This includes dispatch, barriers and idle time; it is not a pure task-scheduler timer. Mixed-block eligibility scans total about 133 μs (parent) versus 84 μs (candidate) per full-mmax instrumented transform, and 18 μs versus 10 μs at reduced mmax. These costs are small beside recurrence and scaled accumulation.

`workspace()` was measured separately in three paired fresh-process runs, 100 allocations per process. At q8/full mmax, median workspace construction was 123 μs for parent and 3.53 ms for candidate; at q8/reduced mmax it was 58 μs and 84 μs. The 32-ring workspace reserves 23 recurrence workers rather than 12, increasing measured recurrence-worker payload from 805,392 to 1,543,668 bytes (`+738,276` bytes). Workspace construction is outside the warm-transform timings above; adding one construction to a single call still leaves the measured q8 transform gains substantial, while callers that reuse workspaces amortize it. q1 allocation medians were effectively unchanged at full mmax (2.68 ms parent, 2.69 ms candidate) and close at reduced mmax (40–43 μs parent, 41–44 μs candidate). Exact per-process samples are in [workspace-allocation.csv](workspace-allocation.csv).

The exact candidate patch and temporary instrumentation patch are preserved as [candidate.patch.gz](candidate.patch.gz) and [instrumentation.patch.gz](instrumentation.patch.gz); hashes of the uncompressed patches are in `results.json`.

The q8 route census was stable across three instrumented runs:

| mmax | source | total blocks | polar-inactive | mixed | fully fused | scaled-seed fallback | declines / retries |
|---:|---|---:|---:|---:|---:|---:|---:|
| 720 | parent | 8,652 | 1,937 | 1,216 | 1,744 | 3,755 | 0 / 0 |
| 720 | candidate | 16,583 | 3,837 | 1,199 | 4,337 | 7,210 | 0 / 0 |
| 180 | parent | 2,172 | 28 | 152 | 922 | 1,070 | 0 / 0 |
| 180 | candidate | 4,163 | 28 | 152 | 2,036 | 1,947 | 0 / 0 |

The candidate roughly halves full and reduced q8 time without changing the fallback contract. Per-transform callback accounting shows full mmax lane-pair visits falling from 52.70M to 48.18M and reduced mmax from 21.46M to 19.49M. The corresponding useful recurrence updates are 52.42M→47.93M and 21.39M→19.43M. Exponent-zero visits fall by the same 4.52M / 1.96M, but nonzero-exponent scaled contributions do not change: **13.165M full and 3.974M reduced**. That remaining scalar scaled-contribution path is the next arithmetic target. Scan/staging counts, recurrence updates, and per-worker totals are in [block-work.csv](block-work.csv), [routes.csv](routes.csv), and [worker-profiles.csv](worker-profiles.csv). These counters came from temporary isolated instrumentation and were absent from timed binaries.

## Adjacent controls and correctness

The f32 scalar-analysis controls were faster than the parent in the three-pair screen (candidate/parent 0.9548 q1 and 0.9358 q8, bitwise outputs). f32 spin-1 analysis was near parent (1.0007 q1, 1.0079 q8, bitwise). f32 spin-1 synthesis q8 was 0.9804 over three pairs and bitwise; an extended 15-pair q1 check was 1.0113 with 5/15 pairs faster and bootstrap interval `[0.9894, 1.0192]`. The q1 selector returns no fine group and the caller's `use_parallel` condition is false, so that small q1 movement is **not attributed to task scheduling**; its cause is unresolved.

Protected f64 GL T1023 spin-1 synthesis stayed bitwise identical. q1 candidate/parent was 0.9932 over three pairs; q8 was 0.9806 over fifteen pairs. No meaningful protected-cell regression was measured. Paired timings, output hashes and numerical errors for these controls are in [results.json](results.json).

Supplemental T720 f32 acceptance bounds are based on independent low-order analytic modes, the `l=m=720` sectoral mode, coefficient recovery, scalar adjoint identity, extreme values and pinned-DUCC output comparison. They do not relax existing test tolerances. Observed low-mode rel-L2/max-abs was `7.23e-8`/`5.96e-8`; sectoral rel-L2/max-abs was `1.07e-5`/`5.89e-5`; coefficient-recovery rel-L2/max-abs was `4.50e-5`/`4.59e-6`; adjoint relative error was `2.11e-5`. Smallest positive normal, smallest subnormal, `1e30`, and `f32::MAX` cases remained finite. Repeated calls, reset behavior and polar paths passed. The recorded supplemental criteria and observations are in `results.json`.

Focused routing test passed. `cargo test --locked --offline --features parallel --lib f32_` passed 17 tests (1 ignored), and `cargo test --locked --offline --features parallel --test f32` passed 11 (1 ignored); format and diff checks passed. No tolerance was changed.

## Next measured arithmetic experiment

Pinned DUCC's `iter_to_ieee` advances the scaled recurrence prefix while updating lane scales. Once every vector lane is in the ordinary range, `calc_alm2map` corrects the current recurrence states once and hands the suffix to `alm2map_kernel`, which vectorizes recurrence and coefficient accumulation in unit-stride SIMD. RustSHT keeps its native-f32 recurrence shared, but on scaled-seed blocks the synthesis callback still visits each degree/lane in scalar code and calls `multiply_complex_scaled_f32` for each nonzero exponent. The 13.165M/3.974M per-transform nonzero-exponent contribution counts make this the dominant bounded arithmetic opportunity; scans are two orders of magnitude smaller by elapsed counter time.

The proposed candidate is a **small native-precision SIMD scaled-contribution leaf inside the existing shared paired recurrence callback**. It will consume full contiguous f32 lane packets at each already-computed degree; it will not restart recurrence, find a suffix, compact lanes, or add a second f32 recurrence. A packet can use SIMD coefficient FMA, native-f32 products and per-lane exponent-field adjustment only when every lane's inputs and products are normal and the adjusted exponent remains normal. If any lane is subnormal, zero where the product is not normal, nonfinite, underflowing, overflowing, or in the tail, that packet uses the existing scalar helper unchanged. This directly targets the measured nonzero-exponent contributions and differs from the rejected safe-suffix visitor, mixed-lane mask, vector retry and exponent-zero-only fast path.

Before implementation, measure all-lanes-safe packet frequency and packet tails in an isolated build; then test the small precision-policy leaf with exact scalar equivalence and the T720 criteria above. Keep the f64 path and its GL T1023 spin-1 synthesis q1/q8 controls unchanged.
