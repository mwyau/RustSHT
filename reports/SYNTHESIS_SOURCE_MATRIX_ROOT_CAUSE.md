# General-grid synthesis source-matrix root cause

**Status: PRELIMINARY.** The large f64 gaps reproduce. Fine-grained attribution identifies an out-of-line Pulp/AVX2 intrinsic boundary in the scalar ordered synthesis path. A three-attribute inline-boundary candidate has large matched instruction and GL/CC spin-0 timing gains, with spin-1 timings unchanged. Wider parity and regression validation remain pending.

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

## Inline-boundary candidate: measured mechanism and current decision

The exact candidate parent is `39dde511`; the baseline extension was built from `c61ebcfb`, with only documentation changes between those revisions. Candidate source is `ef319aa9` and differs only by `#[inline(always)]` on `ScalarOrderedSynthesis::with_simd`, `synthesize_batch`, and `synthesize_ordinary_suffix_two_vectors` in `src/legendre/scalar_ordered.rs:460,803,1122`. The optimized extension was rebuilt with rustc 1.99.1 / LLVM 23.1.1 on `x86_64-unknown-linux-gnu`, using the default release profile, no custom `target-cpu` or `RUSTFLAGS`; Pulp runtime dispatch selected AVX2 `V3` on this Ryzen 9 5950X host. DUCC is the source-matrix build identified above. Callgrind profiles use the same warmed, one-transform client-request protocol as the parent profiles.

| GL T320 q1 | Parent Ir | Candidate Ir | Reduction | `_mm256_fmadd_pd` calls | `_mm256_set1_pd` calls |
|---|---:|---:|---:|---:|---:|
| spin 0 | 429.930M | 119.614M | 72.2% | 11.590M → 0.036M | 6.435M → 0.626M |
| spin 1 | 899.112M | 260.816M | 71.0% | 23.333M → 0.073M | 12.939M → 1.244M |

These are whole-program Callgrind instructions, not cycles. The candidate spin-0 `V3::vectorize::imp<ScalarOrderedSynthesis>` is 46.743M exclusive Ir; the candidate spin-1 `V3::vectorize::imp_fastcall<PreparedComponentBatch>` is 58.206M. In optimized AVX2 disassembly, the ordinary two-vector suffix now contains direct YMM `vfmadd*pd` instructions in its loop instead of repeatedly calling the six-instruction `_mm256_fmadd_pd` wrapper. The hot FMA-helper call count falls by 99.7%; `set1_pd` calls fall by about 90.3%. This confirms removal of ABI call/argument-marshalling work. It does not establish cycles saved by instruction category. The candidate still has a 0x2b8-byte scalar-kernel frame and visible YMM stack traffic in the hot loop; register spills were not eliminated. No claim of spill-free code is made.

A q8 process profile confirms the same instruction mechanism on scalar synthesis: GL T320 spin 0 parent/candidate/DUCC totals are **430.154M / 119.879M / 86.781M Ir**. The AVX2 helper-call counts are unchanged from q1 (parent/candidate FMA calls 11.590M/0.036M; `set1_pd` 6.435M/0.626M). DUCC's `alm2map_kernel` is again 28.725M Ir (33.1% of its q8 process profile). Thus q8 scalar synthesis uses the same Pulp/AVX2 boundary.

Spin-1 q8 is a separate route and does not share this gain: GL T320 parent/candidate/DUCC totals are **604.649M / 605.134M / 226.425M Ir**. The top Rust AVX2 helper counts are unchanged across parent and candidate, and DUCC's `alm2map_spin_kernel` is 110.741M Ir (48.9%). This matches timing equivalence at q8. The q1 spin-1 profile reduction is specific to its prepared-component path; q8 ordinary grouped recurrence does not execute that inlining boundary.

The first full GL T1023 spin-1 q1 Callgrind profile (parent `39dde511`, source-matrix geometry 2046×4096) completed after about two minutes of instrumentation. It records **22.030B Ir**; `scalar_ordered::synthesize_batch` is the largest exclusive source region at **14.170B Ir (64.3%)**, with **658.731M** calls to `_mm256_fmadd_pd` and **351.991M** calls to `_mm256_set1_pd`. The matched wall-time screen was 1298.178 ms parent versus 1283.110 ms candidate, so these static counts alone do not predict elapsed benefit. Candidate and DUCC T1023 profiles are pending; do not extrapolate their costs from T320.

