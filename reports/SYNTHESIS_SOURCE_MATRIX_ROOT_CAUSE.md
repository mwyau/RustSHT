# General-grid synthesis source-matrix root cause

**Status: PRELIMINARY.** The large f64 gaps reproduce. Current-source stage probes place most GL T320 work in recurrence and spectrum accumulation, but exclusive instruction attribution and AVX2 comparison are still pending. No production optimization has been selected.

## Revisions, binaries and protocol

| Item | Source / binary |
|---|---|
| Published RustSHT | source `579c09e5810045187e08f9525f7aaa4908a78adf`; Python extension SHA-256 `96e51f92183cb0ca6dc47d7da03affaf70f02a725d6a9f8567fbc175de59d09a` |
| Current RustSHT screen | source `c61ebcfb` (the subsequent `dd6f7804` commit changed docs only); locally built optimized extension in a fresh Python 3.14 environment |
| Source-matrix DUCC | `31d9fe99fefa5ca29df17b7e6d2362b8c5c4a05f`; extension SHA-256 `27067a9d69d226d301300cc12592b2c5650febb61d0074c1a20ec6addf2149d4` |
| Separate parity reference | pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; not used for the source-matrix rows below |
| Recorded runner | `sht_bench` commit `c2fc302`; recorded runner hash `db72cc9fcd8c6320d6fa8391daa95cf0f4cb13a80b4e8b8e307c2af9882d82e9`, equal to the committed `scripts/run_fast_scaling.py` hash. The checkout now has later uncommitted changes to that script and two unrelated files; those files were left untouched. |
| Host | AMD Ryzen 9 5950X, 16 physical cores, AVX2/FMA; perf counters unavailable (`perf_event_paranoid=4`) |
| Rust build | release/optimized wheel; rustc and Cargo 1.99.0. No explicit `RUSTFLAGS` or `target-cpu` was recorded. Pulp uses runtime architecture dispatch. |
| DUCC build | GCC 15.3.0, CMake Release, `-O3 -DNDEBUG -flto=auto -march=native -O3 -ffast-math`; the host has AVX2/FMA and no AVX-512 |
| Published timing protocol | one process round, one warm call, three timed samples, 5 ms calibration target, full `mmax=lmax`, native allocating calls. All 1,456 rows are marked `timed_successfully`; the run is explicitly timing-only, with no numerical-accuracy claim. |

The runner sets affinity to the first requested physical CPUs, sets `RAYON_NUM_THREADS` before importing RustSHT, and passes `nthreads=q` to DUCC. RustSHT constructs a reusable `ShtPlan` before timing; DUCC uses its public `synthesis_2d` call, with no reusable 2D plan exposed. Both allocate the output in the timed call. Coefficient generation, spin sign conversion, and output conversion are outside timing. The transform call and its returned output allocation are timed for both backends. The per-cell results record affinity but not load average, effective worker count, or runqueue wait, so the original q8 timings cannot exclude contention.

The published `timings.json` records `nlat=2*lmax` for GL and `nlat=2*lmax+1` for CC. The bounded exact-quadrature controls below hold `nlon` fixed and use `lmax+1` GL or `lmax+2` CC. The source-matrix DUCC revision remains distinct from the pinned parity DUCC revision.

## Published timing and independent reproduction

Published GL T320 and CC T360 f64 rows:

| Grid / `nlat×nlon` | q | Spin 0 Rust / DUCC | Ratio | Spin 1 Rust / DUCC | Ratio |
|---|---:|---:|---:|---:|---:|
| GL T320 / 640×1280 | 1 | 97.912 / 5.728 ms | 17.1× | 217.924 / 16.351 ms | 13.3× |
| GL T320 / 640×1280 | 8 | 12.927 / 1.103 ms | 11.7× | 15.894 / 2.375 ms | 6.7× |
| CC T360 / 721×1440 | 1 | 146.739 / 6.942 ms | 21.1× | 34.201 / 17.135 ms | 2.0× |
| CC T360 / 721×1440 | 8 | 20.289 / 1.430 ms | 14.2× | 18.000 / 3.288 ms | 5.5× |

