# GL T720 f32-I/O / f64-internal diagnostic — 2026-10-28

**Decision:** keep production native f32 unchanged. The scaled-to-normal suffix transition is blocked for real nonzero-exponent recurrence states: applying their exponents in f32 can underflow a nonzero state to zero while later coefficient multiplication still has a representable result. The diagnostic also shows that precision range is only part of the gap. RustSHT's existing f64 reflected order-major route closely matches DUCC, while the same all-ring paired traversal remains much slower. All four native-f32 GL T720 scalar-synthesis cells remain **GAP**.

## Revisions and method

- Repository head and isolated source worktree: `096216465ea35ab747ce977d5b65cf27dc4681f1`. The production transform source is unchanged from `54b166f2c0698f1c70e88f8679d12fa4376704fe`; only temporary profiling features and example hooks were added in the isolated worktree.
- Pinned DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host/compiler: AMD Ryzen 9 5950X, rustc 1.99.1 / LLVM 23.1.1, GCC 15.2.0. Builds used AVX2/FMA (`x86-64-v3`). q1 was pinned to CPU 3; q8 to physical CPUs 8–15.
- Geometry: GL T720, `ntheta=721`, `phi0=0.23`; full `mmax=720`, `nphi=1458`; reduced `mmax=180`, `nphi=378`. The f32 fixtures and hashes match `scripts/benchmark_ducc_f32_native.py`. f64 inputs are exact f32-to-f64 promotions made before timing. f64 map outputs are cast back to f32 outside timing for DUCC comparison.
- The uninstrumented whole-transform screen used three fresh-process, randomized/interleaved pairs per cell with a reusable plan/workspace and the same warm-call batch boundary. Process startup and setup are outside the reported batch medians. All workers reported finite output and repeat/reset consistency. This three-pair diagnostic is not a promotion-quality timing campaign.
- Separate feature-instrumented binaries measured one first-call Legendre stage and route counters. Those stage times are not mixed into the uninstrumented timing results. The forced-paired f64 build used a scratch-only `m6-force-paired` feature and `M6_FORCE_PAIRED_F64=1`; it is a diagnostic control, not a production route.

Binary hashes, fixture hashes, commands/build settings, per-pair results, per-worker profiles, and AVX2 extracts are preserved beside this report. The scratch-only profiling hooks and f64 route override are reproducible from the [isolated instrumentation patch](instrumentation.patch.gz). See [build provenance](build-provenance.json), [whole-transform pairs](whole-transform-pairs.json), [first-call profile summary](first-call-profile-summary.json), and [assembly summary](assembly-summary.json).

## Whole-transform timings

Medians of the three paired worker medians, in milliseconds. Ratios are paired Rust/DUCC medians. `f64 default` is the existing f64 reflected order-major route with f32 inputs promoted before timing; `f64 paired` forces the shared all-ring paired recurrence.

| GL T720 scalar synthesis | Native f32 ms | DUCC ms | f32 / DUCC | f64 default ms / DUCC | f64 paired ms / DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 723.093 | 19.548 | 36.802× | 15.229 / 0.779× | 91.416 / 4.680× |
| Full q8 | 112.588 | 2.756 | 40.487× | 2.129 / 0.769× | 33.869 / 12.260× |
| Reduced q1 | 218.628 | 8.606 | 25.376× | 6.418 / 0.746× | 31.960 / 3.714× |
| Reduced q8 | 57.919 | 1.254 | 46.062× | 0.947 / 0.755× | 16.851 / 13.442× |

The native-f32 ratios reproduce the earlier 15-pair source-specific baseline within run variation. The f64 default path reaches 0.75–0.78× DUCC on these diagnostic fixtures. Forcing that same f64 build through the all-ring paired traversal costs 3.7–13.4× DUCC, so the reflected-ring algorithm is a separate large factor beyond f32 range handling.

Native-f32 versus DUCC relative-L2 / maximum absolute error was `1.7234e-4` / `9.6441e-4` full and `2.7255e-4` / `1.5479e-3` reduced. The f64-default and forced-paired f64 outputs, cast to f32, were `3.89e-8` relative L2 full and `3.88e-8` reduced; maximum absolute errors were `1.19e-7` and `5.96e-8`. These are random-fixture comparisons, not independent analytic or exceptional-value acceptance tests.

## Legendre-stage profiles and useful work

Instrumented first-call stage durations in milliseconds; Rust measures the scalar Legendre/spectrum stage and DUCC measures `alm2leg`. These are diagnostic stage times, not whole-transform timings.

| Cell | Rust f32 paired stage | Rust f64 default reflected stage | Rust f64 forced-paired stage | DUCC `alm2leg` |
|---|---:|---:|---:|---:|
| Full q1 | 690.307 | 12.657 | 85.127 | 15.811 |
| Full q8 | 106.921 | 2.137 | 35.308 | 2.548 |
| Reduced q1 | 208.101 | 5.722 | 29.511 | 7.832 |
| Reduced q8 | 55.520 | 1.051 | 15.678 | 1.375 |