The candidate profile records **4.215B Ir**, an 80.9% whole-transform reduction from that parent profile. `PreparedComponentBatch` contributes **1.378B exclusive Ir (32.7%)**. FMA-wrapper calls fall to **10.649M** (−98.4%) and `set1_pd` calls to **27.388M** (−92.2%). Yet the matched uninstrumented timing improves only 1.2%. This sharply separates Callgrind instruction reduction from wall-time savings on this spin-1 path. The generated candidate function also has repeated YMM stack traffic in its degree loop; whether those accesses are register spills or intentional stack-resident values remains unresolved. DUCC T1023 attribution is still pending.

Interleaved whole-transform screen: three rounds per cell, each sample averaging three calls, same fixture and CPU affinity for parent/candidate/source-matrix DUCC. Times are medians in ms; host load/runqueue data was not captured, so these remain a focused screen rather than quiet-host acceptance data.

| Workload | q | Spin | Parent | Candidate | DUCC |
|---|---:|---:|---:|---:|---:|
| GL T320 oversampled | 1 | 0 | 96.105 | 15.494 | 6.394 |
| GL T320 oversampled | 1 | 1 | 39.875 | 38.159 | 17.451 |
| GL T320 oversampled | 8 | 0 | 12.523 | 2.184 | 1.521 |
| GL T320 oversampled | 8 | 1 | 13.722 | 13.712 | 3.285 |
| CC T360 oversampled | 1 | 0 | 133.486 | 20.213 | 7.552 |
| CC T360 oversampled | 1 | 1 | 50.812 | 50.062 | 18.238 |
| CC T360 oversampled | 8 | 0 | 17.584 | 2.949 | 2.187 |
| CC T360 oversampled | 8 | 1 | 17.775 | 17.591 | 4.799 |
| GL T511 oversampled | 1 | 0 | 353.612 | 41.680 | 19.606 |
| GL T511 oversampled | 8 | 0 | 47.111 | 6.253 | 3.400 |
| GL T1023 oversampled | 1 | 0 | 2534.961 | 302.441 | 132.196 |
| GL T1023 oversampled | 8 | 0 | 340.496 | 45.929 | 24.871 |
| GL T1023 oversampled | 1 | 1 | 1298.178 | 1283.110 | 531.982 |

Candidate spin-0 synthesis is 5.7–8.5× faster than its parent across the eight GL/CC cells, including T511 and T1023. T1023 spin-1 q1 improves only 1.2%, which is not a material timing change. Rust remains slower than the source-matrix DUCC build in every listed cell. The simple inline-boundary source change is retained as a production improvement, subject to the wider regression matrix.