I reran these same oversampled cases from the published RustSHT and DUCC Python environments using the recorded adapter, input fixture, affinity and allocating API. The three-sample medians stayed within roughly 2–10% of the published values; all large ratios reproduced. This is a bounded reproduction, not a quiet-host acceptance run.

Current-dev screen (three samples per cell, allocating API, source-matrix DUCC `31d9fe99`):

| Geometry | q | Spin 0 Rust / DUCC | Ratio | Spin 1 Rust / DUCC | Ratio |
|---|---:|---:|---:|---:|---:|
| GL T320 exact / 321×1280 | 1 | 50.029 / 3.099 ms | 16.1× | 101.232 / 8.510 ms | 11.9× |
| GL T320 exact / 321×1280 | 8 | 6.588 / 0.593 ms | 11.1× | 10.824 / 1.308 ms | 8.3× |
| GL T320 oversampled / 640×1280 | 1 | 98.039 / 5.759 ms | 17.0× | 200.734 / 16.311 ms | 12.3× |
| GL T320 oversampled / 640×1280 | 8 | 13.158 / 0.959 ms | 13.7× | 13.887 / 2.453 ms | 5.7× |
| CC T360 exact / 362×1440 | 1 | 68.232 / 3.946 ms | 17.3× | 19.284 / 11.405 ms | 1.7× |
| CC T360 exact / 362×1440 | 8 | 9.081 / 0.621 ms | 14.6× | 18.709 / 1.965 ms | 9.5× |
| CC T360 oversampled / 721×1440 | 1 | 137.040 / 6.744 ms | 20.3× | 34.910 / 17.266 ms | 2.0× |
| CC T360 oversampled / 721×1440 | 8 | 18.296 / 1.601 ms | 11.4× | 18.729 / 4.014 ms | 4.7× |

These short screens used the same three-sample timing policy as the source matrix and were run sequentially on the available host. Their purpose is route and scale comparison, not a final performance claim. The optimized current Rust extension was built from the active source with rustc 1.99.1; Python 3.14.9 and NumPy 2.5.4 matched the benchmark environments.

## Current Rust route and phase evidence

**Scalar GL T320:** test-only strategy inspection selected `Recurrence` for q1 and q8 on both exact and oversampled geometries. `synthesis_into` dispatches to the ordered scalar path for GL (`src/transform.rs:1164–1250`); q1 runs the serial ordered recurrence and q8 partitions orders through `synthesize_scalar_ordered_parallel` (`src/transform.rs:952–1157`). Thus the geometry change does not select a different scalar algorithm. Five in-place samples gave q1 about 50 ms exact and 98 ms oversampled; q8 was about 6.6 ms exact and 12.9 ms oversampled after excluding one warm outlier. The test-only q8 phase probe placed about 6.34 ms exact and 12.47 ms oversampled in ordered recurrence plus spectrum accumulation, versus 0.13 and 0.22 ms in the inverse FFT/writeback stage. The probe reports a nested `ordered scalar recurrence` observation as well as its enclosing Legendre/spectrum stage; those values overlap and must not be added.

**Spin-1 GL T320 q1:** a test-only counter reported 321 successful prepared-component orders for both exact and oversampled geometry. The current path is not declining that adapter on the oversampled grid. The per-degree fallback counters were 94 blocks exact and 110 blocks oversampled, with 11,398 and 26,566 reported fallback lane-pair updates respectively; these counters do not include the prepared-component route's main work. Low-overhead stage probes measured:

| Geometry | Total | Setup | Order recurrence and spectrum | Two inverse FFTs / writeback |
|---|---:|---:|---:|---:|
| GL T320 exact q1 | 100.0 ms | 0.14 ms | 98.28 ms | 1.56 ms |
| GL T320 oversampled q1 | 197.5 ms | 0.41 ms | 193.71 ms | 3.34 ms |
| GL T320 oversampled q8 | 13.13 ms | 0.32 ms | 12.17 ms | 0.61 ms |