The f32 and forced-paired f64 profiles report identical all-ring work: 161.20M / 81.21M individual degree–latitude contributions, 80.70M / 40.64M paired accumulation slots, and 80.31M / 40.51M recurrence updates for full/reduced mmax. DUCC accumulates 80.73M / 40.66M northern-hemisphere degree–latitude terms, then reconstructs the reflected rings; it performs 40.32M / 20.31M suffix state updates. Thus DUCC obtains both physical rings from about half as many recurrence updates. Contribution and state-update units differ, and padded lanes are reported separately in the profile JSON.

The q8 f32 ring scheduler ran 23 group jobs and used all eight workers in both cells. In the instrumented full case, accumulated per-worker group time ranged from 58.40 to 106.71 ms (1.83×), with 642.96 ms worker-time sum and 106.92 ms stage wall time. Reduced ranged from 9.04 to 55.46 ms (6.13×), with 202.10 ms summed worker time and 55.52 ms stage wall time. This indicates greater reduced-mmax imbalance, but q1 remains 25–37× DUCC, so scheduling cannot explain the dominant disparity.

## DUCC path, RustSHT path, and machine code

Pinned DUCC's `init_lambda` initializes the two recurrence states plus a per-lane scale. `iter_to_ieee` advances the scaled prefix in two-degree pairs and renormalizes the state. `calc_alm2map` accumulates any remaining scaled prefix with its correction factor, then applies the residual correction to both states once, clears the scale, and enters `alm2map_kernel`. On these random T720 fixtures the counters saw zero skipped-prefix and zero scaled-prefix contributions: the useful terms entered the ordinary suffix. The suffix is unrolled over degree pairs and updates recurrence plus complex accumulation in AVX2/FMA vectors. DUCC's GL synthesis batches northern rings and `extract_a2m` reconstructs both north/south outputs from even/odd sums.

RustSHT's f32 `synthesize_paired_scalar_order_prepared_f32` runs the shared paired recurrence for all physical rings. `F32SynthesisAccumulator` tries a SIMD packet only when all lanes' exponent factors and intermediate coefficient, basis, product, contribution, and next sum pass its normal/finite checks. Otherwise it falls back per lane to `multiply_complex_scaled_f32`. The ordinary-range block kernel is real AVX2 f32 SIMD, but the scaled callback still repeats exponent checks and scalar component scaling for each degree contribution.

The current-source f64 default route is `scalar_ordered`, which is explicitly f64-only and a native translation of DUCC's reflected order-major algorithm. Its AVX2 suffix processes reflected ring data together. The f64 forced-paired control uses the shared all-ring paired traversal and is much slower than that route.

Executed V3/AVX2 disassembly confirms the difference. The f32 scaled-component helper body contains scalar `vmulss`, integer exponent extraction/adjustment, and many exceptional branches (239 static instruction lines and 35 branch instructions in the extracted function). The f32 ordinary fused block uses eight `f32` lanes and FMA, with dynamic per-degree range predicates. DUCC's active suffix uses four `f64` lanes, an unrolled sequence of packed FMA operations and no helper call in the loop. The f64 reflected suffix also uses packed FMA while producing both reflected-ring sums. Static function-body counts include multiple loop/tail/decline paths; they are not dynamic instruction counts. The compressed instruction extracts are included here for direct inspection.

The exact-source Callgrind comparison from the existing `54b166f2` screen reports, for one warmed q1 transform, `7,389,576,420` Rust versus `260,801,202` DUCC Ir at full mmax and `2,388,011,998` versus `108,641,459` at reduced mmax. Normalized by required output degree–latitude slots, that is 45.84 versus 1.62 Ir/full slot and 29.41 versus 1.34/reduced slot. `multiply_component_scaled_with_normal_factor_f32` alone accounted for 2.726B self Ir (39.62% of the candidate whole-call total in that profile), and the rejected suffix handoff did not reduce it. These are instruction references, not cycles or elapsed time; exact values and profile provenance are in the [packet suffix Callgrind data](../f32-packet-suffix-candidate/callgrind-work.json).

## Transition result and next hypothesis

The prior real-state replay at the same transform source found a full-q1 packet at `m=40` with exponents `-320..-160`. Applying those exponents to nonzero f32 recurrence states turned every state into zero, while scaled scalar multiplication still produced representable suffix terms. With that loss guarded, no nonzero-exponent suffix packet completed. See the [real-state replay](../f32-exact-zero-operand-suffix-rejection/README.md). A native-f32 one-time conversion of the recurrence states is therefore unsafe for the observed nonzero-exponent route.

The measurements point to two linked costs: repeated native-f32 scaled contribution work, and computing both reflected GL hemispheres as independent recurrence work. q8 load imbalance raises the reduced-cell cost but is secondary. The next algorithmic hypothesis is to reuse the existing reflected order-major strategy through a shared typed recurrence/accumulation path, keeping native-f32 SIMD and the exact scaled exceptional fallback. The current f64 implementation is hard-coded to `Complex64`; this diagnostic does not widen production f32 or establish that a generic route is safe or fast. Any candidate still needs exact-parent/DUCC timing, analytic and adjoint checks, poles, extreme/subnormal/overflow cases, SIMD tails, transactional reset, and protected f64 controls.

**No production code changed. M6 remains active and all native-f32 scalar-synthesis cells remain GAP.**