Parent/candidate output arrays are bitwise identical for GL T320 and CC T360 at q1/q8 for both spin 0 and spin 1, and for GL T511 at q1/q8 spin 0/1 plus GL T1023 q1 spin 1 (13 allocating transforms; max absolute difference 0). Focused checks actually run on this source: `cargo test --lib scalar_ordered` (18 passed, 2 ignored; includes extreme/subnormal fallback, exact CC/F1 comparison, q1/q8, and the >T511 parallel-order case); `cargo test --lib spin1_scalar_fast_synthesis_matches_general_channel_cases` (1 passed); `cargo test --lib spin1_fine_synthesis_groups_preserve_t720_adjoint_identities` (1 passed); `cargo clippy --lib -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. No tolerance was changed. Candidate extension size grew by 16,576 bytes (6,296,240→6,312,816; +0.26%); the change adds no branches, scratch storage or fallback state.

## Next measurement

- Finish normalizing the hot loop counts by active latitude × degree × channel updates, and capture the matched q8 profile/route where it changes.
- Add only a low-overhead test-only phase probe for the exact-q8 hemisphere route, then compare with oversampled q8. Capture CC T360 spin-1 q8 worker balance before attributing its poor scaling to recurrence.
- Complete the source-matrix DUCC GL T1023 spin-1 q1 profile, then normalize parent/candidate/DUCC exclusive PCs against active latitude/degree/channel updates. Explain why an 80.9% Callgrind Ir drop yields only 1.2% wall-time improvement and why the candidate remains about 2.4× slower than this DUCC binary. The matched benchmark is not the pinned-DUCC parity result; avoid extrapolating from T320.

`perf stat` failed because hardware events are unavailable to this process. Callgrind and disassembly are still available. Any percentages reported here are wall-time phase shares or explicit route counters, never Callgrind-cycle estimates.

## Callgrind and AVX2 attribution (PRELIMINARY)

Profiles isolate one allocating API transform with client-controlled Callgrind instrumentation, after a warm transform. Both sides use GL T320 oversampled geometry (640×1280), full mmax, q1, f64, one thread, the same fixture/adapter and a single CPU. Rust code is `97fbcbea` and code-equivalent to `c61ebcfb`; the source-matrix DUCC binary is from `31d9fe99`. Totals exclude import and plan setup, but include the transform's output allocation. Counts are instructions, not cycles.

| Workload | RustSHT total Ir | Rust top exclusive region | DUCC total Ir | DUCC top exclusive region |
|---|---:|---|---:|---|
| spin 0 q1 | 429.93M | `scalar_ordered::synthesize_batch` 253.00M (58.9%); AVX2 FMA wrapper 69.54M (16.2%); `set1_pd` wrapper 32.17M (7.5%) | 86.98M | `alm2map_kernel` 28.73M (33.0%); FFT `rfftp4` 23.64M (27.2%); `leg2map` worker 5.84M (6.7%) |
| spin 1 q1 | 899.11M | `scalar_ordered::synthesize_batch` 509.24M (56.6%); AVX2 FMA wrapper 140.00M (15.6%); `set1_pd` wrapper 64.70M (7.2%) | 226.53M | `alm2map_spin_kernel` 110.74M (48.9%); FFT `rfftp4` 47.27M (20.9%); `leg2map` worker 11.67M (5.2%) |

For spin 0, Callgrind attributes 11.59M calls to Rust's `_mm256_fmadd_pd` and 6.43M calls to `_mm256_set1_pd`. The AVX2 FMA wrapper is a six-instruction out-of-line body: two YMM loads, one FMA, a YMM store, `vzeroupper`, and `ret`. Its call sites repeatedly marshal vector arguments/results through stack memory. This is measured ABI traffic around intrinsic calls, distinct from inferring spills from the function's stack-frame size. DUCC's corresponding AVX2 `alm2map_kernel` contains 11 inline FMA PCs in the dominant inner loop, each executed 960,404 times (10.56M vector FMA instructions); it broadcasts coefficients directly and has no per-FMA calls. This is the same complete transform geometry, but the exact active-latitude/degree/channel normalization is still pending, so the count ratio is not yet treated as a recurrence-work ratio.

The spin-0 Rust `synthesize_batch` symbol is 16 KiB and has a 0x540-byte frame. Disassembly around its ordinary suffix shows repeated XMM stack transfers, pointer setup, and calls to the AVX2 intrinsic wrappers; the frame size alone is not being used as spill evidence. DUCC's `alm2map_kernel` is a small inlined AVX2 loop with broadcast loads, YMM recurrence/accumulator state and counted backedges. Rust's principal hot exclusive region is the recurrence and accumulation, not FFT or Python/API setup.

Two diagnostic builds help isolate the compiler mechanism. Thin LTO (one codegen unit) retained the intrinsic helper symbols and essentially all wrapper call sites. `#[inline(always)]` on `synthesize_batch` alone also retained 11.59M FMA calls and increased spin-0 whole-transform Ir slightly (429.93M→431.80M); this source trial was reverted. A separate `target-cpu=native` diagnostic emitted no AVX2 FMA wrapper calls and recorded 70.11M spin-0 Ir. That build is hardware-specific and changes code generation globally, so it is evidence that target-feature inlining explains most excess Ir, not a production candidate or a matched runtime result.

**Current root-cause hypothesis:** the generic Pulp callback/helper boundaries are not consistently inlined into the runtime-selected `V3` target-feature body. The resulting AVX2 intrinsics cross an ordinary ABI boundary, causing millions of call/argument-marshalling operations. This is directly supported by the disassembly and the target-native diagnostic. The smallest runtime-dispatch-safe inline boundary, its spin-1 effect, exact work normalization, and elapsed benefit remain unresolved.