The q1 probe used a test-only in-place transform and stage-boundary timers with per-block profiling disabled; its totals are close to the allocating Python screen. Earlier high-frequency block timers inflated q8 time and are not used in this table.

**Spin-1 geometry dispatch:** exact GL q8 T320 is eligible for the reflected-hemisphere route (`spin1_hemisphere_worker_count` returns 16 groups); oversampled GL q8 is ineligible and uses the general ring-group path. The route guard is exact `ntheta == lmax+1` in `src/spin_transform.rs:75–90`; the `hemisphere::synthesize` call occurs before the general stage probe at `src/spin_transform.rs:1185–1197`. Exact GL q1 and oversampled GL q1 both use the prepared-component order route in this case. Exact GL also permits spin-1 coefficient-table and coefficient-cache guards; oversampled geometry does not (`src/spin_transform.rs:1220–1232`, `1290–1310`). These differences are route facts; they do not explain the instruction gap yet. The exact q8 hemisphere path's disjoint phase timing remains unresolved because it returns before the current generic phase probe.

The q8 oversampled spin-1 route counters recorded 2,270 ordinary fused block successes over 12.85M lane-pair updates. They also recorded 534 checked fallback entries; for low orders, 292 transition successes and 116 generic per-degree blocks were counted, with 110 polar split attempts (96 successes, 14 declines). These are route counters from the current test-only profiler, not a full work-normalized Rust/DUCC comparison. Exact q8 hemisphere route work has not yet been normalized.

## Provisional findings and unresolved costs

1. **Largest measured region: Rust recurrence and spectrum accumulation.** For GL T320, this stage accounts for about 96–98% of current Rust spin-1 q1/oversampled q8 wall time, and ordered recurrence is the dominant scalar stage. FFT/writeback is only about 1–4% in these samples. This makes recurrence or its accumulation the strongest optimization area; it does not yet identify which instructions are excessive.
2. **Geometry work approximately doubles.** GL T320 scalar and spin-1 Rust q1 times roughly double when `nlat` changes from 321 to 640; the recurrence stage does likewise. Rust and DUCC both do more work, while their large ratio persists. Oversampling alone cannot explain the gap.
3. **Thread/route effects remain material.** Exact GL q8 spin-1 uses hemisphere synthesis; oversampled q8 uses ordinary grouped recurrence. CC T360 spin-1 q8 scales poorly on the exact control (18.7 ms versus 19.3 ms q1), while DUCC falls from 11.4 to 2.0 ms. The source-matrix q8 comparison has no load/runqueue capture; whether this is mainly kernel or scheduling cost remains unresolved.

The fair allocating API comparison and the current coarse phase probes rule out one-sided coefficient conversion, plan setup, output materialization, or FFT as explanations for the largest GL ratios. The source-matrix run itself did not validate numerical accuracy, so all timing rows remain exploratory.

## Next measurement before a code change

- Capture single-transform Callgrind Ir, executed PCs and disjoint exclusive symbols for current GL T320 q1 spin-0 and spin-1, matched with source-matrix DUCC `31d9fe99`; normalize loop counts by active latitude × degree × channel updates. Inspect AVX2 assembly for the actual Rust ordered scalar and prepared-component loops, plus DUCC's `calc_alm2map` / `alm2map_kernel` equivalents.
- Add only a low-overhead test-only phase probe for the exact-q8 hemisphere route, then compare with oversampled q8. Capture CC T360 spin-1 q8 worker balance before attributing its poor scaling to recurrence.
- Select one experiment only after those exclusive instruction and work-normalized comparisons. No source-level optimization has been attempted in this investigation.

`perf stat` failed because hardware events are unavailable to this process. Callgrind and disassembly are still available. Any percentages reported here are wall-time phase shares or explicit route counters, never Callgrind-cycle estimates.
