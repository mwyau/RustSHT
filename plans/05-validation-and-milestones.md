# Verification, benchmarks and milestones

## Validation contract

A transform is not validated by a single roundtrip. For each supported geometry **GL, CC, F1**, spin, dtype, operator and grid sampling, compare to pinned DUCC output **and** at least one independent mathematical check.

Use the pinned Rust 1.99.0 stable toolchain for development, CI, release builds and benchmarks. Do not add nightly-only features without a demonstrated need. CI runs this toolchain; earlier compiler-version entries below record historical test results.

- **Synthesis**: constants, isolated normalized `Y_lm`, pure sectoral/near-sectoral modes, mixed random coefficients and high-`m` polar cases.
- **Analysis**: recover coefficients of independently synthesized bandlimited fields; compare to direct quadrature for sufficiently small GL problems; test CC/F1 high-bandlimit cases that ordinary low-resolution quadrature would not recover.
- **Adjoints**: verify separately `<S a, x>_map = <a,S* x>_alm` and `<A x,a>_alm = <x,A* a>_map` with the real harmonic inner product (factor two for `m>0` modes) and documented map dot product.
- **Spin/vector**: spin=0,1,2 standard cases, E/B pure components, geographic `(-v,u)` convention, analytic derivative identities. Spin 3 and 4 pass all-grid full-operation checks at `lmax=spin`, sparse synthesis checks through T2047, and selected exact-analysis coefficient checks through T1023; spin 5 and above remain outside the operation contract pending separate verification.
- **Derivative modes**: `GRAD_ONLY` equals STANDARD synthesis when curl coefficients are zero; check `DERIV1` signs/scaling and the corresponding **adjoint-synthesis** modes. `analysis_2d` remains STANDARD-only.
- **Grid geometry**: validate nodes, `4*pi` summed weights, CC poles, F1 half-grid offset, GL ordering and longitude `phi0` shift. For exact analysis, require `lmax<=ntheta-1` (GL/F1), `lmax<=ntheta-2` (CC), and `mmax<=floor((nphi-1)/2)`; test both `nphi=2*mmax+1` and `2*mmax+2`.
- **Layout and safety**: `mmax<lmax`, rejection of nonreal input `m=0` coefficients and nonzero `l<spin` coefficients, odd/even longitude, contiguous/strided and batched inputs where supported, aliasing, invalid sizes and integer overflow.
- **Threading and SIMD**: scalar and vectorized kernels agree within established tolerances; results are independent of thread count within documented floating-point reproducibility limits.

Use absolute and norm-relative errors with a sensible near-zero floor; report worst error and RMS/distribution, not just roundtrip l2. Define acceptance thresholds *before* evaluating benchmarks. Distinguish algorithmic error from known floating-point order differences; never relax a tolerance merely to hide failures.

**Source caution:** Use the mathematical cases in DUCC's Python test suite as a guide, but do not copy `python/test/test_sht.py` verbatim: its header is GPL-3.0-or-later. See the [license plan](01-ducc-provenance-and-license.md).

## Initial f64 acceptance

For GL spin-0 through T255, start with provisional relative L2 error `<=5e-11` for independently evaluated analytic fields and pinned-DUCC comparisons, and normalized adjoint identity residual `<=1e-10`. Define relative L2 error as `||computed-reference||_2 / max(||reference||_2, absolute_floor)`; choose and record a fixed `absolute_floor`, an absolute near-zero tolerance and worst absolute error before tests. These limits are targets, not achieved results. Define separate high-L, spin and f32 thresholds from numerical evidence, not from failing tests.

## FFT, quadrature and SIMD checks during M1

- Compare GL nodes/weights from the port against pinned DUCC and independent quadrature moments at `ntheta=1,2,3,99,100,101,127,255,1024` and available larger sizes. Check northern/southern symmetry and high-degree polar accuracy; account for DUCC's `long double` small-grid calculation.
- Compare the custom real-input FFT against direct full-complex RustFFT for odd/even `nphi=1,2,3,4,5,6,7,8,31,32,33` and larger lengths. Cover impulse, constant, Fourier modes, DC/Nyquist, phase shift, truncation and inverse scaling.
- Compare scalar and SIMD recurrences through T255 (and higher where feasible), including pole rescaling, near-sectoral modes, FMA differences and all implemented spin/adjoint modes. Do not infer support for AVX-512/NEON from compilation without execution.
- Benchmark one-thread, Rayon, scalar, AVX2 and AVX-512 paths on supported CPUs, reporting FFT, recurrence, packing, setup, workspace and total release-mode execution. Preserve a test-only scalar/ISA override if feasible without a new public backend API.

## Bandlimit and memory ladder

| Tier | Example | Gate |
| --- | --- | --- |
| Basic | T0, T1, T5, T42 | Normalization, shapes, signs, invalid inputs |
| CI | T127, T255 | Required spins 0–2; current spin-3/4 regression suite |
| High | T511, T1023 | Scalar and spin 1 first, then spin 2; preserve recorded spin-3/4 limits |
| Extended | T2047, T4095 | Opt-in high-bandlimit comparisons for spins 0–2; retain current spin-3/4 coverage |
| Stress | T8191+ | Explicit opt-in, track peak memory and numerical behavior |

Stress tiers are **not release promises**. Publish a tested maximum for each `(grid, dtype, spin/mode, operation)` rather than asserting a uniform numerical maximum from one case. Test minimum permitted ring count and oversampled ring count separately.

## Spin priorities

Scalar and spin-1 correctness, performance and release coverage take priority. Spin 1 covers horizontal vector winds, gradients, E/B decomposition, divergence and vorticity workflows. Optimize spin 2 after spins 0 and 1; spins 0–2 remain the required parity set. The current API and tested operation limit remains spin 4. Preserve spin-3/4 behavior and run its existing regression tests after shared-kernel changes, but defer new specialization and high-bandlimit expansion. Do not implement spin 5+ without new authorization.

## Milestones (approved D11 ordering)

**M0 — Project setup:** GPL license, citations, AGENTS.md, plans, Cargo scaffold and CI configuration.

**M1 — Complete GL operation family**

- **M1a:** Core Rust harmonic/grid specs, packed layout, faithful DUCC GL iterative/Bogaert node/weight algorithms, RustFFT plus a private real-input adapter, stable scalar f64 synthesis/analysis, scalar reference, `pulp` SIMD recurrence, coarse Rayon scheduling and initial release-mode profiling.
- **M1b:** GL spin=1/2 STANDARD synthesis/analysis, vector E/B and applicable GRAD_ONLY/DERIV1 synthesis on the shared DUCC-derived scaled SIMD recurrence. Design for higher spins; record validated range.
- **M1c:** GL adjoint synthesis and adjoint analysis; GRAD_ONLY/DERIV1 adjoint synthesis; inner-product proofs/tests, GL full-operation parity and high-`L` accuracy gate.

**M2 — Same engine, additional grids**

- **M2a:** CC geometry and analysis/adjoint-analysis prepared-CC resampling; test every GL-supported operator on CC, including near-minimal ring counts.
- **M2b:** F1 staggered geometry and its resampling/adjoint; repeat the full operation matrix.

**M3 — High-bandlimit acceptance:** Complete all-grid f64 parity for spins 0–2, analytic/DUCC comparisons and actual tested degree/spin limits at T511/T1023 and opt-in T2047+. Preserve the current spin-3/4 operation limit and regression coverage without expanding its high-bandlimit range.

**M4 — Performance tuning and dtype:** Optimize scalar and spin-1 SIMD, recurrence, accumulation, scheduling and scratch layout first, then spin 2. Verify f32 at affordable bandlimits and report reproducible DUCC benchmarks. Run current spin-3/4 regression tests after shared-kernel changes; defer new spin-3/4 specialization and high-bandlimit expansion.

**M5 — Python binding:** The PyO3/NumPy plan and procedural APIs, reusable per-plan workspace, safe borrowed inputs and caller-provided standard outputs are implemented. Maintain packaging, GIL, shape/dtype, aliasing and wheel-license checks.

**M6 — spharmgrid:** Optional RustSHT backend matching its scalar/derivative/vector adapter, with independent parity, thread behavior and packaging review. Keep DUCC default until tested.

## Long autonomous implementation sessions

For a new implementation session, start from `main` on a feature branch. Continue an active session on its existing branch. Use [00](00-scope-and-decisions.md) without routine human checkpoints. M1 and M2 operation work and the recorded M3 checks are implemented; M4 performance tuning remains active, while the M5 Python API exists and has additional performance work outstanding.

1. Continue scalar and spin-1 performance work, then optimize spin 2. Use pinned-DUCC comparisons, independent analytic checks and the spin-3/4 regression suite for shared-kernel changes.
2. Complete remaining M4 validation and f32 checks at affordable bandlimits. Compare AVX2/FMA with AVX-512 only on hardware that executes the relevant ISA.
3. Maintain M5 Python packaging and binding checks, then proceed to the optional M6 spharmgrid adapter when M4 priorities and numerical behavior are verified.
4. Commit each independently functioning, tested increment. Run relevant formatting, lint and numerical checks before commits, plus full suites periodically. Do not add unfinished placeholders, weaken tests, create temporary GitHub Actions jobs or commit large generated fixtures. Skip resource-intensive optional stress/benchmark tests with an explicit note rather than blocking other work.

End with branch and commit list, operators/grids/spins/degrees tested, reference revision, command results, blockers and skipped checks. Push only when explicitly authorized for that session; do not merge into `main`.

## Required evidence for each merged numerical milestone

- A mapped algorithm/reference source and source-provenance note if any code is adapted.
- Analytic and DUCC-backed tests demonstrating the claimed operator and grid.
- Explicit tested precision/spin/bandlimit/shape matrix with failure cases.
- Profiling only when performance is claimed; commit the benchmark methodology and environment, not large unexplained generated fixtures.

The milestone ordering is fixed by D11; exact spin range, release tolerances, threading defaults and high-`L` performance acceptance remain open in [00](00-scope-and-decisions.md).

## Recorded implementation and numerical checks

### GL geometry (2026-10-12)

`GridSpec` now constructs north-to-south GL colatitudes and ring weights for `ntheta >= 1`. The implementation uses the DUCC Newton iteration through 100 rings and its modified Bogaert/FastGL approximations above 100 rings. The Rust Newton path uses f64; DUCC uses C++ `long double` for this branch.

The comparison executable compiled `ducc0/math/gl_integrator.cc` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` with g++ 15.2.0 and `-O2`. It compared every colatitude and `2π`-scaled ring weight for `ntheta = 1, 2, 3, 99, 100, 101, 127, 255, 1024` (1,712 values per quantity). The largest absolute colatitude difference was `1.946e-15` rad. The largest absolute ring-weight difference was `7.416e-16`; the largest relative ring-weight difference was `1.607e-13`, at the small endpoint weights for `ntheta=100`. The outputs for `ntheta >= 101` matched at the printed f64 precision.

Independent checks cover closed-form `ntheta=1,2,3` rules, all polynomial moments through degree `2*ntheta-1` for `ntheta=1..16`, symmetry and `4π` weight sums at `ntheta=99,100,101,127,255,1024`, and invalid dimensions and `phi0`. On an AMD Ryzen 9 5950X with rustc 1.99.0, `cargo fmt --all -- --check`, `cargo test --workspace --all-features` (6 passed), and `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass. No transform operator or SIMD recurrence has been validated yet.

### Packed harmonics and real FFT adapter (2026-10-12)

`HarmonicSpec` stores inclusive degree/order bounds and nonnegative spin, computes a checked order-major packed length, and provides safe `(l,m)` indexing. Its input validator checks the packed length, real `m=0` coefficients, and zero coefficients below the spin floor. Public coefficients use a direct `num-complex` dependency and the crate's `Complex64` alias.

The private FFT adapter uses a half-length complex RustFFT with pair packing and Hermitian reconstruction for even longitude counts; odd counts use a full-complex RustFFT. Forward spectra use the unnormalized DFT convention. Inverse execution divides by the transform length. Plans hold the RustFFT plans and even-length twiddles; each caller-owned workspace holds transform and scratch buffers.

FFT comparisons use the full-complex RustFFT at lengths `1,2,3,4,5,6,7,8,31,32,33`, covering impulses, constants, cosine modes, a mixed row, phase-shifted modes, real DC/Nyquist edges and inverse scaling. Arbitrary valid half-spectra are also checked against a direct Hermitian inverse sum. The harmonic metadata tests cover triangular and rectangular layouts, invalid bounds, overflow, coefficient lengths, `m=0` reality and the spin floor. On the AMD Ryzen 9 5950X with rustc 1.99.0, all 15 Rust unit tests, the workspace clippy command with warnings denied, formatting check, and `uv run pytest python/tests` (2 passed) pass. These checks validate the metadata and FFT helper only; no SHT transform operator or SIMD recurrence has been validated.

### Scalar GL synthesis and analysis (2026-10-12)

`ShtPlan` now combines GL geometry, packed scalar harmonics, the real FFT adapter and sectoral normalization metadata. `ShtWorkspace` holds the per-ring Fourier spectra, longitude row, FFT scratch and a reusable epsilon array. The transform fills epsilon factors once per order and reuses them across latitude rings. Scalar synthesis allows the real Nyquist order on even `nphi` and folds its positive/negative pair into the real Nyquist bin. Exact scalar GL analysis requires `ntheta >= lmax+1` and `mmax <= floor((nphi-1)/2)`. Both operations reject nonzero spin until spin recurrences are implemented.

The scalar kernel uses DUCC's sectoral normalization, normalized three-term epsilon recurrence, shared binary rescaling and polar order cutoff, adapted from `sht_inner_loop.h` at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Independent closed forms cover `Y00`, `Y10`, `Y20`, `Y21`, and `Y22`; analytic synthesis and exact analysis use nonzero `phi0`, `mmax < lmax`, and both minimum odd (`nphi=5`) and minimum even (`nphi=6`) longitude counts for `mmax=2`. An even-longitude synthesis test covers the real Nyquist order.

Static transform references generated with DUCC package `ducc0==0.41.0` cover scalar synthesis and analysis at `lmax=mmax=2`, `ntheta=3`, `nphi=5`, `phi0=0.37`, and one thread. On an AMD Ryzen 9 5950X with rustc 1.99.0 release builds, full 256-by-513 T255 synthesis maps at `phi0=0.37` were compared against `ducc0==0.41.0` with one thread. A sectoral field (`a_255,255=1`) had maximum absolute error `1.710e-14`, relative L2 error `3.412e-15`, and RMS error `1.088e-15`. A mixed field populated each packed coefficient as `10^-5 * (((7l+11m) mod 19)-9 + i*(((13l+5m) mod 23)-11))`, with the imaginary part set to zero for `m=0`; it had maximum absolute error `5.558e-15`, relative L2 error `3.102e-14`, and RMS error `1.835e-16`. Analysis of the map `map[i,j] = sin(0.019i + 0.037j) + 0.3cos(0.071i - 0.013j)` produced maximum absolute coefficient error `8.876e-15`, relative error `2.229e-14` in the real harmonic inner product, and coefficient RMS error `2.275e-16`. Unit tests retain selected DUCC samples and coefficients for these cases. The tests cover f64, spin zero, GL, scalar S/A through T255; adjoints, multithread scheduling, spins and other grids remain unvalidated. The 30-test workspace suite, formatting check and clippy with warnings denied pass on Rust 1.89.0 and 1.99.0; `uv run pytest python/tests` passes 2 tests.

### Latitude-blocked SIMD recurrence (2026-10-12)

The scalar GL synthesis and analysis operations dispatch their normalized epsilon recurrences through `pulp 0.22.3`. Each order traverses contiguous latitude lanes in blocks of 64 rings, with per-ring binary exponents retained for high-degree scaling. The scalar recurrence remains the fallback. Workspace scratch holds three recurrence buffers and exponents for at most one block, plus cached latitude sines and cosines and the per-order polar cutoff mask for the full grid. The crate enables pulp's AVX2/FMA and AVX-512 backends for runtime CPU detection. This host is an AMD Ryzen 9 5950X with AVX2/FMA and no AVX-512, so AVX2/FMA and scalar fallback paths were executed; AVX-512 and AArch64 NEON were not executed.

The recurrence tests compare the scalar reference, pulp scalar fallback and runtime-selected backend for orders 0 through 8 and degree 32 over 11 latitudes, then compare order 255 through degree 768 over 11 near-polar and interior latitudes. A separate test forces both binary rescaling directions in independent lanes. These cases cover vector blocks, tail lanes and recurrence scratch reused across ring blocks. Analytic checks and DUCC 0.41.0 static references exercise the dispatched transform path through T255. The complete Rust workspace has 32 passing tests; format, clippy with warnings denied, and Python import tests pass on Rust 1.89.0 and 1.99.0.

The pre-Rayon benchmark mode measured T511 scalar GL synthesis and analysis on an AMD Ryzen 9 5950X with rustc 1.99.0, release profile, AVX2/FMA runtime dispatch and one thread. Each operation had one warmup followed by five timed samples; each reported sample is the median. Across three runs, the 64-ring-block synthesis medians were 1.592 s, 1.588 s and 1.611 s, and analysis medians were 2.282 s, 2.278 s and 2.295 s. With a single 512-ring block, the synthesis medians were 1.533 s, 1.532 s and 1.541 s, and analysis medians were 2.529 s, 2.541 s and 2.564 s. Comparing the medians of these run medians, 64-ring blocks reduced analysis time by 10.2% and increased synthesis time by 3.8%. The estimated numeric-array working set was 12.05 MiB, including input/output coefficients, map, Fourier spectra, grid arrays and workspace arrays, and excluding RustFFT scratch and executable/runtime allocations. This single-host measurement supports the block choice for analysis but does not establish a general speedup; the synthesis cost remains a tuning target.

### Coarse Rayon scheduling (2026-10-12)

With the `parallel` feature, synthesis partitions contiguous latitude groups across Rayon tasks. Analysis partitions disjoint order-major coefficient ranges across tasks. Each task uses a recurrence scratch set owned by `ShtWorkspace`; the serial path uses one set. A workspace starts with the active Rayon pool's size as its execution capacity, capped at 64, and `ShtPlan::prepare_workspace` can explicitly change that limit and reserve its recurrence scratch. Actual transform concurrency is bounded by both the workspace capacity and active pool. A pool change alone neither changes the capacity nor allocates recurrence workers. The real FFT row loop remains serial.

The 33-test workspace suite now includes one-thread versus two-thread comparisons for analytic synthesis and analysis on 131 latitude rings. DUCC 0.41.0 reference checks run through the parallel dispatch path on the available global Rayon pool. `cargo test -p rustsht --no-default-features` passes all 32 non-Rayon tests. The AMD Ryzen 9 5950X has 16 physical cores and 32 hardware threads; AVX2/FMA is available and AVX-512 is not. AVX2/FMA and scalar fallback paths passed recurrence checks. AVX-512 has not been executed. On Rust 1.89.0 and 1.99.0, the workspace tests, formatting check and Clippy with warnings denied pass; `uv run pytest python/tests` passes two tests.

The T511 harness measured one-thread and eight-thread runs with rustc 1.99.0, release profile, f64 and AVX2/FMA runtime dispatch. Each operation had one warmup and five timed samples per run. Across two runs, synthesis medians were 1.555 s and 1.564 s with one thread, and 0.227 s and 0.234 s with eight threads. Analysis medians were 2.276 s and 2.283 s with one thread, and 0.556 s and 0.563 s with eight threads. Ratios of the average run medians give 6.8x synthesis and 4.1x analysis speedups. Estimated numeric-array storage was 12.10 MiB for the eight-worker workspace, excluding RustFFT scratch and executable/runtime allocations. This timing covers the recurrence scheduling benefit; the forward and inverse FFT loops are still serial, and no explicit scalar-versus-AVX2 or AVX-512 timing was recorded.

Analysis now splits order ranges recursively with `rayon::join`, which removes the previous per-call job descriptor vector while retaining disjoint coefficient and recurrence scratch slices. A one-run T511 confirmation after this change measured synthesis at 1.561 s with one thread and 0.230 s with eight, and analysis at 2.301 s with one thread and 0.556 s with eight. These medians are not included in the two-run summary above.

### GL spin and derivative synthesis (2026-10-12)

Spin-1 and spin-2 GL plans now provide STANDARD E/B synthesis and analysis. The shared spin recurrence also supports synthesis with B set to zero (`GRAD_ONLY`) and scalar angular-derivative synthesis (`DERIV1`). The recurrence coefficients, half-angle initialization and normalization are adapted from DUCC's `sht_inner_loop.h` at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; the Rust implementation uses per-lane binary scaling and caller-owned latitude-block scratch. The public numeric contract remains spin 1/2. Spin 3 and higher, adjoints and non-GL grids remain unimplemented.

The static DUCC fixtures use `ducc0==0.41.0`. For spin 1 and 2, synthesis and analysis samples on GL 3x7 at `phi0=0.37` match the reference within `5e-14` for map samples and `8e-13` for selected E/B coefficients. Analytic checks cover the lowest-degree pure E modes, gradient equivalence with STANDARD synthesis at B=0, and the spin-1 scalar derivative identity. GL T255 tests cover mixed E/B synthesis samples, sectoral modes and selected analysis coefficients for both spins; the retained fixture tolerances are `2e-12` for synthesis samples, `3e-11` for sectoral samples, and `2e-10` for selected analysis coefficients. Exact analysis tests include minimum odd/even longitude lengths and rectangular `mmax < lmax` layouts. A one-thread/two-thread comparison covers synthesis and analysis on GL 131x11. The 41-test Rust workspace passes with Rust 1.89.0 and 1.99.0; Clippy with warnings denied passes on both, format checks pass, and `uv run pytest python/tests` passes two tests. Current evidence is f64, GL, spin 1 and 2, STANDARD synthesis/analysis and gradient/derivative synthesis through T255; it does not yet include an adjoint identity or full coefficient/map error distribution, which remain M1c validation.

Adding spin scratch also increases each workspace's numeric-array estimate. A single T511 run on the AMD Ryzen 9 5950X with rustc 1.99.0, release profile, f64 and AVX2/FMA runtime dispatch measured synthesis at 1.572 s with one thread and 0.232 s with eight, and analysis at 2.264 s with one thread and 0.546 s with eight; each value is the median of five timed samples after one warmup. The harness estimates 16.23 MiB for the eight-worker numeric arrays, including a second Fourier spectrum and per-worker positive/negative spin recurrences and factors; RustFFT scratch and executable/runtime allocations remain excluded. This one-run measurement records the current workspace footprint and does not replace the two-run M1a timing summary above.

### GL adjoints (2026-10-12)

Scalar GL plans now provide `S*` and `A*`. Spin-1/2 plans provide STANDARD `S*` and `A*`, and the gradient-only and scalar DERIV1 synthesis modes have their own adjoint-synthesis operations. Exact analysis and its adjoint enforce `ntheta >= lmax+1` and `mmax <= (nphi-1)/2`. Synthesis and its adjoints accept `mmax <= nphi/2`, including even-longitude Nyquist. The `S*` map product is an unweighted pixel sum and the real harmonic product weights `m>0` by two; `A*` scales synthesized samples on each GL ring by `ring_weight/nphi`.

Static DUCC fixtures use `ducc0==0.41.0` and the pinned transform implementation at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Scalar `S*`/`A*`, spin-1/2 STANDARD `S*`/`A*`, GRAD_ONLY `S*`, and DERIV1 `S*` are checked at GL 3x7 with `phi0=0.37`; scalar, spin and gradient STANDARD `S*` are also checked at GL 3x4 with the Nyquist order and nonzero `phi0`. The tests check separate adjoint identities rather than roundtrips. T255 identities cover scalar `S*`, `A*` and DERIV1 `S*`, and spin-1/2 STANDARD `S*`, `A*` and GRAD_ONLY `S*`; the scalar identity tolerance is `3e-10` relative to `max(|reference|,1)`, and the spin identity tolerance is `2e-8` absolute. At M1c, `cargo test --workspace --all-features` passed 47 tests with rustc 1.99.0 and 1.89.0; `cargo test -p rustsht --no-default-features` passed 45 tests. Formatting and Clippy with warnings denied passed on both compiler versions, and `uv run pytest python/tests` passed two tests.

### CC geometry and prepared-grid analysis (2026-10-12)

`GridSpec` constructs CC colatitudes at `theta_i = i*pi/(ntheta-1)` with both poles and the FFT-based Waldvogel weights. `ShtPlan` prepares the CC resampling grids and FFTs. Exact analysis maps longitude Fourier modes to the prepared CC grid, applies the CC meridional weights and projects with the scalar or spin recurrence. `A*` synthesizes on that prepared grid and applies `resample_from_prepared_CC`; `S*` continues to use the unweighted grid projection. Exact CC analysis requires `ntheta >= lmax+2` and `mmax <= floor((nphi-1)/2)`. The prepared ring count follows DUCC's `good_size_complex(lmax+1)+1` rule. The translated code in `grid.rs` and `meridional.rs` cites DUCC revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; the FFT-size helper uses the GPL-2.0-or-later option of the DUCC FFT header's dual license.

Static geometry references from `ducc0==0.41.0` cover CC weights at `ntheta=2..5`; independent moments through degree `ntheta-1` and `4*pi` sums pass for `ntheta=2..20`. Scalar and spin-1/2 `S`, `A`, `S*` and `A*` references cover the minimum CC ring count at `4x5` with `phi0=0.37`, including adjoint identities. Scalar and spin analysis also cover the minimum even longitude count (`nphi=6`), and scalar and spin `A`/`A*` cover oversampled CC rings at `7x5`. Scalar DERIV1 and spin-1/2 GRAD_ONLY synthesis and their adjoints have static DUCC references and adjoint identity checks at `4x5`. The scalar T255 sectoral field at minimum `ntheta=257`, `nphi=513` agrees with its closed-form map within `4e-12` at every sample; exact analysis recovers every packed coefficient within `2e-9`.

The M2a tests cover f64, scalar and spin 1/2, CC, scalar T255 and low-degree derivative and spin modes through T2. F1, f32 and spin 3+ are not implemented. On an AMD Ryzen 9 5950X, `cargo test --workspace --all-features` passed 57 tests with rustc 1.99.0 and Rust 1.89.0; `cargo test -p rustsht --no-default-features` passed 55 tests with rustc 1.99.0. Formatting and Clippy with warnings denied passed on both compiler versions, and `uv run pytest python/tests` passed two tests.

### F1 geometry and prepared-grid analysis (2026-10-12)

`GridSpec` constructs pole-excluded F1 colatitudes at `theta_i = (i+1/2)*pi/ntheta` and Waldvogel weights. The prepared-CC operator handles F1's reflected Fourier profiles with the half-ring phase shift, truncation or padding to the prepared CC grid, and the matching adjoint resampling. Exact F1 analysis requires `ntheta >= lmax+1`; longitude limits match GL and CC. The implementation and quadrature translations cite DUCC revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

Static `ducc0==0.41.0` references cover scalar and spin-1/2 `S`, `A`, `S*`, `A*`, scalar DERIV1 and spin GRAD_ONLY at the minimum F1 ring count (`3x5`). Adjoint identities are checked for every operation family. Minimum even longitude tests (`3x6`) cover scalar and both spins; oversampled rings (`7x5`) cover scalar and both spins. F1 weights match DUCC at `ntheta=1..6`; independent moments through degree `ntheta-1` and `4*pi` sums pass for `ntheta=1..20`. At the scalar T255 minimum grid (`256x513`), synthesis agrees with the sectoral closed form at every sample within `4e-12`, and analysis recovers all packed coefficients within `2e-9`.

The M2b validation domain is f64, GL/CC/F1, scalar and spin 1/2, scalar T255, and low-degree derivative/gradient modes through T2. Spin 3+ and f32 remain unimplemented. On an AMD Ryzen 9 5950X, `cargo test --workspace --all-features` passed 65 tests with rustc 1.99.0 and Rust 1.89.0; `cargo test -p rustsht --no-default-features` passed 63 tests with rustc 1.99.0. Formatting and Clippy with warnings denied passed on both compiler versions, and `uv run pytest python/tests` passed two tests.

### High-bandlimit f64 checks (2026-10-12)

Opt-in release tests extend scalar sectoral synthesis and exact analysis to T511, T1023 and T2047 on GL, CC and F1. The tests use `nphi=2*lmax+3`, minimum exact-analysis ring counts, and compare every synthesized pixel with the closed-form sectoral field. Five middle-ring values per grid and degree are compared with static outputs from `ducc0==0.41.0`. Analysis checks every packed coefficient against the input sectoral coefficient. The maximum errors across the three grids were:

| Degree | DUCC sample max absolute error | Analytic map max absolute error | Analytic map RMS error | Analysis coefficient max absolute error | Analysis coefficient RMS error |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | `5.773e-15` | `1.742e-12` | `5.489e-14` | `1.288e-14` | `3.556e-17` |
| T1023 | `1.257e-14` | `4.132e-12` | `1.070e-13` | `1.199e-14` | `1.657e-17` |
| T2047 | `2.083e-13` | `9.958e-12` | `2.241e-13` | `4.197e-14` | `2.898e-17` |

A second opt-in release test covers spin-1 and spin-2 synthesis and analysis on all three grids at T511 with minimum exact-analysis sampling. Static DUCC references cover five middle-ring samples for each map component and selected coefficients from independently generated maps; analysis of the synthesized sectoral fields checks every E/B coefficient. Across the six `(grid, spin)` cases, the largest DUCC synthesis-sample error was `8.837e-14` (largest case RMS `5.464e-14`), the largest selected DUCC analysis-coefficient error was `2.533e-17` (largest case RMS `1.446e-17`), and the largest synthesis-analysis coefficient error was `3.575e-14` (largest case RMS `6.976e-17`).

A third opt-in release test covers sparse spin-1 and spin-2 synthesis at T1023 on all three grids with 65 rings and `nphi=2049`. Static `ducc0==0.41.0` fixtures compare five middle-ring samples for each map component. The largest fixture error across the six `(grid, spin)` cases is `2.154e-13` (largest case RMS `1.359e-13`). STANDARD synthesis with B set to zero agrees with GRAD_ONLY synthesis at every map sample. The separate exact-sampling test below covers T1023 spin analysis.

A fourth opt-in release test covers spin-1 and spin-2 synthesis and exact analysis at T1023 on GL/F1 with 1024 rings and CC with 1025 rings; each uses `nphi=2049`. Static DUCC fixtures compare five middle-ring samples per component and selected coefficients from independently generated maps. The largest DUCC synthesis-sample error across the six `(grid, spin)` cases is `3.020e-13` (largest case RMS `1.901e-13`), the largest selected analysis-coefficient error is `9.022e-18` (largest case RMS `4.533e-18`), and the largest synthesis-analysis coefficient error is `6.839e-14` (largest case RMS `6.677e-17`). Analysis of each synthesized high-degree mode checks every E and B coefficient.

An opt-in release test checks sparse spin-1 through spin-4 synthesis at T2047 on GL, CC and F1 with 65 rings and `nphi=4097`. A single E-sectoral coefficient is synthesized with zero B coefficients. Every map sample is compared with the closed form for the normalized sectoral spin harmonics. Across grids, the maximum DUCC sample error, analytic map error and analytic RMS error for each spin are:

| Spin | DUCC sample max absolute error | Analytic map max absolute error | Analytic map RMS max |
| ---: | ---: | ---: | ---: |
| 1 | `2.146e-13` | `1.344e-11` | `5.274e-13` |
| 2 | `2.112e-13` | `1.236e-11` | `3.883e-13` |
| 3 | `2.169e-13` | `1.218e-11` | `4.259e-13` |
| 4 | `2.096e-13` | `1.013e-11` | `2.122e-13` |

Five middle-ring samples per component are compared with static `ducc0==0.41.0` references. The three middle rings lie at the equator. The expanded test ran in 33.80 s in release mode on an AMD Ryzen 9 5950X with rustc 1.99.0 and AVX2/FMA runtime dispatch; DUCC used one thread.

A routine test validates spin 3 and 4 on all three grids at `lmax=mmax=spin`, with minimum exact-analysis latitude counts and `nphi=2*lmax+3`. Synthesis of the `a^E_(spin,0)=1` field matches its analytic form, `q(theta)=-sqrt((2spin+1) binom(2spin,spin)/(4*pi)) (sin(theta)/2)^spin`, with a zero second component at every sample. DUCC 0.41.0 references cover three synthesis samples per grid/spin and selected coefficients from arbitrary-map analysis. Both spins pass STANDARD `S/A/S*/A*`, `GRAD_ONLY` synthesis and its adjoint identity on GL, CC and F1. Across the six cases, maximum DUCC synthesis-sample error is `2.776e-16`, maximum analytic-map error is `1.665e-16` (largest RMS `7.905e-17`), maximum selected DUCC analysis-coefficient error is `1.776e-15`, maximum `A(S)` coefficient error is `1.157e-15`, and the largest absolute adjoint inner-product residual is `3.331e-16`. This full-operation check covers spin 3/4 through `lmax=spin`; spin 5 and above remain rejected.

An opt-in release check extends spin-3/4 STANDARD synthesis and exact analysis to T511 on GL, CC and F1, using minimum exact-analysis latitude counts and `nphi=1025`. Sectoral synthesis is compared at five middle-ring samples with static `ducc0==0.41.0` references and at every pixel with the closed-form Wigner-d expression. Analysis of deterministic arbitrary maps is compared with DUCC for three selected E/B mode pairs per case. Across six cases, the maximum DUCC synthesis-sample error is `6.528e-14`, the maximum analytic-map error is `2.169e-12` (largest RMS `9.632e-14`), and the maximum selected DUCC analysis-coefficient error is `2.273e-17`. The test ran in 3.63 s in release mode on the AMD Ryzen 9 5950X with rustc 1.99.0 and AVX2/FMA runtime dispatch; DUCC used one thread.

An additional opt-in test checks spin-3/4 exact analysis at T1023 on GL, CC and F1 using the minimum exact-analysis latitude counts and `nphi=2049`. Analysis of deterministic arbitrary maps is compared with DUCC for the lowest `m=0` coefficient and the sectoral `(lmax,mmax)` coefficient in both E and B. The largest absolute coefficient error across six cases is `8.882e-18`. This test checks selected output coefficients and does not claim a full coefficient comparison at T1023.

The checks ran with rustc 1.99.0 in release mode on an AMD Ryzen 9 5950X with AVX2/FMA runtime dispatch; DUCC reference outputs use one thread. The Rust tests use the global Rayon pool. The high-bandlimit tests are ignored by the routine suite and run explicitly with `cargo test --release --lib transform::tests::m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_t511_all_grids_matches_ducc_and_roundtrips -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_t1023_sparse_synthesis_matches_ducc -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_t1023_exact_analysis_matches_ducc -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_three_and_four_t1023_exact_analysis_matches_ducc -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_t2047_sparse_synthesis_matches_ducc_and_sectoral_formula -- --ignored --exact --nocapture`, and `cargo test --release --lib spin_transform::tests::m3_spin_three_and_four_t511_all_grids_match_ducc_and_analytic_fields -- --ignored --exact --nocapture`. All seven checks pass. The spin-3/4 operation check runs in the routine suite as `cargo test --lib spin_transform::tests::spin_three_and_four_match_ducc_analytic_fields_and_adjoint_identities -- --exact --nocapture`. Validation does not cover spin 5+, f32, spin-3/4 synthesis above T2047, their exact analysis above T1023, or their adjoints above `lmax=spin`.

### Latitude block-size tuning (M4, 2026-10-12)

The T511 scalar GL benchmark compared 16, 32, 64 and 128-ring blocks on an AMD Ryzen 9 5950X with 16 cores, 32 hardware threads, AVX2/FMA and no AVX-512. It used rustc 1.99.0, the release profile, one warmup, five timed samples per operation and one/eight-thread Rayon pools. The table reports per-run medians in milliseconds; 32-ring results average three run medians, 64-ring results average two, and 16/128 are single exploratory runs.

| Block rings | Synthesis, 1 thread | Synthesis, 8 threads | Analysis, 1 thread | Analysis, 8 threads |
| ---: | ---: | ---: | ---: | ---: |
| 16 | 1605.7 | 242.2 | 2058.5 | 484.0 |
| 32 | 1566.6 | 230.7 | 2112.4 | 510.8 |
| 64 | 1536.3 | 228.0 | 2198.8 | 525.2 |
| 128 | 1543.5 | 428.1 | 2285.4 | 1079.1 |

The 32-ring block reduces the equal-weight mean of scalar synthesis and analysis times by 1.5% at both thread counts compared with 64 rings. Scalar synthesis takes 1.2–2.0% longer and analysis takes 2.7–3.9% less time. A separate spin-1 GL T511 benchmark compared 32 and 64 rings over two runs per setting, using the same warmup and sampling method. The table reports averages of the two per-run medians:

| Block rings | Synthesis, 1 thread | Synthesis, 8 threads | Analysis, 1 thread | Analysis, 8 threads |
| ---: | ---: | ---: | ---: | ---: |
| 32 | 1706.5 | 249.8 | 1713.5 | 435.7 |
| 64 | 1675.3 | 250.9 | 1687.4 | 431.3 |

The spin measurements favor 64 rings in three of the four cases. The 16-ring block has lower scalar analysis times and higher scalar synthesis times; 128 rings provide only four blocks on this grid and reduce eight-thread throughput. The implementation retains the shared 64-ring block for scalar and spin kernels. Run the harnesses with `cargo run --release --example m1a_bench` and `cargo run --release --example m1b_bench`. The eight-worker run estimates 16.23 MiB for all numeric arrays, including maps, coefficient buffers, plan geometry and workspace scratch; RustFFT scratch and executable/runtime allocations are excluded. Numerical correctness comparisons use `ducc0==0.41.0`; the benchmark runs do not time DUCC. These measurements cover scalar and spin-1 f64 GL T511 on this CPU; they do not measure AVX-512 or other architectures. The f32/complex64 core remains unimplemented.

### Scaled recurrence multiply fast paths (M4, 2026-10-12)

Stage timers at T511 showed the Legendre recurrence took 1537 ms in synthesis and 2153 ms in analysis; FFT stages took 5.1 ms and 3.1 ms, respectively. The first change makes `scale_binary` adjust stored exponent bits when both the input and result are normal numbers, retaining the staged multiplication path for subnormal, overflow and underflow boundaries. A test compares its result bit-for-bit against the former staged path across signed zero, normal and subnormal values, and exponents from -4097 to 4097.

Callgrind on an optimized T127 scalar GL run then attributed 66.29% of instruction references to `multiply_component_scaled`. The second change multiplies finite normal operands directly when their product is also normal, then applies the recurrence scale. It returns the direct result only when that scaled result is normal; zero, subnormal and overflow outputs use the split path. The bitwise test compares both paths across varied normal and subnormal values, signed zero, an infinite factor and exponents from -4097 to 4097, including a subnormal-output rounding boundary.

The release harnesses used rustc 1.99.0, f64, T511 GL `512 x 1025`, one warmup and five samples per operation, with one-worker and eight-worker Rayon pools. Before and exponent-path values are single-run medians; guarded direct-multiply values average two run medians. Timings are in milliseconds on the AMD Ryzen 9 5950X with AVX2/FMA and no AVX-512:

| Operation | Threads | Before | After exponent path | After direct multiply | Overall change |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 1557.719 | 779.198 | 667.130 | -57.17% |
| Scalar synthesis | 8 | 232.519 | 120.363 | 103.903 | -55.31% |
| Scalar analysis | 1 | 2174.312 | 1062.602 | 806.614 | -62.90% |
| Scalar analysis | 8 | 529.998 | 278.115 | 216.405 | -59.17% |
| Spin-1 synthesis | 1 | 1652.310 | 1393.163 | 1399.512 | -15.30% |
| Spin-1 synthesis | 8 | 245.744 | 204.563 | 204.958 | -16.60% |
| Spin-1 analysis | 1 | 1669.132 | 1425.995 | 1426.612 | -14.53% |
| Spin-1 analysis | 8 | 425.267 | 355.187 | 362.421 | -14.78% |

Callgrind on the optimized T127 scalar GL harness recorded 253,637,022 instruction references before the common-case reordering and 174,268,270 afterward, a 31.3% reduction. These runs used the rustc 1.99.0 release build on an AMD Ryzen 9 5950X with AVX2/FMA runtime dispatch. `multiply_component_scaled` fell from 49.19% to 26.83% of total references. The final path checks normal operands before zero and nonfinite boundary cases. After confirming that their product is normal, `scale_normal_binary` adjusts its exponent bits when the scaled result is normal and delegates boundary results to the general scaler. Bitwise tests compare that helper with the staged reference across normal values and exponents from -4097 to 4097; the general scaling test also covers signed zero, infinities and NaN.

The common-case scalar measurements average three per-run medians after one warmup and five timed samples. They use the same rustc 1.99.0 release build and AMD Ryzen 9 5950X host as the earlier T511 measurements:

| Operation | Threads | Guarded multiply, earlier order | Common-case path | Change from earlier order |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 667.130 | 461.915 | -30.76% |
| Scalar synthesis | 8 | 103.903 | 73.677 | -29.09% |
| Scalar analysis | 1 | 806.614 | 551.584 | -31.62% |
| Scalar analysis | 8 | 216.405 | 147.511 | -31.84% |

The next change shares the normal-factor check between the real and imaginary components of each complex recurrence value. Its bitwise regression compares both components with the staged reference across the same value, factor and exponent cases, including a complex value whose normal real component takes the fast path while its subnormal imaginary component takes the fallback. On the same T127 harness, Callgrind instruction references fell from 174,268,270 to 168,222,614 (3.47%); the component helper fell to 24.49% of references. Three-run T511 timing averages also improved across all four scalar operations, by 0.62–2.42%:

| Operation | Threads | Common-case path | Shared factor check | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 461.915 ms | 450.753 ms | -2.42% |
| Scalar synthesis | 8 | 73.677 ms | 72.680 ms | -1.35% |
| Scalar analysis | 1 | 551.584 ms | 543.562 ms | -1.45% |
| Scalar analysis | 8 | 147.511 ms | 146.592 ms | -0.62% |

The T511 timings use the same rustc 1.99.0 release harness, CPU, one warmup and five timed samples per operation recorded above.

These timings are for the implementation that passed all seven high-bandlimit release checks against `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`), covering scalar and spin transforms through T2047 and analytic recurrence checks.

An independent Python comparison used the rebuilt editable extension, `ducc0==0.41.0`, deterministic T511 coefficients and seeded random analysis maps on the same GL shape. Each operation had one warmup and five timed samples; RustSHT and DUCC used the same one- or eight-thread setting. With one thread, scalar synthesis and analysis took 471.541 ms and 552.281 ms in RustSHT versus 12.043 ms and 12.240 ms in DUCC; spin-1 took 1362.377 ms and 1389.893 ms versus 25.781 ms and 25.992 ms. With eight threads, scalar took 79.025 ms and 146.604 ms versus 1.871 ms and 1.773 ms; spin-1 took 206.881 ms and 358.471 ms versus 3.742 ms and 3.722 ms. Scalar synthesis map maximum absolute/RMS errors were `2.627e-14`/`7.674e-16`; analysis coefficient errors were `1.436e-15`/`2.179e-16`. Spin-1 map errors were `3.054e-14`/`7.729e-16`, and analysis coefficient errors were `1.659e-15`/`2.410e-16`. Across these operations and thread counts, RustSHT is 39–96 times slower than DUCC; M4 performance acceptance remains open.

### Fused spin recurrence step (M4, 2026-10-12)

`recurrence_step` advances each spin basis with `next = (a * cos(theta) - b_eff) * current - previous`, where `b_eff` has a branch-specific sign. The implementation now expresses both stages as fused multiply-add operations through `pulp::Simd::mul_add_f64s` and `f64::mul_add`. This reduces spin-1 T511 GL runtime in every measured operation relative to commit `d4ec0c4`. Each value is an average of three per-run medians from the release `m1b_bench` harness, with one warmup and five timed samples per operation, using rustc 1.99.0, f64, an AMD Ryzen 9 5950X with AVX2/FMA and one- or eight-thread Rayon pools:

| Operation | Threads | Before FMA | FMA | Change |
| --- | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | 1371.815 ms | 1335.613 ms | -2.64% |
| Spin-1 synthesis | 8 | 203.655 ms | 198.386 ms | -2.59% |
| Spin-1 analysis | 1 | 1412.318 ms | 1372.486 ms | -2.82% |
| Spin-1 analysis | 8 | 354.049 ms | 343.635 ms | -2.94% |

All seven release-mode high-bandlimit checks pass against `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; they cover scalar and spin transforms through T2047, including analytic recurrence checks. The rebuilt Python extension also passes all eight DUCC comparison tests.

### f32 plan, workspace and transform support (M4, 2026-10-12)

`ShtPlan<T = f64>` and `ShtWorkspace<T = f64>` support f32 and f64. `Complex32` and `Complex64` identify the matching coefficient types. `GridSpec` stores coordinates and quadrature weights as f64; plan construction converts the numerical metadata to the selected arithmetic type. The FFT adapter and CC/F1 resampler execute in that type. The f64 and f32 recurrences use runtime-dispatched `pulp` SIMD over 64-ring latitude blocks, with scalar fallback. Binary scaling and recurrence initialization remain lane-wise.

The Rust integration tests use `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) and float32/complex64 inputs. Static scalar synthesis and analysis references cover T2 on GL `3x5`, CC `4x5` and F1 `3x5`, all with `phi0=0.37`. A spin-1 GL `2x3` synthesis reference covers T1 at `phi0=0.23`. A spin-1 sparse GL synthesis case compares selected map samples at T127. Independent f32 analytic checks cover scalar `m=0` fields on all three grids, `Y_11` derivatives on GL, and a T127 scalar sectoral field. Scalar `S*` and `A*` identities pass on GL/CC/F1; spin-1 through spin-4 STANDARD `S`, `A`, `S*`, `A*`, GRAD_ONLY synthesis and its adjoint pass at `lmax=spin` with exact-analysis minimum latitude counts and minimum odd longitude counts. DERIV1 synthesis and its adjoint pass at T2. The FFT helper compares f32 spectra and roundtrips with full-complex RustFFT at lengths 1, 2, 3, 4, 5, 8, 31, 32 and 33. The T127 f32 cases cover scalar and spin-1 GL synthesis; they do not test T127 analysis or other grids.

The initial release benchmark, before f32 SIMD, used `examples/m4_f32_bench.rs` on an AMD Ryzen 9 5950X (16 cores, 32 threads), x86_64 AVX2/FMA, rustc 1.99.0, f32 spin-1, GL `512x1025`, T511 and `phi0=0.37`. RustSHT reuses its plan and workspace. `ducc0==0.41.0` with NumPy 2.4.6 uses `synthesis_2d` and `analysis_2d` on the same dtype, grid, coefficients, and thread counts. Each operation has one warmup and five timed samples; the table reports each median.

| Operation | Threads | RustSHT | DUCC | RustSHT / DUCC |
| --- | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | 1340.185 ms | 27.495 ms | 48.7x |
| Spin-1 synthesis | 8 | 197.964 ms | 3.795 ms | 52.2x |
| Spin-1 analysis | 1 | 1405.229 ms | 25.061 ms | 56.1x |
| Spin-1 analysis | 8 | 330.506 ms | 3.522 ms | 93.8x |

The initial RustSHT medians show 6.8x synthesis and 4.3x analysis speedups from one to eight threads. The estimated numeric arrays use 12.13 MiB and exclude RustFFT plan and execution scratch, allocator metadata, and process memory. The scalar-lane implementation took 48.7–93.8 times longer than DUCC in these measurements. These initial one-host measurements do not establish a performance acceptance result.

### f32 latitude recurrence SIMD (M4, 2026-10-12)

The scalar and spin f32 recurrence kernels dispatch through `pulp::Arch` and operate on 64-ring blocks. Recurrence arithmetic uses f32 SIMD; initial half-angle or sectoral powers and binary rescaling remain per lane. `Arch::Scalar` provides the portable fallback. Unit checks compare scalar fallback with `Arch::new()` across three blocks and a one-ring tail, near-polar latitudes, scalar orders through T127, and spin 1–4 through T96. The integration suite runs the runtime-selected path and retains DUCC 0.41.0, analytic and T127 checks. On this AMD Ryzen 9 5950X host, `Arch::new()` selects AVX2/FMA; AVX-512 is unavailable.

The release benchmark repeats the initial RustSHT setup on rustc 1.99.0. Both the earlier scalar-lane and current SIMD RustSHT runs use `examples/m4_f32_bench.rs`, which reuses the plan and workspace. The DUCC measurements use `ducc0==0.41.0`, NumPy 2.4.6, float32 maps, complex64 coefficients, and the same grid and coefficients; each operation has one warmup and five timed samples. Values are medians in milliseconds.

| Operation | Threads | Scalar-lane RustSHT | SIMD RustSHT | SIMD reduction | DUCC | SIMD RustSHT / DUCC |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | 1340.185 | 1177.619 | 12.1% | 27.464 | 42.9x |
| Spin-1 synthesis | 8 | 197.964 | 181.242 | 8.4% | 6.598 | 27.5x |
| Spin-1 analysis | 1 | 1405.229 | 1281.740 | 8.8% | 25.292 | 50.7x |
| Spin-1 analysis | 8 | 330.506 | 297.164 | 10.1% | 3.464 | 85.8x |

The SIMD run gives 6.5x synthesis and 4.3x analysis speedups from one to eight RustSHT threads. The estimated numeric-array footprint remains 12.13 MiB, excluding RustFFT plan and execution scratch, allocator metadata, and process memory. SIMD reduces the scalar-lane medians by 8.4–12.1%; RustSHT remains 27–86 times slower than DUCC in these measurements. f32 performance acceptance and memory-traffic optimization remain open.

### Reuse Fourier scale constants in transform loops (M4, 2026-10-12)

Scalar and spin synthesis now convert the longitude count to the transform precision outside the order and ring loops. Scalar and spin analysis reuse that conversion in their coefficient loops. Spin synthesis also computes its Nyquist scale once per ring group. These changes retain the same multiplication and division order.

Callgrind instruction references for one f32 spin-1 GL synthesis at T127 fell from 272,923,166 to 265,643,293 (2.67%) on the AMD Ryzen 9 5950X with rustc 1.99.0, release optimization, AVX2/FMA dispatch and one Rayon thread. The T511 release benchmark compared two runs before and two after the change, each with one warmup and five samples per operation. Values below are averages of the per-run medians in milliseconds; the workload used f32, spin 1, GL `512x1025`, and one or eight Rayon threads.

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 1174.303 | 1168.563 | -0.49% |
| Synthesis | 8 | 184.243 | 178.572 | -3.08% |
| Analysis | 1 | 1286.292 | 1273.005 | -1.03% |
| Analysis | 8 | 295.586 | 297.207 | +0.55% |

The estimated numeric-array footprint remains 12.13 MiB. These two-run timings show a small, mixed change; the Callgrind result supports a lower instruction count for the profiled synthesis, but these measurements do not establish a general speedup. No new DUCC timing was collected for this change.

### f32 normal binary scaling fast path (M4, 2026-10-12)

The f32 scalar recurrence now adjusts the exponent bits directly when multiplying a normal value by a power of two produces another normal value. Subnormal, overflow, and extreme-exponent results use the general binary scaler. The fast path is used by both scalar recurrence multiply helpers; zero, nonfinite, and boundary products retain the general or split-binary paths.

Unit tests compare normal-value scaling with staged multiplication and compare the optimized complex multiply with a split-binary reference. The cases include signed zero, normal and subnormal inputs and outputs, infinities, NaN, and exponents from -4097 to 4097. Existing f32 integration checks compare transform results with `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) and analytic fields.

Callgrind instruction references for one f32 scalar GL synthesis at T127 fell from 182,424,872 at `897ed70` to 174,130,980 with the fast path, a 4.55% reduction. The release profile used rustc 1.99.0, AVX2/FMA runtime dispatch, and one Rayon thread on an AMD Ryzen 9 5950X. A T511 scalar benchmark used f32, GL `512x1025`, one or eight Rayon threads, one warmup, and five timed samples per run. Each table value averages two per-run medians in milliseconds; the plan and workspace were reused. The estimated core numeric-array footprint was 8.13 MiB.

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 756.300 | 728.566 | -3.7% |
| Synthesis | 8 | 143.088 | 138.013 | -3.5% |
| Analysis | 1 | 887.067 | 842.292 | -5.0% |
| Analysis | 8 | 195.777 | 186.292 | -4.8% |

The two-run timing averages are lower in all four cases. They measure one host and workload; they do not establish a general speedup or close the f32 performance gap with DUCC. The benchmark did not collect DUCC timings.

On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 73 library unit tests and 10 f32 integration tests; seven opt-in high-bandlimit tests are ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 71 library unit tests and 10 f32 integration tests, with the same seven tests ignored. Workspace Clippy with warnings denied passes on both compilers. Formatting and `git diff --check` pass. The Python binding tests were not run because this change does not modify the bindings.

### Skip duplicate scaling at f32 recurrence boundaries (M4, 2026-10-12)

`scale_normal_binary` now returns a value only when exponent adjustment keeps the product normal. For subnormal, overflow and extreme-exponent results it returns `None`, allowing the caller to use the split-binary path without first calculating and discarding a result from the general scaler. This preserves the split path's rounding at subnormal boundaries.

The bitwise unit tests compare fast-path results with staged power-of-two scaling and compare the complex multiply with the split-binary reference across signed zero, normal and subnormal operands, infinities, NaN, and exponents from -4097 to 4097. They cover the underflow tie where staged scaling rounds to the minimum normal value while exponent adjustment requires the split path. The existing f32 integration suite compares transforms with `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) and analytic fields.

Callgrind 3.26.0 instruction references for one f32 scalar GL synthesis at T127 fell from 174,130,980 at `3124301` to 154,501,046, an 11.27% reduction. The release build used rustc 1.99.0, AVX2/FMA runtime dispatch, and one Rayon thread on an AMD Ryzen 9 5950X.

The T511 scalar benchmark used f32, GL `512x1025`, one or eight Rayon threads, one warmup and five timed samples per run. The plan and workspace were reused. Each table value averages two per-run medians in milliseconds. The comparison used the same temporary harness and compiler at `3124301` and the working tree with the boundary fallback change. The estimated core numeric-array footprint was 8.13 MiB.

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 728.566 | 617.665 | -15.2% |
| Synthesis | 8 | 138.013 | 112.768 | -18.3% |
| Analysis | 1 | 842.292 | 735.521 | -12.7% |
| Analysis | 8 | 186.292 | 162.783 | -12.6% |

These measurements cover one host, dtype, grid and bandlimit. No DUCC timing was collected for this change.

On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 73 library unit tests and 10 f32 integration tests; seven opt-in high-bandlimit tests are ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 71 library unit tests and 10 f32 integration tests, with the same seven tests ignored. Workspace Clippy with warnings denied passes on both compilers. Formatting and `git diff --check` pass. The Python binding tests were not run because this change does not modify the bindings.

### f32 recurrence normalization magnitude checks (M4, 2026-10-12)

`normalize_pair_lanes` now selects the larger absolute recurrence value and checks its scale thresholds using the unsigned magnitude bits. For finite IEEE-754 values, unsigned magnitude order matches numeric order; the current recurrence supplies finite pairs, and debug builds assert that constraint. The rescaling operations and exponent updates are unchanged.

Callgrind 3.26.0 instruction references for one f32 scalar GL synthesis at T127 fell from 154,494,737 at `e224955` to 149,319,467, a 3.35% reduction. The release build used rustc 1.99.0, AVX2/FMA runtime dispatch, and one Rayon thread on an AMD Ryzen 9 5950X.

The T511 scalar benchmark used f32, GL `512x1025`, one or eight Rayon threads, one warmup and five timed samples per run. The plan and workspace were reused. Each table value averages two per-run medians in milliseconds; the estimated numeric-array footprint was 8.13 MiB.

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 617.665 | 591.653 | -4.2% |
| Synthesis | 8 | 112.768 | 110.297 | -2.2% |
| Analysis | 1 | 735.521 | 720.346 | -2.1% |
| Analysis | 8 | 162.783 | 160.041 | -1.7% |

These measurements cover one host, dtype, grid and bandlimit. No DUCC timing was collected for this change.

The unit test compares the bit-based normalization with the previous float-magnitude algorithm at zero, subnormal, `SMALL` and `BIG` thresholds, and large finite values. On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 74 library unit tests and 10 f32 integration tests; seven opt-in high-bandlimit tests are ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 72 library unit tests and 10 f32 integration tests, with the same seven ignored. Workspace Clippy with warnings denied passes on both compilers. Formatting and `git diff --check` pass. Python binding tests were not run because this change does not modify the bindings.

### Spin f32 recurrence normalization magnitude checks (M4, 2026-10-12)

The spin f32 `normalize_pair_lanes` function now selects the larger absolute recurrence value and checks its scale thresholds using unsigned magnitude bits. For finite IEEE-754 values, unsigned magnitude order matches numeric order; the recurrence supplies finite pairs, and debug builds assert that constraint. Rescaling and exponent updates are unchanged. A unit test compares the bit-based implementation with the former float-magnitude algorithm at signed zero, subnormal values, one-ULP neighbors of `SMALL` and `BIG`, and large finite values.

Callgrind 3.26.0 instruction references for one f32 spin-1 GL synthesis at T127 fell from 265,642,604 at `564e76c` to 255,304,260, a 3.89% reduction. The release build used rustc 1.99.0, AVX2/FMA runtime dispatch, and one Rayon thread on an AMD Ryzen 9 5950X.

The T511 spin benchmark used f32, spin 1, GL `512x1025`, one or eight Rayon threads, one warmup, and five timed samples per run. The plan and workspace were reused. The before measurements used `564e76c`; the after measurements used the working tree with this change. Each table value averages two per-run medians in milliseconds. The estimated core numeric-array footprint was 12.13 MiB.

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 1164.235 | 1132.685 | -2.7% |
| Synthesis | 8 | 180.424 | 175.707 | -2.6% |
| Analysis | 1 | 1271.015 | 1241.490 | -2.3% |
| Analysis | 8 | 293.103 | 286.721 | -2.2% |

The four two-run timing averages are lower for this workload. They measure one host, dtype, grid and bandlimit; no DUCC timing was collected for this change.

On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 75 library unit tests and 10 f32 integration tests; seven opt-in high-bandlimit tests are ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 73 library unit tests and 10 f32 integration tests, with the same seven ignored. Workspace Clippy with warnings denied passes on both compilers. Formatting and `git diff --check` pass. Python binding tests were not run because this change does not modify the bindings.

A follow-up prototype updated the threshold tracker's exponent bits instead of multiplying the temporary maximum after each rescale. Its focused reference test passed, and Callgrind 3.26.0 references for T127 spin-1 GL synthesis fell from 255,300,338 to 253,145,506 (0.84%) on the same CPU and compiler. Two-run T511 f32 GL measurements increased in all four cases: synthesis from 1132.685 to 1165.414 ms at one thread and 175.707 to 177.780 ms at eight; analysis from 1241.490 to 1259.368 ms at one thread and 286.721 to 289.445 ms at eight. The prototype was reverted because the complete workload was slower despite the lower instruction count.

### f32 phase and pole edge cases (M4, 2026-10-12)

Scalar and spin transforms retain `phi0` as f64 while forming each order phase, then convert the sine and cosine to the selected transform type. If `m*phi0` overflows f64, phase setup derives the integer-order phase by exponentiating `e^(i phi0)`. Grid and prepared CC colatitudes remain f64 through sine/cosine evaluation; the exact `0` and `pi` endpoints produce exact zero sine values before conversion to f32.

Regression tests synthesize scalar and spin fields at `phi0=f64::MAX`, requiring finite f32/f64 outputs that agree within `3e-6`; an independent scalar `Y_22` check compares the order-two phase with the square of `e^(i phi0)`. A CC scalar `Y_11` test requires every f32 sample at the south pole to be exactly zero. On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 71 unit tests and 10 f32 integration tests, with seven opt-in high-bandlimit tests ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 69 unit tests and 10 f32 integration tests, with the same seven tests ignored. `cargo test --test f32`, Clippy with warnings denied on both compilers, formatting checks and `uv run pytest python/tests` pass; pytest reports eight tests. These edge-case tests do not extend the high-bandlimit f32 validation domain or address the open f32 SIMD performance gap.

### Python plan binding (M5, 2026-10-12)

The PyO3 extension exposes `rustsht.ShtPlan` and top-level procedural functions for GL, CC and F1 grids. They accept `float64` maps and `complex128` coefficients in component-first NumPy arrays. The plan methods and procedural functions cover scalar and spin STANDARD synthesis, analysis and both adjoints, spin gradient synthesis and its adjoint, and scalar DERIV1 synthesis and its adjoint. `get_gridweights` returns the grid ring weights. Inputs must be C-contiguous; each call copies its inputs into Rust-owned vectors, releases the GIL for workspace creation and transform execution, and returns a new NumPy array. The package includes the PEP 561 marker and `__init__.pyi` stub.

Python tests convert between RustSHT's order-major coefficient packing and DUCC's default alm layout. Plan and procedural scalar S, A, S* and A* are compared with `ducc0==0.41.0` on GL, CC and F1; the procedural ring weights are compared on each grid. Spin-2 STANDARD S, A, S* and A*, GRAD_ONLY S and S*, and scalar DERIV1 S and S* are compared through both interfaces on GL. The tests use `phi0 != 0`, check the scalar real-harmonic adjoint identity, and reject incorrect shapes, dtypes and Fortran-contiguous arrays. The largest comparison tolerance is `2e-12`; DUCC runs with one thread. The pinned DUCC source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

On an AMD Ryzen 9 5950X, `cargo fmt --all -- --check`, workspace Clippy with warnings denied and `cargo test --workspace --all-features` pass on rustc 1.89.0 and 1.99.0. Each workspace test run passes 68 tests with seven high-bandlimit tests ignored. The Rust 1.99.0 no-default-features suite passes 66 tests with the same seven tests ignored.

`uv run pytest python/tests` passes eight tests with Python 3.11.15, NumPy 2.4.6 and `ducc0==0.41.0`. It validates f64/complex128 inputs and the listed plan and procedural operations against the editable package.

`uv build` creates a CPython 3.11 abi3 Linux x86_64 wheel and source archive. Both include the GPL license, `LICENSE-THIRD-PARTY`, the typing stub and PEP 561 marker; the wheel metadata declares NumPy and GPL-2.0-or-later. The third-party notice records selected license texts for 42 dependencies in the Linux x86_64 extension build graph. A clean Python 3.11.15 environment installs the wheel, imports it and passes a GL grid-weight smoke check. The full numerical suite was run against the editable package, not the installed wheel. f32/complex64 and the spharmgrid adapter are not covered.

### Performance baseline before Fourier-weight hoisting (M4/M5, 2026-10-12)

Measurements used an AMD Ryzen 9 5950X (16 cores, 32 threads), Rust 1.99.0 release builds, x86_64 AVX2/FMA dispatch and no AVX-512. Python measurements used Python 3.11.15, NumPy 2.4.6 and `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The editable extension was rebuilt with `maturin develop --release`; its build ID matched `target/release/lib_rustsht.so`. RustSHT and DUCC used the same `phi0=0.37`, grid, coefficients, map values and thread counts. For each Python run, `RAYON_NUM_THREADS` was set to the requested thread count before process startup; the harness checks this setting before importing NumPy, DUCC or RustSHT. `examples/python_perf_baseline.py` records the Python plan, procedural and DUCC measurements. Each timed operation had one warmup and five samples per run, with two runs. Tables report the median of the run medians and their spread.

The native benchmark reused one plan and workspace. Python plan methods reuse the plan but still copy inputs and allocate a workspace and output for each call; procedural functions construct a plan and workspace per call. At T511 GL `512x1025`, f64, scalar and spin 1, the native Rust results were:

| Operation | Threads | Run medians (ms) | Median (ms) | Estimated numeric arrays |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 535.197, 532.742 | 533.970 | 16.23 MiB |
| Scalar synthesis | 8 | 84.013, 84.180 | 84.097 | 16.23 MiB |
| Scalar analysis | 1 | 564.943, 570.466 | 567.705 | 16.23 MiB |
| Scalar analysis | 8 | 150.777, 153.673 | 152.225 | 16.23 MiB |
| Spin-1 synthesis | 1 | 1398.575, 1413.754 | 1406.165 | 24.24 MiB |
| Spin-1 synthesis | 8 | 223.924, 209.477 | 216.701 | 24.24 MiB |
| Spin-1 analysis | 1 | 1463.448, 1437.675 | 1450.562 | 24.24 MiB |
| Spin-1 analysis | 8 | 409.633, 370.695 | 390.164 | 24.24 MiB |

The Python T511 GL plan, procedural and DUCC results were:

| Operation | Threads | RustSHT plan (median; spread) | RustSHT procedural (median; spread) | DUCC (median; spread) | Plan / DUCC |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 507.187; 2.916 ms | 508.031; 7.307 ms | 11.780; 0.039 ms | 43.1x |
| Scalar synthesis | 8 | 86.531; 1.447 ms | 84.006; 0.076 ms | 1.884; 0.194 ms | 45.9x |
| Scalar analysis | 1 | 559.307; 3.210 ms | 558.501; 1.922 ms | 12.386; 0.013 ms | 45.2x |
| Scalar analysis | 8 | 152.961; 0.793 ms | 151.996; 0.238 ms | 1.857; 0.097 ms | 82.4x |
| Spin-1 synthesis | 1 | 1442.409; 1.818 ms | 1436.607; 3.778 ms | 25.424; 0.810 ms | 56.7x |
| Spin-1 synthesis | 8 | 219.832; 1.971 ms | 216.967; 3.625 ms | 3.878; 0.186 ms | 56.7x |
| Spin-1 analysis | 1 | 1497.584; 4.821 ms | 1488.491; 5.510 ms | 26.034; 0.300 ms | 57.5x |
| Spin-1 analysis | 8 | 379.849; 2.547 ms | 377.871; 6.676 ms | 4.014; 0.021 ms | 94.6x |

The scalar synthesis map error against DUCC had maximum absolute error `2.627e-14`, RMS error `7.674e-16` and relative L2 error `6.487e-14`; scalar analysis coefficient errors were `1.010e-14`, `2.281e-16` and `4.467e-14`. Spin-1 synthesis errors were `2.363e-14`, `8.126e-16` and `6.675e-14`; analysis errors were `1.463e-14`, `2.298e-16` and `4.500e-14`. Python plan construction took about `0.036 ms`; a separate five-sample copy check measured coefficient copies at `0.04–0.10 ms` and map copies at `0.09–0.21 ms`; output allocation took `0.001 ms`. A native T511 workspace took `3.68–3.93 ms` at the median of eleven measurements. These binding and allocation costs are small relative to transform execution.

Representative Python plan timings on CC and F1 used T255, f64, one thread, `nphi=513`, and exact-analysis latitude counts (CC `257`, F1 `256`). Values are median and spread in milliseconds for the two runs:

| Grid | Spin | Synthesis RustSHT / DUCC | Analysis RustSHT / DUCC |
| --- | ---: | ---: | ---: |
| CC | 0 | 68.384 / 1.973 (0.254 / 0.088) | 73.545 / 5.246 (0.723 / 0.056) |
| CC | 1 | 189.635 / 7.948 (1.508 / 0.708) | 195.422 / 14.742 (0.272 / 0.144) |
| F1 | 0 | 68.052 / 1.949 (0.320 / 0.034) | 73.331 / 5.309 (0.363 / 0.116) |
| F1 | 1 | 186.212 / 5.835 (0.112 / 0.013) | 195.379 / 14.113 (1.147 / 0.214) |

Maximum absolute, RMS and relative L2 errors on CC were `3.490e-15`, `1.648e-16`, `2.793e-14` for scalar synthesis; `1.169e-14`, `2.425e-16`, `2.376e-14` for scalar analysis; `4.338e-15`, `1.426e-16`, `2.349e-14` for spin-1 synthesis; and `9.907e-15`, `1.782e-16`, `1.747e-14` for spin-1 analysis. The corresponding F1 errors were `4.625e-15`, `1.727e-16`, `2.922e-14`; `1.185e-14`, `2.448e-16`, `2.399e-14`; `5.627e-15`, `1.516e-16`, `2.492e-14`; and `9.921e-15`, `1.785e-16`, `1.749e-14`.

Callgrind 3.26.0 profiled one-thread T127 GL f64 analysis. Scalar analysis collected `211,363,418` instruction references; `for_each_y_lm_dispatched` accounted for `96.65%` inclusive, its callback `77.94%`, and `multiply_complex_scaled` `48.57%`. Spin-1 analysis collected `318,081,053` references; `for_each_spin_lm_dispatched` accounted for `96.91%` inclusive, its callback `59.62%`, `recurrence_step` `17.22%`, `scaled_basis_value` `16.62%` and `RealFftPlan::forward` `1.10%`. These inclusive counts overlap. CC and F1 T127 spin analysis collected `331,041,516` and `332,475,882` references; the geometry differences include latitude count and resampling. Linux `perf` could not access hardware events because `perf_event_paranoid=4`. Callgrind instruction counts identify recurrence and callback work as the dominant costs, while the fused callback does not expose separate elapsed times for recurrence setup, each recurrence update, scaled multiplication and complex accumulation. The first optimization target is repeated Fourier phase and quadrature work in those callbacks; FFT threading is deferred until the recurrence work is reduced.

### Analysis Fourier-weight hoisting (M4/M5, 2026-10-12)

Scalar and spin analysis now prepare the order phase and applicable ring scaling once per `(m, ring)` and reuse those Fourier values for all degree updates. Each recurrence worker uses three 64-lane complex buffers; scalar analysis uses one and spin analysis uses two. Ring blocks and degree accumulation order are unchanged. GL weights and longitude normalization remain in the analysis projection. CC/F1 prepared-grid weighting and Fourier normalization remain in the resampling stage.

Focused Rust checks passed for scalar and spin DUCC references and adjoint identities on GL, CC and F1. f32 scalar and spin all-grid DUCC/analytic checks and adjoint identities also passed. The T511 Python comparisons after rebuilding the extension retained the baseline error values at printed precision: scalar synthesis/analysis relative L2 errors were `6.487e-14`/`4.467e-14`, and spin-1 errors were `6.675e-14`/`4.500e-14`.

Native release comparisons used rustc 1.99.0, AVX2/FMA dispatch, f64, GL `512x1025`, T511, `phi0=0.37`, one warmup and five timed samples per operation on an AMD Ryzen 9 5950X. Plans and workspaces were reused. The before values are the two run medians reported in the baseline; three runs were made after the change. Synthesis is included to show the unaffected path.

| Operation | Threads | Before run medians (mean ms) | After run medians (median ms) | Change vs. before mean |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 535.197, 532.742 (533.970) | 535.780, 530.185, 525.899 (530.185) | -0.7% |
| Scalar synthesis | 8 | 84.013, 84.180 (84.097) | 85.044, 85.123, 82.767 (85.044) | +1.1% |
| Scalar analysis | 1 | 564.943, 570.466 (567.705) | 531.494, 523.459, 529.094 (529.094) | -6.8% |
| Scalar analysis | 8 | 150.777, 153.673 (152.225) | 142.007, 140.031, 139.524 (140.031) | -8.0% |
| Spin-1 synthesis | 1 | 1398.575, 1413.754 (1406.165) | 1441.264, 1431.320, 1556.177 (1441.264) | +2.5% |
| Spin-1 synthesis | 8 | 223.924, 209.477 (216.701) | 215.173, 214.867, 211.066 (214.867) | -0.8% |
| Spin-1 analysis | 1 | 1463.448, 1437.675 (1450.562) | 1372.452, 1369.446, 1499.743 (1372.452) | -5.4% |
| Spin-1 analysis | 8 | 409.633, 370.695 (390.164) | 341.385, 345.875, 336.099 (341.385) | -12.5% |

Callgrind 3.26.0 instruction references for one-thread T127 GL f64 analysis fell from `211,363,418` to `183,229,986` for scalar (`-13.3%`) and from `318,081,053` to `272,490,557` for spin 1 (`-14.3%`). The current Callgrind profiles provide whole-operation instruction totals; the available `perf` hardware counters remain blocked by `perf_event_paranoid=4`.

Python comparisons used Python 3.11.15, NumPy 2.4.6, `ducc0==0.41.0`, the rebuilt editable extension and the same T511 grid/data/thread settings as the baseline. Each median is the median of two run medians; each run had one warmup and five timed samples. Plan methods, procedural functions and DUCC were measured separately. The before values below are plan medians; their spreads and procedural timings are in the baseline table above.

| Operation | Threads | Plan before (ms) | Plan after (median; spread ms) | Procedural after (median; spread ms) | DUCC after (median; spread ms) | Plan change | Plan / DUCC after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar analysis | 1 | 559.307 | 481.628; 1.848 | 483.365; 2.510 | 12.383; 0.114 | -13.9% | 38.9x |
| Scalar analysis | 8 | 152.961 | 129.949; 1.506 | 132.104; 2.210 | 1.871; 0.129 | -15.1% | 69.4x |
| Spin-1 analysis | 1 | 1497.584 | 1349.172; 9.721 | 1352.689; 3.281 | 25.653; 0.205 | -9.9% | 52.6x |
| Spin-1 analysis | 8 | 379.849 | 347.557; 4.466 | 341.697; 2.273 | 3.622; 0.001 | -8.5% | 96.0x |

This change reduces native analysis time and total profiled instruction references without changing the DUCC error at the reported precision. T511 GL remains 39–96 times slower than DUCC in these Python plan measurements. The new buffers add `3 * 64 * sizeof(Complex64)` bytes per recurrence worker (`0.0234 MiB` for eight workers); the estimated T511 eight-worker numeric-array footprints become 16.25 MiB for scalar and 24.26 MiB for spin 1. A separate eleven-sample native measurement found T511 scalar plan construction at `0.0334 ms` median (0.1092 ms spread) and workspace allocation at `3.6745 ms` (0.9984 ms spread). The Python harness measured plan construction at `0.035–0.039 ms`, input copies at `0.052–0.203 ms` and output allocation at `0.001–0.002 ms`; these costs remain small relative to transform execution. No FFT or meridional-resampling implementation changed.

### Spin synthesis phase and FFT scaling hoist (M4/M5, 2026-10-12)

Spin synthesis now accumulates unphased Fourier values across degree for each order and ring, then applies `exp(i*m*phi0)` and the longitude FFT scale once to each accumulated spectrum value. The existing q/u spectrum buffers are updated in place; this adds no full-sized workspace allocation. For even-longitude Nyquist, the accumulated value is phased first, then folded to `2*nphi*real` with zero imaginary part, preserving the previous component and FFT conventions. This changes floating-point operation order in the degree sum.

Focused checks passed for spin STANDARD synthesis and its adjoint, `A*`, GRAD_ONLY, DERIV1 and their adjoints on GL, CC and F1, including even-longitude Nyquist, polar rings, and `phi0=0.37`. Direct f64 comparisons with `ducc0==0.41.0` at T2 covered E-only, B-only and mixed E/B coefficients, GL odd and even longitude counts, CC poles, GRAD_ONLY, DERIV1 and adjoint outputs; the largest absolute discrepancy was below `1.7e-15`. T511 Python spin-1 GL comparison with DUCC had maximum absolute error `2.365e-14`, RMS error `8.126e-16` and relative L2 error `6.675e-14`. f32 focused spin tests passed for all grids and spins 1–4, including adjoint and gradient checks; f32 GL DUCC synthesis and sparse T127 DUCC synthesis also passed. Release DUCC and analytic checks passed for spin 3/4 on GL, CC and F1 at T511, spin 1/2 at T1023, and sparse spin 1–4 synthesis at T2047. These tests retain the established high-bandlimit errors recorded above.

Native release timings used rustc 1.99.0, f64, GL `512x1025`, T511, `phi0=0.37`, AVX2/FMA dispatch, one warmup and five timed samples per run on an AMD Ryzen 9 5950X. The same plan and workspace were reused. Each table entry gives the run medians, their median and range, in milliseconds:

| Threads | Before run medians (median; range) | After run medians (median; range) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `1441.264, 1431.320, 1556.177` (`1441.264`; `124.857`) | `1291.352, 1282.280, 1290.691` (`1290.691`; `9.072`) | `-10.45%` |
| 8 | `215.173, 214.867, 211.066` (`214.867`; `4.107`) | `190.487, 191.759, 191.371` (`191.371`; `1.272`) | `-10.93%` |

The analysis operation is unchanged. Its after medians over the same three runs were `1282.264, 1281.893, 1285.030 ms` at one thread and `321.835, 322.025, 326.827 ms` at eight threads.

The rebuilt Python extension used Python 3.11.15, NumPy 2.4.6 and `ducc0==0.41.0`. Both RustSHT and DUCC used identical T511 GL data, `phi0=0.37`, and one or eight threads. Each run had one warmup and five timed samples; the table reports the median of two run medians and their range. The Python plan before values are from the matched pre-hoist run after analysis Fourier-weight hoisting; procedural functions construct their plan/workspace on each call.

| Operation | Threads | Plan before (ms) | Plan after (median; range ms) | Procedural after (median; range ms) | DUCC after (median; range ms) | Plan change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | `1414.124` | `1295.828; 1.254` | `1291.631; 2.593` | `24.513; 0.049` | `-8.37%` |
| Spin-1 synthesis | 8 | `215.045` | `195.882; 3.562` | `199.649; 1.026` | `4.101; 0.589` | `-8.91%` |

Plan construction measured `0.035–0.041 ms`, input copies `0.104–0.213 ms`, and output allocation `0.001 ms` in these runs. The phase/scale hoist reduces native and Python plan synthesis time while retaining the DUCC error range; it does not close the remaining performance gap. The eight-thread DUCC spread reflects two run medians and should not be read as an optimization effect.

Formatting, workspace Clippy with warnings denied, all-features tests and no-default-features tests passed with Rust 1.89.0 and 1.99.0. Each Rust test suite passed 75 tests with seven opt-in high-bandlimit checks ignored, plus all 10 f32 integration tests. `uv run pytest python/tests` passed all eight tests with Python 3.11.15.

### Scalar recurrence reciprocal-factor investigation (M4, 2026-10-12)

The current recurrence computes `(cos(theta)*current - epsilon[l]*previous)/epsilon[l+1]`. Three T511 release runs established the scalar GL baseline before this investigation on an AMD Ryzen 9 5950X, rustc 1.99.0, f64, AVX2/FMA runtime dispatch, `phi0=0.37`, and one/eight-thread pools. `examples/m1a_bench.rs` reused a plan and workspace, with one warmup and five samples per operation per run:

| Operation | One-thread run medians (median; range ms) | Eight-thread run medians (median; range ms) |
| --- | ---: | ---: |
| Synthesis | `525.214, 525.521, 524.876` (`525.214`; `0.645`) | `81.436, 85.797, 82.154` (`82.154`; `4.361`) |
| Analysis | `516.457, 517.436, 516.911` (`516.911`; `0.979`) | `137.004, 140.699, 136.425` (`137.004`; `4.274`) |

I tested reciprocal multiplication, a fused numerator with reciprocal multiplication, a compensated high/low reciprocal, and precomputed `a_lm=1/epsilon[l+1]`, `b_lm=epsilon[l]/epsilon[l+1]` with a fused update. The scalar-reference test compares the vector-lane recurrence with the retained division-based recurrence at a relative tolerance of `2e-13`, including near-polar and high-order lanes. Reciprocal multiplication first exceeded this limit at `(m,l,ring)=(0,20,15)` with `2.621e-13` relative error and at `(255,467,5)` with `2.654e-13`. The fused-numerator variant reached `5.449e-13` and `1.027e-12` at those tests; the compensated reciprocal reached `2.621e-13` and `5.045e-12`; and the `a,b` variant reached `1.062e-12` and `6.992e-13`. I did not relax the reference tolerance. The variants were rejected before timing because they failed this numerical acceptance check.

The f32 `a,b` prototype passed the dispatched-versus-scalar-backend recurrence check and the existing scalar DUCC checks on GL, CC and F1; that does not override the f64 failure. DUCC's pinned [`Ylmgen::prepare` and scalar kernels](https://raw.githubusercontent.com/mreineck/ducc/c1ed01b8fda69d2a378190f0915f1a19762dddbf/src/ducc0/sht/sht_inner_loop.h) precompute paired-degree `alpha` and `coef` values, then use a two-step recurrence in the normal-range kernels. The simple one-step reciprocal substitutions tested here do not reproduce that method. No recurrence source or workspace change was retained, and no post-change timing is reported because every f64 candidate failed the existing scalar-reference check.

### Paired normal-range complex scaling (M4, 2026-10-12)

Scalar f64 and f32 multiplication now checks the real and imaginary products together. When both products are normal and their binary exponent adjustments remain normal, the function updates both stored exponents and returns the pair. Other inputs use the existing per-component or split-binary path. The change adds no workspace storage and leaves binary scaling in place for subnormal, overflow and underflow cases.

The bitwise split-reference tests pass for both precisions, including signed zero, subnormal operands and results, infinities, NaN, exponents from -4097 to 4097, unequal component magnitudes, and normal-range boundary transitions. Scalar GL/CC/F1 DUCC comparisons and scalar adjoint identities pass for f64 and f32. On T511 GL with `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, the Python comparison reports the same scalar errors as the preceding implementation: synthesis maximum absolute/RMS/relative L2 errors are `2.627e-14`/`7.674e-16`/`6.487e-14`; analysis errors are `1.010e-14`/`2.281e-16`/`4.467e-14`.

Native release timings use rustc 1.99.0, f64, T511 GL `512x1025`, `phi0=0.37`, AVX2/FMA dispatch, reused plans and workspaces, an AMD Ryzen 9 5950X, one warmup and five timed samples per operation. Each cell gives the three run medians, their median and range, in milliseconds:

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `527.717, 526.529, 527.055` (`527.055`; `1.188`) | `371.174, 369.799, 370.029` (`370.029`; `1.375`) | `-29.8%` |
| Synthesis | 8 | `85.373, 85.653, 81.474` (`85.373`; `4.179`) | `60.166, 62.665, 60.328` (`60.328`; `2.499`) | `-29.3%` |
| Analysis | 1 | `517.767, 519.898, 518.155` (`518.155`; `2.131`) | `337.816, 338.390, 338.872` (`338.390`; `1.056`) | `-34.7%` |
| Analysis | 8 | `137.261, 137.142, 138.100` (`137.261`; `0.958`) | `88.888, 92.461, 91.198` (`91.198`; `3.573`) | `-33.6%` |

Callgrind 3.26.0 profiled the T127 GL f64 release harness with one thread and five samples per operation. Total instruction references fell from `4,343,017,620` to `3,098,538,801` (`-28.7%`). Inclusive references attributed to `multiply_complex_scaled` fell from `2,483,130,192` to `1,238,658,360` (`-50.1%`). The harness includes scalar synthesis and analysis; inclusive function counts overlap.

The rebuilt Python extension used Python 3.11.15, NumPy 2.4.6, and `ducc0==0.41.0`. Each Python timing is the median of two run medians; each run had one warmup and five samples. RustSHT plan methods reuse a plan, while procedural functions construct a plan and workspace for each call:

| Operation | Threads | RustSHT plan | RustSHT procedural | DUCC | Plan / DUCC |
| --- | ---: | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `366.764 ms` | `369.891 ms` | `11.810 ms` | `31.1x` |
| Synthesis | 8 | `59.607 ms` | `61.514 ms` | `1.811 ms` | `32.9x` |
| Analysis | 1 | `332.942 ms` | `334.447 ms` | `11.862 ms` | `28.1x` |
| Analysis | 8 | `89.563 ms` | `89.638 ms` | `1.766 ms` | `50.7x` |

Input copies took `0.051–0.087 ms` and output allocation took `0.001 ms` in the Python runs. The paired scaling path reduces native scalar transform time; Python data copying remains small relative to the transform. The measured speedup does not close the remaining gap to DUCC.

### Spin synthesis coefficient-rotation hoist trial (M4, 2026-10-12)

A prototype moved the complex multiplications by `i` and `-i` on E/B coefficients outside the latitude-lane loop in `synthesize_spin_ring_group`. The spin f64 operator suite passed 23 tests with six high-bandlimit cases ignored; all ten f32 integration tests passed, including DUCC comparisons, adjoints, GRAD_ONLY and DERIV1.

Native release timings used rustc 1.99.0, f64, spin 1, T511 GL `512x1025`, `phi0=0.37`, AVX2/FMA dispatch, reused plan/workspace, an AMD Ryzen 9 5950X, one warmup and five samples per operation. The three run medians had no consistent synthesis improvement:

| Operation | Threads | Before (median; range ms) | Prototype (median; range ms) | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `1275.961; 17.201` | `1280.270; 8.904` | `+0.34%` |
| Synthesis | 8 | `192.712; 6.608` | `192.134; 0.817` | `-0.30%` |

Spin analysis, which the prototype did not change, measured `1309.095 ms` before and `1310.403 ms` with one thread, and `335.251 ms` before and `331.392 ms` with eight threads. Callgrind 3.26.0 T127 GL profiles recorded `6,161,382,864` and `6,161,387,569` total instruction references. Inclusive references for `synthesize_spin_ring_group` (`2,887,657,092`) and its recurrence callback (`1,444,837,152`) were identical. The compiler had already moved these coefficient rotations out of the lane loop. The prototype was removed because it did not reduce instructions or measured execution time.

### Weighted Rayon analysis splitting (M4/M5, 2026-10-12)

Scalar and spin analysis now split order ranges at the boundary that most evenly divides the estimated coefficient work, `W(m0,m1)=sum(lmax-m+1)`. Recursive `rayon::join` and disjoint coefficient slices remain in use. Each branch receives its own recurrence workspace slice; the split gives both branches at least one order and one workspace.

At T511 with `lmax=mmax=511`, eight equal-order leaves previously carried estimated work values of `30,752, 26,656, 22,560, 18,464, 14,368, 10,272, 6,176, 2,080` coefficients, a `14.8:1` largest-to-smallest ratio. The weighted leaves carried `16,368, 16,614, 16,131, 16,512, 16,248, 16,559, 16,425, 16,471`, a `1.03:1` ratio. These values estimate coefficient work; they do not account for ring-specific polar cutoffs.

Native release measurements used rustc 1.99.0, f64, T511 GL `512x1025`, `phi0=0.37`, AVX2/FMA dispatch, reused plans and workspaces, an AMD Ryzen 9 5950X, one warmup and five timed samples per thread count. Two independent runs were made for scalar and spin-1 analysis at each thread count. Each table entry gives the median of the two run medians, their range, and the change from equal-order splitting, in milliseconds:

| Operation | Threads | Equal-order median (range) | Weighted median (range) | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar analysis | 1 | `323.619 (0.104)` | `335.277 (2.466)` | `+3.6%` |
| Scalar analysis | 2 | `251.907 (1.355)` | `178.855 (2.265)` | `-29.0%` |
| Scalar analysis | 4 | `153.974 (2.061)` | `97.392 (5.713)` | `-36.7%` |
| Scalar analysis | 8 | `85.994 (0.581)` | `52.574 (0.860)` | `-38.9%` |
| Scalar analysis | 16 | `85.515 (0.348)` | `51.352 (1.733)` | `-40.0%` |
| Spin-1 analysis | 1 | `1324.675 (11.812)` | `1302.151 (15.520)` | `-1.7%` |
| Spin-1 analysis | 2 | `1023.100 (7.219)` | `674.128 (4.653)` | `-34.1%` |
| Spin-1 analysis | 4 | `609.514 (5.414)` | `352.158 (7.576)` | `-42.2%` |
| Spin-1 analysis | 8 | `337.681 (4.760)` | `187.450 (3.810)` | `-44.5%` |
| Spin-1 analysis | 16 | `335.214 (0.254)` | `187.262 (0.902)` | `-44.1%` |

The one-thread path does not call the recursive splitter; its run-to-run differences measure benchmark variation. At eight threads, the rebuilt Python plan measured scalar analysis at `53.957 ms` (range `3.940 ms`) and spin-1 analysis at `197.045 ms` (range `2.651 ms`), versus `1.786 ms` (range `0.026 ms`) and `3.818 ms` (range `0.489 ms`) for `ducc0==0.41.0`. Procedural RustSHT analysis measured `53.157 ms` for scalar and `194.924 ms` for spin 1. The Python run used Python 3.11.15, NumPy 2.4.6, and the same T511 grid, coefficients, map, and `phi0` as the native benchmark.

The Python DUCC comparison reports scalar analysis maximum absolute/RMS/relative L2 errors of `1.010e-14`/`2.281e-16`/`4.467e-14`; spin-1 errors are `1.463e-14`/`2.298e-16`/`4.500e-14`. Focused f64 scalar and spin tests pass against DUCC and adjoint identities on GL, CC and F1. The weighted-boundary unit test checks full-bandlimit, rectangular, offset, small and two-order ranges and verifies that the selected boundary minimizes the estimated work difference. Rust 1.99 all-features tests pass 76 library tests and 10 f32 integration tests, with seven high-bandlimit tests ignored; no-default-features tests pass 73 library tests and 10 f32 integration tests, with the same seven ignored. Rust 1.89 all-features tests, formatting and Clippy with warnings denied pass. `uv run pytest python/tests` passes all eight tests.

### Spin basis conversion for zero recurrence exponent (M4, 2026-10-12)

The f64 `scaled_basis_value` returns `value * alpha_norm` directly when the recurrence exponent is zero. It keeps the zero checks before multiplication and uses `scale_binary` for every nonzero exponent. The path adds no workspace data. Its bitwise regression compares the shortcut with the former binary-scaling path across signed zeros, normal and subnormal values, infinities, NaN, and exponents from -4097 to 4097. Focused spin tests pass against DUCC and adjoint identities on GL, CC and F1; the T511 Python DUCC errors are unchanged at `2.365e-14` maximum absolute/`8.126e-16` RMS/`6.675e-14` relative L2 for synthesis and `1.463e-14`/`2.298e-16`/`4.500e-14` for analysis.

Callgrind 3.26.0 profiled f64 spin-1 T127 GL synthesis and analysis with one thread and five timed samples per operation. Total instruction references fell from `6,161,382,864` to `5,504,145,390` (`-10.7%`). References attributed to `scaled_basis_value` fell from `1,268,460,624` to `611,779,824` (`-51.8%`). `recurrence_step` and `normalize_pair_lanes` counts were unchanged.

Native f64 T511 spin-1 synthesis timings used rustc 1.99.0, AVX2/FMA dispatch, GL `512x1025`, `phi0=0.37`, reused plan and workspace, an AMD Ryzen 9 5950X, one warmup and five samples per operation. Three runs produced these medians and ranges in milliseconds:

| Threads | Before (median; range) | After (median; range) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `1275.961; 17.201` | `1261.893; 12.116` | `-1.1%` |
| 8 | `192.712; 6.608` | `187.478; 2.021` | `-2.7%` |

Native spin analysis used an analysis-only harness with the same T511 GL map components in two independent runs per thread count. The table gives the median of those run medians, their range, and the change:

| Threads | Before (median; range ms) | After (median; range ms) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `1302.151; 15.520` | `1287.612; 1.470` | `-1.1%` |
| 2 | `674.128; 4.653` | `658.970; 2.715` | `-2.2%` |
| 4 | `352.158; 7.576` | `343.406; 7.407` | `-2.5%` |
| 8 | `187.450; 3.810` | `183.233; 2.465` | `-2.2%` |
| 16 | `187.262; 0.902` | `184.304; 2.426` | `-1.6%` |

The rebuilt Python extension used Python 3.11.15, NumPy 2.4.6 and `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Each timing is the median of two run medians, with their range in milliseconds. RustSHT plan methods reuse a plan; procedural calls construct a plan and workspace:

| Operation | Threads | Plan before | Plan after | Procedural after | DUCC after |
| --- | ---: | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | `1295.828; 1.254` | `1234.085; 4.594` | `1233.950; 3.192` | `24.555; 0.795` |
| Spin-1 synthesis | 8 | `195.882; 3.562` | `188.329; 4.827` | `188.333; 2.448` | `4.071; 0.275` |
| Spin-1 analysis | 1 | `1349.172; 9.721` | `1237.097; 7.446` | `1236.646; 7.812` | `24.745; 0.243` |
| Spin-1 analysis | 8 | `197.045; 2.651` | `177.081; 0.186` | `176.728; 0.911` | `3.342; 0.031` |

The T511 Python plan errors match the preceding implementation. RustSHT remains `46–53x` slower than DUCC in these four comparisons.

The same zero-exponent shortcut was tested in f32, then removed. Its bitwise reference tests passed, but the T127 Callgrind total rose from `5,242,865,006` to `5,496,043,461` instructions (`+4.8%`). Two T511 native runs also increased all four times: synthesis from `1000.371` to `1041.570 ms` at one thread and `156.030` to `157.441 ms` at eight; analysis from `1020.537` to `1055.027 ms` at one thread and `151.225` to `153.853 ms` at eight. The accepted change applies only to f64.

With the final source, Rust 1.99 all-features tests pass 77 library tests and 10 f32 integration tests, with seven opt-in high-bandlimit tests ignored. No-default-features tests pass 74 library tests and the same 10 f32 tests. Rust 1.89 passes the new binary-scaling regression test. Formatting and all-feature Clippy with warnings denied pass; `uv run pytest python/tests` passes all eight tests.

With the final source, Rust 1.99 all-features tests pass 77 library tests and 10 f32 integration tests, with seven opt-in high-bandlimit tests ignored. No-default-features tests pass 74 library tests and the same 10 f32 tests. Rust 1.89 passes the new binary-scaling regression test. Formatting and all-feature Clippy with warnings denied pass; `uv run pytest python/tests` passes all eight tests.

### Fused scalar synthesis accumulation trial (M4, 2026-10-12)

I prototyped a `YlmVisitor` path that accumulated scalar synthesis values across latitude lanes with `pulp` multiply-adds. The fast branch required zero recurrence exponents, normal recurrence values, and normal-range products; scaled lanes and blocks with polar-inactive rings used `multiply_complex_scaled` in scalar code. Per-worker real and imaginary lanes reused the existing Fourier scratch size. The implementation passed the T255 scalar DUCC and analytic checks, all 10 f32 integration tests, and a T127 Python comparison with `ducc0==0.41.0` at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. T127 synthesis maximum absolute/RMS/relative L2 error was `4.302e-16`/`3.681e-17`/`1.243e-14`; analysis error was `3.490e-15`/`2.189e-16`/`1.046e-14`.

The native release benchmark used rustc 1.99.0, f64, T511 GL `512x1025`, `phi0=0.37`, an AMD Ryzen 9 5950X with AVX2/FMA, reused plans/workspaces, one warmup and five timed samples per operation. The before and trial values are three run medians, their median and range, in milliseconds:

| Operation | Threads | Before | Fused trial | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | `378.752, 405.141, 382.180` (`382.180`; `26.389`) | `504.780, 505.291, 505.042` (`505.042`; `0.511`) | `+32.1%` |
| Scalar synthesis | 8 | `69.452, 70.510, 67.089` (`69.452`; `3.421`) | `82.522, 79.422, 79.839` (`79.839`; `3.100`) | `+15.0%` |
| Scalar analysis | 1 | `337.745, 348.843, 342.813` (`342.813`; `11.098`) | `334.373, 336.556, 335.874` (`335.874`; `2.183`) | `-2.0%` |
| Scalar analysis | 8 | `61.920, 60.858, 51.921` (`60.858`; `9.999`) | `54.552, 50.450, 52.486` (`52.486`; `4.102`) | `-13.8%` |

Callgrind 3.26.0 counted one T127 GL f64 synthesis after plan and workspace setup. Instruction references fell from `134,680,957` to `96,449,265` (`-28.4%`). In the baseline profile, the scalar callback accounted for `92,125,424` inclusive references and `multiply_complex_scaled` for `55,416,586`. In the trial, `accumulate_scalar_synthesis` accounted for `48,899,366`, `normalize_pair_lanes` for `31,472,256`, and `multiply_complex_scaled` for `2,434,112`; these inclusive counts overlap. The lower instruction count did not reduce native runtime. The analysis timings do not isolate the scratch-layout change from the visitor change; the eight-thread ranges overlap, and the one-thread change is small. I removed the trial and retained the existing scalar callback and complex scratch layout.

### Scalar analysis order-major Fourier storage (M4/M5, 2026-10-12)

Scalar analysis now copies the active Fourier orders into contiguous latitude slices before recurrence accumulation. The transpose also applies the longitude phase and any GL ring weight and longitude normalization. CC/F1 prepared spectra retain their existing meridional normalization and are not weighted a second time. The scalar path uses the workspace's spin-spectrum arrays, which scalar transforms do not otherwise access, and removes one per-worker Fourier buffer. It adds no workspace allocation. Order-major input gives each recurrence block contiguous Fourier values; the transpose is included in the timings below.

Native release measurements used rustc 1.99.0, f64, T511 GL `512x1025`, `phi0=0.37`, AVX2/FMA dispatch, reused plans and workspaces, an AMD Ryzen 9 5950X, one warmup and five timed samples per run. The same `m1a_bench` executable measured one and eight Rayon threads. Each table entry gives three run medians, their median and range in milliseconds:

| Layout | Threads | Run medians (median; range ms) | Estimated workspace change |
| --- | ---: | ---: | ---: |
| Ring-major | 1 | `339.279, 337.550, 334.853` (`337.550`; `4.426`) | Baseline |
| Tiled, 64 latitudes | 1 | `322.722, 321.959, 321.559` (`321.959`; `1.163`) | About `+0.5 MiB` at T511 |
| Full order-major | 1 | `318.332, 318.615, 319.234` (`318.615`; `0.902`) | No added allocation |
| Ring-major | 8 | `55.900, 64.290, 53.165` (`55.900`; `11.125`) | Baseline |
| Tiled, 64 latitudes | 8 | `63.131, 55.411, 53.754` (`55.411`; `9.377`) | About `+0.5 MiB` at T511 |
| Full order-major | 8 | `56.491, 48.717, 54.108` (`54.108`; `7.774`) | No added allocation |

The full order-major path reduces one-thread analysis time by `5.6%` against the ring-major median. The eight-thread medians differ by `3.2%`, within the broad run-to-run variation. The tiled path reduces the one-thread median by `4.6%` and has no measured eight-thread gain. The full order-major scratch overlays the already allocated scalar-plan spin spectra; the removed per-worker buffer saves `1 KiB` per worker for f64. The tile requires about `4 MiB` at T4095, while the full order-major active range fits in the existing spectrum allocation. The full order-major path is selected for scalar analysis.

The rebuilt extension was compared with `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, using Python 3.11.15, NumPy 2.4.6, T511 GL data, and two runs of one warmup plus five samples. Plan analysis took `332.538 ms` (range `3.037 ms`) with one thread and `63.610 ms` (range `7.013 ms`) with eight threads; DUCC took `12.339 ms` (range `0.136 ms`) and `1.857 ms` (range `0.154 ms`). The measured plan-to-DUCC ratios were `26.9x` and `34.3x`. Analysis errors were `1.010e-14` maximum absolute, `2.281e-16` RMS and `4.467e-14` relative L2 at both thread counts. Input copying measured `0.052–0.084 ms`, and output allocation measured `0.001 ms`.

Rust 1.89 all-feature tests passed 77 library tests and 10 f32 integration tests, with seven opt-in high-bandlimit tests ignored. The no-default-features suite passed 74 library tests and the same 10 f32 tests. Formatting and workspace Clippy with warnings denied passed; `uv run pytest python/tests` passed all eight tests. Scalar DUCC comparisons and adjoint identities passed on GL, CC and F1.

### Scalar polar-block skipping trial (M4, 2026-10-12)

The prototype skipped a 64-ring scalar recurrence block when every ring's existing `ring_order_limits` excluded the current order. Partially active blocks kept their per-ring cutoff checks. The recurrence and its cutoff were unchanged, and workspace memory did not increase. Rust 1.89 all-feature tests passed 77 library tests and 10 f32 integration tests, with seven high-bandlimit tests ignored; scalar DUCC/analytic cases and the spin-3/4 regression test passed.

Native release timings used rustc 1.99.0, f64, T511 GL `512x1025`, `phi0=0.37`, AVX2/FMA dispatch, reused plans and workspaces, an AMD Ryzen 9 5950X, one warmup and five timed samples per run. Each table entry gives three run medians, their median and range in milliseconds:

| Operation | Threads | Before (median; range) | Trial (median; range) | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `357.103; 6.183` | `365.469; 3.299` | `+2.3%` |
| Synthesis | 8 | `55.907; 0.695` | `64.249; 6.822` | `+14.9%` |
| Analysis | 1 | `318.615; 0.902` | `313.851; 1.067` | `-1.5%` |
| Analysis | 8 | `54.108; 7.774` | `53.544; 4.808` | `-1.0%` |

The analysis differences fall within the run variation, and synthesis is slower in both thread settings. Callgrind instruction counts were not collected because the whole-transform measurements did not meet the performance criterion. The block-skipping change was removed.

### Scalar normal-range recurrence stage (M4, 2026-10-12)

The f64 scalar recurrence now enters an ordinary-arithmetic stage after every latitude lane in its 64-ring block reaches exponent zero. It uses the existing SIMD three-term update without calling lane normalization while each previous/current pair remains within the existing `2^-400` to `2^400` range. A per-degree magnitude check returns the whole block to the scaled path when a lane pair crosses that range, before the next recurrence step. The transition uses the same recurrence operations and adds no workspace memory. This first stage retains the normalized epsilon recurrence; paired-degree coefficient transformation remains a separate experiment.

The transition test uses order 255 and four latitudes. One near-polar lane begins with a negative binary exponent and reaches exponent zero during the recurrence; an interior lane remains in ordinary range. Every degree is checked against the retained scalar recurrence with relative error at most `2e-13`. The release T2047 scalar test also passed DUCC and closed-form comparisons on GL, CC and F1.

Native release measurements used rustc 1.99.0, f64, AVX2/FMA dispatch, an AMD Ryzen 9 5950X, reused plans and workspaces, one warmup and five timed samples per operation. Each entry is the median and range of three run medians, in milliseconds. T511 and T1023 used GL `512x1025` and `1024x2049`; T127 used GL `128x257`:

| Bandlimit | Operation | Threads | Before (median; range) | Staged (median; range) | Change | Estimated numeric arrays |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 127 | Synthesis | 1 | `6.829; 0.134` | `6.349; 0.091` | `-7.0%` | `1.11 MiB` |
| 127 | Synthesis | 8 | `3.586; 0.280` | `3.371; 0.074` | `-6.0%` | `1.11 MiB` |
| 127 | Analysis | 1 | `5.863; 0.173` | `5.649; 0.120` | `-3.7%` | `1.11 MiB` |
| 127 | Analysis | 8 | `3.177; 0.124` | `3.051; 0.050` | `-4.0%` | `1.11 MiB` |
| 511 | Synthesis | 1 | `358.573; 1.131` | `344.432; 1.882` | `-3.9%` | `16.25 MiB` |
| 511 | Synthesis | 8 | `55.687; 5.655` | `53.063; 1.673` | `-4.7%` | `16.25 MiB` |
| 511 | Analysis | 1 | `320.139; 1.435` | `315.721; 1.094` | `-1.4%` | `16.25 MiB` |
| 511 | Analysis | 8 | `49.656; 2.771` | `51.369; 7.307` | `+3.5%` | `16.25 MiB` |
| 1023 | Synthesis | 1 | `2878.420; 15.965` | `2718.456; 27.871` | `-5.6%` | `64.43 MiB` |
| 1023 | Synthesis | 8 | `470.642; 34.063` | `443.733; 18.029` | `-5.7%` | `64.43 MiB` |
| 1023 | Analysis | 1 | `2590.039; 106.294` | `2376.549; 33.930` | `-8.2%` | `64.43 MiB` |
| 1023 | Analysis | 8 | `415.100; 6.779` | `391.370; 7.145` | `-5.7%` | `64.43 MiB` |

The T511 eight-thread analysis medians have overlapping run ranges and do not establish a slowdown. The other native comparisons improve by `1.4–8.2%`. The workspace footprint is unchanged. Callgrind 3.26.0 counted one-thread T127 synthesis and analysis together: instruction references fell from `259,840,696` to `241,482,840` (`-7.1%`). References attributed to `multiply_complex_scaled` remained `89,595,102`; the reduction came from recurrence range handling.

The Python extension was rebuilt for the staged source and compared with `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Python 3.11.15 and NumPy 2.4.6 were used. Every timing is the median of two run medians, with their range in parentheses. Rust plan methods reuse their plan and construct a workspace per call; procedural calls construct both plan and workspace:

| Bandlimit | Operation | Threads | Plan before → staged | Procedural before → staged | DUCC before → staged | Plan speedup |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 511 | Synthesis | 1 | `377.374 (0.112) → 345.362 (0.230)` | `385.920 (1.714) → 352.847 (3.204)` | `11.552 (0.056) → 11.667 (0.073)` | `8.5%` |
| 511 | Synthesis | 8 | `64.312 (1.532) → 52.893 (1.173)` | `64.141 (11.509) → 59.464 (1.474)` | `1.852 (0.033) → 1.804 (0.005)` | `17.7%` |
| 511 | Analysis | 1 | `334.200 (0.396) → 297.634 (0.270)` | `336.377 (0.086) → 299.457 (0.980)` | `11.930 (0.036) → 12.164 (0.017)` | `10.9%` |
| 511 | Analysis | 8 | `54.438 (1.682) → 48.815 (2.176)` | `59.450 (2.721) → 47.977 (0.029)` | `1.675 (0.032) → 1.971 (0.029)` | `10.3%` |

The staged T511 plan-to-DUCC ratios are `29.6x` and `29.3x` for synthesis, and `24.5x` and `24.8x` for analysis at one and eight threads. Synthesis maximum absolute/RMS/relative L2 error is `2.627e-14`/`7.674e-16`/`6.487e-14`; analysis error is `1.010e-14`/`2.281e-16`/`4.467e-14`. At T511, NumPy input-copy measurements were `0.052–0.085 ms` and output allocation was `0.001 ms`.

At T1023 GL, the staged Python benchmark reported these medians and ranges of two run medians, in milliseconds:

| Operation | Threads | Rust plan | Procedural Rust | DUCC | Plan-to-DUCC |
| --- | ---: | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `2626.664 (16.403)` | `2619.677 (0.867)` | `113.304 (0.745)` | `23.2x` |
| Synthesis | 8 | `426.694 (9.767)` | `435.032 (14.290)` | `16.753 (0.646)` | `25.5x` |
| Analysis | 1 | `2286.198 (7.162)` | `2286.934 (0.771)` | `117.131 (1.400)` | `19.5x` |
| Analysis | 8 | `373.047 (1.153)` | `375.066 (7.508)` | `16.120 (0.284)` | `23.1x` |

Synthesis error was `1.459e-13` maximum absolute, `2.748e-15` RMS and `1.161e-13` relative L2; analysis error was `2.109e-14`, `2.208e-16` and `8.642e-14`. These errors matched at one and eight threads. T1023 NumPy input copies took `0.145–1.195 ms`; output allocation took `0.001–0.002 ms`.

Rust 1.89 passed the focused scaled-to-normal transition test. Rust 1.99 all-feature tests passed 78 library tests and 10 f32 integration tests, with seven high-bandlimit tests ignored; no-default-feature tests passed 75 library tests and the same 10 f32 tests. The release T2047 scalar DUCC/analytic test passed. Formatting, workspace Clippy with warnings denied, and all eight Python tests passed. The existing spin-3/4 regression test passed.

### Scalar north/south paired analysis (M4, 2026-10-12)

Scalar analysis pairs mirrored GL, CC and F1 latitudes, including the prepared CC/F1 grids. For each order, it forms the sum and difference of the already phased and weighted Fourier samples. The scalar parity relation `Y_lm(pi-theta) = (-1)^(l+m) Y_lm(theta)` selects the pair for each degree, so the Legendre recurrence runs on the northern half of the latitude grid. The equatorial sample is evaluated once and contributes through either parity sum. Per-ring polar limits are applied to each member of the pair. The implementation reuses the recurrence worker's Fourier scratch and adds no workspace memory. The same path handles adjoint synthesis; spin-weighted recurrences are unchanged.

Native release measurements used rustc 1.99.0, f64, AVX2/FMA dispatch, the AMD Ryzen 9 5950X (16 cores, 32 hardware threads), reused plans and workspaces, one warmup and five timed samples per run. Each entry gives the median and range of three run medians in milliseconds. The before values use commit `1be81f7` (the normal-range recurrence stage); GL shapes were `128x257`, `512x1025` and `1024x2049` for T127, T511 and T1023. Synthesis does not use this path and is omitted.

| Bandlimit | Threads | Analysis before (median; range) | Paired (median; range) | Change | Estimated numeric arrays |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 127 | 1 | `5.649; 0.120` | `2.612; 0.106` | `-53.8%` | `1.11 MiB` |
| 127 | 8 | `3.051; 0.050` | `1.688; 0.161` | `-44.7%` | `1.11 MiB` |
| 511 | 1 | `315.721; 1.094` | `168.799; 1.603` | `-46.5%` | `16.25 MiB` |
| 511 | 8 | `51.369; 7.307` | `33.283; 4.132` | `-35.2%` | `16.25 MiB` |
| 1023 | 1 | `2376.549; 33.930` | `1392.168; 11.730` | `-41.4%` | `64.43 MiB` |
| 1023 | 8 | `391.370; 7.145` | `281.124; 4.154` | `-28.2%` | `64.43 MiB` |

Callgrind 3.26.0 instruction references for T127 one-thread synthesis plus analysis fell from `241,482,840` to `179,841,418` (`-25.5%`). References attributed to `multiply_complex_scaled` fell from `89,595,102` to `66,971,461` (`-25.3%`). The work reduction comes from evaluating half as many latitude lanes in the recurrence and accumulation.

The rebuilt Python extension was compared with `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` on T511 GL. RustSHT plan analysis took `169.535 ms` with one thread and `34.593 ms` with eight threads; DUCC took `12.433 ms` and `1.704 ms`. The plan-to-DUCC ratios were `13.6x` and `20.3x`. Maximum absolute/RMS/relative L2 coefficient errors were `6.960e-15`/`1.602e-16`/`3.137e-14` at both thread counts. The release T511/T1023/T2047 scalar comparison against DUCC and analytic fields passed on GL, CC and F1. `cargo test --workspace --all-features` passed 78 library tests and 10 f32 integration tests, with seven high-bandlimit tests ignored; `cargo test -p rustsht --no-default-features` passed 75 library tests and the same 10 f32 tests. Rust 1.89 passed the transform test set (41 passed, seven ignored). Formatting, workspace Clippy with warnings denied, and all eight Python tests passed. The existing spin-3/4 regression also passed.

### DUCC transformed paired-degree scalar recurrence (M4, 2026-10-12)

The f64 scalar synthesis and analysis paths now use DUCC's transformed two-degree recurrence from `sht_inner_loop.h` at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Each order prepares DUCC's `alpha`, `a` and `b` factors and transforms the harmonic coefficients before recurrence accumulation; analysis applies the transpose coefficient transform after latitude accumulation. Sectoral initialization and recurrence values retain binary exponents near the poles. Once a latitude block enters the existing normal-value interval, the paired recurrence uses SIMD multiply-adds without per-degree normalization until a lane crosses the interval. The scaled path remains active for transition and out-of-range lanes. f32 keeps the existing one-degree recurrence. The paired factors reuse existing per-worker spin-factor scratch; the scalar epsilon array adds three values per worker (24 bytes for f64). No dense basis or additional spectrum workspace is allocated.

The recurrence test checks the transformed basis and both coefficient transforms for every order through T128 at polar, interior and equatorial latitudes, comparing `Arch::Scalar` and runtime dispatch, including a SIMD tail. A separate T768, order-255 case confirms three near-polar lanes transition from scaled to ordinary values and match the retained scalar recurrence after coefficient transformation; an interior lane remains unscaled. The release scalar T511/T1023/T2047 DUCC and analytic checks passed on GL, CC and F1. Scalar DUCC comparisons and adjoint identities, f32 scalar/spin checks and the spin-3/4 shared-kernel regression passed. Rust 1.89 passed the paired transition test and transform tests (41 passed, seven opt-in high-bandlimit tests ignored).

Native release measurements used rustc 1.99.0, f64, AVX2/FMA, an AMD Ryzen 9 5950X, GL grids with `nphi=2*lmax+3`, `phi0=0.37`, reused plans/workspaces, one warmup and five samples per operation. The before medians use commit `fb470b5`; the after values are medians and ranges of three run medians, in milliseconds. The earlier synthesis record preserved medians but not every run range.

| Bandlimit | Operation | Threads | Before median | Paired median (range) | Change | Estimated numeric arrays |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 127 | Synthesis | 1 | `6.349` | `5.569 (0.386)` | `-12.3%` | `1.11 MiB` |
| 127 | Synthesis | 8 | `3.371` | `3.056 (0.143)` | `-9.3%` | `1.11 MiB` |
| 127 | Analysis | 1 | `2.612` | `2.710 (0.015)` | `+3.8%` | `1.11 MiB` |
| 127 | Analysis | 8 | `1.688` | `1.590 (0.035)` | `-5.8%` | `1.11 MiB` |
| 511 | Synthesis | 1 | `344.432` | `300.864 (1.139)` | `-12.7%` | `16.25 MiB` |
| 511 | Synthesis | 8 | `53.063` | `48.611 (4.658)` | `-8.4%` | `16.25 MiB` |
| 511 | Analysis | 1 | `168.799` | `171.860 (0.321)` | `+1.8%` | `16.25 MiB` |
| 511 | Analysis | 8 | `33.283` | `31.945 (4.589)` | `-4.0%` | `16.25 MiB` |
| 1023 | Synthesis | 1 | `2718.456` | `2268.994 (9.786)` | `-16.5%` | `64.43 MiB` |
| 1023 | Synthesis | 8 | `443.733` | `388.398 (6.937)` | `-12.5%` | `64.43 MiB` |
| 1023 | Analysis | 1 | `1392.168` | `1410.099 (8.312)` | `+1.3%` | `64.43 MiB` |
| 1023 | Analysis | 8 | `281.124` | `273.985 (13.297)` | `-2.5%` | `64.43 MiB` |

Synthesis improves in all measured cases. Analysis is 1.3–3.8% slower at one thread and 2.5–5.8% faster at eight threads; the small changes should be treated as neutral at this sample count. Callgrind 3.26.0 instruction references for a warmed T127 one-thread synthesis plus analysis fell from `179,841,418` to `166,142,881` (`-7.6%`). The rounded numeric-array estimates are unchanged; the only storage increase is 24 bytes per f64 recurrence worker.

The rebuilt Python extension was measured against `ducc0==0.41.0` at the same DUCC revision, with Python 3.11.15, NumPy 2.4.6, T511 GL and `phi0=0.37`. Each timing is the median of two run medians, with their range in parentheses; each run used one warmup and five timed samples. RustSHT plan methods reuse the plan and create a workspace per call. Errors are identical at one and eight threads:

| Operation | Threads | RustSHT plan (ms) | DUCC (ms) | RustSHT / DUCC |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | `306.301 (1.487)` | `11.765 (0.020)` | `26.0x` |
| Scalar synthesis | 8 | `50.160 (0.598)` | `1.752 (0.006)` | `28.6x` |
| Scalar analysis | 1 | `165.310 (0.854)` | `11.870 (0.048)` | `13.9x` |
| Scalar analysis | 8 | `31.774 (0.631)` | `1.740 (<0.001)` | `18.3x` |

Scalar synthesis maximum absolute/RMS/relative L2 error was `2.316e-14`/`7.232e-16`/`6.114e-14`; analysis error was `3.722e-15`/`1.045e-16`/`2.046e-14`. The same rebuilt extension measured the unchanged spin-1 path at T511: synthesis was `1262.916 ms` and `195.012 ms` with one and eight threads, versus DUCC's `24.308 ms` and `3.793 ms`; analysis was `1280.497 ms` and `185.167 ms`, versus `24.757 ms` and `3.644 ms`. Spin-1 maximum absolute errors remained `2.365e-14` for synthesis and `1.463e-14` for analysis. The paired recurrence improves scalar synthesis but does not close the remaining DUCC gap; spin 1 remains a higher-priority kernel target.

### DUCC scalar-conversion path for spin-1 STANDARD transforms (M4/M5, 2026-10-12)

For spin-1 STANDARD synthesis at `lmax >= 500`, RustSHT now converts E/B coefficients into two scalar coefficient rows, evaluates the paired f64 scalar recurrence, and divides the middle-latitude sums by `sin(theta)`. Rings with `sin(theta) < 0.0001` use the general spin recurrence. Spin-1 exact analysis uses the transpose coefficient conversion and the paired scalar recurrence on GL rings whose sine meets the same cutoff; Fourier phase, ring quadrature, and `1/nphi` scaling are applied once to each input component. The coefficient conversions adapt DUCC `spin1to0` and `spin0to1` in `sht.cc` at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. CC/F1 analysis, GRAD_ONLY, DERIV1, and adjoint-synthesis paths use the general spin recurrence. Standard synthesis and its adjoint-analysis use the same standard synthesis operator.

The fast path is selected only for f64 STANDARD spin-1 operations with both coefficient channels. Four per-worker `Complex64` arrays of length `lmax+2` hold the converted and transposed coefficients. This adds `32,832` bytes per worker at T511 and `131,136` bytes per worker at T2047. The T511 one-thread Python workspace uses one worker (`32.1 KiB`); its eight-thread workspace uses eight workers (`256.5 KiB`). No map-sized or harmonic-matrix workspace is added.

The focused synthesis regression compares the fast and general spin recurrences over 65 T511 GL rings, including the near-pole fallback, for E-only, B-only, and mixed E/B inputs at `phi0=0.37`. The largest Fourier-coefficient discrepancy is bounded by `3e-11`. The coefficient-conversion test checks the spin-1 basis through T16 and verifies the adjoint coefficient transform for every order. The release T511 spin-1 DUCC check passed on GL, CC and F1; the T1023 exact-analysis comparison passed on all three grids. The release T2047 sparse spin-1/2 synthesis check passed on all grids. The T511 spin-3/4 all-grid regression and T2047 sparse spin-3/4 check passed after the shared-kernel change. STANDARD spin adjoint identities, spin-1/2 regression tests, GRAD_ONLY and DERIV1 tests, and the f32 spin-1 through spin-4 tests passed.

Native release timings used `examples/m1b_bench.rs`, rustc 1.99.0, f64, T511 GL `512x1025`, `phi0=0.37`, an AMD Ryzen 9 5950X with AVX2/FMA dispatch and no AVX-512, reused plans/workspaces, one warmup, five timed samples, and three repeated runs. Before values are the general spin recurrence from the previous implementation; after values enable spin-1 scalar conversion for synthesis and one-worker analysis. Entries give the median and range of the three run medians in milliseconds:

| Operation | Threads | General (median; range) | Scalar conversion (median; range) | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `1261.893; 12.116` | `555.661; 6.893` | `-56.0%` |
| Synthesis | 8 | `187.478; 2.021` | `98.354; 1.582` | `-47.5%` |
| Analysis | 1 | `1287.612; 1.470` | `743.343; 4.235` | `-42.3%` |
| Analysis | 8 | `183.233; 2.465` | `178.864; 3.355` | `-2.4%` |

An isolated analysis-path comparison measured the general and scalar-conversion variants at `1247.876 ms` and `743.343 ms` with one worker (`-40.4%`). At eight workers, scalar conversion took `205.712 ms` versus `178.864 ms` for the general path (`+15.0%`). The analysis fast path therefore runs only with one Rayon worker. Synthesis improved at both measured thread counts.

The rebuilt Python extension was compared with `ducc0==0.41.0` from the same source revision on T511 GL, with Python 3.11.15 and NumPy 2.4.6. Each result is the median of two run medians; ranges are in milliseconds. Each run used one warmup and five timed samples. RustSHT plan methods reuse the plan and allocate a workspace per call; procedural functions construct a plan and workspace:

| Operation | Threads | RustSHT plan (median; range) | Procedural RustSHT (median; range) | DUCC (median; range) | Plan / DUCC |
| --- | ---: | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `557.368; 3.742` | `557.511; 0.903` | `24.479; 0.543` | `22.8x` |
| Synthesis | 8 | `105.087; 1.430` | `101.582; 0.263` | `3.750; 0.123` | `28.0x` |
| Analysis | 1 | `696.101; 0.699` | `692.377; 0.359` | `24.882; 0.055` | `28.0x` |
| Analysis | 8 | `190.822; 7.762` | `184.648; 0.018` | `3.535; 0.033` | `54.0x` |

Synthesis errors against DUCC were `2.515e-14` maximum absolute, `7.703e-16` RMS and `6.328e-14` relative L2 at both thread counts. Analysis errors were `1.237e-14`, `2.126e-16` and `4.163e-14` with one thread, and `1.463e-14`, `2.298e-16` and `4.500e-14` with eight threads. The measured plan-construction time was `0.035–0.037 ms`; NumPy input copies took `0.087–0.106 ms` for coefficients and `0.136–0.193 ms` for maps; output allocation took `0.001 ms`. These Python costs are small relative to transform execution. RustSHT remains `22.8–54.0x` slower than DUCC for these plan-method comparisons.

`cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test --workspace --all-features` (82 library tests and 10 f32 integration tests passed; seven opt-in checks ignored), `cargo test -p rustsht --no-default-features` (79 library tests and 10 f32 integration tests passed; seven opt-in checks ignored), and `uv run pytest python/tests` (eight passed) passed. The release T2047 sparse spin check and T511 spin-3/4 regression also passed. No instruction-count profile was collected for this change.

### SIMD backend and register-resident scalar kernel evaluation (M4, 2026-10-12)

I compared `fearless_simd` 1.1.0 with `pulp` 0.22.3 in a temporary f64 paired-recurrence path. The runtime-dispatched kernels used equivalent coefficients, contiguous latitude lanes, FMA arithmetic, and scalar tails. The native recurrence microbenchmark used order 17, a 64-lane block or a 61-lane block with a three-lane tail, and three runs of five timed samples. Each T127 sample ran 1,000 recurrence executions; each T511 sample ran 250. The cells give the median of the three run medians and their range, in milliseconds:

| Grid | Lanes | Pulp AVX2 | Fearless AVX2 | Maximum absolute error vs. Pulp scalar | Maximum error divided by `max(1, reference maximum)` |
| --- | ---: | ---: | ---: | ---: | ---: |
| T127 | 64 | `0.701; 0.002` | `0.695; 0.003` | `0` | `0` |
| T127 | 61 | `0.700; 0.004` | `0.698; 0.003` | `0` | `0` |
| T511 | 64 | `0.767; 0.008` | `0.760; 0.006` | `0` | `0` |
| T511 | 61 | `0.780; 0.006` | `0.782; 0.004` | `0` | `0` |

The full-transform comparison used `examples/m1a_bench.rs`, release builds, f64 GL grids (`128x257` and `512x1025`), `phi0=0.37`, reused plans/workspaces, one warmup and five timed samples. Three independent runs were made at T127 and four at T511. Each table cell gives the median and range of those run medians, in milliseconds:

| Grid | Operation | Threads | Pulp | Fearless | Fearless change |
| --- | --- | ---: | ---: | ---: | ---: |
| T127 | Synthesis | 1 | `5.677; 0.159` | `5.552; 0.021` | `-2.2%` |
| T127 | Synthesis | 8 | `3.132; 0.029` | `2.950; 0.459` | `-5.8%` |
| T127 | Analysis | 1 | `2.607; 0.044` | `2.676; 0.087` | `+2.6%` |
| T127 | Analysis | 8 | `1.544; 0.379` | `1.487; 0.055` | `-3.7%` |
| T511 | Synthesis | 1 | `308.062; 5.197` | `301.054; 3.771` | `-2.3%` |
| T511 | Synthesis | 8 | `50.531; 1.076` | `50.835; 3.765` | `+0.6%` |
| T511 | Analysis | 1 | `164.792; 1.719` | `168.305; 13.382` | `+2.1%` |
| T511 | Analysis | 8 | `31.426; 0.374` | `31.757; 1.391` | `+1.1%` |

The measured AVX2 dispatch was `Level::Avx2` on an AMD Ryzen 9 5950X with rustc 1.99.0. The full T127 Callgrind 3.26.0 run, including the one- and eight-thread synthesis and analysis measurements, counted `2,060,735,023` instructions for Pulp and `2,069,555,455` for Fearless (`+0.4%`). Whole-transform timings vary by operation and thread count and do not show a repeatable Fearless advantage. The T127 eight-thread synthesis difference is within its run spread. The evaluation build added no workspace memory.

Both AVX2 implementations emit four-lane `f64` loads/stores and packed FMA in the recurrence. Neither recurrence requires a horizontal lane reduction or masked operation; each lane remains an independent latitude. Both use scalar tails. The fused kernels also emit packed FMA and have no YMM stack spills in the inspected inner loops. Pulp emitted an AVX-512 implementation and Fearless emitted its AVX-512 dispatch target, but the 5950X does not support AVX-512, so neither implementation was executed at that ISA. `perf` hardware events were unavailable because `perf_event_paranoid=4`; Callgrind supplied the instruction count.

The fallback timings are not an equal-width comparison. `pulp::Arch::Scalar` is one scalar lane, while Fearless's x86-64 `Level::Fallback` uses its two-lane baseline `f64` representation. For 64 latitudes, Pulp scalar took `13.639 ms` versus `1.404 ms` for Fearless fallback over 1,000 T127 recurrence executions; at T511, the times were `15.260 ms` versus `1.575 ms` over 250 executions. Their outputs differed from the Pulp scalar reference by at most `3.801e-13` absolute (maximum error divided by `max(1, reference maximum)`: `2.319e-14`) at T127 and `7.475e-12` absolute (`1.138e-13` by the same scale) at T511. The width difference makes the fallback times unsuitable for selecting a library on AVX2 hardware.

I also tested a different fused kernel from the earlier rejected visitor trial above. This kernel keeps the paired recurrence state and real/imaginary sums in SIMD registers across degree pairs; it writes only the final latitude sums and does not call a visitor or store recurrence vectors after each degree. Its separate-path reference dispatches once per block and keeps scalar lane accumulation, matching the existing loop structure. The experiment used an interior GL latitude block, order 17, preallocated output and recurrence buffers, and three runs of five samples. Times are milliseconds per 200 T127 runs or 50 T511 runs; ranges are across run medians:

| Grid | Lanes | Backend | Separate (median; range) | Fused (median; range) | Change |
| --- | ---: | --- | ---: | ---: | ---: |
| T127 | 64 | Pulp | `0.360; 0.001` | `0.248; 0.075` | `-31%` |
| T127 | 64 | Fearless | `0.377; 0.003` | `0.207; <0.001` | `-45%` |
| T127 | 61 | Pulp | `0.377; 0.073` | `0.199; 0.001` | `-47%` |
| T127 | 61 | Fearless | `0.334; 0.001` | `0.203; <0.001` | `-39%` |
| T511 | 64 | Pulp | `0.389; 0.001` | `0.223; 0.006` | `-43%` |
| T511 | 64 | Fearless | `0.412; 0.004` | `0.224; <0.001` | `-46%` |
| T511 | 61 | Pulp | `0.345; 0.002` | `0.221; 0.005` | `-36%` |
| T511 | 61 | Fearless | `0.365; 0.001` | `0.223; <0.001` | `-39%` |

The fused accumulation order changed the output by at most `2.842e-14` absolute (maximum error divided by `max(1, reference maximum)`: `1.344e-16`) at T127 and `1.137e-13` (`2.070e-17`) at T511. These measurements cover one all-normal order and do not establish a whole-transform gain; the production path still needs its scaled and polar fallback and end-to-end benchmarking.

I retained Pulp. AVX2 recurrence and fused timings are within measurement variation, the complete transforms show no consistent improvement, the T127 Callgrind count is slightly higher with Fearless, and the temporary Fearless integration duplicated the paired recurrence control flow. The comparison adds no permanent SIMD dependency or workspace storage. The f64 scalar synthesis kernel described below uses Pulp. DUCC and analytic checks passed with the Fearless dispatcher at T511, T1023 and T2047 on GL, CC and F1. The full `cargo test -p rustsht --features fearless-simd-eval` run passed 82 library and 10 f32 tests, with seven opt-in checks ignored.

### Register-resident f64 scalar synthesis (M4, 2026-10-12)

F64 scalar synthesis now accumulates the DUCC-transformed even and odd degree coefficients while the paired recurrence values remain in SIMD registers. Each worker block writes one real and imaginary sum per latitude after its degree loop. A block uses this path only when every ring is active for the order and sectoral initialization is already in the ordinary range. Other blocks use the scaled paired recurrence. The fast path adds no workspace arrays and preserves the existing block size. F32 and all analysis and spin operators continue through their existing paths.

The new test compares fused sums against the scaled paired recurrence for orders 0, 1, 17, 78 and 79 through T95, with 64-lane and 61-lane blocks under scalar and dispatched Pulp backends. A T768 order-255 test covers a normal-range high-order block over 64 latitudes. Separate cases verify fallback for an inactive ring and scaled sectoral initialization. Scalar DUCC and analytic tests pass through T255; the release T511/T1023/T2047 test passes synthesis and exact analysis against DUCC and analytic sectoral fields on GL, CC and F1. The spin-3/4 DUCC, analytic and adjoint regression passes. These cases check the changed summation order against the general recurrence and the transform output against independent references.

Native release measurements used `examples/m1a_bench.rs`, rustc 1.99.0, f64, GL grids with `nphi=2*lmax+3`, `phi0=0.37`, reused plans and workspaces, and an AMD Ryzen 9 5950X with AVX2/FMA runtime dispatch. Each run used one warmup and five timed samples for one- and eight-thread synthesis and analysis. Three after runs were made; values give the median and range of those run medians. Before values are the Pulp results recorded in the SIMD comparison above, from three T127 runs and four T511 runs with the same harness and settings:

| Grid | Operation | Threads | Before (median; range) | Fused (median; range) | Change |
| --- | --- | ---: | ---: | ---: | ---: |
| T127 | Synthesis | 1 | `5.677; 0.159 ms` | `1.992; 0.051 ms` | `-64.9%` |
| T127 | Synthesis | 8 | `3.132; 0.029 ms` | `1.193; 0.190 ms` | `-61.9%` |
| T127 | Analysis | 1 | `2.607; 0.044 ms` | `2.814; 0.013 ms` | `+7.9%` |
| T127 | Analysis | 8 | `1.544; 0.379 ms` | `1.667; 0.167 ms` | `+8.0%` |
| T511 | Synthesis | 1 | `308.062; 5.197 ms` | `82.875; 0.993 ms` | `-73.1%` |
| T511 | Synthesis | 8 | `50.531; 1.076 ms` | `30.958; 0.537 ms` | `-38.7%` |
| T511 | Analysis | 1 | `164.792; 1.719 ms` | `171.912; 2.208 ms` | `+4.3%` |
| T511 | Analysis | 8 | `31.426; 0.374 ms` | `32.879; 6.336 ms` | `+4.6%` |

The analysis implementation did not change; its measured medians varied by `4.3–8.0%` from the earlier run, with overlapping eight-thread variation at T511. The optimization is accepted for scalar synthesis, where the one-thread T511 median fell by `73.1%` and the eight-thread median by `38.7%`. Callgrind 3.26.0 counted `1,158,116,417` instructions for the T127 benchmark process, down from `2,060,735,023` in the Pulp baseline (`-43.8%`). The estimate of T511 numeric-array memory remains `16.25 MiB`; the kernel reuses existing recurrence scratch.

The rebuilt Python plan was compared with `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, using Python 3.11.15, NumPy 2.4.6, two runs of one warmup and five timed samples, and the same input data and thread count. Plan methods construct a workspace per call. At T511, RustSHT plan synthesis medians were `84.661 ms` and `32.531 ms` at one and eight threads; DUCC medians were `11.698 ms` and `1.780 ms`. Analysis medians were `170.061 ms` and `32.448 ms`; DUCC medians were `12.306 ms` and `1.784 ms`. T511 synthesis errors were `2.316e-14` maximum absolute, `7.232e-16` RMS and `6.113e-14` relative L2. Analysis errors were `3.722e-15`, `1.045e-16` and `2.046e-14`. At T127, the Python plan synthesis medians were `3.498 ms` and `4.055 ms`, compared with DUCC's `1.098 ms` and `0.243 ms`; analysis medians were `2.667 ms` and `1.604 ms`, compared with `0.988 ms` and `0.257 ms`. The T127 eight-thread synthesis run medians differed by `1.952 ms`, so that comparison is noisy. T127 errors were `4.588e-16` maximum absolute, `3.603e-17` RMS and `1.217e-14` relative L2 for synthesis; analysis errors were `1.485e-15`, `1.002e-16` and `4.784e-15`. The end-to-end comparison shows improved scalar synthesis, but RustSHT remains slower than DUCC, especially with eight threads.

`cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test --workspace --all-features` (85 library tests and 10 f32 integration tests passed; seven opt-in high-bandlimit tests ignored), `cargo test -p rustsht --no-default-features` (82 library tests and 10 f32 tests passed; seven opt-in tests ignored), and `uv run pytest python/tests` (eight passed) passed. The release T511/T1023/T2047 scalar DUCC and analytic check passed on all three grids.

### DUCC scalar-conversion path for spin-2 STANDARD transforms (M4/M5, 2026-10-12)

For f64 spin-2 STANDARD transforms with `lmax >= 500`, synthesis converts E/B coefficients to two scalar coefficient rows through degree `lmax+2`, uses the paired scalar recurrence, and divides the middle-latitude result by `sin(theta)^2`. Rings with `sin(theta) < 0.01` retain the general spin recurrence. The conversion factors and spin-2-to-scalar formulas adapt DUCC `spin2to0` in `sht.cc` at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The scalar-to-spin-2 analysis conversion is the exact transpose of RustSHT's synthesis conversion, including both extension degrees. The fast analysis path is limited to exact GL analysis with one Rayon worker; CC/F1 analysis and eight-worker GL analysis use the general recurrence. GRAD_ONLY and DERIV1 remain on their existing paths. No spin-3/4 specialization was added.

The added plan storage is two `f64` factor arrays, 8,192 bytes at T511 and 32,768 bytes at T2047. Four worker coefficient arrays use 33,024 bytes per worker at T511 and 131,328 bytes per worker at T2047. The extra recurrence factor uses eight bytes per worker. Thus eight-worker scratch increases by about 258 KiB at T511 and 1.00 MiB at T2047; no map-sized or dense harmonic workspace is added. The implementation is f64-only and keeps the existing portable scalar recurrence fallback.

The coefficient-conversion test compares the spin-2 scalar conversion with the general spin recurrence at six latitudes through `lmax=16`, and checks its transpose against basis-vector inner products for every order. The release T511 spin-1/2 DUCC test passed for GL, CC and F1; spin-2 maximum DUCC synthesis-sample error was `5.107e-15`, maximum selected analysis-coefficient error `2.533e-17`, and maximum synthesis-analysis coefficient error `1.310e-14`. At T1023, spin-2 sparse synthesis passed on all grids with maximum DUCC sample error `4.115e-15`; exact analysis had maximum selected coefficient error `8.962e-18` and maximum synthesis-analysis coefficient error `3.175e-14`. The T2047 sparse test passed on all grids with maximum DUCC sample error `2.085e-13` and maximum analytic field error `1.236e-11` (RMS `3.883e-13`). The T511 spin-3/4 all-grid DUCC and analytic regressions and the routine spin-3/4 adjoint identities passed after the shared changes.

Native release timings used `examples/m1b_bench.rs`, rustc 1.99.0, f64, T511 GL `512x1025`, `phi0=0.37`, an AMD Ryzen 9 5950X with AVX2/FMA dispatch and no AVX-512, reused plans/workspaces, one warmup, five timed samples, and three repeated runs. The before timings used the general spin recurrence; after timings enable scalar conversion for synthesis and one-worker analysis. Values are medians and ranges of the three run medians in milliseconds:

| Operation | Threads | General (median; range) | Scalar conversion (median; range) | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `1265.391; 3.790` | `572.065; 11.017` | `-54.8%` |
| Synthesis | 8 | `188.096; 0.907` | `98.378; 3.112` | `-47.7%` |
| Analysis | 1 | `1310.395; 17.579` | `740.092; 8.492` | `-43.5%` |
| Analysis | 8 | `184.454; 4.611` | `184.236; 2.627` | `-0.1%` |

The first eight-worker analysis run with scalar conversion enabled measured `214.279 ms`, so the fast analysis path is disabled above one worker. This retains the one-worker improvement without a multithread regression.

### Comparative profiling baseline and normal-range multiplication (M4, 2026-10-12)

The T511 phase profile identifies recurrence and harmonic accumulation as the largest excess-cost phases. RustSHT longitude FFTs are a small fraction of one-thread scalar and spin-1 transforms. At eight threads, FFT work and Python-side workspace allocation become more visible, but the recurrence and accumulation phases still dominate the absolute gap.

Measurements ran on an AMD Ryzen 9 5950X (16 cores, 32 logical CPUs) under Linux 7.0.14, pinned to CPUs 0–7. The host supports AVX2 and FMA, but not AVX-512. Rust used `rustc 1.99.0` (LLVM 23.1.1), `cargo build --release`, and the default release flags `-C opt-level=3 -C embed-bitcode=no -C strip=debuginfo`; no `RUSTFLAGS`, `target-cpu=native`, LTO, or fast-math flags were used. Pulp selected its AVX2/FMA runtime path. Native benchmark examples reused plans and workspaces. Python benchmarks used Python 3.11.15, NumPy 2.4.6, T511 GL grids (`512x1025`), `lmax=mmax=511`, `phi0=0.37`, one warm-up and five timed samples per run, and two process runs. Rust plan methods allocate a workspace per call; the procedural functions also construct a plan per call.

The end-to-end reference was the installed `ducc0==0.41.0` wheel at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The wheel contains GCC 14.2.1 and GCC 8.5.0 build identifiers, but does not record its SHT compiler flags. Its SHT disassembly uses two-lane XMM instructions and contains no YMM, ZMM, or FMA instructions. A temporary source build at the same revision identifies itself as DUCC 0.41.1. CMake 4.2.3 and GCC 15.2.0 compiled it with `-O3 -DNDEBUG -flto=auto -fno-fat-lto-objects -fPIC -fvisibility=hidden -march=native -g -ffast-math`; that build emits YMM and FMA instructions. Source-build phase results below are therefore separate from the installed 0.41.0 wheel timings.

| T511 operation | Threads | RustSHT Python plan (ms) | DUCC 0.41.0 (ms) | Ratio | Excess (ms) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | `81.670` | `13.843` | `5.9x` | `67.827` |
| Scalar analysis | 1 | `160.008` | `12.270` | `13.0x` | `147.738` |
| Scalar synthesis | 8 | `29.833` | `3.690` | `8.1x` | `26.143` |
| Scalar analysis | 8 | `31.325` | `1.681` | `18.6x` | `29.644` |
| Spin-1 synthesis | 1 | `482.704` | `26.571` | `18.2x` | `456.133` |
| Spin-1 analysis | 1 | `700.185` | `24.989` | `28.0x` | `675.196` |
| Spin-1 synthesis | 8 | `85.325` | `5.744` | `14.9x` | `79.581` |
| Spin-1 analysis | 8 | `185.610` | `3.373` | `55.0x` | `182.237` |

The Rust plan and procedural function medians differed by at most 1.9% at T511; plan construction is about `0.036 ms`. At one thread, NumPy copies took `0.05–0.11 ms` for coefficients and `0.07–0.21 ms` for maps; output allocation took `0.001 ms`. At T1023, coefficient and map copies took `1.3 ms` and `3.8 ms`, compared with transform times from `196 ms` to `7,980 ms`. Python copying does not explain the transform gap. The native scalar workspace estimate is `16.08 MiB` at one worker and `16.25 MiB` at eight workers; the multiplication change below adds no scratch storage.

T127 Python-plan results are sensitive to Rayon scheduling because the transforms are small. Rust scalar analysis measured `2.577 ms` at one thread and `1.856 ms` at eight, against DUCC `0.999 ms` and `0.206 ms`; the eight-thread Rust run spread was `0.738 ms`. Spin-1 synthesis measured `21.229 ms` and `13.401 ms`, against DUCC `2.487 ms` and `0.611 ms`; the eight-thread Rust spread was `4.608 ms`. T127 is used for instruction profiling, not stable throughput ratios.

T1023 checks show the same kernel scaling at larger grids. Each value is a median of five timed samples from one process run; errors compare RustSHT against DUCC 0.41.0:

| T1023 operation | Threads | RustSHT Python plan (ms) | DUCC 0.41.0 (ms) | Max absolute error |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | `647.842` | `120.055` | `1.575e-13` |
| Scalar analysis | 1 | `1,327.678` | `118.734` | `1.041e-14` |
| Scalar synthesis | 8 | `195.732` | `18.551` | `1.575e-13` |
| Scalar analysis | 8 | `270.569` | `15.956` | `1.041e-14` |
| Spin-1 synthesis | 1 | `3,698.866` | `248.707` | `1.611e-13` |
| Spin-1 analysis | 1 | `7,980.375` | `250.312` | `1.730e-14` |
| Spin-1 synthesis | 8 | `685.743` | `40.148` | `1.611e-13` |
| Spin-1 analysis | 8 | `1,591.153` | `38.462` | `1.812e-14` |

Phase timers cover complete longitude loops, preparation stages, and recurrence/accumulation scheduling. The Rust phases use reused native workspaces. DUCC phases use the temporary 0.41.1 source build; whole DUCC Python measurements include wrapper and output handling. Phase subtotals therefore do not equal the DUCC whole-call times, especially for eight-thread synthesis.

| T511 phase | Threads | RustSHT (ms) | DUCC source 0.41.1 (ms) | Ratio | Excess (ms) | Rust share of whole | DUCC share of whole |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar synthesis recurrence and accumulation | 1 | `75.27` | `6.66` | `11.3x` | `68.61` | `92.2%` | `68.0%` |
| Scalar synthesis inverse FFT and row copy | 1 | `3.48` | `1.79` | `1.9x` | `1.69` | `4.3%` | `18.2%` |
| Scalar synthesis recurrence and accumulation | 8 | `25.78` | `1.04` | `24.8x` | `24.74` | `86.4%` | `33.2%` |
| Scalar synthesis inverse FFT and row copy | 8 | `3.52` | `0.38` | `9.3x` | `3.14` | `11.8%` | `12.1%` |
| Scalar analysis recurrence and accumulation | 1 | `154.9` | `4.49` | `34.5x` | `150.4` | `97.6%` | `69.7%` |
| Scalar analysis FFT and packing | 1 | `3.4` | `1.85` | `1.8x` | `1.6` | `2.1%` | `28.7%` |
| Scalar analysis recurrence and accumulation | 8 | `27.1` | `0.71` | `38.2x` | `26.4` | `87.7%` | `66.2%` |
| Scalar analysis FFT and packing | 8 | `3.4` | `0.30` | `11.3x` | `3.1` | `11.0%` | `28.0%` |
| Spin-1 synthesis recurrence, conversion and accumulation | 1 | `469.4` | `14.8` | `31.7x` | `454.6` | `98.3%` | `72.9%` |
| Spin-1 synthesis inverse FFT and row copy | 1 | `7.8` | `3.74` | `2.1x` | `4.1` | `1.6%` | `18.4%` |
| Spin-1 synthesis recurrence, conversion and accumulation | 8 | `75.9` | `2.00` | `38.0x` | `73.9` | `90.4%` | `46.9%` |
| Spin-1 synthesis inverse FFT and row copy | 8 | `7.9` | `0.58` | `13.6x` | `7.3` | `9.4%` | `13.5%` |
| Spin-1 analysis recurrence, conversion and accumulation | 1 | `684.3` | `8.88` | `77.1x` | `675.4` | `99.0%` | `69.3%` |
| Spin-1 analysis FFT and packing | 1 | `6.7` | `3.83` | `1.7x` | `2.9` | `1.0%` | `29.9%` |
| Spin-1 analysis recurrence, conversion and accumulation | 8 | `173.9` | `1.40` | `124.2x` | `172.5` | `96.0%` | `68.1%` |
| Spin-1 analysis FFT and packing | 8 | `7.1` | `0.62` | `11.5x` | `6.5` | `3.9%` | `30.0%` |

Warm phase totals reconcile with native Rust timings within `2–7 ms`. Rust scalar analysis at T511 takes about `158.7 ms` at one thread and `30.9 ms` at eight across the timed phases. The DUCC source analysis phases take about `6.4 ms` and `1.0 ms`; whole-call medians were `6.443 ms` and `1.072 ms`. The larger spin-1 synthesis difference at eight threads includes about `1.7 ms` of DUCC Python wrapper and output handling outside the phase timers. Across the table, recurrence and accumulation account for the largest absolute excess; FFT threading is not the next optimization target.

The scalar synthesis phase measurements use reused native Rust workspaces and source-build DUCC whole-call medians of `9.800 ms` (one thread) and `3.126 ms` (eight threads). Rust phase medians were `75.27/3.48 ms` and `25.78/3.52 ms` for recurrence plus accumulation and inverse FFT plus row copy. DUCC phase medians were `6.66/1.79 ms` and `1.04/0.38 ms`. DUCC's per-call scratch allocation phase was `1.319 ms` and `1.514 ms` at one and eight threads, respectively, but individual samples ranged from about `0.006 ms` to `1.54 ms`; treat these allocator measurements as variable. Repeated native Rust workspace creation took `3.77–3.91 ms` with one worker and `3.74–4.04 ms` with eight workers for scalar and spin-1 workspaces. Plan construction took `0.034–0.039 ms`. The Python plan-method timings above include this workspace creation on each call; native phase rows exclude it by reusing the workspace.

`perf stat` cannot access hardware events because `perf_event_paranoid=4`. Callgrind 3.26.0 and disassembly were used instead. In the T127 scalar analysis Callgrind process, Rust's old `multiply_complex_scaled_product` accounted for `118,789,020` instruction references (`17.6%`). Its normal-range branch computed a product, then called `scale_complex_normal_binary`, which returned the same value when the recurrence exponent was zero. Rust generated scalar `mulsd` instructions and a function call on this path. DUCC's source Callgrind profile attributes `4,693,350` inclusive instruction references to `map2alm_kernel` across six T127 calls; its source disassembly uses four-lane YMM operations and FMA. The installed 0.41.0 wheel uses two-lane XMM operations. These function boundaries differ, so the instruction totals are hotspot evidence rather than an end-to-end instruction ratio. The profiles show that Rust's analysis accumulation still performs per-lane complex work after its SIMD recurrence update, while DUCC combines recurrence and accumulation in its SIMD kernel.

The first measured change bypasses binary scaling when a complex product has exponent zero and both output components are normal. The scaled fallback still handles underflow, overflow, subnormals, signed zero and non-finite inputs. This path also applies to `multiply_complex_scaled`, which is used by spin transforms. It adds no workspace bytes.

Native T511 release timings used reused plans and workspaces, one warm-up, five samples per run and three process runs. Values are medians and ranges of run medians:

| Operation | Threads | Before (ms) | After (ms) | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar analysis | 1 | `163.962; 1.186` | `157.944; 1.958` | `-3.7%` |
| Scalar analysis | 8 | `31.452; 1.337` | `30.787; 0.927` | `-2.1%` |
| Spin-1 synthesis | 1 | `562.386; 1.753` | `486.063; 3.115` | `-13.6%` |
| Spin-1 synthesis | 8 | `96.761; 1.618` | `82.886; 0.415` | `-14.3%` |
| Spin-1 analysis | 1 | `716.505; 6.820` | `695.025; 5.726` | `-3.0%` |
| Spin-1 analysis | 8 | `183.996; 0.122` | `175.986; 2.500` | `-4.4%` |

Callgrind instruction references for the T127 scalar analysis benchmark process fell from `673,623,657` to `627,299,930` (`-6.9%`); data references fell from `218,216,893` to `190,544,213` (`-12.7%`). The before process ran 12 analysis transforms across one-thread and one-worker Rayon pools. The after profile let LLVM inline the normal-range multiplication into the recurrence callback. T511 scalar and spin-1 errors against DUCC remained at the prior recorded values: scalar synthesis maximum absolute/RMS/relative L2 `2.316e-14`/`7.232e-16`/`6.113e-14`; scalar analysis `3.722e-15`/`1.045e-16`/`2.046e-14`; spin-1 synthesis `2.515e-14`/`7.703e-16`/`6.328e-14`; spin-1 analysis `1.237e-14`/`2.126e-16`/`4.163e-14` at one thread. The routine spin-3/4 regression passed after this shared arithmetic change.

`cargo test --workspace --all-features` passed 88 library tests and 10 f32 integration tests, with seven opt-in high-bandlimit tests ignored. `cargo test -p rustsht --no-default-features` passed 85 library tests and 10 f32 tests, with the same seven tests ignored. `uv run pytest python/tests` passed eight tests. The measured scalar-analysis improvement is smaller than the remaining kernel gap; the next experiment targets register-resident SIMD accumulation in scalar analysis.

The rebuilt release Python extension was compared with `ducc0==0.41.0` at the same pinned source revision. The host was the Ryzen 9 5950X; Python was 3.11.15 and NumPy 2.4.6. The harness used T511 GL inputs, reused the plan, allocated a workspace per RustSHT call, and took two run medians of five samples after a warmup. RustSHT plan timings include their per-call workspace allocation; all times are milliseconds:

| Operation | Threads | RustSHT plan (median; range) | DUCC (median; range) | Plan / DUCC |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | `575.255; 0.463` | `26.345; 0.215` | `21.8x` |
| Synthesis | 8 | `102.512; 2.649` | `4.088; 0.080` | `25.1x` |
| Analysis | 1 | `696.290; 2.904` | `27.862; 0.220` | `25.0x` |
| Analysis | 8 | `188.181; 1.842` | `4.147; 0.078` | `45.4x` |

Synthesis errors were `5.055e-14` absolute (`1.639e-15` RMS, `1.347e-13` relative L2) at both thread counts. Analysis errors were `1.317e-14` absolute (`2.707e-16` RMS, `5.301e-14` relative L2) with one thread, and `1.527e-14` (`2.541e-16`, `4.974e-14`) with eight. The T511 plan constructor took `0.036–0.039 ms`; NumPy coefficient/map copies took `0.104–0.207 ms`, and output allocation took `0.001 ms`. The scalar-conversion path improves RustSHT substantially but leaves a `21.8–45.4x` gap to DUCC, so it is not evidence of parity.

### SIMD latitude reduction for scalar analysis (M4, 2026-10-12)

The phase profile showed that scalar analysis still spent `154.9 ms` of its `158.7 ms` one-thread T511 execution and `27.1 ms` of its `30.9 ms` eight-thread execution in recurrence and accumulation. The paired recurrence already advances latitude lanes with Pulp, but its visitor accumulated each harmonic coefficient through scalar complex operations for every lane. The new f64 path packs same- and opposite-parity Fourier components into four bounded worker arrays once per latitude block, then uses Pulp vector products and horizontal reductions for each paired degree. Lanes with nonzero binary exponents retain the existing scaled scalar path. The f32 recurrence and spin kernels keep their existing accumulation paths.

The workspace adds four 64-element f64 buffers per worker: `2 KiB` per worker, `16 KiB` for eight workers. The native T511 numeric-array estimate rises from `16.25 MiB` to `16.26 MiB`; no map-sized or harmonic-sized temporary is added. The packing occurs inside the timed recurrence-and-accumulation phase.

Native T511 GL f64 analysis timings used the same deterministic input, release profile, reused plan/workspace, one warm-up and five samples per run across three process runs. Values are medians and ranges of run medians:

| Threads | Before SIMD reduction (ms) | After SIMD reduction (ms) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `157.944; 1.958` | `77.898; 1.054` | `-50.7%` |
| 8 | `30.787; 0.927` | `21.881; 0.999` | `-28.9%` |

The Python plan benchmark used the rebuilt release extension, `ducc0==0.41.0`, T511 GL, two run medians of five samples, and a warm-up before each run. Rust plan methods include per-call workspace construction. One-thread Rust analysis fell from the prior `160.008 ms` baseline to `77.982 ms`; DUCC measured `12.315 ms`, giving a `6.3x` ratio. At eight threads Rust measured `23.041 ms` and DUCC `1.770 ms`, a `13.0x` ratio. Rust run spreads were `1.967 ms` and `3.761 ms`; DUCC spreads were `0.033 ms` and `0.005 ms`. The maximum absolute, RMS and relative L2 errors against DUCC were `3.794e-15`, `1.046e-16` and `2.049e-14` at both thread counts. Input copies remained below `0.1 ms`.

Matched native phase timers show that the change reduced recurrence and accumulation from `154.9` to `74.55 ms` at one thread and from `27.1` to `17.62 ms` at eight threads. Forward FFT and packing remained near `3.4 ms` and `3.6 ms`; Fourier weighting and order layout remained below `0.5 ms` in the representative run. The largest remaining absolute excess is still in recurrence and accumulation. The eight-thread whole-call spread includes scheduling and workspace timing variability.

Callgrind 3.26.0 ran the T127 scalar analysis example with the same 12 transforms and cache simulation settings before and after the change. Instruction references fell from `627,299,932` to `277,679,056` (`-55.7%`); data references fell from `190,544,213` to `82,547,944` (`-56.7%`). Simulated D1 misses rose from `352,833` to `416,037` and simulated last-level misses from `27,025` to `42,745`; both remained below `0.6%` of data references. These cache counts are simulator results, not hardware events. `perf stat` remains blocked by `perf_event_paranoid=4`. Disassembly of the runtime-dispatched x86-v3 accumulation function contains packed AVX2 `vmulpd`, `vaddpd` and `vhaddpd` instructions, with scalar tail code. The vector reduction changes summation order.

The focused Pulp-versus-scalar accumulation test covers 64-lane blocks and incomplete tails, and a separate test checks that nonzero exponents use the scaled path bit-for-bit. The T511/T1023/T2047 sparse scalar acceptance test passed for GL, CC and F1; at T2047 the maximum DUCC sample error was `2.083e-13`, maximum analytic-map error was `9.958e-12`, and maximum recovered-coefficient error was `4.175e-14`. Scalar adjoint identities, T255 DUCC coefficients, one-versus-two-thread equivalence and all routine spin-3/4 regressions passed. `cargo fmt --all -- --check`, Clippy with all targets/features, `cargo test --workspace --all-features` (90 unit and 10 f32 tests), `cargo test -p rustsht --no-default-features` (87 unit and 10 f32 tests), and `uv run pytest python/tests` (8 tests) passed.

The SIMD reduction is retained. The next optimization should reduce the remaining per-degree dispatch and callback overhead or fuse the paired recurrence update with accumulation, then remeasure scalar analysis and the shared spin-1 bottlenecks.

### Comparative spin-1 analysis profile and polar cutoff (2026-10-12)

The post-SIMD baseline used commit `4c160f1`, Rust 1.99.0/LLVM 23.1.1, release builds, the Ryzen 9 5950X (AVX2/FMA, no AVX-512), f64 spin-1 STANDARD analysis, T511 GL `512x1025`, `phi0=0.37`, and explicit one- and eight-thread execution. Native `m1b_bench` reused plans and workspaces; the Python-plan comparison used `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, two run medians of five samples after a warm-up, and included Rust workspace creation on each call.

| Threads | RustSHT native (ms) | RustSHT Python plan (ms) | DUCC Python (ms) | Python-plan ratio | Rust excess (ms) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | `696.019 (6.542)` | `693.372 (3.517)` | `25.575 (0.754)` | `27.1x` | `667.797` |
| 8 | `182.730 (10.426)` | `183.418 (4.168)` | `3.666 (0.021)` | `50.0x` | `179.752` |

Parentheses show the range across three native run medians or two Python run medians, respectively. DUCC used `nthreads=1/8`; Rust used one- and eight-worker Rayon pools. Python 3.11.15 and NumPy 2.4.6 were used. Input-map copies took `0.13–0.25 ms`, plan construction `0.035–0.038 ms`, and output allocation `0.001 ms`. Maximum absolute/RMS/relative L2 analysis errors were `1.237e-14`/`2.126e-16`/`4.163e-14` at one thread and `1.463e-14`/`2.298e-16`/`4.500e-14` at eight threads.

After the cutoff change, temporary phase timers measured Rust's one-thread recurrence, conversion and accumulation at `457.021 ms` and its forward FFT and packing at `6.656 ms`; at eight threads the corresponding phases took `174.235 ms` and `7.139 ms`. Each value is the median of five samples in one native `m1b_bench` run; whole-call medians were `464.004 ms` and `181.464 ms` in that instrumented run. The timers were removed after measurement. The pinned DUCC 0.41.1 source build measured `8.88/1.40 ms` for recurrence and accumulation and `3.83/0.62 ms` for FFT and packing. The source build and installed wheel have different whole-call timings; this phase comparison uses the source build, while the preceding table uses the installed 0.41.0 wheel. The Rust/DUCC source phase ratios are `51.5x/124.5x` for recurrence and accumulation and `1.7x/11.5x` for FFT and packing at one/eight threads. The absolute excess is `448.141/172.835 ms` for recurrence and accumulation and `2.826/6.519 ms` for FFT and packing. Recurrence and accumulation account for `98.6%/96.1%` of Rust's phase total and `69.9%/69.3%` of the DUCC source phase total. Python copying is below `0.3 ms`.

Callgrind 3.26.0 profiled the isolated T127 spin-1 analysis command in release mode with debug symbols. The harness performed 12 transforms across its one-thread and one-worker pools. `analyze_spin_order_group` accounted for `2.836` billion of `2.896` billion instruction references (`97.9%` inclusive); the two longitude FFT loops accounted for about `42` million references (`1.4%`). The spin recurrence used packed AVX2/FMA updates, but called `normalize_pair_lanes` for both spin branches at each degree and converted each lane with `scaled_basis_value` before scalar complex accumulation. These two operations accounted for about `21%` and `11%` of process instruction references, respectively. `fill_spin_factors` accounted for `0.4%`. Callgrind totals include both analysis timings in the harness and are instruction counts, not cycle or runtime ratios. A DUCC wheel Callgrind run did not attribute its native extension instructions; DUCC instruction comparisons therefore use the prior source-build profile and disassembly. `perf stat` remains unavailable with `perf_event_paranoid=4`.

The one-thread scalar-conversion analysis path used the sine threshold to select middle-latitude rings but did not apply `ring_order_limits` inside that range. Synthesis and general spin analysis apply the per-ring order cutoff. The fast analysis path now skips fully inactive latitude blocks before recurrence and skips inactive rings during accumulation, matching the synthesis cutoff. It reuses the existing order-limit metadata and adds no workspace memory. A T511 test isolates an excluded order-450 ring and confirms that its contribution is omitted; the release T511 spin-1/2 all-grid DUCC regression and the routine spin adjoint identities pass.

Three native release runs used the same T511 GL input, reused plans and workspaces, one warm-up, and five timed samples per run. Cells give the median and range of run medians:

| Threads | Before cutoff (ms) | After cutoff (ms) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `696.019; 6.542` | `454.112; 7.068` | `-34.8%` |
| 8 | `182.730; 10.426` | `176.596; 0.831` | `-3.4%` |

The eight-thread path does not select scalar conversion; its measured change is within the prior run variability and is not attributed to this change. The rebuilt Python extension measured `488.258 ms` versus DUCC `25.417 ms` at one thread (`19.2x`) and `184.571 ms` versus `3.726 ms` at eight threads (`49.6x`). Errors were unchanged from the pre-change comparison. The cutoff reduces the one-thread excess by `34.8%` while preserving the pinned DUCC result within the existing tolerance. A separate general-analysis prototype skipped 64-ring blocks when every order limit excluded `m`. It removed an estimated `4.66%` of T511 degree-lane recurrence updates, but reduced the eight-thread native median by only `1.0%` (`180.576 ms`, range `2.260 ms`, to `178.754 ms`, range `0.760 ms`). This reduction is comparable to the pre-change run spread, so the block-skip check was removed. The f64 spin accumulation now uses Pulp vector products and horizontal reductions for ordinary, active lanes. Its remaining scaled recurrence and per-degree dispatch costs are the next targets.

### SIMD latitude accumulation for spin analysis (M4/M5, 2026-10-12)

The T127 Callgrind profile showed scalar per-lane spin basis conversion and complex accumulation after the Pulp recurrence update. The f64 analysis visitor now packs the weighted Q/U Fourier components into four existing 64-lane worker arrays, computes E/B contributions with Pulp vector arithmetic, and reduces latitude lanes once per degree. It selects this path only when all lane exponents are zero, all Fourier and recurrence inputs are normal values, and all rings pass the order cutoff. Other blocks use the scaled scalar expression with the per-ring cutoff. The method includes scalar tails and does not change workspace size. F32 uses its previous scalar accumulation.

The vector operation is dispatched through `pulp::Arch` for each degree callback. The Legendre recurrence still writes latitude values to worker scratch before accumulation, so this increment vectorizes accumulation but does not fuse the recurrence update with the reduction. The runtime-dispatched x86-v3 disassembly contains four-lane YMM `vmulpd`, `vaddpd` and `vsubpd` operations in the accumulation loop. No vector register spills appeared in that loop; recurrence updates use FMA. AVX-512 was not available on the Ryzen 9 5950X.

The native comparison used Rust 1.99.0/LLVM 23.1.1, standard release optimization with portable runtime dispatch, f64 spin-1 STANDARD analysis, T511 GL `512x1025`, `phi0=0.37`, reused plans and workspaces, one warm-up, five samples and three process runs. The code before this change was `4bbb93f`:

| Threads | Before SIMD (ms) | After SIMD (ms) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `453.706; 5.009` | `463.678; 3.816` | `+2.2%` |
| 8 | `180.576; 2.260` | `167.487; 1.837` | `-7.2%` |

The one-thread run uses the scalar-conversion path and does not enter the new accumulation kernel; the small timing increase is within host variation. At eight threads the native median fell by `13.089 ms`. The rebuilt Python plan method, which constructs a workspace per call, measured `169.731 ms` against installed `ducc0==0.41.0` at `3.744 ms` (`45.3x`). The previous Python-plan median was `184.571 ms` against DUCC at `3.726 ms` (`49.6x`). Python 3.11.15, NumPy 2.4.6 and the pinned DUCC revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` were used. The maximum absolute/RMS/relative L2 errors were `1.463e-14`/`2.297e-16`/`4.498e-14`; the previous values were `1.463e-14`/`2.298e-16`/`4.500e-14`. The T511 spin-1/2 DUCC regression passed on GL, CC and F1. The T511 spin-3/4 DUCC and analytic regression also passed on all grids.

Temporary phase timers measured the post-change native T511 recurrence, coefficient conversion and accumulation at `159.348 ms` and forward FFT plus packing at `7.206 ms` with eight threads; the instrumented whole call took `166.684 ms`. Before SIMD, the same phase boundaries measured `174.235 ms` and `7.139 ms`. Against the pinned DUCC source-build phase timings of `1.40 ms` for recurrence and accumulation and `0.62 ms` for FFT and packing, the post-change ratios are `113.8x` and `11.6x`; the absolute excesses are `157.948 ms` and `6.586 ms`. These phases account for `95.7%/4.3%` of Rust's `166.554 ms` phase total and `69.3%/30.7%` of DUCC's `2.02 ms` source phase total. The phase timers were removed after measurement.

Callgrind 3.26.0 profiled the same T127 example used for the pre-change profile: 12 one-thread spin-1 analysis transforms, optimized release with debug symbols. Instruction references fell from `2,896,243,058` to `2,595,550,434` (`-10.4%`). These are simulator counts, not CPU cycles. The installed DUCC wheel's native instructions remain unavailable to Callgrind; phase comparisons use the local pinned-source build. The new vector accumulation is retained for its `7.2%` eight-thread native improvement and unchanged error. It reduces scalar basis conversion work, while the recurrence still calls `normalize_pair_lanes` twice per degree and the block callback dispatches once per degree. A fused register-resident recurrence and accumulation kernel remains untested.

### Rejected SIMD normalization precheck (2026-10-12)

Callgrind attributed about `21%` of the T127 spin-1 analysis instruction references to the per-lane `normalize_pair_lanes` checks. A prototype added Pulp vector min/max reductions before the existing scaling loop and returned early when the whole latitude block fell inside the established `SMALL`/`BIG` bounds. A focused test covered exact bounds, zero lanes, scaled values, and the scalar tail; it matched the original normalization results on scalar and dispatched Pulp paths.

The prototype was slower on the target transform. With the same release `m1b_bench` workload (spin-1, f64, T511 GL, reused workspace, one warm-up and five timed samples), a pre-change run measured `468.049 ms` at one thread and `166.660 ms` at eight. Two post-change runs measured `469.223/469.154 ms` at one thread and `349.829/349.477 ms` at eight. The one-thread scalar-conversion path does not use this normalizer; eight-thread general recurrence time more than doubled, consistent with blocks paying for both the SIMD scan and scalar fallback when any lane needs scaling. The precheck was removed without changing the recurrence. Its temporary test and implementation were not retained.

### Rejected IEEE-bit normalization branch (2026-10-12)

A second prototype used the ordered positive-f64 bit encoding to select the existing `BIG` and `SMALL` scaling loops, preserving the loops themselves and their exponent updates. Its boundary test matched the original floating-comparison reference for zero, exact thresholds, subnormals, 64-lane blocks, and a scalar tail. The optimized disassembly still used scalar integer and floating comparisons, with an added integer range check and branch; it introduced no packed arithmetic.

The release T511 spin-1 analysis benchmark before this change measured `466.491 ms` at one thread and `166.437 ms` at eight. Three post-change runs measured one-thread medians of `463.504`, `462.437`, and `461.765 ms`, and eight-thread medians of `166.606`, `171.078`, and `167.459 ms`. The one-thread scalar-conversion path does not use this normalizer. The eight-thread median matches the prior three-run baseline of `167.487 ms` within run variation, so the bit-range branch was removed as it showed no measurable improvement.

### Rejected fused spin-analysis callback (2026-10-12)

A private visitor prototype passed the selected Pulp backend from the recurrence into the spin-analysis accumulation, removing one `Arch::dispatch` call per degree. The T511 native benchmark used f64 spin-1 STANDARD analysis, GL `512x1025`, a reused plan/workspace, one warm-up, five samples and two runs. The previous median was `167.487 ms` at eight threads; the visitor measured `441.721` and `439.994 ms` (`440.858 ms` median). One-thread measurements were `463.678 ms` before and `458.396 ms` after; the one-thread scalar-conversion path did not use this visitor.

Callgrind 3.26.0 on `m1b_bench 1 127 1 analysis` recorded `3,454,390,270` instruction references, compared with `2,595,550,434` for the prior SIMD-accumulation implementation under the same 12-transform, one-worker harness (`+33.1%`). The profile attributes `698,571,732` references (`20.2%`) to the repeated per-degree vectorizable check. Removing the runtime dispatch did not offset the code generated around the visitor and its lane checks. Spin-1/2/3/4 correctness tests and f32 spin adjoint tests passed during the experiment, but the prototype was removed for its substantial throughput and instruction-count regressions. The profile points to hoisting block-invariant Fourier and cutoff eligibility checks before attempting another fused visitor.

### Hoisted spin-analysis SIMD eligibility checks (2026-10-12)

The preceding spin-analysis profile showed that each degree callback rescanned its Q/U Fourier lanes and ring-order limits before selecting vector accumulation. These inputs and cutoffs are invariant for an `(m, latitude block)`. The f64 path now evaluates that eligibility once before the degree recurrence and passes the result to each accumulation. Per-degree checks for spin normalization and recurrence exponents remain. Scaled or inactive blocks retain the scalar fallback. The change adds no worker storage and leaves f32 behavior unchanged.

Native release measurements used rustc 1.99.0/LLVM 23.1.1, standard release optimization with portable AVX2/FMA runtime dispatch, an AMD Ryzen 9 5950X, f64 spin-1 STANDARD analysis, T511 GL `512x1025`, `phi0=0.37`, reused plan/workspace, one warm-up, five samples per run, and three process runs. Each entry is the median and range of run medians in milliseconds:

| Threads | Before (ms) | After (ms) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `463.678; 3.816` | `454.324; 2.841` | `-2.0%` |
| 8 | `167.487; 1.837` | `148.630; 3.285` | `-11.3%` |

The one-thread scalar-conversion path does not use the changed vector accumulation gate; its small timing shift is not attributed to this change. With eight threads, recurrence, conversion and accumulation fell from `159.348 ms` to `143.868 ms` (`-9.7%`). The instrumented whole call was `151.260 ms`; recurrence/conversion/accumulation, FFT and preparation took `143.868`, `7.102` and `0.164 ms`, summing to `151.134 ms`. This timer excluded packing from the FFT phase, unlike the prior phase report, so FFT timings are not compared. The pinned DUCC 0.41.1 source-build recurrence and accumulation phase was `1.40 ms`: RustSHT took `102.8x` as long, with `142.468 ms` absolute excess. The recurrence phase was `95.2%` of RustSHT's measured phase total and `69.3%` of DUCC's previously measured `2.02 ms` recurrence-plus-FFT phase total. This remains the largest measured excess.

Callgrind 3.26.0 profiled `m1b_bench 1 127 1 analysis` in optimized release mode with debug symbols. In the same 12-transform, one-worker harness, instruction references fell from `2,595,550,434` to `2,179,100,442` (`-16.0%`). These are simulator instruction counts, not CPU cycles. `perf stat` remains blocked by `perf_event_paranoid=4`; Callgrind did not attribute instructions inside the installed DUCC wheel. No runtime-counter or register-spill comparison is claimed for this change.

The rebuilt Python extension used Python 3.11.15, NumPy 2.4.6 and installed `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. T511 GL plan analysis results are medians of two run medians of five samples after warm-up:

| Threads | RustSHT plan (ms) | DUCC0 (ms) | Ratio | Max absolute / RMS / relative L2 error |
| ---: | ---: | ---: | ---: | ---: |
| 1 | `470.353` | `24.790` | `19.0x` | `1.237e-14 / 2.126e-16 / 4.163e-14` |
| 8 | `150.389` | `3.580` | `42.0x` | `1.463e-14 / 2.297e-16 / 4.498e-14` |

T1023 GL `1024x2049` plan analysis used the same benchmark and DUCC wheel. Values are medians of two run medians; errors compare Rust packed coefficients with DUCC:

| Threads | RustSHT plan (ms) | DUCC0 (ms) | Ratio | Max absolute / RMS / relative L2 error |
| ---: | ---: | ---: | ---: | ---: |
| 1 | `3,486.459` | `246.945` | `14.1x` | `1.730e-14 / 1.958e-16 / 7.666e-14` |
| 8 | `1,205.138` | `38.488` | `31.3x` | `1.811e-14 / 2.123e-16 / 8.311e-14` |

At T1023, plan construction took `0.10 ms`, NumPy input copies took `1.0–1.3 ms` for coefficients and `3.6–3.8 ms` for maps, and output allocation took `0.01 ms`. Python-plan timings include Rust workspace creation per call; these wrapper costs remain small beside the transform. The eight-thread T511 native improvement agrees with the instruction-count reduction, so the gate is retained. The remaining `42.0x` Python-plan gap makes spin recurrence and accumulation the next target.

Focused spin-analysis tests (seven passed), all ten f32 integration tests, T511 all-grid spin-1/2 DUCC and spin-3/4 regressions, formatting, workspace Clippy with warnings denied, `cargo test --workspace --all-features` (93 unit tests passed, seven opt-in tests ignored, ten f32 tests passed), `cargo test -p rustsht --no-default-features` (90 unit tests passed, seven opt-in tests ignored, ten f32 tests passed), and `uv run pytest python/tests` (eight passed) passed. High-bandlimit checks compare with DUCC and analytic fields; routine all-grid spin adjoint identities also pass.

### SIMD reduction in spin-1 scalar-conversion analysis (2026-10-12)

The high-bandlimit spin-1 scalar-conversion analysis path still accumulated Q and U contributions with scalar complex operations inside each latitude-lane loop. It now uses the existing Pulp paired scalar-analysis reducer for Q and U, producing the even-degree `basis` and odd-degree `basis*cos(theta)` sums. The existing worker arrays hold the weighted Fourier components. Rings excluded by the order cutoff are set to zero before reduction, preserving the cutoff without allocating scratch. Scaled recurrence lanes retain the helper's scalar binary-scaled fallback. The change affects f64 spin-1 scalar-conversion analysis; it adds no workspace storage and does not change synthesis or f32.

Native release measurements used rustc 1.99.0/LLVM 23.1.1, Cargo release optimization, portable runtime AVX2/FMA dispatch, no `target-cpu=native`, the AMD Ryzen 9 5950X, f64 spin-1 STANDARD analysis, T511 GL `512x1025`, `phi0=0.37`, reused plans/workspaces, one warm-up and five samples per run. Three process runs gave the median and range of run medians in milliseconds:

| Threads | Before (ms) | After (ms) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `454.324; 2.841` | `212.476; 0.635` | `-53.2%` |
| 8 | `148.630; 3.285` | `150.192; 4.586` | `+1.0%` |

The eight-thread path selects the general spin recurrence, so its overlapping run ranges show no attributable change. The f64 scalar-conversion path runs only with one Rayon worker. A repeated Python-plan comparison against installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` used Python 3.11.15, NumPy 2.4.6, two runs of five samples and a warm-up per run. At T511 GL, one-thread RustSHT plan analysis fell from `470.353 ms` to `219.749 ms`; DUCC measured `24.646 ms`, so the ratio fell from `19.0x` to `8.9x`. Eight-thread plan timings were `159.335 ms` for RustSHT and `3.856 ms` for DUCC (`41.3x`); variation from the preceding run is consistent with the native benchmark's overlapping ranges. The Python plan constructs a workspace per call. Input copies were `0.07–0.11 ms` for coefficients and `0.14–0.18 ms` for maps, and output allocation was `0.001 ms`.

The Python-plan maximum absolute/RMS/relative L2 coefficient errors were `1.240e-14`/`2.106e-16`/`4.123e-14` at one thread and `1.463e-14`/`2.297e-16`/`4.498e-14` at eight threads. With `RAYON_NUM_THREADS=1`, the release T511 DUCC/adjoint check passed for spins 1 and 2 on GL, CC and F1. Spin-1 maximum DUCC synthesis-sample error was `4.885e-15`; analysis-coefficient error was `4.934e-18`; synthesis-analysis coefficient error was `1.232e-14`. Spin-2 corresponding errors were `5.107e-15`, `3.332e-17` and `1.310e-14`. Callgrind was not run on this T511 branch because the T127 profile already takes about 20 seconds for twelve transforms and the T511 transform is two orders of magnitude longer. `perf stat` remains unavailable at `perf_event_paranoid=4`; this change therefore has matched wall-time and DUCC-error evidence but no instruction-counter comparison.

The reducer gives a repeatable `53.2%` one-thread improvement while keeping the eight-thread general path unchanged within native run variation, so the change is retained. The one-thread Python-plan gap falls to `8.9x`; the eight-thread general path remains the larger absolute transform cost and the next candidate is to re-evaluate its parallel scalar-conversion selection with the new SIMD accumulation.

### Parallel spin-1 scalar-conversion analysis (2026-10-12)

The spin-1 scalar-conversion analysis path was previously restricted to a one-worker Rayon pool because the measured eight-worker path was 15% slower than the general spin recurrence. That comparison used scalar lane accumulation. After SIMD reduction was added to the conversion path, the eight-worker experiment was repeated. The fast path is now enabled for STANDARD f64 spin-1 analysis with any Rayon worker count; spin 2 remains limited to one worker. The GL sine cutoff and general polar fallback are unchanged. Each worker uses its existing scalar recurrence buffers, so workspace size is unchanged.

Native release measurements used rustc 1.99.0/LLVM 23.1.1, Cargo release optimization, portable AVX2/FMA runtime dispatch, no `target-cpu=native`, the AMD Ryzen 9 5950X, f64 spin-1 STANDARD analysis, T511 GL `512x1025`, `phi0=0.37`, reused plans/workspaces, one warm-up and five samples per run. Three process runs gave medians and ranges of run medians in milliseconds:

| Threads | Before selection change, commit `befea59` (ms) | After selection change (ms) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `212.476; 0.635` | `213.841; 0.755` | `+0.6%` |
| 8 | `150.192; 4.586` | `47.806; 1.181` | `-68.1%` |

At one worker the fast path was already selected, so the new thread-selection condition does not materially change that result. At eight workers, the fast path removes `102.386 ms` from the median analysis time. The new path was then checked against installed `ducc0==0.41.0`, revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, using Python 3.11.15, NumPy 2.4.6 and the plan benchmark (two runs of five samples after warm-up). T511 GL eight-thread RustSHT plan analysis fell from `159.335 ms` to `51.138 ms`; DUCC measured `3.635 ms`. The ratio fell from `41.3x` to `14.1x`. Python-plan timings include Rust workspace allocation. The one-thread path remained at `219.749 ms` versus `24.646 ms` DUCC (`8.9x`).

At T1023 GL `1024x2049`, RustSHT plan analysis changed from the `7a59057` baseline of `3,486.459 ms` to `1,282.778 ms` at one thread and from `1,205.138 ms` to `321.159 ms` at eight threads. The one-thread change includes the SIMD reducer from the preceding increment; the eight-thread comparison shows the effect of selecting that reduced scalar-conversion path instead of the general recurrence. DUCC measured `245.799 ms` and `37.845 ms`; the ratios fell from `14.1x`/`31.3x` to `5.2x`/`8.5x`. Rust errors against DUCC were `1.796e-14` maximum absolute, `1.924e-16` RMS and `7.532e-14` relative L2 at both thread counts. At T511, errors were `1.240e-14`/`2.106e-16`/`4.123e-14` at one thread and `1.240e-14`/`2.106e-16`/`4.123e-14` at eight threads. Copy and allocation costs remained below `4 ms` at T1023 and below `0.3 ms` at T511.

The T511 release spin-1/2 DUCC and adjoint checks passed on GL, CC and F1 with eight Rayon workers. T1023 exact-analysis checks passed for spin 1 and 2 on all three grids. T511 spin-3/4 DUCC, analytic and adjoint regressions passed on all grids after the selection change. The specialized path is retained: it improves the target eight-worker spin-1 analysis by `68.1%`, and the T1023 Python-plan/DUCC gap falls from `31.3x` to `8.5x`. The remaining gap includes recurrence and accumulation work; the latest phase measurements predate this selection change, so no updated phase split or instruction-count reduction is claimed.

### Fused spin-1 scalar-conversion synthesis (2026-10-12)

The spin-1 STANDARD synthesis conversion accumulated transformed scalar coefficients through a degree callback and scalar latitude loops. It now runs the existing register-resident paired scalar synthesis kernel on each latitude block for both converted coefficient rows, then applies `1/sin(theta)` once per ring. Blocks excluded by the polar order limits or requiring binary scaling use the paired recurrence and scaled multiplication path. The converted coefficient arrays are degree-indexed, so each kernel call receives the order-local slice beginning at `m`. The change reuses recurrence scratch and adds no workspace storage.

Native measurements used `m1b_bench 1 511 8 synthesis`, f64, T511 GL `512x1025`, `phi0=0.37`, reused plans and workspaces, one warm-up and five timed samples per process run. The before build was commit `c96fa86`; both builds used rustc `1.99.0 (b940084d7 2026-09-28)`, Cargo's standard release optimization, Pulp runtime AVX2/FMA dispatch, and no `target-cpu=native`. The AMD Ryzen 9 5950X host ran Linux `7.0.14`; each process was pinned to CPUs 0–7. Values are medians and ranges of three run medians in milliseconds:

| Threads | Before (`c96fa86`) | Fused synthesis | Change |
| ---: | ---: | ---: | ---: |
| 1 | `472.932; 2.101` | `133.496; 2.412` | `-71.8%` |
| 8 | `83.031; 0.340` | `46.471; 0.208` | `-44.0%` |

The Python-plan comparison used Python `3.11.15`, NumPy `2.4.6`, installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, and three run medians of five samples after warm-up. It used the same T511 GL geometry, coefficients, `phi0`, eight-CPU affinity and explicit one/eight-thread settings. Python-plan timings include Rust workspace creation per call; the native measurements reuse workspaces. The ratio is RustSHT time divided by DUCC0 time:

| Threads | RustSHT plan (ms) | DUCC0 (ms) | Ratio | RustSHT/DUCC0 output max absolute / RMS / relative L2 error |
| ---: | ---: | ---: | ---: | ---: |
| 1 | `131.899; 0.653` | `27.126; 1.551` | `4.9x` | `2.519e-14 / 7.700e-16 / 6.326e-14` |
| 8 | `48.394; 6.659` | `5.404; 1.578` | `9.0x` | `2.519e-14 / 7.700e-16 / 6.326e-14` |

The T511 spin-1 fast-path comparison with the general recurrence passed for E-only, B-only and mixed E/B coefficients at `3e-11` tolerance. Release DUCC and roundtrip checks passed for spin 1 and 2 on GL, CC and F1 at T511. Sparse T1023 synthesis passed for spins 1 and 2 on all three grids; spin-3/4 T511 DUCC and analytic regressions also passed. Workspace tests cover scalar and spin adjoints. No new phase timer or hardware-counter result was collected for this change; `perf_event_paranoid=4` blocks `perf stat`. The whole-transform result and unchanged workspace size support this kernel reuse for the tested T511 workload.

### Spin-2 fast-analysis polar cutoff (2026-10-12)

The spin-2 GL scalar-conversion analysis path now applies the per-ring order limit used by the spin-1 path. It skips a 64-ring block when every ring excludes `m` and sets Fourier inputs to zero for excluded lanes in partially active blocks. The recurrence and coefficient conversion therefore omit high-order polar contributions that the general path skips. The change adds no scratch storage.

The focused T511 test uses `m=450` on a ring with `0.4 < sin(theta) < 0.6` and `m > ring_order_limit`. The fast analysis returns zero coefficients when only that excluded ring is populated, and returns a nonzero result when the same ring is marked active.

Native measurements used `m1b_bench 2 511 8 analysis`, f64, T511 GL `512x1025`, `phi0=0.37`, reused plans/workspaces, one warm-up, five timed samples per process, and six process runs. The before build was `c69dae9`; the test host was the Ryzen 9 5950X under Linux `7.0.14`, with processes pinned to CPUs 0–7. Rust used `1.99.0 (b940084d7 2026-09-28)`, standard Cargo release optimization, and portable AVX2/FMA dispatch. Values are medians and ranges of the six process medians in milliseconds:

| Threads | Before (`c69dae9`) | Cutoff handling | Change |
| ---: | ---: | ---: | ---: |
| 1 | `688.406; 4.045` | `531.851; 2.735` | `-22.7%` |
| 8 | `148.851; 1.001` | `151.626; 5.966` | `+1.9%` |

The one-worker scalar-conversion path benefits from skipping inactive latitude blocks. The eight-worker path uses the general spin recurrence and does not execute the new cutoff checks; its measured shift is not attributed to this change. A Python-plan comparison against installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` used Python `3.11.15`, NumPy `2.4.6`, two run medians of five samples, and the same T511 inputs:

| Threads | RustSHT plan (ms) | DUCC0 (ms) | Ratio | RustSHT/DUCC0 coefficient max absolute / RMS / relative L2 error |
| ---: | ---: | ---: | ---: | ---: |
| 1 | `530.594; 0.231` | `27.786; 0.223` | `19.1x` | `1.317e-14 / 2.707e-16 / 5.301e-14` |
| 8 | `150.139; 0.609` | `3.819; 0.004` | `39.3x` | `1.527e-14 / 2.540e-16 / 4.973e-14` |

The T511 all-grid spin-1/2 DUCC and roundtrip test passed with one thread. The T1023 exact-analysis DUCC test passed for spins 1 and 2 on GL, CC and F1 with one thread. Spin-2 maximum coefficient errors were `3.332e-17`, `2.558e-17`, and `1.410e-17` at T511, and `1.178e-17`, `8.491e-18`, and `8.223e-18` at T1023, respectively. The numerical results include analysis/adjoint and synthesis-analysis checks. The one-worker fast path is retained; the eight-worker general path remains a separate performance target.

### Overflow-safe scalar SIMD analysis reduction (2026-10-12)

The paired scalar-analysis SIMD reducer multiplies Fourier values by normalized recurrence lanes before reducing latitude contributions. Finite Fourier values can still overflow in an intermediate product even when the scaled scalar result is finite. A once-per-block check now bounds every input and the absolute sum across the block. The vector reducer runs only when this bound is safe and all recurrence exponents are zero. Otherwise, the existing binary-scaled scalar reducer handles the block. Nonfinite inputs also use that fallback. The check adds no workspace storage and does not change the ordinary SIMD arithmetic.

The focused test uses a finite `1e308` Fourier value, a recurrence value of `2`, and `cos(theta)=0.5`. Direct multiplication overflows before the final factor can bring the result back to `1e308`; the scaled fallback returns the finite reference value. Ordinary SIMD results continue to match the scaled lane reference, and `spin2_scalar_fast_analysis_honors_the_polar_order_cutoff` passes with the reducer gate present.

Native measurements used `m1a_bench 511 8 analysis`, f64, T511 GL `512x1025`, reused plan and workspace, one warm-up and five timed samples per process, three process runs, and affinity to CPUs 0–7 on the AMD Ryzen 9 5950X. Rust was `1.99.0 (b940084d7 2026-09-28)` with Cargo release optimization and portable Pulp AVX2/FMA dispatch. The no-gate baseline was `c69dae9`; the tested version includes the gate. Values are medians and ranges of process medians in milliseconds:

| Threads | Before (`c69dae9`) | With safety check | Observed change |
| ---: | ---: | ---: | ---: |
| 1 | `78.147; 1.798` | `74.864; 0.372` | `-4.2%` |
| 8 | `20.971; 0.134` | `20.814; 0.569` | `-0.7%` |

The timing does not show an overhead from the block check. The one-thread difference is not attributed to the check because the runs were collected in separate sessions. The eight-thread timings overlap. The estimated numeric-array workspace is `16.08 MiB` at one worker and `16.26 MiB` at eight workers in both versions.

The Python extension was rebuilt from the tested source and compared with installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Python was `3.11.15`, NumPy `2.4.6`; each plan measurement used three run medians, warm plans and explicit one/eight-thread settings, pinned to CPUs 0–7. RustSHT plan calls create a workspace per invocation. The ratio is RustSHT/DUCC0:

| Grid and threads | RustSHT (ms) | DUCC0 (ms) | Ratio | Max absolute / RMS / relative L2 coefficient error |
| --- | ---: | ---: | ---: | ---: |
| T511, 1 | `80.399` | `12.149` | `6.6x` | `3.794e-15 / 1.046e-16 / 2.049e-14` |
| T511, 8 | `21.576` | `1.660` | `13.0x` | `3.794e-15 / 1.046e-16 / 2.049e-14` |
| T1023, 1 | `683.366` | `120.193` | `5.7x` | `1.036e-14 / 1.259e-16 / 4.927e-14` |
| T1023, 8 | `196.234` | `16.152` | `12.1x` | `1.036e-14 / 1.259e-16 / 4.927e-14` |

At T1023, plan construction took `0.10 ms`; NumPy input copies took `0.13–1.26 ms`; output allocation took `0.001 ms`. The paired SIMD analysis tests, extreme finite-product fallback, spin-2 high-order cutoff regression, scalar and spin adjoint identities, DUCC comparisons and analytic tests passed. `cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test --workspace --all-features` (95 unit tests and 10 f32 tests passed; seven opt-in tests ignored), `cargo test -p rustsht --no-default-features` (92 unit tests and 10 f32 tests passed; seven opt-in tests ignored), and `uv run pytest python/tests` (eight passed) passed. The Python extension was rebuilt before the final Python tests and timing comparisons. `perf stat` remains unavailable at `perf_event_paranoid=4`; no hardware-counter comparison was collected.

### SIMD and parallel spin-2 scalar-conversion analysis (2026-10-12)

The spin-2 GL analysis conversion now stores weighted Q and U components in existing real/imaginary worker buffers and uses the paired Pulp reducer for even- and odd-degree accumulation. Binary-scaled recurrence lanes and unsafe Fourier products use the scalar scaled reducer. The path applies the per-ring polar order limits used by general analysis. The high-bandlimit path is selected for one or multiple Rayon workers, each with its own recurrence and accumulation buffers.

Native measurements used `m1b_bench 2 511 8 analysis`, f64, T511 GL `512x1025`, `phi0=0.37`, reused plan and workspace, one warm-up and five timed samples per process, three process runs, and affinity to CPUs 0–7 on the AMD Ryzen 9 5950X. Rust was `1.99.0 (b940084d7 2026-09-28)` with Cargo release optimization and portable Pulp AVX2/FMA dispatch. Each entry is the median and range of process medians in milliseconds:

| Implementation | 1 thread | 8 threads |
| --- | ---: | ---: |
| Before the spin-2 SIMD reducer | `523.755; 1.480` | `151.583; 2.376` |
| SIMD reducer, parallel fast path disabled | `214.774; 4.489` | `151.999; 0.601` |
| SIMD reducer selected for parallel analysis | `214.981; 3.152` | `48.218; 1.468` |

The SIMD reducer lowered one-thread time by `59.0%`. Selecting the conversion in parallel lowered eight-thread time by `68.3%`; one-thread timings before and after that selection differ by `0.1%`. The reducer uses existing worker arrays and adds no workspace storage.

The Python extension was rebuilt from this source and compared with installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Python was `3.11.15`, NumPy `2.4.6`; each plan measurement used three run medians, warm plans and explicit one/eight-thread settings, pinned to CPUs 0–7. RustSHT plan calls create a workspace per invocation. The ratio is RustSHT/DUCC0:

| Grid and threads | RustSHT (ms) | DUCC0 (ms) | Ratio | Max absolute / RMS / relative L2 coefficient error |
| --- | ---: | ---: | ---: | ---: |
| T511, 1 | `219.745` | `28.182` | `7.8x` | `1.274e-14 / 2.432e-16 / 4.762e-14` |
| T511, 8 | `49.543` | `3.924` | `12.6x` | `1.274e-14 / 2.432e-16 / 4.762e-14` |
| T1023, 1 | `1,319.507` | `261.388` | `5.1x` | `1.600e-14 / 1.924e-16 / 7.533e-14` |
| T1023, 8 | `310.705` | `40.390` | `7.7x` | `1.600e-14 / 1.924e-16 / 7.533e-14` |

Release DUCC and roundtrip checks passed for spins 1 and 2 at T511 on GL, CC and F1 with one and eight Rayon workers. T1023 exact-analysis checks passed for spins 1 and 2 on all three grids with one and eight workers. Spin-2 analysis coefficient maximum errors at T511 were `1.944e-17`, `2.558e-17` and `1.410e-17` on GL, CC and F1; at T1023 they were `8.664e-18`, `8.491e-18` and `8.223e-18`. The T511 spin-3/4 all-grid DUCC, analytic and adjoint regression passed with eight workers. Spin adjoint identities, the focused spin-2 polar-cutoff test, formatting, Clippy with warnings denied, workspace tests (95 unit tests and 10 f32 tests passed; seven opt-in tests ignored), no-default-feature tests (92 unit tests and 10 f32 tests passed; seven opt-in tests ignored), and Python tests (eight passed) passed. `perf stat` remains blocked by `perf_event_paranoid=4`; hardware counters were not collected for this change.

### Fused spin-2 scalar-conversion synthesis (2026-10-12)

The spin-2 GL scalar-conversion synthesis path now sends each converted scalar coefficient row through the register-resident Pulp synthesis block in 64-ring groups. The kernel accumulates degrees in SIMD registers; the caller applies `1/sin(theta)^2` after each block. A block that contains a polar-cutoff lane or needs binary recurrence scaling uses the existing scaled scalar recurrence. The implementation reuses the existing coefficient, recurrence and spectrum buffers.

Callgrind 3.26.0 compared one release T511 spin-2 synthesis on the Ryzen 9 5950X, with one Rayon worker, `phi0=0.37`, and the same E/B coefficient arrays. The temporary harness includes plan and workspace construction as well as the transform. The no-fusion run was measured before the change; the fused run used the same harness and compiler. Callgrind instruction references fell from `10,459,343,346` to `3,164,579,254` (`-69.7%`). Data references fell from `3,116,256,432` to `877,826,918` (`-71.8%`), simulated L1 data misses from `53,385,528` to `12,157,285` (`-77.2%`), and branch instructions from `1,525,279,101` to `518,515,607` (`-66.0%`). Last-level cache misses, including instruction and data misses, were `350,103` before and `352,236` after. The pre-change self profile attributed `6.01 billion` instructions (`57.4%`) to `multiply_complex_scaled_product`; after the change, the largest self entry was the Pulp x86-v3 `FusedSynthesisBlock` at `903 million` instructions (`28.6%`). These simulator counts are not CPU cycles. `perf stat` remains unavailable because `perf_event_paranoid=4`.

Native release measurements used `m1b_bench 2 511 8 synthesis`, f64, T511 GL `512x1025`, `phi0=0.37`, reused plan and workspace, one warm-up and five timed samples per process, three process runs, and affinity to CPUs 0–7. Rust was `1.99.0 (b940084d7 2026-09-28)` with Cargo release optimization and portable Pulp AVX2/FMA dispatch. Values are medians and ranges of process medians in milliseconds:

| Threads | Before fusion | Fused synthesis | Change |
| ---: | ---: | ---: | ---: |
| 1 | `479.266; 4.469` | `142.715; 1.656` | `-70.2%` |
| 8 | `82.165; 2.146` | `51.980; 1.059` | `-36.7%` |

The rebuilt Python extension was compared with installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, using Python `3.11.15`, NumPy `2.4.6`, three run medians, warm plans, and the same T511 input at explicit one/eight-thread settings. RustSHT plan calls create a workspace per invocation:

| Threads | RustSHT plan (ms) | DUCC0 (ms) | Ratio | Max absolute / RMS / relative L2 map error |
| ---: | ---: | ---: | ---: | ---: |
| 1 | `146.078` | `28.371` | `5.2x` | `5.624e-14 / 1.611e-15 / 1.323e-13` |
| 8 | `54.910` | `5.656` | `9.7x` | `5.624e-14 / 1.611e-15 / 1.323e-13` |

The T511 all-grid spin-1/2 DUCC and roundtrip test passed with one and eight workers. Sparse T1023 spin-1/2 synthesis passed against DUCC and the GRAD_ONLY relation on GL, CC and F1 at both thread counts; spin-2 maximum map errors were `4.115e-15`, `3.463e-15` and `3.934e-15`. Sparse T2047 synthesis passed against DUCC and the sectoral analytic formula at both thread counts for spins 1–4; spin-2 middle-ring DUCC sample error was `2.085e-13` and analytic map maximum/RMS errors were `1.236e-11`/`3.88e-13`. Workspace spin adjoint identities and the full formatting, Clippy, Rust and Python suites passed. The tested improvement is retained; the remaining T511 plan gap is `5.2x` at one worker and `9.7x` at eight.

### Shared recurrence for spin-2 scalar synthesis (2026-10-12)

The spin-2 conversion produces two scalar coefficient rows evaluated at the same latitudes. The synthesis fast path now accumulates both rows while advancing the paired scalar recurrence once per lane. It uses the existing analysis real/imaginary worker arrays for its four SIMD accumulators; no workspace allocation or public API changed. Polar or scaled blocks continue through the existing scaled recurrence.

Native measurements used `m1b_bench 2 511 8 synthesis`, f64, T511 GL `512x1025`, `phi0=0.37`, reused plan and workspace, one warm-up, five samples per process and three process runs pinned to CPUs 0–7. Rust was `1.99.0 (b940084d7 2026-09-28)` with Cargo release optimization and portable Pulp AVX2/FMA dispatch. Entries give the median and range of process medians in milliseconds:

| Threads | Before shared recurrence | Shared recurrence | Change |
| ---: | ---: | ---: | ---: |
| 1 | `143.100; 0.987` | `137.541; 0.501` | `-3.9%` |
| 8 | `51.566; 0.214` | `51.833; 0.497` | `+0.5%` |

The one-thread improvement is repeatable; the eight-thread ranges overlap, so no eight-thread speedup is claimed. A matched Callgrind 3.26.0 T511 single-synthesis harness recorded `3,164,579,256` versus `2,891,841,862` instruction references (`-8.6%`). The fused Pulp block's self instruction count fell from `903,376,048` to `677,187,311` (`-25.0%`). These counts are simulator results, not CPU cycles. `perf stat` remains blocked by `perf_event_paranoid=4`.

The Python extension was rebuilt and measured with `examples/python_perf_baseline.py`, Python `3.11.15`, NumPy `2.4.6`, installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, three run medians of five samples, and identical T511 inputs. Python plan methods allocate a workspace per call. RustSHT/DUCC ratios are:

| Threads | RustSHT plan (ms) | DUCC0 (ms) | Ratio | Max absolute / RMS / relative L2 map error |
| ---: | ---: | ---: | ---: | ---: |
| 1 | `142.093` | `28.195` | `5.0x` | `5.624e-14 / 1.611e-15 / 1.323e-13` |
| 8 | `53.062` | `5.653` | `9.4x` | `5.624e-14 / 1.611e-15 / 1.323e-13` |

The focused kernel comparison passed for scalar fallback and dispatched SIMD with 64-lane and tail blocks. T511 spin-1/2 DUCC, roundtrip and adjoint checks passed on GL, CC and F1 with one and eight workers. T2047 sparse synthesis passed DUCC and sectoral analytic checks for spins 1–4 on all three grids at one and eight workers; spin-2 DUCC sample error was `2.085e-13`, with analytic map maximum/RMS errors `1.236e-11`/`3.88e-13`. Workspace size is unchanged. The change is retained for its repeatable single-thread gain; the multithreaded analysis profile is the next target.

### SIMD lane accumulation for spin-2 analysis (2026-10-12)

The paired scalar-analysis SIMD reducer accumulated a horizontal reduction for each 4-lane AVX2 block. It now keeps the partial sums in SIMD lanes across the full latitude block and reduces each result once at the end. Scalar tails and the overflow-safe scaled fallback are unchanged. This path is used by f64 scalar analysis and the f64 spin-1/2 scalar-conversion analysis; f32 continues to use its existing implementation. The vector reduction changes summation order.

Callgrind 3.26.0 profiled one f64 spin-2 T511 GL analysis with one Rayon worker on the Ryzen 9 5950X. The harness, plan, workspace and Rust 1.99.0 release build were matched. Whole-process instruction references fell from `4,437,096,021` to `4,280,713,039` (`-3.5%`). The x86-v3 Pulp `AnalysisPairAccumulation` body fell from `445,781,364` to `289,398,412` references (`-35.1%`). These are instruction-simulator counts, not cycles; `perf stat` remains blocked by `perf_event_paranoid=4`.

Native release timings used `m1b_bench 2 511 8 analysis`, reused plan and workspace, T511 GL `512x1025`, `phi0=0.37`, one warm-up and five samples per run. Each version used three process runs pinned to CPUs 0–7 on the Ryzen 9 5950X, with Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo release optimization and portable Pulp AVX2/FMA dispatch. Values are medians and ranges of process medians in milliseconds:

| Threads | Before | Deferred horizontal reduction | Change |
| ---: | ---: | ---: | ---: |
| 1 | `218.476; 3.010` | `201.506; 1.425` | `-7.8%` |
| 8 | `48.265; 1.001` | `46.357; 0.630` | `-4.0%` |

The rebuilt Python plan comparison used Python `3.11.15`, NumPy `2.4.6`, installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, three run medians of five samples, and matching maps and transform parameters. Ratios are RustSHT/DUCC0; the Python plan allocates its workspace per call:

| Grid and threads | RustSHT (ms) | DUCC0 (ms) | Ratio | Max absolute / RMS / relative L2 coefficient error |
| --- | ---: | ---: | ---: | ---: |
| T511, 1 | `200.350` | `28.954` | `6.9x` | `1.274e-14 / 2.426e-16 / 4.751e-14` |
| T511, 8 | `46.249` | `3.846` | `12.0x` | `1.274e-14 / 2.426e-16 / 4.751e-14` |
| T1023, 1 | `1,172.082` | `261.085` | `4.5x` | `1.527e-14 / 1.920e-16 / 7.518e-14` |
| T1023, 8 | `289.316` | `39.922` | `7.2x` | `1.527e-14 / 1.920e-16 / 7.518e-14` |

The scalar T511/T1023/T2047 DUCC and analytic checks passed on GL, CC and F1 with one and eight workers. T511 spin-1/2 DUCC, roundtrip and adjoint checks passed on all three grids at one and eight workers. Exact spin-1/2 T1023 analysis passed on all three grids at one and eight workers; the largest spin-2 analysis coefficient errors were `1.944e-17` at T511 and `8.664e-18` at T1023. The focused overflow-fallback test and paired reducer scalar-reference test passed. The reducer adds no workspace storage. This change is retained for its measured one- and eight-thread improvement; further profiling is needed to isolate the remaining polar recurrence and accumulation costs.

A prototype combined the spin-2 Q/U calls into one Pulp dispatch and shared the exponent-zero check. The paired-channel reference test, high-order polar-cutoff test, T511 spin-2 GL DUCC test and spin adjoint identity passed. Native T511 medians changed from `201.506` to `204.672 ms` at one thread (`+1.6%`) and from `46.357` to `46.505 ms` at eight threads (`+0.3%`, within run variability). The prototype was removed because it did not improve the target parallel workload.

### Analysis scratch for larger Rayon pools (2026-10-12)

Workspace construction previously sized recurrence scratch only for the latitude groups used by synthesis. At T511, this gave analysis at most eight workspaces even when its recursive order split ran in a 16- or 32-thread Rayon pool. Workspace construction now also reserves up to `min(pool threads, mmax + 1)` analysis workspaces. Synthesis continues to use only the scratch for its latitude groups. This adds no map- or spectrum-sized buffers and does not change the number of workers for one- or eight-thread pools.

The regression test creates a T127 GL workspace inside a 16-thread pool, verifies that it retains two latitude groups and has 16 analysis workspaces, then compares its analysis with one-thread output. The scalar SIMD extreme-finite-product test was rerun in the same release library suite and confirmed that `1e308 * 2` takes the established scaled fallback before multiplication; the ordinary reducer path is unchanged.

Native T511 spin-2 analysis timings used `m1b_bench 2 511 <threads> analysis`, Rust `1.99.0 (b940084d7 2026-09-28)`, release optimization, AVX2/FMA runtime dispatch, the Ryzen 9 5950X, and a reused plan/workspace. Each process warmed the transform and reported the median of five samples. New measurements used three processes, pinned to CPUs `0`, `0-7`, `0-15`, or `0-31` for 1, 8, 16, or 32 threads respectively. They measured `208.453 ms` at one thread (range `2.608 ms`), `46.876 ms` at eight (range `1.093 ms`), `34.009 ms` at 16 (range `7.777 ms`), and `27.948 ms` at 32 (range `2.138 ms`). Before this change, scratch capacity and repeated measurements were `1`/`8`/`8`/`8` workers and analysis medians were `201.506`/`46.357`/`46.357`/`46.262 ms` at 1/8/16/32 threads; the 16- and 32-thread baselines each comprise two process runs. One- and eight-thread behavior is unchanged by the allocation logic. The 16-thread measurements have substantial run variation; the 32-thread result is a repeatable `39.6%` decrease from its prior median.

The 32-thread workspace adds 24 recurrence workspaces over the eight-thread case, approximately `1.38 MiB` for T511 spin-2. Workspace allocation measured `4.42 ms` median at 32 threads versus `3.73 ms` at eight threads; plan construction remained below `0.05 ms`. No numerical data layout or arithmetic changed.

The rebuilt Python plan method at T511 measured `203.225/47.455/29.419 ms` at 1/8/32 threads against installed `ducc0==0.41.0` (`28.146/3.819/2.706 ms`), giving ratios of `7.2x/12.4x/10.9x`. DUCC used source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The maximum absolute/RMS/relative-L2 coefficient error was `1.274e-14 / 2.426e-16 / 4.751e-14` at each thread count. The installed Python timing includes per-call workspace creation for RustSHT.

Release Rust tests passed, including the focused capacity test, scalar overflow-fallback test, one-versus-16-thread analysis comparison, spin adjoint identities, and the existing spin-2 polar-order cutoff regression. The 32-thread T511 spin-1/2 all-grid DUCC and roundtrip test and the T1023 spin-1/2 exact-analysis DUCC test passed. The T2047 sparse spin-1–4 synthesis check passed on GL, CC and F1 at one and eight threads; the largest DUCC sample error was `2.169e-13` and the largest analytic map error was `1.344e-11`. The additional scratch is retained because 32-thread analysis improved materially while one- and eight-thread workspace sizing and synthesis grouping remain unchanged.

A spin-2 synthesis prototype initialized cutoff-inactive latitude lanes to zero so a partially active block could use the fused Pulp kernel. Its focused scalar and dispatched SIMD reference test passed, as did the T511 all-grid DUCC/roundtrip checks and T2047 spin-1–4 DUCC/analytic checks at one and eight threads. The initially reported Callgrind comparison of `12,100,699,039` versus `12,100,698,475` instruction references is invalid: both temporary harnesses linked a stale `librustsht.rlib` from before the compared source changes. Do not use those counts as evidence. The native T511 comparison was built from the intended revisions: medians changed from `138.164` to `127.282 ms` at one thread and from `52.125` to `52.526 ms` at eight threads. Since eight-thread execution did not improve and the one-thread change was collected in separate runs, the masked-lane prototype and its test were removed. The remaining mixed-cutoff and scaled-lane costs have not been isolated.

### Skip fully inactive spin-2 synthesis blocks (2026-10-12)

The spin-2 scalar-conversion synthesis loop now skips a latitude block before dispatch when every ring has `m > ring_order_limit`. The general fused-kernel guard still sends partially inactive or scaled blocks through the existing scaled recurrence. This avoids recurrence work only where the current per-ring cutoff already excludes every lane. The code uses existing order limits and spectrum buffers and adds no workspace storage. The focused spin-2 polar-cutoff regression and the scalar SIMD extreme-finite-product regression both pass on this source.

Native measurements used `m1b_bench 2 511 8 synthesis`, f64, T511 GL `512x1025`, `phi0=0.37`, reused plan and workspace, one warm-up and five timed samples per process, three process runs, and CPU affinity to cores 0 or 0–7 on the Ryzen 9 5950X under Linux `7.0.14`. Rust was `1.99.0 (b940084d7 2026-09-28)`, standard Cargo release optimization, portable Pulp AVX2/FMA dispatch, without `target-cpu=native`. Values are median and range of process medians in milliseconds:

| Threads | Before block skip | After block skip | Change |
| ---: | ---: | ---: | ---: |
| 1 | `138.164; 3.252` | `133.064; 0.729` | `-3.7%` |
| 8 | `52.125; 1.373` | `49.154; 0.600` | `-5.7%` |

A matched one-call T511 synthesis harness in Callgrind 3.26.0, with one worker, recorded `2,890,752,857` versus `2,787,622,914` instruction references (`-3.57%`), `907,612,162` versus `882,766,071` data references (`-2.74%`), `447,771,491` versus `426,846,448` branches (`-4.67%`), and `12,295,804` versus `12,002,798` simulated L1 data misses (`-2.38%`). These are simulator counts, not CPU cycles. `perf stat` remains blocked by `perf_event_paranoid=4`.

The rebuilt Python plan comparison used Python `3.11.15`, NumPy `2.4.6`, installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, warm plans, and explicit one/eight-thread settings on the same T511 inputs. RustSHT plan calls allocate a workspace per invocation. At one thread, RustSHT took `135.271 ms` and DUCC took `30.005 ms` (`4.5x`); at eight threads they took `50.644 ms` and `5.587 ms` (`9.1x`). The RustSHT/DUCC maximum absolute, RMS and relative L2 map errors were `5.624e-14`, `1.611e-15` and `1.323e-13` at both thread counts. The T511 spin-2 all-grid DUCC and roundtrip checks passed at one and eight threads. T2047 sparse spin-1–4 synthesis passed DUCC and sectoral analytic checks on GL, CC and F1 at both thread counts; spin-2 DUCC sample error was `2.085e-13`, with analytic map maximum error `1.236e-11`. The skip is retained for its measured native and whole-transform improvement; further spin-2 synthesis profiling should focus on the remaining active/scaled recurrence work.

### Active-ring compaction audit and measurement (2026-10-12)

The pinned DUCC checkout was verified at `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. In `sht.cc`, `make_ringdata()` sorts rings by `sin(theta)`, pairs north/south rings with their original indices, and stores one shared cutoff and geometry for each pair. `fill_a2m()` and `fill_a2m_spin()` append only pairs whose cutoff admits the current order to fixed SIMD batches; they write zero Fourier values for excluded pairs and retain original indices for extraction. RustSHT processes contiguous 64-ring blocks. Before compaction, a mixed-cutoff block entered the general recurrence across all lanes and its accumulation callback skipped excluded rings. The current path skips fully inactive blocks, sends fully active normal blocks through the register-resident paired kernel, gathers active rings from mixed blocks into bounded worker scratch for the existing scaled recurrence, and leaves fully active numerically scaled blocks on that same fallback. RustSHT computes each hemisphere's cutoff and signed cosine independently, so compaction carries per-ring geometry and destination indices; it does not infer the southern output from the northern lane.

The current native screening baseline used `m1a_bench` for spin 0 and `m1b_bench` for spins 1/2, with release binaries, f64 GL grids, `phi0=0.37`, reused plan/workspace, one warm-up, five timed samples per invocation, Rust `1.99.0 (b940084d7 2026-09-28)`, standard Cargo release optimization, portable AVX2/FMA dispatch, and no `target-cpu=native`. The Ryzen 9 5950X host ran Linux `7.0.14`; processes were pinned to CPUs 0–7. Each cell reports synthesis one/eight-thread time, then analysis one/eight-thread time, in milliseconds:

| Bandlimit | Scalar synthesis / analysis | Spin-1 synthesis / analysis | Spin-2 synthesis / analysis |
| ---: | ---: | ---: | ---: |
| T127 | `1.859/1.047 / 1.072/0.572` | `20.689/10.975 / 17.873/3.354` | `20.870/11.338 / 17.916/2.931` |
| T511 | `79.390/28.853 / 73.458/20.778` | `131.989/46.044 / 197.102/45.086` | `133.422/50.062 / 198.544/45.475` |
| T1023 | `646.466/194.384 / 627.054/189.553` | `1,055.791/306.470 / 1,160.468/289.843` | `1,005.933/305.745 / 1,145.592/285.266` |

The T511 spin-2 values are the median of three process medians; their process ranges were `2.865/0.791 ms` for synthesis and `2.000/0.921 ms` for analysis at one/eight threads. Other cells are one process median and are screening values, not repeated-run estimates. The T511 Python-plan comparison immediately after the block skip remains `135.271/50.644 ms` for spin-2 synthesis against installed `ducc0==0.41.0` at `30.005/5.587 ms` (`4.5x/9.1x`), with maximum/RMS/relative-L2 map error `5.624e-14/1.611e-15/1.323e-13`. Prior scalar and spin-1 DUCC comparisons and T1023 plan comparisons are recorded in the preceding M4/M5 sections; this screening run did not rebuild all Python comparators.

A one-call T511 Callgrind profile of the current spin-2 synthesis executable recorded `2.776` billion instruction references. The largest self costs were `multiply_complex_scaled_product` at `860.9` million (`31.0%`), the Pulp `FusedSynthesisPairBlock` at `679.3` million (`24.5%`), and `normalize_pair_lanes` at `229.8` million (`8.3%`); the scaled fallback callback accounted for `368.4` million (`13.3%`). These simulator counts include plan and workspace setup from the temporary harness and are not elapsed time or CPU cycles. They show that both the scaled/general path and fused normal path remain substantial, but do not by themselves distinguish cutoff fallback from numerical-scaling fallback or prove that compaction will win.

The independent read-only review by the reviewer Sol agent at `e400fea` found the all-inactive skip consistent with DUCC cutoff behavior and found no synthesis arithmetic or indexing bug in that change. The review identified that the existing focused spin-2 cutoff test covered analysis only. `spin2_scalar_fast_synthesis_honors_full_and_mixed_polar_blocks` now checks excluded output in a fully inactive block, zero output for excluded lanes in a mixed block, and agreement of active mixed-block lanes with an all-active reference; it passes in release mode.

A temporary test-only probe recorded per-order, per-worker block and ring-pair counts for one synthesis. It counted degree-lane work as the number of recurrence degrees advanced times the lane count; excluded lanes in partial blocks are reported separately from full-active blocks rejected because their recurrence needs scaling. T511 1T/8T work was 64.364/64.350 million executed degree-lanes, of which 60.001 million were active. Mixed-cutoff fallback advanced 10.325/10.310 million degree-lanes, with only 5.962/5.961 million active; the other 4.363/4.349 million degree-lanes were cutoff-excluded work (6.78% of all executed work). Full-active scaled blocks accounted for a separate 2.751/2.764 million degree-lanes. T1023 1T/8T work was 475.023/475.065 million executed degree-lanes, of which 451.049 million were active. Mixed-cutoff fallback advanced 51.578/51.693 million degree-lanes, with 27.603/27.676 million active; 23.974/24.017 million degree-lanes were cutoff-excluded work (5.05% of all executed work). Full-active scaled blocks accounted for a separate 48.061/48.077 million degree-lanes.

| Bandlimit | Threads | Excluded blocks | Mixed blocks | Fused blocks | Full-active scaled blocks | Total/active degree-lanes | Mixed executed/active degree-lanes |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | 527 | 797 | 2,663 | 109 | 64.364m / 60.001m | 10.325m / 5.962m |
| T511 | 8 | 528 | 796 | 2,662 | 110 | 64.350m / 60.001m | 10.310m / 5.961m |
| T1023 | 1 | 3,756 | 1,792 | 9,618 | 1,218 | 475.023m / 451.049m | 51.578m / 27.603m |
| T1023 | 8 | 3,747 | 1,792 | 9,621 | 1,224 | 475.065m / 451.049m | 51.693m / 27.676m |

The GL north/south pair probe found no one-active pairs at either bandlimit. At T511, 102,599 of 131,072 order-pairs were active in both hemispheres and 28,473 were excluded in both; at T1023 the counts were 376,250 of 524,288 and 148,038. Applying the spin-2 fast-segment threshold `sin(theta) >= 0.01` classified 130,560 T511 pairs as fast (102,494 both-active) and 512 as polar-general (105 both-active); at T1023 it classified 521,216 as fast (375,924 both-active) and 3,072 as polar-general (326 both-active). These counts describe the ring segments selected by the current spin-2 high-bandlimit path, not proof of a speedup from compaction.

At eight workers, the degree-lane load ranged from 6.848m to 8.471m per worker at T511 and from 40.739m to 67.437m at T1023. The greater T1023 imbalance is a separate scheduling candidate; this measurement does not include worker completion time or waiting time, so it does not establish a scheduling bottleneck.

Fresh repeated native screening used three release-process runs of `m1b_bench 2 T 8 synthesis`, five timed samples per run, reused plans and workspace, on the same Ryzen 9 5950X host and CPUs 0–7. For T511, one/eight-thread medians across process medians were `132.704/49.777 ms` (ranges `131.578–133.381/49.055–49.811 ms`). For T1023 they were `1,013.624/302.198 ms` (ranges `1,002.847–1,014.120/302.011–303.344 ms`). The profile counters came from a separate `cfg(test)` diagnostic build and do not report timings by path. The counters and this baseline make mixed-block compaction worth a bounded prototype, but the maximum directly removable lane work is 6.78% at T511 and 5.05% at T1023 before gather/scatter cost.

The baseline review cautioned that promoting compacted mixed blocks to the fused path based only on its initial sectoral-state check would change the numerical regime: the fused loop does not perform later binary rescaling. The prototype therefore keeps mixed blocks on the existing scaled recurrence and compacts only their active latitude lanes. It gathers sine, signed cosine and local destination indices into reusable per-worker scratch, then scatters the existing scaled contributions into the original spectrum. Fully active blocks keep the existing fused eligibility. No compact full-spectrum buffer is allocated. The review found that each recurrence lane is normalized independently and that removing inactive lanes preserves active-lane mathematics, subject to SIMD tail roundoff.

The measured prototype improved complete native synthesis at both target sizes. Each entry is the median of three release-process medians, with five timed samples per process; parenthesized values are the minimum and maximum process medians, in milliseconds:

| Bandlimit | Threads | Before | Compacted | Change |
| ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | `132.704 (131.578–133.381)` | `126.395 (124.948–126.488)` | `-4.75%` |
| T511 | 8 | `49.777 (49.055–49.811)` | `47.053 (46.904–47.214)` | `-5.47%` |
| T1023 | 1 | `1,013.624 (1,002.847–1,014.120)` | `987.013 (985.470–993.294)` | `-2.63%` |
| T1023 | 8 | `302.198 (302.011–303.344)` | `294.007 (293.917–296.780)` | `-2.71%` |

These runs used the same Ryzen 9 5950X, Linux `7.0.14`, CPUs 0–7, rustc `1.99.0 (b940084d7 2026-09-28)`, standard Cargo release settings, portable AVX2/FMA dispatch, T-grid GL inputs, `phi0=0.37`, and reused plan/workspace. Relative changes are larger than the process-to-process variation. The added per-worker scratch is 64 lanes of sine, cosine and `usize` ring index: 1.5 KiB per worker for f64 and 1 KiB for f32, or 12 KiB/8 KiB at eight workers. It is independent of grid size. `examples/m1a_bench.rs` includes this storage in its workspace estimate.

The rebuilt T511 Python-plan comparison used Python `3.11.15`, NumPy `2.4.6`, installed `ducc0==0.41.0` from the pinned revision, three process medians of five timed samples, and matching `RAYON_NUM_THREADS`/DUCC thread counts. RustSHT took `128.150/47.696 ms` at one/eight threads; DUCC took `28.248/5.638 ms`. The resulting RustSHT/DUCC ratios are `4.54x/8.46x`; maximum absolute, RMS and relative L2 map errors were `5.624e-14`, `1.611e-15` and `1.323e-13` at both thread counts. The absolute gap remains substantial, so this improvement does not establish parity.

The focused high-order synthesis test now also reruns its mixed block as a segment with a nonzero global ring start and compares every destination to the full-group result. It passes. Release validation passed for T511 spins 1 and 2 DUCC and roundtrips on GL/CC/F1, T1023 spins 1 and 2 exact analysis on GL/CC/F1, T2047 sparse spins 1–4 synthesis against DUCC and sectoral formulas on GL/CC/F1, spin adjoint identities, and f32 spins 1–4 operations and adjoints. `cargo fmt --all -- --check`, Clippy with warnings denied, workspace tests (97 unit and 10 f32 tests), no-default-feature tests (93 unit and 10 f32 tests), and Python tests (8 passed) passed. The reviewer Sol agent's pre-commit read-only review found no cutoff, indexing, workspace ownership or scaled-fallback blocker and confirmed the measured whole-transform improvements justify retaining the prototype. The compact path is retained as a contained synthesis improvement; worker completion time remains unmeasured, and T1023's degree-lane counts show scheduling imbalance for a later investigation.

### Spin-2 scaled-to-normal recurrence profile (2026-10-12)

A test-only profile measured the paired scalar recurrence and spin-2 scalar-conversion accumulation after sectoral initialization. It used `0310046`, rustc `1.99.0 (b940084d7 2026-09-28)`, Cargo release optimization, the AMD Ryzen 9 5950X, Linux `7.0.14`, CPUs 0–7, f64 GL, dense deterministic E/B coefficients, and explicit one- and eight-thread Rayon pools. Each probe executed one synthesis. The reported durations are sums of per-block `Instant` intervals across workers; they are not whole-transform wall times, CPU-cycle counts, or DUCC phase timings. Plan and workspace construction, FFTs and scheduling are outside the intervals. The timer started after sectoral initialization and initial pair normalization. Separate degree-lane counters were collected in a test-only run with the timer disabled.

| Bandlimit | Threads | Paired blocks | Initially normal | Entered normal range | Never entered | Left normal range | Scaled prefix (ms) | All-lanes normal segment (ms) | Total paired recurrence and accumulation (ms) | Normal segment share |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | 906 | 400 | 498 | 8 | 0 | 15.179 | 51.651 | 66.830 | 77.3% |
| T511 | 8 | 906 | 400 | 498 | 8 | 0 | 15.728 | 53.441 | 69.169 | 77.3% |
| T1023 | 1 | 3,010 | 377 | 2,633 | 0 | 0 | 110.199 | 478.064 | 588.264 | 81.3% |
| T1023 | 8 | 3,016 | 376 | 2,640 | 0 | 0 | 113.565 | 493.656 | 607.221 | 81.3% |

In separate count runs, degrees visited after all lanes reached ordinary range accounted for 79.1% of fallback degree-lanes at T511 and 84.5% at T1023. Lanes with nonzero binary exponents accounted for 1.18% and 5.52%, respectively. The timing profile therefore identifies a large normal-range segment inside the current scaled fallback, but it does not predict an equivalent speedup: the interval includes callback accumulation, recurrence updates and profiling overhead, and a fused continuation has its own state-transfer and safety costs. Dense probe coefficients produced a normal contribution at degree `m` in every fallback block; a transition kernel must preserve all scaled-prefix contributions.

The pinned DUCC `iter_to_ieee()` path advances through small recurrence states before switching to ordinary kernels. Its skip rule cannot be copied directly: the Rust probe accumulated nonzero contributions from the sectoral degree onward, and Rust's all-lanes-normal predicate differs from DUCC's scale threshold. The current paired iterator also rotates its logical previous/current/next slices after each degree pair. A continuation must receive the exact post-swap slices and pair index, begin at `l+2` once, and add its partial sums to the already accumulated scaled prefix. A later unsafe recurrence state or coefficient product must return to the scaled implementation or trigger a full fallback before output spectra are written.

The reviewer Sol agent independently checked these transition measurements and the candidate handoff. The reviewer found the work counts sufficient to justify an isolated prototype, but not a correctness or speedup assumption. The reviewer identified the rotated-slice handoff, preservation of the prefix, the need to check later recurrence states and coefficient products, and the inability to transfer DUCC's early-term skip. The next prototype will preserve the scaled path and reject the fast continuation if a checked state leaves the normal range. No transition optimization is accepted by these measurements alone.

### Spin-2 synthesis scaled-to-normal continuation (2026-10-12)

For each active spin-2 block that initially needs scaled recurrence, the new f64 path preserves the scaled paired prefix through the first all-lanes-normal state, then continues the paired recurrence and both scalar-conversion channels in the Pulp fused kernel. It applies the spin factor `1/sin(theta)^2` after the degree sum. The scaled path remains the fallback. Before accepting the staged prefix, guards reject any nonzero product or sum component that rounded to zero or became subnormal; the continuation also declines if later recurrence lanes leave the normal range. A final pass checks the spin-weighted staged values for finiteness before writing Fourier spectra. Rejected work may change only worker scratch, which the scaled fallback reinitializes. No workspace capacity or transform-sized storage changed.

Native release timings used `m1b_bench 2 T 8 synthesis`, f64 GL (`512x1025` at T511 and `1024x2049` at T1023), `phi0=0.37`, reused plan/workspace, one warm-up and five timed samples per process. Three process runs were pinned to CPUs 0–7 on the AMD Ryzen 9 5950X, Linux `7.0.14`, with rustc `1.99.0 (b940084d7 2026-09-28)`, standard Cargo release optimization and portable Pulp AVX2/FMA dispatch; no `target-cpu=native` was used. Entries are medians and ranges of process medians in milliseconds:

| Bandlimit | Threads | Active-lane compaction | Scaled-to-normal continuation | Change |
| ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | `126.395 (124.948–126.488)` | `87.301 (86.930–88.874)` | `-30.9%` |
| T511 | 8 | `47.053 (46.904–47.214)` | `33.640 (33.532–34.332)` | `-28.5%` |
| T1023 | 1 | `987.013 (985.470–993.294)` | `606.553 (606.403–612.696)` | `-38.5%` |
| T1023 | 8 | `294.007 (293.917–296.780)` | `199.977 (198.776–200.667)` | `-32.0%` |

The guard costs approximately 6–11% compared with the earlier unguarded prototype, but retains the measured whole-transform improvement. The final finite-output preflight has no resolved timing cost: the final repeated-run intervals overlap those from the guarded build before that check. Worker workspace size is unchanged.

The rebuilt Python plan benchmark used Python `3.11.15`, NumPy `2.4.6`, `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, identical T511 GL coefficients, maps and `phi0`, three process runs of five samples, and explicit Rayon/DUCC thread counts. Rust plan calls include per-call workspace allocation; DUCC uses its direct Python API. RustSHT/DUCC medians were `90.527/28.789 ms` at one thread (`3.14x`) and `35.670/5.983 ms` at eight threads (`5.96x`). The map difference was max absolute `5.624e-14`, RMS `1.611e-15`, and relative L2 `1.323e-13`. This is not parity; it confirms the native gain on an end-to-end Python path while retaining a substantial absolute gap.

Focused scalar-reference tests cover the transition with a 63-lane tail, late-tail rejection, and the underflow case: at T511, a zero-coefficient control enters the transition, while a tiny nonzero sectoral coefficient declines because the unweighted product is zero although the scaled fallback produces a nonzero subnormal after spin weighting. Release DUCC checks passed for spin 1 and 2 at T511 on GL/CC/F1; the T511 spin-2 GL synthesis error was included in the all-grid maximum of `5.107e-15`. Spin adjoint identities passed. The T2047 sparse DUCC and analytic synthesis check passed for spins 1–4 on GL/CC/F1 at one and eight threads; spin-2 DUCC sample error was `2.085e-13`, and analytic map maximum error was `1.236e-11`. Full `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (100 passed, 7 ignored; 10 f32 tests passed), `cargo test -p rustsht --no-default-features` (96 passed, 7 ignored; 10 f32 tests passed), and `uv run pytest python/tests` (8 passed) passed. The reviewer Sol agent performed a strictly read-only review after the guard changes and found no actionable correctness or ownership issue; its last requested check was the timing of the finite-output preflight, which showed no resolved overhead. The continuation is retained. Its follow-up profile and scheduling experiment are recorded below.

### Comparative spin-2 synthesis profile and scheduling experiment (2026-10-12)

The baseline used commit `8e4a8df`, `examples/m1b_bench.rs`, f64 GL T511/T1023, spin-2 STANDARD synthesis, `phi0=0.37`, deterministic dense coefficients, reused plan/workspace, one warm-up and five timed samples per process. Three process runs were pinned to CPUs `0-7` for 1/8 threads, `0-15` for 16, and `0-31` for 32. The host was an AMD Ryzen 9 5950X with Linux `7.0.14`; rustc was `1.99.0 (b940084d7 2026-09-28)`, using standard Cargo release settings and runtime AVX2/FMA dispatch. No native CPU flags were enabled. Entries are medians and ranges of process medians in milliseconds.

| Bandlimit | Threads | RustSHT synthesis |
| ---: | ---: | ---: |
| T511 | 1 | `88.487 (87.215–88.659)` |
| T511 | 8 | `33.232 (33.077–33.728)` |
| T1023 | 1 | `603.572 (603.483–606.235)` |
| T1023 | 8 | `198.433 (197.967–199.220)` |
| T1023 | 16 | `175.467 (167.119–183.260)` |
| T1023 | 32 | `181.063 (180.743–184.951)` |

The matched Python-plan comparison used Python `3.11.15`, NumPy `2.4.6`, installed `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`), the same `phi0`, deterministic E/B coefficients and input maps, three process runs of five samples, explicit Rayon/DUCC thread counts, and GL grids. It includes Python-call and output-allocation costs; Rust plan calls also allocate a workspace per call. Errors are RustSHT synthesis against DUCC.

| Bandlimit | Threads | RustSHT plan (ms) | DUCC (ms) | Ratio | Max abs / RMS / relative L2 error |
| ---: | ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | `88.946` | `27.676` | `3.21x` | `5.624e-14 / 1.611e-15 / 1.323e-13` |
| T511 | 8 | `33.975` | `5.598` | `6.07x` | `5.624e-14 / 1.611e-15 / 1.323e-13` |
| T1023 | 1 | `646.902` | `256.535` | `2.52x` | `1.526e-13 / 3.253e-15 / 1.336e-13` |
| T1023 | 8 | `243.866` | `41.187` | `5.92x` | `1.526e-13 / 3.253e-15 / 1.336e-13` |

The benchmark measured NumPy input copies at `0.07–0.22 ms` for T511 and `0.73–3.93 ms` for T1023, much less than either transform. This does not account for the remaining gap. The DUCC timing is an end-to-end package call; it was not decomposed into native phases.

The test-only path profile used one warmed synthesis per setting and is diagnostic, not a clean timing comparison. At T1023/8T it measured 8 contiguous 128-ring groups with elapsed group intervals from `60.063` to `130.475 ms`; groups at the polar ends were heavier. Its whole-call timing was `207.841 ms`, compared with the uninstrumented `198.433 ms` baseline. The `transition` interval combines the scaled prefix, fused continuation, and its checks; it is not a prefix-only cost. The coefficient/factor setup interval is repeated per ring group and was about `15 ms` per group. At T511/8T, 906 blocks attempted the transition, 898 completed it, and 8 used the scaled fallback. At T1023/8T, all 3,016 attempts completed the transition. The instrumentation and profiler source were removed after collecting these results.

Active-ring work is already compacted in mixed blocks. In the test-only T1023/8T profile, 1,792 blocks were mixed and 3,747 were fully excluded. Across the profile's block accounting, 451.049 million of 536.331 million estimated degree-lanes were active; the 85.283 million difference (15.9%) includes both fully excluded blocks and inactive lanes in mixed blocks. It is not executed recurrence work: fully excluded blocks are skipped, and mixed-block fallback compacts active lanes before recurrence. An earlier mixed-block-only estimate counted 24.017 million inactive lanes (5.06% of the 475.065 million potential lanes in blocks with at least one active lane). The measured compaction gather intervals summed to about `0.17 ms` across groups; staged scatter intervals also remained small. The profile timer did not isolate the finite-output preflight. These measurements do not support a larger active-ring packing structure as the next change.

A bounded scheduling prototype doubled the target group count for parallel workspaces, changing T1023/8T from 8 groups of 128 rings to 16 groups of 64 rings while retaining the 64-lane recurrence block size. The focused release regression `m4_spin2_t1023_synthesis_matches_one_and_eight_threads` passed: sparse spin-2 output matched within `2e-12`, with an active ring near the `sin(theta)=0.01` fast-path boundary and an exactly zero polar ring below the order cutoff. T1023/1T measured `607.213 ms` (`605.133–610.708`), overlapping the baseline range. T1023/8T measured `219.432 ms` (`216.870–222.942`), a `10.6%` regression against the clean `198.433 ms` baseline. The instrumented profile confirmed that the 16 tasks reduce individual group duration but repeat about `15 ms` of coefficient conversion and factor setup per task; the extra groups also leave substantial per-worker load spread. Eight extra spin-2 worker scratch objects add an estimated `0.9 MiB`; no spectrum buffer is duplicated. The prototype was reverted because its whole-transform result was slower. The regression test remains as high-bandlimit coverage for thread-group and cutoff-boundary equivalence.

The source audit and profiling reviews were read-only. Reviewer Sol confirmed that the task-size change was a suitable bounded test, then noted that a 64-ring boundary can repartition the spin-2 threshold tail and should be checked numerically; the added T1023 test covers that case and polar exclusion. The reviewer also cautioned that the block intervals are not CPU cycles and that the test-only phase timer does not isolate the scaled prefix. In a follow-up review, Sol confirmed the corrected lane accounting, ratio and regression coverage; its final arithmetic correction changed the mixed-block-only share to `5.06%`. The next profile follows the updated priority toward spin-0 and spin-1 STANDARD, GRAD_ONLY and DERIV1 on GL, CC and F1, including lower-bandlimit fast-path eligibility. Active-lane compaction and scaled-to-normal continuation will be measured independently on those paths. FFT optimization remains out of scope.

### Spin-0 fully excluded synthesis blocks (2026-10-12)

At `674e5ed`, scalar synthesis sent every 64-ring block through the paired fused attempt. If any lane exceeded the polar order limit, the fused attempt declined and the scaled recurrence evaluated all lanes. The accumulation callback then skipped excluded rings. The synthesis loop now skips a block when `m` exceeds every ring limit in that block. It writes no Fourier values for an excluded block, so phase and inverse-FFT handling are unchanged.

A temporary release diagnostic counted potential paired recurrence lane work as `block_len * ((lmax - m) / 2 + 1)`. It separated fully excluded blocks from inactive lanes in mixed blocks and from fully active blocks rejected by sectoral scaling. These counts describe RustSHT's potential recurrence work; they are not elapsed time, executed instructions, or DUCC batch counts. DUCC `fill_a2m()` compacts active north/south ring pairs into SIMD batches, while this change skips only blocks with no active ring. The diagnostic used the production polar-order limits on GL, CC and F1. CC/F1 analysis counts used the prepared meridional grid. A read-only review by Sol confirmed the paired degree count and that skipping a fully excluded synthesis block cannot affect another block's recurrence state.

Fully excluded scalar-synthesis pair-lane work / total potential pair-lane work was:

| Grid | T63 | T127 | T255 | T511 | T1023 |
| --- | ---: | ---: | ---: | ---: | ---: |
| GL | `0 / 67,584` | `0 / 532,480` | `0 / 4,227,072` | `1,590,016 / 33,685,504` | `31,457,152 / 268,959,744` |
| CC | `0 / 68,640` | `196 / 536,640` | `6,084 / 4,243,584` | `1,642,820 / 33,751,296` | `31,764,356 / 269,222,400` |
| F1 | `0 / 67,584` | `0 / 532,480` | `0 / 4,227,072` | `1,607,296 / 33,685,504` | `31,550,720 / 268,959,744` |

At T1023 GL, fully excluded blocks account for `11.7%` of potential paired lane work and `38.1%` of the lane work in blocks rejected by the fused attempt. Mixed-block inactive lanes account for a separate `4.6%` of total potential lane work. T255 has no fully excluded GL or F1 blocks, and CC has `0.14%`; low-band synthesis should gain little from this skip.

The native comparison used `examples/m1a_bench.rs`, f64 GL, full `mmax`, `phi0=0.37`, reused plans and workspaces, one warm-up and five timed samples per process, with three process runs pinned to CPUs 0–7 on an AMD Ryzen 9 5950X (Linux `7.0.14`). Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo release optimization with runtime AVX2/FMA dispatch and no `target-cpu=native`. Each entry is the median and range of process medians in milliseconds:

| Bandlimit | Threads | Before | After | Change |
| ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | `79.9 (79.1–80.7)` | `75.3 (74.5–76.4)` | `-5.7%` |
| T511 | 8 | `29.1 (28.5–29.8)` | `27.0 (26.9–30.4)` | `-7.3%` |
| T1023 | 1 | `655.5 (652.8–660.1)` | `569.2 (567.9–571.9)` | `-13.2%` |
| T1023 | 8 | `193.4 (192.6–196.3)` | `161.2 (160.1–161.6)` | `-16.7%` |

Percentage changes use the unrounded process medians. The T511 eight-thread ranges overlap. At T1023, the measured synthesis reductions were `86.3 ms` with one thread and `32.2 ms` with eight. The skip adds no workspace memory. The T511/T1023/T2047 scalar analytic and DUCC test passed against `ducc0==0.41.0` built from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` on GL, CC and F1. At T2047 the largest analytic map error was `9.96e-12`, and the largest DUCC sample error was `2.08e-13`. Scalar synthesis/analysis adjoint tests passed on GL, CC and F1. The corresponding analysis-pair work estimate motivated the separate skip below.

### Spin-0 fully excluded analysis pair blocks (2026-10-12)

`analyze_symmetric_order_group()` formed a north/south spectrum pair for every 64-lane block and advanced the recurrence even when the polar cutoff excluded every pair in that block. The analysis loop now skips such blocks. Scalar order limits rise monotonically from each pole toward the equator, so the nearest-equator north ring (`block_end - 1`) and mirrored south ring (`latitude_count - block_end`) bound the cutoff for all pairs in a block. A per-worker minimum of these block bounds avoids checking blocks for orders that cannot exclude any block. This retains the existing per-ring zeroing for mixed blocks and adds no workspace storage. The equatorial singleton remains in the final block and is never double-counted.

A corrected cutoff diagnostic estimated `15,728,576 / 134,479,872` potential paired degree-lanes in fully excluded GL blocks at T1023 (`11.7%`). Inactive lanes in mixed blocks account for a separate `4.6%`; this change does not compact those lanes. These are potential lane counts, not elapsed time or executed instruction counts. A read-only Sol review checked the monotonic endpoint bound, mirrored indices and equatorial handling.

The native comparison used `examples/m1a_bench.rs` with f64 GL, `phi0=0.37`, full `mmax`, reused plan/workspace, one warm-up and five timed samples per process. Three process runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X (Linux `7.0.14`), with rustc `1.99.0 (b940084d7 2026-09-28)`, Cargo release optimization, runtime AVX2/FMA dispatch and no `target-cpu=native`. The before build was `a329461`; after uses the candidate source. Each cell gives the median and range of process medians in milliseconds; percentages use unrounded medians:

| Bandlimit | Threads | Before | After | Change |
| ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | `76.5 (76.3–77.0)` | `60.9 (60.0–61.7)` | `-20.3%` |
| T511 | 8 | `21.7 (21.2–21.8)` | `15.9 (15.9–16.1)` | `-26.7%` |
| T1023 | 1 | `657.6 (653.7–657.9)` | `364.1 (363.9–366.0)` | `-44.6%` |
| T1023 | 8 | `193.3 (192.4–195.1)` | `102.5 (102.2–104.1)` | `-47.0%` |

The reductions at T511 were `15.5 ms` and `5.8 ms`; at T1023 they were `293.6 ms` and `90.8 ms`. Low-band checks showed no material regression: T63 was `0.191/0.133` to `0.189/0.130 ms`, T127 `1.142/0.586` to `1.119/0.606 ms`, and T255 `9.945/3.257` to `9.628/3.308 ms` at one/eight threads. T127 and T255 were slightly faster at one thread; eight-thread process ranges overlap. The scalar analysis adjoint tests passed on GL, CC and F1. The T511/T1023/T2047 all-grid test against static `ducc0==0.41.0` references from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` and the analytic sectoral field passed; maximum DUCC sample error was `2.08e-13`, and the maximum T2047 analysis coefficient error was `4.18e-14`. The new branch adds no memory.

### Spin-0/1 baseline and priority reset (2026-10-12)

The numerical library baseline is commit `01c387e`; the GL/CC/F1 and mode options are in benchmark-harness commit `f0d6894`. The native harness accepts spin-1 STANDARD, GRAD_ONLY and DERIV1 operations; DERIV1 leaves its ignored scalar monopole zero. The Python harness compares plan calls against installed `ducc0==0.41.0`. The pinned DUCC source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Measurements use an AMD Ryzen 9 5950X, Linux `7.0.14`, CPUs 0–7, `rustc 1.99.0 (b940084d7 2026-09-28)`, Cargo release settings, runtime AVX2/FMA dispatch and no `target-cpu=native`; the host has no AVX-512. Native calls reuse plans and workspaces. Python calls reuse the plan but allocate a workspace per operation. Both harnesses warm each operation and take five timed samples per process; the native T63–T511 table is the median of three process medians. T1023 STANDARD is three runs, while T1023 GRAD_ONLY/DERIV1 are single-run screening values. Each Python comparison has one process run of five samples. All grids use `phi0=0.37`, full `mmax=lmax`, and `nphi=2*lmax+3`.

Native f64 GL measurements are milliseconds. Each paired value is one thread/eight threads:

| Bandlimit | Scalar S | Scalar A | Spin-1 STANDARD S | Spin-1 STANDARD A | GRAD_ONLY S | DERIV1 S |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T63 | `0.183/0.210` | `0.180/0.122` | `2.703/2.722` | `2.323/0.460` | `2.738/2.733` | `2.722/2.737` |
| T127 | `1.850/1.087` | `1.176/0.609` | `20.934/11.124` | `17.937/3.000` | `20.891/11.192` | `21.068/11.147` |
| T255 | `14.684/7.233` | `9.961/3.259` | `162.088/45.455` | `139.923/20.353` | `162.469/45.283` | `163.025/45.971` |
| T511 | `80.379/28.959` | `77.521/21.358` | `132.388/46.838` | `200.157/46.059` | `1,265.935/189.453` | `1,261.595/188.407` |
| T1023 | `660.226/196.632` | `655.321/192.676` | `1,067.116/312.864` | `1,155.684/293.053` | `9,952.165/1,466.865` | `9,958.954/1,485.947` |

At low orders, scalar GL is close to the Python DUCC call: T63 scalar synthesis is `0.174/0.181 ms` versus `0.183/0.322 ms` at one/eight threads, and scalar analysis is `0.178/0.203 ms` versus `0.222/0.344 ms`. The same T63 spin-1 STANDARD calls take `2.786/2.848 ms` for synthesis and `2.395/2.464 ms` for analysis, against DUCC `0.350/0.476 ms` and `0.387/0.535 ms`. At T127 the spin-1 STANDARD synthesis is `21.384/18.600 ms` versus `2.422/0.561 ms`, and analysis is `18.590/11.982 ms` versus `2.192/0.386 ms`. These Python comparisons use one process per cell; treat them as screening results, especially at eight threads.

Matched Python-plan comparisons against DUCC on GL used the same coefficients, map, `phi0`, bandlimit and thread count. Times are milliseconds; each entry is one process median over five timed samples.

| Operation | T | RustSHT 1T/8T | DUCC 1T/8T | Rust/DUCC ratio 1T/8T | Relative L2 error |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar S | 255 | `15.478/15.264` | `2.238/0.722` | `6.9x/21.1x` | `3.09e-14` |
| Scalar A | 255 | `10.336/9.999` | `2.089/1.159` | `4.9x/8.6x` | `1.53e-14` |
| Scalar S | 511 | `81.539/31.138` | `13.355/3.432` | `6.1x/9.1x` | `6.11e-14` |
| Scalar A | 511 | `78.023/22.208` | `12.130/1.689` | `6.4x/13.1x` | `2.06e-14` |
| Scalar S | 1023 | `654.191/199.938` | `117.205/18.723` | `5.6x/10.7x` | `1.15e-13` |
| Scalar A | 1023 | `668.519/201.200` | `121.937/16.794` | `5.5x/12.0x` | `4.92e-14` |
| Spin-1 STANDARD S | 255 | `161.159/45.509` | `6.526/2.053` | `24.7x/22.2x` | `2.32e-14` |
| Spin-1 STANDARD A | 255 | `139.726/25.828` | `5.966/0.922` | `23.4x/28.0x` | `1.50e-14` |
| Spin-1 GRAD_ONLY S | 255 | `164.539/46.202` | `5.107/1.501` | `32.2x/30.8x` | `2.63e-14` |
| Spin-1 DERIV1 S | 255 | `161.158/45.320` | `4.475/1.522` | `36.0x/29.8x` | `3.05e-14` |
| Spin-1 STANDARD S | 511 | `134.965/48.565` | `26.619/5.314` | `5.1x/9.1x` | `6.33e-14` |
| Spin-1 STANDARD A | 511 | `217.457/49.136` | `25.407/3.459` | `8.6x/14.2x` | `4.12e-14` |
| Spin-1 GRAD_ONLY S | 511 | `1,247.137/192.098` | `32.851/6.133` | `38.0x/31.3x` | `5.22e-14` |
| Spin-1 DERIV1 S | 511 | `1,258.686/189.062` | `31.791/6.044` | `39.6x/31.3x` | `6.10e-14` |
| Spin-1 STANDARD S | 1023 | `1,095.100/352.204` | `248.976/39.326` | `4.4x/9.0x` | `1.15e-13` |
| Spin-1 STANDARD A | 1023 | `1,260.419/319.918` | `248.446/38.311` | `5.1x/8.4x` | `7.53e-14` |
| Spin-1 GRAD_ONLY S | 1023 | `9,855.019/1,486.739` | `289.840/47.693` | `34.0x/31.2x` | `1.01e-13` |
| Spin-1 DERIV1 S | 1023 | `9,878.177/1,489.879` | `293.972/47.552` | `33.6x/31.3x` | `1.18e-13` |

At pre-change HEAD `1f5d939`, the spin-1 scalar-conversion synthesis path is gated at `lmax >= 500`, so spin-1 STANDARD at T63/T127/T255 uses the general spin recurrence. GRAD_ONLY and DERIV1 do not use that conversion at any bandlimit. DUCC's threshold is a path-selection heuristic; the coefficient conversion itself supports lower degrees. A reviewer Sol read-only audit confirmed that E-only STANDARD conversion is mathematically the same as GRAD_ONLY when the B channel is zero. DERIV1 needs a separate degree normalization and was evaluated in the follow-up below. The baseline comparison targets the spin-1 threshold without changing the polar cutoff or recurrence formula. `plan.rs` must expand spin-1 fast scratch together with the runtime gate; at T63 this adds about 4.1 KiB per recurrence worker for four `Complex64` rows, or about 33 KiB for eight workers. Record workspace allocation and Python-plan timing with native reused-workspace timing so this cost is visible.

The grid comparison shows scalar performance is similar across GL/CC/F1, and spin-1 synthesis has the same order of magnitude across those grids. Spin-1 analysis has a distinct CC/F1 gap because scalar-conversion analysis is currently enabled only on GL: at T511, CC Rust/DUCC analysis is `1,105.587/57.122 ms` at one thread and `199.530/10.132 ms` at eight; F1 is `1,091.454/55.335 ms` and `171.858/7.839 ms`. This remains a later shared-kernel target after the lower-band spin-1 synthesis path is measured.

The T511 Rust input-copy cost is below `0.3 ms`, far below transform time, so bindings do not explain the remaining gap. `perf_event_paranoid=4` blocks hardware counters; Callgrind is available, but this baseline has not yet added phase timers to the spin-0/1 paths. The next diagnostic measures active, mixed and excluded ring work and the scaled/normal path distribution separately for scalar and spin-1 STANDARD/GRAD_ONLY/DERIV1 on GL, then checks CC/F1 behavior. No FFT changes are included.

### Lower-band spin-1 scalar conversion (2026-10-12)

The spin-1 paired-scalar path now starts at `lmax=63` for f64. Synthesis uses it on GL, CC and F1; analysis uses it on GL, where exact ring weights are available. GRAD_ONLY uses the E-only conversion, with absent B treated as zero. DERIV1 remains on the general path because its degree normalization differs. Spin 2 retains its `lmax >= 500` threshold, and f32 remains on the existing recurrence. The spin-1 polar threshold remains `sin(theta) >= 1e-4`.

The comparison used the same native benchmark inputs as the preceding baseline, with execution pinned to CPUs 0–7 on the Ryzen 9 5950X. Release builds used `rustc 1.99.0`, runtime AVX2/FMA dispatch, and no `target-cpu=native`. The values below are medians of three process runs, each with five timed samples, in milliseconds. Each pair is one/eight threads; `before -> after` compares the previous GL baseline with the updated source.

| Bandlimit | Operation | Before | After | Time reduction |
| ---: | --- | ---: | ---: | ---: |
| T63 | STANDARD synthesis | `2.703/2.722` | `0.345/0.356` | `87%/87%` |
| T63 | STANDARD analysis | `2.323/0.460` | `0.457/0.245` | `80%/47%` |
| T63 | GRAD_ONLY synthesis | `2.738/2.733` | `0.362/0.361` | `87%/87%` |
| T127 | STANDARD synthesis | `20.934/11.124` | `3.346/2.141` | `84%/81%` |
| T127 | STANDARD analysis | `17.937/3.000` | `3.640/1.750` | `80%/42%` |
| T127 | GRAD_ONLY synthesis | `20.891/11.192` | `3.430/2.211` | `84%/80%` |
| T255 | STANDARD synthesis | `162.088/45.455` | `24.248/11.748` | `85%/74%` |
| T255 | STANDARD analysis | `139.923/20.353` | `32.894/10.128` | `76%/50%` |
| T255 | GRAD_ONLY synthesis | `162.469/45.283` | `25.302/12.257` | `84%/73%` |
| T511 | GRAD_ONLY synthesis | `1,265.935/189.453` | `132.620/46.765` | `90%/75%` |
| T1023 | GRAD_ONLY synthesis | `9,952.165/1,466.865` | `1,075.352/311.154` | `89%/79%` |

The current STANDARD code path at T511/T1023 was already active before this change. Its new pinned medians were T511 synthesis `129.777/45.740 ms`, analysis `205.941/46.763 ms`, and T1023 synthesis `1,041.672/302.010 ms`, analysis `1,197.383/298.197 ms`; these are within 4% of the previous measurements. Representative three-process ranges for the changed GL synthesis timings were T63 STANDARD `0.336–0.384/0.350–0.359 ms`, T127 `3.340–3.350/2.042–2.205 ms`, and T255 `24.226–24.675/11.652–12.019 ms`. T63 eight-thread STANDARD analysis ranged `0.229–0.248 ms`; T255 ranged `10.096–10.315 ms`.

The Python plan comparison used `ducc0==0.41.0`, pinned to CPUs 0–7, with three groups of five timed samples per operation. At T255, RustSHT/DUCC times in milliseconds and relative L2 error were:

| Grid | Operation | RustSHT 1T/8T | DUCC 1T/8T | Relative L2 error |
| --- | --- | ---: | ---: | ---: |
| GL | STANDARD synthesis | `25.042/12.484` | `6.433/1.667` | `2.76e-14` |
| GL | STANDARD analysis | `32.787/10.314` | `5.886/0.856` | `1.64e-14` |
| GL | GRAD_ONLY synthesis | `26.217/12.815` | `4.791/1.464` | `2.79e-14` |
| CC | STANDARD synthesis | `28.452/13.972` | `8.462/1.835` | `2.66e-14` |
| CC | STANDARD analysis | `146.396/23.771` | `13.950/2.854` | `1.75e-14` |
| CC | GRAD_ONLY synthesis | `28.841/14.409` | `6.342/1.650` | `2.57e-14` |
| F1 | STANDARD synthesis | `26.312/13.195` | `6.483/1.634` | `2.85e-14` |
| F1 | STANDARD analysis | `146.398/24.389` | `13.833/2.922` | `1.75e-14` |
| F1 | GRAD_ONLY synthesis | `26.745/13.382` | `5.145/1.493` | `3.01e-14` |

GL Python comparisons also matched DUCC at T63 (STANDARD synthesis/analysis `8.49e-15/8.98e-15`; GRAD_ONLY synthesis `8.35e-15`) and T127 (`1.35e-14/1.05e-14`; GRAD_ONLY `1.29e-14`). Existing high-band Python checks at T511/T1023 had GRAD_ONLY relative errors `5.64e-14/1.06e-13`. The new low-band all-grid test compares fast spectra with the general spin recurrence for E-only, B-only, mixed E/B and GRAD_ONLY, and checks STANDARD/GRAD_ONLY equality. The T63 GL gradient-adjoint identity uses the general unweighted spin recurrence; it does not exercise scalar-conversion analysis. STANDARD GL analysis comparisons against DUCC cover that conversion path. The release suite passed 102 Rust tests and 10 f32 tests (8 opt-in tests remain ignored); all-feature workspace tests, no-default-feature tests, clippy with warnings denied, formatting, and all 8 Python tests passed. The ignored T511 all-grid spin test also passed for spins 1 and 2; maximum synthesis sample error versus DUCC was `5.2e-15` and maximum analysis coefficient error was `2.6e-17`.

Workspace scratch grows by four `Complex64` coefficient rows per recurrence worker at the new threshold: 4.1 KiB per worker at T63, or 33 KiB for eight workers. T63 workspace allocation was `0.05 ms` median; plan construction was `0.024 ms`. Python 8-thread runs at T63/T127 and F1 analysis had scheduler outliers, so native reused-workspace medians are the performance acceptance measure. The change reduces the measured GL workload at all targeted bands and adds no FFT work. The DERIV1 continuation is measured below; next, profile active-ring fractions and scaled/normal recurrence work for scalar and spin-1 STANDARD/GRAD_ONLY independently on GL, CC and F1.

### Spin-1 mixed-cutoff active-lane compaction (2026-10-12)

At `204d83f`, mixed polar-cutoff blocks in the f64 spin-1 scalar-conversion synthesis path fell back to the scaled paired recurrence with all 64 latitude lanes, although some lanes were known to contribute zero. The prototype compacts only active lanes into existing per-worker theta and ring-index scratch, runs the same scaled recurrence, then scatters into the original Fourier rows. Fully active fused blocks keep their existing path; fully excluded blocks are skipped. It adds no allocation or full-spectrum workspace. It applies to spin-1 STANDARD and GRAD_ONLY scalar-conversion synthesis on GL, CC and F1; DERIV1 remains on the general spin path.

The comparison used `examples/m1b_bench.rs`, f64 spin-1 synthesis, reused plans/workspaces, one warm-up and five samples per process, three release-process runs per cell, `phi0=0.37`, full `mmax`, and CPUs 0–7 pinned on an AMD Ryzen 9 5950X (Linux `7.0.14`). Rust was `1.99.0 (b940084d7 2026-09-28)`, with standard Cargo release settings and runtime AVX2/FMA dispatch; `target-cpu=native` was not enabled. Entries show medians of process medians (milliseconds), paired as one/eight threads:

| Grid / mode | T | Before 1T/8T | Compacted 1T/8T | Change 1T/8T |
| --- | ---: | ---: | ---: | ---: |
| GL STANDARD | 63 | `0.353/0.376` | `0.355/0.359` | `+0.6%/-4.5%` |
| GL GRAD_ONLY | 63 | `0.356/0.367` | `0.348/0.422` | `-2.2%/+15.0%` |
| GL STANDARD | 127 | `3.404/2.072` | `3.364/2.126` | `-1.2%/+2.6%` |
| GL GRAD_ONLY | 127 | `3.454/2.113` | `3.393/2.131` | `-1.8%/+0.9%` |
| GL STANDARD | 255 | `24.986/12.186` | `23.475/11.810` | `-6.0%/-3.1%` |
| GL GRAD_ONLY | 255 | `25.525/12.593` | `24.114/11.913` | `-5.5%/-5.4%` |
| GL STANDARD | 511 | `134.770/47.491` | `120.604/41.064` | `-10.5%/-13.5%` |
| GL GRAD_ONLY | 511 | `135.137/47.901` | `121.386/42.354` | `-10.2%/-11.6%` |
| GL STANDARD | 1023 | `1,057.488/305.959` | `942.564/264.240` | `-10.9%/-13.6%` |
| GL GRAD_ONLY | 1023 | `1,074.426/309.332` | `947.604/268.856` | `-11.8%/-13.1%` |

At T511, CC STANDARD/GRAD_ONLY improved `8.2–9.7%` and F1 improved `8.5–12.8%` at one/eight threads. These are complete native transforms including FFT and scheduling. Small-band changes are within the run-to-run variation; GL T63/T127 before/after process ranges overlap. Callgrind at T127 GL STANDARD recorded `853,547,901` instruction references before and `841,501,995` after (`-1.41%`); recurrence references fell `2.8%` and lane-normalization references `13.8%`. These simulated instruction-reference counts are not cycles or elapsed time.

The T511 GL regression creates mixed and fully excluded blocks at order `m=450`, checks exact zero output for excluded rings, and compares the nearest active boundary ring to a one-ring uncompactified paired-recurrence reference at relative tolerance `1e-11`. Existing T511 all-grid spin-1 DUCC checks passed with maximum synthesis sample error `4.885e-15` and maximum analysis coefficient error `4.934e-18`; T1023 sparse synthesis checks passed with maximum sample errors `3.893e-15` (GL), `3.150e-15` (CC), and `3.567e-15` (F1). Spin adjoint identities passed. Sol's strictly read-only review found no cutoff, indexing, scratch ownership or race defect; it recommended the active-boundary relative check, now included. The improvement is retained for measured medium/high-band workloads. No FFT implementation was changed.

### Spin-0 mixed-cutoff active-lane compaction (2026-10-12)

Relative to `97f2fab`, the candidate f64 scalar synthesis compacts active latitude lanes for mixed cutoff blocks only after the fused normal-range kernel declines and only when `lmax >= 255`. It uses existing per-worker theta, cosine and ring-index buffers, runs the existing scaled paired recurrence on compact lanes, and scatters each result to its original Fourier row. Fully active blocks and low-band blocks retain their existing paths; fully excluded blocks are skipped by the preceding cutoff check. This preserves the scaled fallback for the same lanes and adds no workspace allocation. The focused T255 regression compares active spectra with an all-active cutoff reference, checks exact zeros for excluded rings, and compares a nonzero boundary-ring reference at relative tolerance `1e-11`.

The before source was commit `97f2fab`; the candidate includes this compaction. Measurements used `examples/m1b_bench.rs`, f64 full-band GL scalar synthesis with `phi0=0.37`, reused plans/workspaces, one warm-up and five timed samples per process, and three process runs per case. Processes were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X running Linux `7.0.14`. Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo release settings used runtime AVX2/FMA dispatch, and no `target-cpu=native` flag was set. Entries are medians of process medians in milliseconds; ranges are minimum and maximum process medians:

| Grid / bandlimit | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| GL T63 | 1 / 8 | `0.176 / 0.176` | `0.189 / 0.189` | no resolved change |
| GL T127 | 1 / 8 | `1.870 / 1.074` | `1.872 / 1.126` | no resolved change |
| GL T255 | 1 / 8 | `14.677 / 7.002` | `13.500 / 6.517` | `-8.0% / -6.9%` |
| GL T511 | 1 / 8 | `74.262 / 26.120` | `66.519 / 23.480` | `-10.4% / -10.1%` |
| GL T1023 | 1 / 8 | `564.044 / 157.336` | `529.830 / 144.696` | `-6.1% / -8.0%` |
| CC T511 | 1 / 8 | `75.982 / 33.952` | `65.506 / 28.989` | `-13.8% / -14.6%` |
| F1 T511 | 1 / 8 | `77.050 / 32.070` | `67.610 / 23.934` | `-12.3% / -25.4%` |

The GL process-median ranges were T63 `0.170–0.214 / 0.174–0.227` before and `0.180–0.235 / 0.179–0.230` after; T127 `1.849–2.007 / 1.061–1.084` and `1.850–1.878 / 1.078–1.210`; T255 `14.611–14.771 / 6.906–7.014` and `13.445–13.525 / 6.322–6.700`; T511 `73.872–74.317 / 26.090–26.361` and `66.315–67.448 / 23.164–23.562`; T1023 `560.335–565.069 / 156.337–157.860` and `527.659–531.626 / 143.340–145.182`. T63/T127 show no reliable gain, consistent with the `lmax >= 255` gate. The F1 T511 one-thread range was `76.648–77.067` before and `67.106–68.004` after; its eight-thread before range (`31.892–38.017`) contains a scheduler outlier, so that percentage is less stable than the GL and CC results.

Callgrind on T511 GL recorded `20,047,269,432` instruction references before and `17,760,754,032` after (`-11.4%`); at T127 it recorded `477,045,197` and `477,516,952` (`+0.10%`), consistent with the disabled low-band path. These are simulated instruction references, not cycles or elapsed time. A previous cutoff diagnostic estimated that mixed-block inactive lanes account for `4.6%` of total potential paired lane work at T1023 GL; fully excluded blocks are already skipped. The per-worker scratch is reused, so allocated workspace size is unchanged.

The existing release scalar T511/T1023/T2047 analytic and DUCC comparison passed on GL, CC and F1 against `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; the largest T2047 analytic map error was `9.96e-12` and largest DUCC sample error was `2.08e-13`. Scalar synthesis/analysis adjoint identities passed on all three grids. The focused scalar and spin-1 compaction regressions, `cargo fmt --all -- --check`, and `cargo clippy -p rustsht --all-targets --all-features -- -D warnings` passed. Sol's strictly read-only review found no mathematical, cutoff, indexing, scratch or race blocker and recommended asserting a nonzero boundary reference; the test now does so. No FFT code changed. The measured high-band whole-transform improvement supports retaining this path while profiling scalar and spin-1 recurrence work separately.

### Spin-1 DERIV1 scalar-conversion synthesis (2026-10-12)

The DERIV1 synthesis bottleneck was the general spin recurrence: at T255 it took `162.506/45.815 ms` and at T1023 `9,961.947/1,468.921 ms` for one/eight threads. DUCC computes DERIV1 with its general spin kernel as well, but RustSHT's existing spin-1 to scalar conversion provides an equivalent route. For degree `l >= 1`, DERIV1 coefficients are multiplied by `sqrt(l(l+1))` before the existing spin-1 conversion, then evaluated with the scalar paired recurrence. The `l=0` coefficient remains zero. Polar rings below the existing spin-1 sine threshold continue through the general recurrence. Coefficient scaling or conversion that becomes nonfinite declines that order to the established scaled path. The new finite-output scan runs only for DERIV1; STANDARD and GRAD_ONLY retain the prior conversion path without this added pass.

The native f64 GL comparison used `examples/m1b_bench.rs`, reused plans and workspaces, one warm-up and five timed samples per process, three process runs per cell, `phi0=0.37`, full `mmax`, CPUs 0–7, Ryzen 9 5950X, Linux `7.0.14`, `rustc 1.99.0 (b940084d7 2026-09-28)`, Cargo release settings, runtime AVX2/FMA and no `target-cpu=native`. Values are medians of process medians in milliseconds. The previous source had the general recurrence; the candidate added the conversion:

| Bandlimit | Before 1T/8T | After 1T/8T | Reduction 1T/8T |
| ---: | ---: | ---: | ---: |
| T63 | `2.730/2.746` | `0.375/0.378` | `86.3%/86.2%` |
| T127 | `21.031/11.135` | `3.494/2.161` | `83.4%/80.6%` |
| T255 | `162.506/45.815` | `24.526/11.818` | `84.9%/74.2%` |
| T511 | `1,263.629/189.315` | `122.312/42.667` | `90.3%/77.5%` |
| T1023 | `9,961.947/1,468.921` | `959.503/268.851` | `90.4%/81.7%` |

Each before/after cell spans the three process medians. T127 before eight-thread runs ranged `11.097–13.300 ms`, with one scheduler outlier; the other before ranges were T63 `2.695–2.823/2.708–2.849`, T255 `161.082–165.909/45.269–46.606`, T511 `1,261.383–1,266.762/188.798–190.247`, and T1023 `9,916.808–10,149.688/1,456.307–1,469.170`. After ranges were T63 `0.372–0.378/0.370–0.410`, T127 `3.431–3.627/2.134–2.227`, T255 `24.353–24.741/11.818–12.058`, T511 `121.121–124.635/42.233–42.744`, and T1023 `955.727–959.855/267.742–270.574`. The one-thread result improves substantially at every tested band; eight-thread T63/T127 are also faster than their before medians.

The rebuilt Python plan was compared against installed `ducc0==0.41.0` with matching coefficients, `phi0`, dimensions and explicit DUCC thread counts. The package source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Each row is one process median of five samples, so these Python results are a cross-library check rather than the native performance acceptance table:

| Grid / bandlimit | Threads | RustSHT ms | DUCC ms | Rust/DUCC | Relative L2 error |
| --- | ---: | ---: | ---: | ---: | ---: |
| GL T63 | 1 | `0.373` | `0.311` | `1.20x` | `9.69e-15` |
| GL T127 | 1 / 8 | `3.531 / 2.990` | `2.219 / 3.947` | `1.59x / 0.76x` | `1.55e-14` |
| GL T255 | 1 / 8 | `25.239 / 17.392` | `6.731 / 1.546` | `3.75x / 11.25x` | `3.31e-14` |
| GL T511 | 1 / 8 | `129.147 / 47.801` | `32.968 / 6.136` | `3.92x / 7.79x` | `6.63e-14` |
| GL T1023 | 1 / 8 | `1,041.313 / 319.200` | `303.177 / 47.203` | `3.43x / 6.76x` | `1.26e-13` |
| CC T255 | 1 / 8 | `29.840 / 14.034` | `6.397 / 1.593` | `4.66x / 8.81x` | `3.05e-14` |
| F1 T255 | 1 / 8 | `25.961 / 13.621` | `5.231 / 1.499` | `4.96x / 9.09x` | `3.49e-14` |

The T127 eight-thread DUCC timing (`3.947 ms`) is slower than its one-thread timing and is treated as a noisy scheduling result; each Python row used one process. The T63 all-grid test compares the fast result to the general recurrence at odd and even `nphi`, nonzero `phi0`, dense coefficients, `m=lmax`, analytic `Y10`/`Y11`, the zero monopole derivative and the synthesis-adjoint identity. It also checks a finite input whose degree scaling overflows and confirms that the order falls back with finite output identical to the general recurrence. The post-conversion finiteness guard remains defensive; the regression does not independently trigger that guard with finite converted inputs. Sol's strictly read-only review found no normalization, sign, cutoff, workspace, ownership or adjoint blocker. It confirmed that both fallback checks occur before the order writes spectra and that the derivative conversion retains the `lmax+1` extension; it also caught the two rounded ratios corrected above. The T1023 GL Python comparison covers the high-band output against DUCC. Workspace memory increases by four `Complex64` coefficient rows per recurrence worker: `4 * (lmax+2) * 16` bytes, or approximately `64.1 KiB` per worker at T1023 (`512.5 KiB` for eight workers). Existing workspace-allocation measurements changed by at most `0.12 ms`; the measured change is small relative to transform time. `cargo test --workspace --all-features` passed 105 Rust tests and 10 f32 tests; `cargo test -p rustsht --no-default-features` passed 101 Rust tests and 10 f32 tests; `uv run pytest python/tests` passed 8 tests. Release T511 spin-1/2 all-grid, T1023 sparse spin-1/2, T2047 sparse spin-1–4, and T511 spin-3/4 all-grid regressions also passed. No FFT work changed.

### Spin-1 staged scalar-conversion synthesis (2026-10-12)

When the fused paired scalar block declines in spin-1 scalar-conversion synthesis, the path tries the paired recurrence's scaled-to-normal transition. For mixed polar-cutoff blocks it first compacts active latitudes into existing per-worker scratch; other eligible blocks use their original latitude rows. If the transition or post-`1/sin(theta)` finiteness check fails, the established scaled recurrence runs. The path starts only at `lmax >= 255`: an ungated trial increased T127 synthesis time by 18–25%, while the T255 and higher measurements improved. The gate leaves T63 and T127 on their prior code path. The transition reuses recurrence and sum buffers and adds no workspace memory. It retains the spin-1 `sin(theta) >= 0.01` transition boundary; more polar lanes use the scaled fallback. No FFT code changed.

The native f64 GL benchmark used `examples/m1b_bench.rs`, full `mmax`, `phi0=0.37`, reused plans and workspaces, one warm-up and five timed samples, and three process runs per result. Processes were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X running Linux `7.0.14`; rustc was `1.99.0 (b940084d7 2026-09-28)`, release settings used runtime AVX2/FMA dispatch, and `target-cpu=native` was not enabled. Values are medians of process medians in milliseconds. Before values are the previous implementation; the T63/T127 gate prevents this change from taking the new path there.

| GL bandlimit | Operation | Before 1T/8T | After 1T/8T | Change 1T/8T |
| ---: | --- | ---: | ---: | ---: |
| T255 | STANDARD | `23.475/11.810` | `18.306/8.822` | `-22.0%/-25.3%` |
| T255 | GRAD_ONLY | `24.114/11.913` | `18.841/8.905` | `-21.9%/-25.2%` |
| T255 | DERIV1 | `24.526/11.818` | `19.055/9.342` | `-22.3%/-21.0%` |
| T511 | STANDARD | `118.042/41.057` | `89.300/31.940` | `-24.3%/-22.2%` |
| T511 | GRAD_ONLY | `121.032/41.451` | `90.633/32.542` | `-25.1%/-21.5%` |
| T511 | DERIV1 | `123.755/42.546` | `93.990/33.272` | `-24.1%/-21.8%` |
| T1023 | STANDARD | `938.591/258.089` | `595.241/188.484` | `-36.6%/-27.0%` |
| T1023 | GRAD_ONLY | `956.545/269.148` | `599.938/188.732` | `-37.3%/-29.9%` |
| T1023 | DERIV1 | `962.399/270.276` | `603.905/193.135` | `-37.3%/-28.5%` |

T63 and T127 showed no reliable change with the transition disabled. The pre-gate T127 trial was slower, which is why this optimization is limited to T255 and higher. Current T511 native CC/F1 measurements are STANDARD/GRAD_ONLY/DERIV1 `98.891/42.805`, `100.322/43.707`, `100.721/43.758 ms` on CC and `89.650/32.018`, `90.944/32.682`, `91.195/33.280 ms` on F1 (one/eight threads). These are current-state measurements; a matched before run was not recorded for CC/F1, so they are not presented as a cross-grid speedup.

Against installed `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, GL T511 output relative-L2 errors were `6.326e-14` for STANDARD, `5.642e-14` for GRAD_ONLY and `6.628e-14` for DERIV1 at both one and eight threads. The largest absolute errors were `2.519e-14`, `2.209e-14` and `8.153e-12`, respectively. CC and F1 T511 one-thread comparisons passed with relative-L2 errors below `8.6e-14`; their largest absolute errors were below `1.2e-11`. The release T511 all-grid spin test and T1023 sparse spin test passed against DUCC. A focused boundary test checks transition rejection below `sin(theta)=0.01`, acceptance for a 63-lane batch above it, and agreement with the general recurrence at both boundary cases. Its direct helper success uses synthetic scalar rows; the full-driver comparison also checks both E/B outputs but does not assert which internal path was selected. The transition/finiteness checks and scaled fallback preserve finite output when the optimization declines. No per-order transition counts were collected, so these timings establish a whole-transform benefit but do not quantify the fraction of work transitioned.

Sol's strictly read-only review found no mathematical, cutoff, north/south indexing, scratch-ownership or race blocker. It recommended retaining the T255 gate after the low-band regression and noted the direct-helper test's synthetic scalar rows; that coverage limit is recorded above. No changes were made in response to the review. The candidate uses no additional workspace memory. Focused spin-1 transition, scalar-compaction and high-band spin tests passed before the full workspace validation.

### Spin-1 prepared-grid STANDARD analysis conversion (2026-10-13)

At `6a9f0e2`, the spin-1 scalar-conversion analysis path accepted only explicit GL ring weights. CC/F1 exact analysis first normalizes longitude spectra and applies meridional weights in `to_prepared`; the prepared spectra therefore need only the scalar-conversion factor `1/sin(theta)`. The analysis driver now marks those spectra as preweighted and enables the existing spin-1 conversion for them. GL continues to apply `ring_weight/nphi/sin(theta)`. Raw-map synthesis adjoints remain on the general recurrence, and spin-2 selection is unchanged. The prepared cutoff metadata and workspaces are reused; the change adds no workspace allocation and does not modify FFT execution.

Native timings used `examples/m1b_bench.rs`, f64 spin-1 STANDARD exact analysis, full `mmax`, `phi0=0.37`, fixed deterministic maps, reused plans and workspaces, one warm-up and five timed samples per process. Three release processes were run per grid/bandlimit, each reporting one- and eight-thread medians. Processes were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X running Linux `7.0.14-20-pve`; Rust was `1.99.0 (b940084d7 2026-09-28)`, standard Cargo release settings used portable AVX2/FMA runtime dispatch, and `target-cpu=native` was not set. The before measurements are the immediately preceding implementation; each pair below is one/eight threads in milliseconds. Entries report the median across process medians.

| Bandlimit | CC before | CC after | CC reduction | F1 before | F1 after | F1 reduction |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T63 | `2.941/0.696` | `0.874/0.414` | `70.3%/40.5%` | `2.946/0.700` | `0.855/0.428` | `71.0%/38.9%` |
| T127 | `19.994/3.799` | `4.931/2.593` | `75.3%/31.7%` | `19.818/3.735` | `5.038/2.666` | `74.6%/28.6%` |
| T255 | `145.086/23.249` | `38.312/13.556` | `73.6%/41.7%` | `145.872/23.643` | `38.152/13.594` | `73.8%/42.5%` |
| T511 | `1,142.218/164.918` | `227.784/62.597` | `80.1%/62.0%` | `1,111.216/167.586` | `227.185/62.957` | `79.6%/62.4%` |
| T1023 | `8,547.097/1,273.916` | `1,286.561/371.109` | `85.0%/70.9%` | `8,617.847/1,274.731` | `1,290.880/374.115` | `85.0%/70.7%` |

The GL control remained within the previous process variation: before/after one/eight-thread medians were T63 `0.480/0.263` and `0.465/0.222 ms`; T127 `3.709/1.788` and `3.634/1.772 ms`; T255 `32.571/10.043` and `32.692/10.027 ms`; T511 `202.189/46.824` and `202.523/46.315 ms`; T1023 `1,173.960/294.667` and `1,179.933/295.184 ms`. One T63 F1 process was a scheduler outlier (`1.420/0.551 ms`); the other two process medians were `0.855/0.428` and `0.850/0.421 ms`. At all larger sizes, process ranges were narrow relative to the measured CC/F1 gains.

The Python plan comparison at T511 used Python `3.11.15`, NumPy `2.4.6`, `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, three process runs of five samples, and explicit matching thread counts. Rust plan calls include per-call workspace creation; DUCC uses its direct analysis API. CC Rust/DUCC medians were `226.781/54.962 ms` at one thread (`4.13x`) and `63.725/7.643 ms` at eight (`8.34x`). F1 medians were `226.526/57.147 ms` (`3.96x`) and `64.499/7.719 ms` (`8.36x`). CC coefficient error was max absolute/RMS/relative L2 `1.576e-14/2.211e-16/4.328e-14`; F1 error was `1.566e-14/2.203e-16/4.313e-14`. This substantially narrows the earlier Python-plan gap, but does not establish parity.

The focused T63/T127 regression compares the conversion to the general recurrence and checks the analysis adjoint identity on GL, CC and F1 with odd and even longitude counts. CC cases include minimum sampling (resampling) and `ntheta=2*lmax+2` (no resampling). The release T511 all-grid spin-1/2 DUCC regression passed; spin-1 maximum analysis coefficient error was `4.934e-18` and maximum synthesis sample error was `4.885e-15`. The release T1023 all-grid exact-analysis regression passed; spin-1 maximum analysis coefficient error was `8.824e-18` and maximum synthesis sample error was `1.296e-14`. Sol's strictly read-only review found no correctness blocker and requested the oversampled CC and even-longitude cases, which were added before final validation. Full workspace tests passed (107 Rust tests, 10 f32 tests; eight opt-in tests ignored), no-default-feature tests passed (103 Rust tests, 10 f32 tests; seven ignored), Clippy with warnings denied passed, formatting passed, and all eight Python tests passed. The whole-transform gains justify retaining this path; the remaining T511 Python-plan gap against DUCC is about `4x` at one thread and `8.3x` at eight. No FFT code changed.

### Spin-1 STANDARD analysis mixed-cutoff compaction (2026-10-13)

At `dddee6a`, `analyze_spin_scalar_fast_order()` zeroed Fourier inputs for rings rejected by the polar-order limit, then evaluated a fixed 64-lane recurrence block. The updated path compacts active ring geometry and the already weighted complex Q/U spectra into the same worker's existing scratch for mixed blocks when `lmax >= 255`. It then uses the existing complex-to-real/imaginary staging and paired analysis recurrence. It applies weights at original ring indices before packing, retains the original active-ring order, skips fully excluded blocks, and leaves fully active blocks and lower bandlimits on the prior path. The in-place packing uses at most 64 lanes and adds no workspace allocation. No FFT code changed.

Native f64 spin-1 STANDARD analysis used `examples/m1b_bench.rs`, GL/CC/F1 grids, `phi0=0.37`, full `mmax`, reused plans and workspaces, one warm-up and five timed samples per process. Three release processes per cell were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X running Linux `7.0.14-20-pve`. Rust was `1.99.0 (b940084d7 2026-09-28)` with standard Cargo release settings and runtime AVX2/FMA dispatch; `target-cpu=native` was not enabled. The before source is `dddee6a`; after timings were collected with the final complex-staging implementation. Entries are medians of process medians in milliseconds, paired as one/eight threads; percentages are time reductions.

| Grid | Bandlimit | Before 1T/8T | After 1T/8T | Reduction 1T/8T |
| --- | ---: | ---: | ---: | ---: |
| GL | T255 | `32.503/9.820` | `19.455/6.596` | `40.1%/32.8%` |
| CC | T255 | `38.454/13.533` | `25.285/9.447` | `34.2%/30.2%` |
| F1 | T255 | `38.133/13.504` | `25.373/9.570` | `33.5%/29.1%` |
| GL | T511 | `202.447/46.600` | `113.255/31.042` | `44.0%/33.4%` |
| CC | T511 | `227.714/62.466` | `139.280/47.065` | `38.8%/24.7%` |
| F1 | T511 | `225.597/62.169` | `139.612/46.752` | `38.1%/24.8%` |
| GL | T1023 | `1,185.967/295.426` | `802.793/208.403` | `32.3%/29.5%` |
| CC | T1023 | `1,280.052/367.347` | `919.889/279.005` | `28.1%/24.0%` |
| F1 | T1023 | `1,284.609/368.190` | `923.156/286.503` | `28.1%/22.2%` |

Process-median ranges for the before/after one/eight-thread cells were: GL T255 `32.501–32.728/9.746–10.144` to `19.443–19.583/6.281–6.720 ms`; CC T255 `38.077–38.972/13.276–13.601` to `25.059–25.510/9.387–9.487`; F1 T255 `37.685–38.236/13.466–13.533` to `25.268–25.704/9.562–9.727`. GL T511 `200.619–204.133/46.091–46.622` to `112.745–113.598/31.032–31.155`; CC T511 `227.576–229.565/62.089–64.016` to `138.921–140.485/47.038–47.116`; F1 T511 `222.670–227.840/61.839–63.085` to `138.662–140.153/46.692–46.832`. GL T1023 `1,179.936–1,191.752/294.497–297.117` to `801.887–809.263/207.932–210.798`; CC T1023 `1,279.362–1,286.711/365.519–371.032` to `918.695–924.910/278.922–287.779`; F1 T1023 `1,282.645–1,286.756/366.972–368.722` to `922.290–927.852/285.517–287.496`.

A temporary GL cutoff count measured inactive degree-lanes inside mixed blocks as `2,022/1,031,680` at T127, `403,596/8,486,912` at T255, `4,494,124/64,332,800` at T511 and `24,956,106/475,615,616` at T1023. These lane counts do not explain the full elapsed-time reduction. The diagnostic did not measure per-worker time or the work diverted to the scaled path, so the cause of the additional gain is not isolated. A separate no-compaction control used different scratch staging and is not treated as a matched ablation. The low-band gate prevents compaction below T255; a T63/T127 screen showed no material regression.

The rebuilt Python plan comparison used Python `3.11.15`, NumPy `2.4.6`, and `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, with matching inputs and explicit one/eight-thread settings. Three process medians were collected per cell, each with five timed calls. At T511, Rust/DUCC analysis medians were GL `114.767/25.167` ms (1T) and `33.390/3.630` ms (8T), CC `142.243/55.614` and `50.530/8.051`, and F1 `143.861/56.738` and `50.199/7.767`. Relative L2 coefficient errors were `4.115e-14`, `4.328e-14` and `4.313e-14` for GL/CC/F1. Python input copies were `0.07–0.23 ms`, small beside the transform. These are Python-plan comparisons, not the native timing acceptance table.

The focused mixed-block test compares compacted E/B outputs against an all-active reference with excluded Fourier inputs zeroed; it checks a block containing both active and excluded rings. The all-grid scalar-conversion test covers T63/T127/T255 and checks the general recurrence and analysis adjoint. A T511 GL test compares one- and eight-thread output. Release DUCC checks passed for T511 all-grid spin 1/2 and T1023 all-grid exact spin analysis: spin-1 T511 maximum synthesis sample errors were `4.885e-15` (GL), `4.441e-15` (CC) and `4.219e-15` (F1), with maximum analysis coefficient errors `4.934e-18`, `4.216e-18` and `4.318e-18`; at T1023 they were `1.205e-14/1.296e-14/1.193e-14` and `8.784e-18/8.824e-18/8.538e-18`. The full test suites passed: 108 Rust and 10 f32 tests with all features, 103 Rust and 10 f32 tests without default features, and 8 Python tests. Formatting and Clippy with warnings denied passed. Sol's strictly read-only review found no correctness, indexing, cutoff, scratch-ownership or race blocker. It cautioned against attributing all measured gain to the 5–7% inactive degree-lanes; per-path timing is still needed. Retain this whole-transform improvement and continue profiling spin-0/1 paths. No FFT code changed.

### Paired analysis normal-range status handoff (2026-10-13)

The paired recurrence already determines whether every lane in a latitude block is in its ordinary floating-point range before each visitor call. Scalar and spin-1 analysis now pass that status to the f64 paired accumulator. The SIMD reducer still requires the existing `products_safe` bound; a false status or unsafe Fourier product uses the binary-scaled fallback. This removes the per-degree scan of the exponent array. The older visitor adapts the new status callback and ignores the status for synthesis; f32 declines the f64 paired-analysis path as before. No workspace memory changes.

Native timings used `examples/m1b_bench.rs`, reused plans and workspaces, f64, `phi0=0.37`, full `mmax`, and `nphi=2*lmax+3`, with one warm-up and five timed samples per process. Three process runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X with Linux `7.0.14-20-pve`, `rustc 1.99.0 (b940084d7 2026-09-28)`, Cargo's default release profile (`opt-level=3`, 16 codegen units, LTO disabled) and runtime AVX2/FMA dispatch; `target-cpu=native` was not enabled. The parent source is `b795fe0`; the candidate removes only the exponent scan and passes the recurrence status. Table entries are one/eight-thread medians in milliseconds, with parent followed by candidate:

| Operation | Bandlimit | GL parent | GL candidate | GL reduction |
| --- | ---: | ---: | ---: | ---: |
| Scalar analysis | T63 | `0.187/0.118` | `0.175/0.115` | `6.4%/2.5%` |
| Scalar analysis | T127 | `1.156/0.618` | `1.105/0.600` | `4.4%/2.9%` |
| Scalar analysis | T255 | `10.447/3.506` | `9.607/3.203` | `8.0%/8.6%` |
| Scalar analysis | T511 | `63.795/15.726` | `59.722/14.948` | `6.4%/4.9%` |
| Scalar analysis | T1023 | `391.024/107.120` | `359.586/102.318` | `8.0%/4.5%` |
| Spin-1 STANDARD analysis | T63 | `0.486/0.230` | `0.441/0.234` | `9.3%/-1.7%` |
| Spin-1 STANDARD analysis | T127 | `3.737/1.787` | `3.301/1.782` | `11.7%/0.3%` |
| Spin-1 STANDARD analysis | T255 | `20.338/6.583` | `17.052/6.153` | `16.2%/6.5%` |
| Spin-1 STANDARD analysis | T511 | `113.449/31.355` | `92.162/28.520` | `18.8%/9.0%` |
| Spin-1 STANDARD analysis | T1023 | `800.241/208.272` | `638.046/181.977` | `20.3%/12.6%` |

Each T63 result used seven alternating parent/candidate process runs for scalar analysis and three runs for spin-1. T127–T1023 results used three runs per source. T63 scalar timing ranges overlap at both thread counts. T63 spin-1 and T127 scalar/spin-1 eight-thread ranges overlap; those small changes are inconclusive. The GL ranges at T255 and above do not overlap. T511 CC/F1 results are also parent/candidate medians from three process runs:

| Operation | Grid | Parent 1T/8T (ms) | Candidate 1T/8T (ms) | Reduction 1T/8T |
| --- | --- | ---: | ---: | ---: |
| Scalar analysis | CC | `71.732/22.937` | `67.977/22.464` | `5.2%/2.1%` |
| Scalar analysis | F1 | `73.184/23.597` | `70.007/23.159` | `4.3%/1.9%` |
| Spin-1 STANDARD analysis | CC | `138.022/46.495` | `115.238/43.106` | `16.5%/7.3%` |
| Spin-1 STANDARD analysis | F1 | `138.408/47.278` | `116.869/43.543` | `15.6%/7.9%` |

The CC/F1 spin-1 ranges do not overlap at either thread count. Scalar one-thread ranges are separate; scalar eight-thread ranges overlap. A T511 GL synthesis control compared scalar STANDARD, spin-1 STANDARD, spin-1 GRAD_ONLY and scalar DERIV1. Parent/candidate medians were `65.995/22.791` to `65.957/22.825`, `88.589/31.734` to `89.287/31.980`, `89.350/32.035` to `90.200/32.689`, and `90.558/32.868` to `90.582/33.210` ms, respectively. Seven alternating process runs for spin-1 STANDARD and three runs for the other controls show overlapping ranges; the synthesis visitor wrapper has no measured material effect.

Temporary counters weighted each recurrence visit by its one- or two-degree contribution and lane count. At T63/T127/T255/T511/T1023 GL, normal-range work was 100/84.5/75.0/84.0/90.6% for scalar analysis and 44.7/84.2/90.5/95.6/96.7% for spin-1 analysis. At T511 CC and F1, the fractions were 84.5% for scalar and 96.1% for spin-1. No measured callback had zero exponents with a false normal-range status. These counts describe recurrence work, not elapsed time; the diagnostic code was removed after measurement.

Callgrind 3.26.0 counted 12 T127 GL analysis transforms across the one-thread and one-worker pools for both builds. Scalar instruction references fell from `267,336,575` to `248,350,091` (`-7.1%`); spin-1 references fell from `821,843,278` to `744,793,486` (`-9.4%`). These simulated instruction references are not cycle counts. `perf stat` remains unavailable because `perf_event_paranoid=4`.

The extreme-finite-input reducer test, scaled-lane fallback test, paired recurrence transition reference, spin-1 STANDARD general-path comparison and analysis adjoint tests passed. Release DUCC 0.41.0 references from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` passed for scalar T511/T1023/T2047 on GL/CC/F1 and spin-1/2 T511/T1023 on GL/CC/F1. Maximum spin-1 analysis coefficient error was `8.824e-18` at T1023; maximum spin-2 analysis error was `2.558e-17` at T511 and `8.491e-18` at T1023. Full checks passed: `cargo fmt --all -- --check`, workspace Clippy with all targets/features and warnings denied, 108 Rust and 10 f32 tests with all features, 103 Rust and 10 f32 tests without default features, and 8 Python tests. Sol's read-only review found no correctness blocker and confirmed the status invariant, f32 fallback, scaled path and synthesis adapter. No FFT code changed.

### Rejected scalar-only split analysis reducer (2026-10-13)

A second independent SIMD accumulation chain was prototyped for scalar analysis while the original non-generic spin-conversion reducer was restored verbatim. The scalar prototype used paired vector chunks, two sets of partial sums, and one final horizontal reduction; unsafe products and scaled lanes delegated to the existing binary-scaled reducer. This added a separate SIMD implementation without changing workspace memory. The preceding const-generic prototype had improved scalar analysis in some runs but regressed spin-1 by about 5–6% at one thread; Sol's read-only review recommended this isolated final test and warned that equal function size did not establish equal generated code.

The isolated candidate was measured against `05e0a50` with `examples/m1b_bench.rs`, f64 GL analysis, `phi0=0.37`, full `mmax`, reused plans/workspaces, one warm-up and five timed samples per process, three alternating process runs per source, pinned CPUs 0–7 on the Ryzen 9 5950X running Linux `7.0.14-20-pve`, `rustc 1.99.0 (b940084d7 2026-09-28)`, Cargo release settings (`opt-level=3`, 16 codegen units, LTO off) and runtime AVX2/FMA dispatch. Timings are medians of process medians in milliseconds, one/eight threads:

| Operation | Bandlimit | Baseline | Isolated candidate | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar analysis | T511 | `60.124/15.008` | `59.940/15.321` | `-0.3%/+2.1%` |
| Spin-1 STANDARD analysis | T511 | `91.378/28.317` | `91.603/28.221` | `+0.2%/-0.3%` |
| Scalar analysis | T1023 | `360.624/101.912` | `357.775/101.873` | `-0.8%/0.0%` |
| Spin-1 STANDARD analysis | T1023 | `632.416/179.894` | `637.082/181.656` | `+0.7%/+1.0%` |

The process ranges overlapped for the small isolated-candidate differences. The final source separation returned spin timing to baseline variability, but the scalar gains also disappeared. The generic version's apparent scalar improvement was not worth its measured spin regression. Focused reducer-reference, scaled-lane and extreme-finite-input tests passed during the experiment; those robustness tests already exist on the baseline. The prototype was removed. No numerical tolerance or public behavior changed, no workspace bytes were added, and no FFT code changed. Sol's independent review correctly predicted that only this isolated variant should be considered; the measured lack of repeatable scalar gain rejects it.

### Scalar analysis mixed-pair compaction (2026-10-13)

`analyze_symmetric_order_group()` now packs active north/south latitude pairs from mixed polar-cutoff blocks for f64 `lmax >= 255`. It tests each hemisphere against its own order limit, zeros excluded Fourier values before forming the even and odd parity sums, and packs the corresponding northern sine and cosine values in the same order. The recurrence and product-safety test use the compact lane count. CC/F1 prepared spectra are already weighted before this step. Fully active blocks, f32, and bandlimits below T255 use the existing full block. The compaction reuses per-worker buffers capped at 64 lanes and adds no workspace allocation.

Temporary lane counters found, at T511 GL, 33.62 million potential pair-degree lanes, 1.58 million fully skipped lanes, and 2.25 million inactive lanes inside mixed blocks. At T1023 GL the counts were 268.70, 31.40 and 12.48 million. Before compaction, mixed-block scaled work was 4.330 million lanes at T511 and 18.667 million at T1023. After packing, it was 0.459 and 3.643 million. The packed active lanes exposed more mixed blocks to normal-range SIMD recurrence. The counters measure executed lane-degree work; they do not isolate the time saved by ring exclusion from the time saved by the resulting normal-range path. The temporary instrumentation was removed.

Native analysis timings used `examples/m1b_bench.rs`, f64 full-band coefficients, `phi0=0.37`, reused plans and workspaces, one warm-up and five timed samples per process, with three process runs for each result. Processes were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X running Linux `7.0.14-20-pve`; Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo release used `opt-level=3`, 16 codegen units and LTO off, and runtime AVX2/FMA dispatch was enabled without `target-cpu=native`. The baseline was measured immediately before the prototype at `78288bb`. Values are medians of process medians in milliseconds, one/eight threads:

| Grid / bandlimit | Baseline | Compacted and inlined | Reduction |
| --- | ---: | ---: | ---: |
| GL T511 | `58.100/15.078` | `32.244/10.644` | `44.5%/29.4%` |
| GL T1023 | `347.760/100.050` | `233.807/75.101` | `32.8%/24.9%` |
| CC T511 | `67.270/22.382` | `40.260/17.617` | `40.2%/21.3%` |
| F1 T511 | `67.379/22.994` | `41.397/18.072` | `38.6%/21.4%` |

Final process-median ranges were GL T511 `32.159–32.251/10.614–10.736 ms`, GL T1023 `233.573–234.249/74.582–76.634 ms`, CC T511 `39.876–40.569/17.470–17.721 ms`, and F1 T511 `40.534–42.790/17.812–18.679 ms`. An exploratory T511 GL process during the first candidate series measured `69.729/19.607 ms`; it did not match the repeated confirmation runs and is excluded from the table. A first helper extraction compiled out of line and measured T1023 at `261.463/84.750 ms`; `#[inline]` removed the separate helper symbol and produced the reported timings. T63 and T127 screens after this change were `0.175/0.129` and `1.074/0.594 ms`; the compaction gate was disabled. Their comparisons with earlier measurements were not alternating controls. T255 was also a screen, with `5.683/2.159 ms` versus `9.697/3.448 ms` from a separate baseline sweep. The T63/T127/T255 results are not used as matched acceptance claims.

The focused helper test compares packed parity sums and geometry with the full cutoff-masked block for north-only, south-only and two-sided active pairs; it also checks an odd-grid equatorial singleton. The T255 GL regression compares selected coefficients with DUCC 0.41.0 and checks analysis-adjoint identities. Release scalar DUCC and analytic checks passed at T511, T1023 and T2047 on GL/CC/F1; the maximum T2047 DUCC sample error was `2.083e-13` and maximum analytic map error was `9.958e-12`. One/eight-thread T255 analysis results differed by less than `4e-13`. Sol's strictly read-only review found no mathematical, cutoff, packing, ownership or aliasing blocker; the requested mixed-pair and equatorial cases are covered by the focused test. Final checks passed: 109 Rust and 10 f32 tests with all features; 104 Rust and 10 f32 tests without default features; workspace Clippy with all targets/features and warnings denied; formatting and `git diff --check`; and all eight Python tests. This result applies to scalar analysis only. The next profile targets spin-0 and spin-1 operations at T63–T1023, including spin-1 STANDARD, GRAD_ONLY and DERIV1 on GL, CC and F1. No FFT code changed.

### Skip spin-1 orders excluded from an entire synthesis segment (2026-10-13)

At the start of each spin-1 latitude segment, synthesis now finds the largest per-ring polar order limit and skips orders above it before coefficient conversion, factor preparation, and recurrence setup. The spectra are zeroed by the synthesis driver, and every ring in a skipped segment excludes those orders. Spin 2 and the general per-ring cutoff are unchanged. The guard adds no workspace memory.

Native f64 timings used `examples/m1b_bench.rs` with reused plans and workspaces, `phi0=0.37`, full `mmax`, and one warm-up plus five timed samples per process. Three process runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X with Linux `7.0.14-20-pve`, `rustc 1.99.0 (b940084d7 2026-09-28)`, Cargo release settings (`opt-level=3`, 16 codegen units, LTO off), and runtime AVX2/FMA dispatch without `target-cpu=native`. The parent is `9e66fe1`; values are process medians in milliseconds, one/eight threads:

| Grid / mode | Parent | Segment skip | Reduction |
| --- | ---: | ---: | ---: |
| CC T511 STANDARD synthesis | `99.281/42.338` | `90.766/38.720` | `8.6%/8.5%` |
| CC T511 GRAD_ONLY synthesis | `99.953/42.803` | `91.379/38.867` | `8.6%/9.2%` |
| CC T511 DERIV1 synthesis | `100.724/43.746` | `93.386/39.940` | `7.3%/8.7%` |

The exact CC pole rows are processed as single-ring general-path segments. The guard removes repeated setup and recurrence visits for orders excluded at those rows. Matched GL and F1 T511 runs showed no resolved change across the three modes. GL T1023 before/after medians also overlapped for STANDARD, GRAD_ONLY and DERIV1; this guard has no measured GL T1023 gain. Lower-band T63–T255 measurements were screening runs without matched parent processes and are not used as acceptance results. A separate scalar segment-order guard was tested and removed because it produced no resolved whole-transform improvement.

The direct CC pole regression sets nonzero Fourier sentinels and verifies that the new guard returns before the previous per-ring cutoff path reaches its phase and scaling epilogue. A public T511 regression covers STANDARD, GRAD_ONLY and DERIV1 on GL, CC and F1, checks zero output on excluded rings, and checks nonzero output on active rings. Sol's strictly read-only review found no issue in the sentinel test or its production slices. Release DUCC `0.41.0` tests from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` passed for T511 spin 1/2 on GL, CC and F1, T1023 sparse spin 1/2 on all three grids, and sparse T2047 spin 1–4 synthesis with analytic sectoral checks. T2047 maximum spin-1 DUCC sample error was `2.137e-13`; maximum analytic map error was `1.344e-11`. No FFT code changed.

Callgrind 3.26.0 on the T255 GL spin-1 GRAD_ONLY benchmark counted `5.139` billion instruction references across the one- and eight-thread harness runs. `scalar::multiply_complex_scaled_product` accounted for `1.350` billion self references (26.3%); inclusive inverse-FFT references were 5.6%. These simulated references are not cycles or elapsed phase times. `perf_event_paranoid=4` still blocks hardware counters. The scalar-scaled multiplication path is the next profiling target for spin-0/1 recurrence and accumulation; FFT work is deferred.

Final checks passed: formatting, workspace Clippy with warnings denied, 111 Rust tests and 10 f32 tests with all features (8 ignored), 106 Rust tests and 10 f32 tests without default features (7 ignored), and all 8 Python tests. The high-bandlimit release DUCC tests listed above passed. Sol's strictly read-only review found no numerical or benchmark blocker; it confirmed that the CC sentinel exercises the segment guard and that the GL/F1 public cases test cutoff output without requiring that guard branch.

### Spin-1 synthesis transition gate at T224 (2026-10-13)

Lowered `SPIN1_TRANSITION_MIN_LMAX` from 255 to 224. This enables the existing guarded scaled-to-normal continuation in the spin-1 scalar-conversion synthesis path at T224. The recurrence, finite-output checks, scaled fallback and worker scratch are unchanged. T63, T127 and T223 remain below the gate; T255 and higher were already eligible. No scalar, spin-2, f32 or FFT path changed.

Native timings used `examples/m1b_bench.rs` with reused plans/workspaces, f64, `phi0=0.37`, full `mmax`, `nphi=2*lmax+3`, one warm-up and five timed calls per process. Three alternating baseline/candidate process runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X, Linux `7.0.14-20-pve`, using `rustc 1.99.0 (b940084d7 2026-09-28)`, Cargo release defaults (`opt-level=3`, 16 codegen units, LTO off) and runtime AVX2/FMA dispatch without `target-cpu=native`. The baseline is `6addcfb`; candidate production code differs only by the threshold constant. Each cell reports one/eight-thread process medians in milliseconds:

| Grid / mode | T224 baseline → candidate | Reduction | T254 baseline → candidate | Reduction |
| --- | ---: | ---: | ---: | ---: |
| GL STANDARD | `14.516/8.518 → 14.169/7.893` | `2.4%/7.3%` | `24.938/12.027 → 20.374/9.566` | `18.3%/20.5%` |
| GL GRAD_ONLY | `15.239/8.754 → 14.620/8.062` | `4.1%/7.9%` | `26.055/12.229 → 20.756/9.887` | `20.3%/19.2%` |
| GL DERIV1 | `15.002/8.832 → 14.563/8.268` | `2.9%/6.4%` | `25.726/12.310 → 20.521/9.637` | `20.2%/21.7%` |
| CC STANDARD | `16.592/9.326 → 15.196/8.209` | `8.4%/12.0%` | `26.983/13.179 → 23.205/11.271` | `14.0%/14.5%` |
| CC GRAD_ONLY | `16.700/9.556 → 15.053/8.342` | `9.9%/12.7%` | `27.519/13.200 → 23.871/11.084` | `13.3%/16.0%` |
| CC DERIV1 | `17.413/9.777 → 15.323/8.587` | `12.0%/12.2%` | `27.621/13.349 → 23.805/11.147` | `13.8%/16.5%` |
| F1 STANDARD | `15.389/9.095 → 13.189/7.446` | `14.3%/18.1%` | `25.929/12.696 → 21.247/9.850` | `18.1%/22.4%` |
| F1 GRAD_ONLY | `15.692/9.226 → 13.246/7.570` | `15.6%/17.9%` | `26.591/12.432 → 21.684/10.171` | `18.5%/18.2%` |
| F1 DERIV1 | `15.811/9.375 → 13.307/7.672` | `15.8%/18.2%` | `26.517/12.605 → 21.570/9.986` | `18.7%/20.8%` |

At T224 GL, baseline and candidate process-median ranges did not overlap in any mode or thread count; for STANDARD they were `14.345–14.654/8.393–8.528` and `13.930–14.180/7.824–8.090` ms. T223 GL controls for STANDARD, GRAD_ONLY and DERIV1 changed by at most 0.7%, within run variability. The T224 gain is smaller on GL than CC/F1, while T254 shows clear whole-transform reductions on all grids and modes.

The candidate passed all-grid T224 spin-1 STANDARD and GRAD_ONLY spectrum comparisons against the general recurrence, and DERIV1 comparisons against the general recurrence with analytic `Y10`/`Y11`, zero-monopole and synthesis-adjoint checks for even and odd `nphi`. Against installed DUCC `0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, T224 synthesis relative-L2 errors across GL/CC/F1, STANDARD/GRAD_ONLY/DERIV1 and one/eight threads were `2.35e-14–3.37e-14`; the largest absolute error was `6.564e-13` in DERIV1. Focused Rust tests passed. Sol's strictly read-only review found no correctness or cutoff blocker; it recommended retaining the gate and treating the smaller GL gain cautiously. No FFT code changed.

### Scalar analysis active-pair compaction crossover (2026-10-13)

Lowered the scalar f64 analysis active-pair compaction gate from T255 to T127. The path packs only north/south ring pairs admitted by their separate polar order limits, retaining the existing parity, equatorial singleton and worker-scratch behavior. No additional workspace memory is used. T63 remains on the uncompressed path. A T191 GL/CC/F1 regression checks selected coefficients against DUCC 0.41.0 and verifies the analysis-adjoint identity.

Native f64 results use reused plans/workspaces, `examples/m1b_bench.rs`, `phi0=0.37`, full `mmax`, one warm-up and five timed calls per process, and pinned CPUs 0–7 on the Ryzen 9 5950X / Linux `7.0.14-20-pve`. Builds used `rustc 1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off, and runtime AVX2/FMA dispatch without `target-cpu=native`. Parent and candidate processes were alternated. T127 used seven process runs; T191 used three. Values are medians of process medians in milliseconds, one/eight threads:

| Grid / bandlimit | Parent | Compacted | Reduction |
| --- | ---: | ---: | ---: |
| GL T127 | `1.110/0.592` | `1.054/0.533` | `5.0%/10.0%` |
| CC T127 | `1.554/1.017` | `1.502/0.951` | `3.3%/6.5%` |
| F1 T127 | `1.563/1.035` | `1.508/0.974` | `3.5%/5.9%` |
| GL T191 | `4.013/1.456` | `2.909/1.011` | `27.5%/30.6%` |
| CC T191 | `4.984/2.511` | `3.825/2.098` | `23.3%/16.4%` |
| F1 T191 | `4.947/2.649` | `3.779/2.076` | `23.6%/21.6%` |

T127 median gains were consistent across seven runs, though their timing ranges overlap; T191 ranges are clearly separated. T63 was below the gate, and T255 remained on its prior code path. Focused and full Rust tests passed. DUCC Python comparisons at T127/T191 across GL/CC/F1 and one/eight threads, including full-array inputs, had maximum relative-L2 error `1.85e-14` and maximum absolute error `8.85e-15`. The independent read-only Sol review found no actionable correctness, indexing, cutoff or ownership issue; it noted that the small embedded coefficient fixture is sparse, while full-array comparisons and mixed/equatorial tests provide broader coverage. No FFT code changed.

### Spin-1 analysis active-pair compaction crossover (2026-10-13)

Lowered the f64 spin-1 STANDARD scalar-conversion analysis compaction gate from T255 to T127. The implementation keeps each hemisphere's polar-order mask, ring weighting and prepared CC/F1 spectra, packs active lanes in order into existing per-worker scratch, and repeats product-safety selection using the compact lane count. f32, GRAD_ONLY/DERIV1 and spin-2 paths are unchanged. The all-grid general-path and adjoint regression now includes T191 in addition to T63/T127/T255.

Native timing conditions match the scalar analysis compaction experiment above: `examples/m1b_bench.rs`, reused f64 plans/workspaces, `phi0=0.37`, full `mmax`, one warm-up and five timed calls, processes pinned to CPUs 0–7 on Ryzen 9 5950X / Linux `7.0.14-20-pve`, `rustc 1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off, runtime AVX2/FMA and no `target-cpu=native`. Three alternating process runs were used per source; values are medians of process medians in milliseconds, one/eight threads:

| Grid / bandlimit | Parent | Compacted | Reduction |
| --- | ---: | ---: | ---: |
| GL T127 | `3.301/1.937` | `3.084/1.515` | `6.6%/21.8%` |
| CC T127 | `4.653/2.600` | `4.384/2.361` | `5.8%/9.2%` |
| F1 T127 | `4.689/2.747` | `4.419/2.338` | `5.8%/14.9%` |
| GL T191 | `12.370/4.494` | `8.337/3.050` | `32.6%/32.1%` |
| CC T191 | `15.528/6.761` | `11.502/4.962` | `25.9%/26.6%` |
| F1 T191 | `15.608/6.823` | `11.663/5.122` | `25.3%/24.9%` |

T63 remains below the gate; T255 was an unchanged-path control. T127 process ranges overlap for some cases, while all T191 candidate ranges are materially lower. The whole-transform gains reflect changed block eligibility for normal-range SIMD as well as fewer active lanes, and are not attributed only to lane removal. The focused candidate tests passed on all grids, comparing E/B conversion with the general recurrence and checking the analysis-adjoint identity. DUCC 0.41.0 full-array comparisons at T127/T191 on GL/CC/F1 and one/eight threads had maximum relative-L2 error `2.032e-14` and maximum absolute error `9.139e-15`. The independent read-only Sol review found no cutoff, indexing, weighting, product-safety, ownership or test blocker and supported acceptance. No FFT code changed.

### Spin-1 scaled-to-normal continuation gate at T191 (2026-10-13)

Lowered `SPIN1_TRANSITION_MIN_LMAX` from 224 to 191. This admits the existing guarded scaled-to-normal continuation for spin-1 scalar-conversion synthesis in STANDARD, GRAD_ONLY and DERIV1 from T191. The polar sine limit, paired recurrence, prefix and late-state checks, finite-output preflight, scaled fallback and per-worker scratch are unchanged. T190 and lower remain below the gate; T224 and higher were already eligible. No scalar, f32, spin-2 or FFT code changed.

The native measurements used `examples/m1b_bench.rs`, f64, full `mmax`, `phi0=0.37`, reused plan/workspace, one warm-up and five timed samples per process, and three alternating baseline/candidate process pairs. Runs were pinned to CPUs 0–7 on a Ryzen 9 5950X with Linux `7.0.14-20-pve`; Rust was `1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off, runtime AVX2/FMA and no `target-cpu=native`. Baseline is `a21b8ac`; table cells are process-median milliseconds, one/eight threads:

| Bandlimit / grid / mode | Baseline | Gate 191 | Reduction |
| --- | ---: | ---: | ---: |
| T191 GL STANDARD | `10.977/5.737` | `11.019/5.582` | `-0.4%/2.7%` |
| T191 GL GRAD_ONLY | `11.227/5.792` | `11.264/5.867` | `-0.3%/-1.3%` |
| T191 GL DERIV1 | `11.416/5.917` | `11.372/5.978` | `0.4%/-1.0%` |
| T191 CC STANDARD | `12.477/6.004` | `11.482/5.628` | `8.0%/6.3%` |
| T191 CC GRAD_ONLY | `12.587/6.175` | `11.613/5.761` | `7.7%/6.7%` |
| T191 CC DERIV1 | `12.979/6.454` | `11.977/5.920` | `7.7%/8.3%` |
| T191 F1 STANDARD | `11.928/6.108` | `10.106/5.149` | `15.3%/15.7%` |
| T191 F1 GRAD_ONLY | `12.103/6.205` | `10.271/5.369` | `15.1%/13.5%` |
| T191 F1 DERIV1 | `12.242/6.304` | `10.409/5.463` | `15.0%/13.3%` |
| T208 GL STANDARD | `12.700/8.096` | `11.979/7.888` | `5.7%/2.6%` |
| T208 GL GRAD_ONLY | `12.664/8.303` | `12.086/8.064` | `4.6%/2.9%` |
| T208 GL DERIV1 | `12.803/8.496` | `12.229/8.105` | `4.5%/4.6%` |
| T208 CC STANDARD | `14.018/8.805` | `12.909/8.081` | `7.9%/8.2%` |
| T208 CC GRAD_ONLY | `14.346/8.999` | `13.014/8.159` | `9.3%/9.3%` |
| T208 CC DERIV1 | `14.524/9.115` | `13.104/8.287` | `9.8%/9.1%` |
| T208 F1 STANDARD | `12.988/8.480` | `11.056/7.242` | `14.9%/14.6%` |
| T208 F1 GRAD_ONLY | `13.244/8.711` | `11.250/7.403` | `15.1%/15.0%` |
| T208 F1 DERIV1 | `13.314/8.753` | `11.387/7.518` | `14.5%/14.1%` |

The three T208 process ranges were separate for every grid/mode/thread result. T191 GL is within measurement noise; CC/F1 improve in all modes. A separate T127 screen lowering the gate all the way to 127 was rejected: it slowed all nine GL/CC/F1 mode medians at one and eight threads by 12.7–35.9%, so T127 remains on the previous fallback. Focused tests compare T190 (below gate), T191 and T208 (inside the newly enabled interval), and T224 across GL/CC/F1. STANDARD E-only, B-only, mixed and GRAD_ONLY spectra are checked against the general recurrence; DERIV1 also checks even/odd longitude, analytic low-degree fields and the synthesis-adjoint identity. Direct Python-plan comparisons with `ducc0==0.41.0`, T191/T208 GL/CC/F1, all three modes and one/eight threads had maximum relative-L2 errors `2.868e-14` and `3.223e-14`, respectively; maximum absolute errors were `2.998e-13` and `4.547e-13` (DERIV1). The full Rust suite, focused expanded tests, Python suite, formatting and whitespace checks passed. Sol's strictly read-only review found no correctness blocker and recommended screening an interior band; T208 was then measured and showed gains across all grids and modes. Existing T511/T1023 spin-1 validation remains on the previously enabled path. No FFT code changed.

### Scalar synthesis continuation at T1023 (2026-10-13)

Enabled the existing paired scalar scaled-to-normal continuation for f64 synthesis from T1023. It runs after the register-resident kernel declines and only for fully active latitude blocks. Orders whose transformed paired coefficients are non-finite use the unpaired binary-scaled recurrence. This fallback is needed because a finite input such as `f64::MAX` can overflow during the paired coefficient transformation even when the final scaled contribution is finite. The T1023 block regression checks that continuation declines for this case and that both `f64::MAX` and the smallest subnormal produce the same spectra as the unpaired scaled reference. The change adds no workspace allocation.

Release measurements used `examples/m1b_bench.rs`, f64 scalar synthesis, full order range, `phi0=0.37`, `nphi=2*lmax+3`, reused plans and workspaces, one warm-up and five timed calls per process. Three alternating baseline/candidate process runs used CPUs 0–7 on an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`, `rustc 1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off, and runtime AVX2/FMA dispatch without `target-cpu=native`. The baseline is `e0eb143`; values are medians of process medians in milliseconds, one/eight threads:

| Grid / bandlimit | Baseline | T1023 continuation | Reduction |
| --- | ---: | ---: | ---: |
| GL T511 control | `65.914/22.902` | `65.582/22.584` | `0.5%/1.4%` |
| GL T1023 | `529.510/142.537` | `440.693/129.575` | `16.8%/9.1%` |
| CC T1023 | `524.844/195.016` | `439.239/165.807` | `16.3%/15.0%` |
| F1 T1023 | `527.728/141.923` | `438.364/129.918` | `16.9%/8.5%` |

The GL T511 source paths are unchanged and the process ranges overlap. T1023 candidate ranges were GL `440.493–441.737/129.187–130.807`, CC `436.825–443.176/165.415–166.786`, and F1 `437.857–442.034/129.231–130.446` ms; baseline ranges were GL `527.476–531.335/142.312–144.478`, CC `524.760–526.426/194.752–199.503`, and F1 `523.564–528.863/141.559–144.496` ms. The coefficient safety check is included in candidate timings.

The focused T1023 extreme-coefficient test and the release scalar GL/CC/F1 DUCC `0.41.0` and analytic checks through T2047 pass. At T1023, the largest selected DUCC sample error was `1.257e-14`; the largest analytic map error was `4.132e-12`. These comparisons cover the normal-coefficient path; the `f64::MAX` and subnormal cases are checked against the scaled scalar reference. Sol's strictly read-only review found no numerical, workspace, or benchmark blocker. No FFT code changed.

### Spin-1 analysis active-ring weighted scheduling (2026-10-13)

`analyze_spin_order_groups_parallel()` previously split each recursive order interval using only degree count. The split now estimates order work as `(lmax - m + 1) * active_ring_count(m)`, where the active count uses the prepared `ring_order_limits` for the analysis rows. It uses the existing recursive Rayon split, coefficient slices and per-branch recurrence workspaces. The extra scan runs only at `lmax >= 255`; T63 and T127 retain the previous split. The estimate uses O(1) scratch and does not compact or copy spectra.

The comparison used `examples/m1b_bench.rs`, f64 spin-1 STANDARD analysis, GL/CC/F1, full `mmax`, `phi0=0.37`, `nphi=2*lmax+3`, reused plans and workspaces, one warm-up and five timed calls per process. Baseline commit `0b1af0c` and the candidate used release `opt-level=3`, 16 codegen units, LTO off, runtime AVX2/FMA dispatch and no `target-cpu=native`. Runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`, with `rustc 1.99.0 (b940084d7 2026-09-28)`. T255 uses three alternating baseline/candidate process pairs; T511 and T1023 use two pairs. Values are medians of process medians in milliseconds, one/eight threads:

| Grid / bandlimit | Baseline | Active-ring split | 8-thread change |
| --- | ---: | ---: | ---: |
| GL T255 | `16.816/6.174` | `16.802/5.610` | `-9.1%` |
| CC T255 | `22.067/8.969` | `21.911/8.826` | `-1.6%` |
| F1 T255 | `22.627/9.413` | `22.391/9.057` | `-3.8%` |
| GL T511 | `90.842/27.751` | `91.420/23.769` | `-14.4%` |
| CC T511 | `114.409/43.064` | `114.929/38.979` | `-9.5%` |
| F1 T511 | `115.265/43.169` | `114.960/39.769` | `-7.9%` |
| GL T1023 | `635.725/178.818` | `629.648/165.386` | `-7.5%` |
| CC T1023 | `744.581/251.378` | `738.664/234.409` | `-6.8%` |
| F1 T1023 | `746.969/257.099` | `743.358/237.522` | `-7.6%` |

At T511/T1023 all one-thread ranges overlap and all eight-thread ranges are separated. The absolute eight-thread reductions at T511 are `3.98`, `4.08` and `3.40 ms` for GL, CC and F1; at T1023 they are `13.43`, `16.97` and `19.58 ms`. T255 GL ranges are separated. T255 CC ranges overlap; its small median change is inconclusive. T255 F1 ranges are separated but remain a screening result. The T511/T1023 eight-thread baseline/candidate ranges were GL `27.707–27.795/23.595–23.942 ms`, `178.594–179.041/164.969–165.803 ms`; CC `42.894–43.233/38.558–39.400 ms`, `250.877–251.879/234.189–234.628 ms`; and F1 `42.816–43.522/39.553–39.984 ms`, `256.610–257.588/237.278–237.766 ms`. The change leaves numerical operations within each order unchanged and adds no memory.

The midpoint helper test, spin-1 fast/general all-grid analysis and adjoint test, spin adjoint identities, and spin-3/4 regression passed. Opt-in release checks `m3_spin_t511_all_grids_matches_ducc_and_roundtrips` and `m3_spin_t1023_exact_analysis_matches_ducc` passed against static DUCC 0.41.0 references from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Formatting, workspace Clippy with warnings denied, all-feature workspace tests (114 Rust and 10 f32 passed; 8 ignored), and no-default-feature Rust tests (108 Rust and 10 f32 passed; 7 ignored) passed. Sol's strictly read-only review found no correctness, cutoff-data, feature-gating, overflow, slice-ownership or workspace blocker. It noted that spin adjoints and spins 2–4 share this scheduler, so the measured gains here apply only to spin-1 STANDARD analysis. The subsequent leaf timing and path-status profiles below found nearly equal active-work estimates but substantial per-leaf time variation; they did not establish another beneficial split formula. No FFT code changed.

### Spin-1 group timing profile and rejected suffix clearing (2026-10-13)

A temporary, release-only diagnostic in a worktree based on `a25d893` measured each recursive spin analysis leaf on one warmed invocation using eight threads. It recorded estimated active degree-lanes and leaf elapsed time; it did not instrument inner recurrence loops. Conditions were the same Ryzen 9 5950X, Linux `7.0.14-20-pve`, Rust 1.99.0, release `opt-level=3`, 16 codegen units, LTO off, AVX2/FMA runtime dispatch, CPUs 0–7, and `examples/m1b_bench.rs` geometry. Across the eight leaves, T511 GL estimates ranged 7.35–7.64 million degree-lanes while elapsed times ranged 8.4–16.5 ms; CC ranged 7.38–7.53 million and 10.0–23.8 ms; F1 ranged 7.38–7.53 million and 9.7–17.7 ms. At T1023, GL ranged 56.04–56.76 million and 62.6–121.9 ms; CC ranged 55.87–56.69 million and 78.2–115.2 ms; F1 ranged 55.87–56.69 million and 78.5–129.0 ms. Near-equal active-work estimates therefore do not explain the 1.5–2x leaf-time spread. This diagnostic does not identify its cause: it did not count fast, scaled and general recurrence work or separate path time. Further scheduler weighting is deferred until those path/status counts are measured.

A separate temporary candidate on the same base removed full-row clearing in spin-1 coefficient conversion and narrowed scalar-analysis output clearing to the degree range written. Focused all-grid general-path, adjoint, f32, T127 DUCC and T511/T1023 release DUCC tests passed. The candidate produced no repeatable whole-transform synthesis improvement and slowed STANDARD analysis: T511 GL one/eight-thread medians changed from `90.724/24.603` to `96.010/24.292` ms; T1023 changed from `631.271/165.307` to `667.108/170.565` ms. Other synthesis and derivative-mode differences were within timing variation. Disassembly confirmed that the candidate removed two full-row `memset` calls, but this did not translate into a net transform gain. It is rejected and no source change was retained. The read-only Sol review also noted that DERIV1's finiteness guard scans the full row, so stale non-finite prefix values could unnecessarily reject the fast path; any future partial-write design must address that guard. This experiment used no FFT changes.

### Spin-1 STANDARD analysis factor setup and path profile (2026-10-13)

When spin-1 scalar-conversion analysis covers every latitude, the scalar paired recurrence overwrites the per-order scalar factors before use. The general polar recurrence has no rows to process in that case. The analysis now calls the paired scalar helper directly and skips the otherwise redundant spin-factor initialization. It advances the packed E/B coefficient offset before continuing, preserving disjoint output slices in recursive Rayon leaves. The branch applies only when the recurrence spin is one and the fast range is exactly the full latitude range. CC/F1 exact grids retain their general polar path at the pole endpoints.

A temporary release diagnostic on `a25d893` counted cutoff-active and normal-range paired lanes for f64 spin-1 STANDARD analysis on GL, CC and F1 at T63, T127, T255, T511 and T1023. It also timed the paired recurrence/accumulation, general polar recurrence, active-lane gather, factor setup and scalar-to-spin coefficient conversion around complete worker leaves. The corrected counters use the same Q/U product-safety and normal-range predicates as production dispatch. The lane percentages below are degree-work estimates, not elapsed-time fractions; a mixed block compacts inactive rings before recurrence.

| Bandlimit | Cutoff-active lane work, GL / CC-F1 | Scaled lanes in paired fast path, GL / CC-F1 | Mixed cutoff blocks, GL / CC-F1 |
| ---: | ---: | ---: | ---: |
| T63 | `100% / 100%` | `0% / 0%` | `0 / 0` |
| T127 | `99.8% / 99.8%` | `13.19% / 10.17%` | `48 / 46` |
| T255 | `95.3% / 95.1%` | `9.49% / 8.09%` | `304 / 302` |
| T511 | `88.7% / 88.5%` | `4.37% / 3.89%` | `808 / 805` |
| T1023 | `83.7% / 83.6%` | `3.32% / 3.19%` | `1816 / 1813` |

At T1023, the general polar recurrence is scaled for `98.28%` of its CC/F1 lane-degree work, but that path covers only the small sine-polar region. The paired fast path is normal-range for about `97%` of its work. This profile therefore does not support another scaled-to-normal transition in spin-1 STANDARD analysis. The existing mixed-block compaction already excludes inactive rings. On four diagnostic runs, worker-leaf elapsed-time sums for paired recurrence and accumulation were `15.4–15.9 ms` at T255, `83–85 ms` at T511 and `580/597/604 ms` for GL/CC/F1 at T1023. At T1023, general-polar recurrence summed to approximately `0/40.4/40.9 ms`, active-lane gather to `16.2/16.5/16.6 ms`, spin-factor setup to `14.7–14.9 ms`, and scalar-to-spin coefficient conversion to `8.2–8.4 ms`. These are sums of instrumented leaf intervals, not whole-transform elapsed times. They show paired recurrence and accumulation dominate this path; the profile does not establish that further compaction or recurrence transitions would improve other operations.

The factor-setup change was compared with `de0db66` using `examples/m1b_bench.rs`, f64 spin-1 STANDARD analysis, reused plans/workspaces, full `mmax`, `phi0=0.37`, `nphi=2*lmax+3`, one warm-up and five samples per process. Five alternating process medians were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X, Linux `7.0.14-20-pve`, with `rustc 1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO disabled and runtime AVX2/FMA dispatch. Values are one/eight-thread milliseconds:

| Grid / bandlimit | Before | After | Median change |
| --- | ---: | ---: | ---: |
| GL T63 | `0.436/0.249` | `0.384/0.243` | `-11.9%/-2.4%` |
| GL T127 | `3.104/1.603` | `2.892/1.444` | `-6.8%/-9.9%` |
| GL T255 | `17.301/5.629` | `16.345/5.628` | `-5.5%/0.0%` |
| GL T511 | `91.579/24.191` | `88.995/23.423` | `-2.8%/-3.2%` |
| GL T1023 | `630.459/166.880` | `619.225/166.108` | `-1.8%/-0.5%` |
| CC T511 control | `116.528/41.363` | `115.273/41.593` | `-1.1%/+0.6%` |
| F1 T511 control | `115.464/40.413` | `115.987/40.423` | `+0.5%/0.0%` |

The GL one-thread process ranges are separated at T63, T127, T255 and T1023; the T511 ranges overlap. Eight-thread ranges overlap at every tested bandlimit, so no multicore speedup is claimed. The CC/F1 control ranges also overlap. The branch adds no allocation or workspace storage. On `main`, all 11 release tests matching `spin1_` passed, including the scalar-fast cutoff, all-grid adjoint comparison and one/eight-thread compacted analysis test. The ignored T511 all-grid DUCC/roundtrip check and T1023 exact-analysis DUCC check passed; spin-1 maximum selected DUCC sample error was `1.296e-14` and maximum coefficient error was `8.824e-18`. The T511 check covered spin 1 and 2 on all grids. Reviewer Sol's read-only review found no cutoff, E/B-slice or workspace issue and supported retaining the change as a GL one-thread improvement; it found no basis for claiming an eight-thread gain. Formatting and workspace Clippy passed; all-feature tests passed (114 Rust and 10 f32, 8 ignored), and no-default-feature tests passed (108 Rust and 10 f32, 7 ignored). DUCC phase timing was not part of this focused candidate comparison. No FFT code changed.

### Spin-1 T127 mixed-cutoff synthesis screening (2026-10-13)

At T127, a temporary diagnostic counted 48 mixed active/inactive latitude blocks for GL and 46 for CC/F1. An unguarded compact fused-synthesis prototype reduced the ordinary-coefficient timing medians in several cases, but Sol's read-only review identified a numerical risk: checking only the final unweighted result cannot detect a recurrence contribution lost to underflow and later amplified by `1/sin(theta)`. That prototype was rejected. A second prototype enabled the existing guarded scaled-to-normal transition only for mixed-cutoff blocks at T127; it kept the binary-scaled fallback when the transition or weighted-output checks declined.

The guarded prototype was compared with `d1f50f6` using `examples/m1b_bench.rs`, f64 T127 synthesis, reused plans and workspaces, full `mmax`, `phi0=0.37`, one warm-up and five timed calls per process. Five alternating baseline/candidate process pairs ran on CPUs 0–7 of an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`, with `rustc 1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off and runtime AVX2/FMA dispatch. Table values are one/eight-thread medians in milliseconds, computed from the five process medians:

| Grid / mode | Baseline | Guarded transition | Median paired change |
| --- | ---: | ---: | ---: |
| GL STANDARD | `3.321/2.075` | `3.322/2.091` | `+0.1%/+0.8%` |
| GL GRAD_ONLY | `3.498/2.259` | `3.419/2.218` | `-2.3%/-1.7%` |
| GL DERIV1 | `3.565/2.330` | `3.431/2.210` | `-3.0%/+1.1%` |
| CC STANDARD | `4.239/2.431` | `4.213/2.398` | `-0.5%/-1.2%` |
| CC GRAD_ONLY | `4.325/2.444` | `4.253/2.443` | `-1.6%/-0.2%` |
| CC DERIV1 | `4.416/2.468` | `4.209/2.441` | `-4.7%/-1.2%` |
| F1 STANDARD | `3.827/2.321` | `3.788/2.316` | `-1.3%/-0.6%` |
| F1 GRAD_ONLY | `3.955/2.406` | `3.914/2.414` | `-1.6%/+0.1%` |
| F1 DERIV1 | `3.965/2.583` | `3.892/2.394` | `-2.0%/-8.6%` |

The per-process timing ranges overlap in every grid, mode and thread comparison. DERIV1 uses the spin-1 scalar-conversion synthesis path at T127 and exercised the mixed-block eligibility trial; its overlapping ranges also show no resolved gain. The guarded branch adds checks without a reproducible whole-transform gain and was rejected. Its temporary extreme-coefficient comparison produced finite, closely matching spectra on GL/CC/F1, but selected the highest mixed order, likely the sectoral order with one degree pair. It did not establish a multi-degree transition handoff. Sol's second read-only review found no safety issue in the guarded branch and noted this test-coverage limit. No source change or regression test from either prototype was retained.

### Scalar analysis active-ring scheduling (2026-10-13)

The scalar analysis order splitter now uses the existing active-ring work estimate from T511. At lower limits, it keeps the degree-only midpoint because T255 comparisons were inconclusive. The change affects only which disjoint order ranges workers receive; recurrence and per-coefficient accumulation are unchanged. It uses the prepared `ring_order_limits` and adds no scratch or workspace storage.

The same release executable, built from `ecee5f0` with a temporary environment-controlled midpoint selection, was used for both schedules. `examples/m1b_bench.rs` measured f64 scalar analysis with full `mmax`, `phi0=0.37`, reused plans/workspaces, one warm-up and five calls per process. Process order alternated, with three paired runs on GL/CC at each bandlimit and six on F1. Runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`; Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo release `opt-level=3`, 16 codegen units, LTO off, and runtime AVX2/FMA dispatch without `target-cpu=native`. Values are median process medians in milliseconds, one/eight threads; the parenthesized ranges are the eight-thread process medians for degree-only / active-ring scheduling:

| Grid / bandlimit | Degree-only | Active-ring | Paired median 8-thread gain |
| --- | ---: | ---: | ---: |
| GL T511 | `32.272/11.039` | `32.086/9.610` (`10.828–11.062 / 9.453–9.787`) | `13.1%` |
| CC T511 | `40.196/17.817` | `40.337/16.887` (`17.687–17.878 / 16.586–17.013`) | `4.5%` |
| F1 T511 | `40.391/18.216` | `40.827/17.307` (`17.562–18.468 / 16.765–18.533`) | `4.7%` |
| GL T1023 | `233.154/75.459` | `238.850/70.376` (`74.752–77.330 / 69.959–72.694`) | `6.0%` |
| CC T1023 | `274.218/111.051` | `272.482/107.161` (`110.191–112.037 / 104.967–107.690`) | `3.9%` |
| F1 T1023 | `279.020/114.953` | `275.429/110.728` (`111.786–117.712 / 108.172–121.320`) | `1.9%` |

The one-thread path does not call the recursive parallel splitter, so its observed differences are controls and are not attributed to scheduling. The GL/CC eight-thread ranges are separated at both bandlimits. F1 is variable: the T511 paired gains were favorable in five of six runs, while T1023 had two noisy losses and overlapping ranges. No one-thread speedup is claimed. The test comparing one/eight-thread scalar analysis outputs passed at T255 and T1023 GL; the T255 DUCC 0.41.0 coefficient fixture, scalar adjoint identities, and release scalar DUCC/analytic checks through T2047 on GL/CC/F1 passed. Sol's strictly read-only review found no mathematical, cutoff, scratch-ownership or race blocker and recommended the T511 gate; its performance conclusion was limited to the eight-thread results. No FFT code changed.

### Scalar and spin-1 synthesis ring grouping (2026-10-13)

The default synthesis grouping gave each worker a contiguous group of 64-ring recurrence blocks. At 8 threads this left T255 GL/F1 with four groups, T255 CC with five, T511 CC with five, and T1023 CC with six. The new policy halves multi-block groups when that leaves workers idle and divides single-block groups into 32-ring tasks on T255/T511-sized grids. The numerical kernels are unchanged; 32-ring tasks produce 32-lane recurrence batches, while larger groups retain 64-ring block alignment. Batch size can change fast/fallback eligibility, which was covered by the DUCC and scalar-reference comparisons. T1023 GL/F1 keep their previous grouping because their eight groups already fill the pool and the spin-1 trials were flat or slower. At 8 threads, the resulting task counts are T255 GL/F1 `8` and CC `9`, T511 GL/F1 `16` and CC `9`, and T1023 CC `9`.

The comparison used `examples/m1b_bench.rs`, baseline `58a4dd5`, f64 full-bandlimit transforms, `phi0=0.37`, `nphi=2*lmax+3`, reused plans/workspaces, one warm-up and five timed calls per process. Two alternating process pairs were used at T255/T511 and three at T1023 CC. Each entry below is the median of the process medians, in milliseconds, for the eight-thread call. Runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`; Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo release `opt-level=3`, 16 codegen units, LTO off, and runtime AVX2/FMA dispatch without `target-cpu=native`.

Scalar STANDARD synthesis:

| Grid / bandlimit | Before | After | Time reduction |
| --- | ---: | ---: | ---: |
| GL T255 | `6.456` | `3.656` | `43.4%` |
| CC T255 | `6.293` | `3.448` | `45.2%` |
| F1 T255 | `6.879` | `3.729` | `45.8%` |
| GL T511 | `22.811` | `16.578` | `27.3%` |
| CC T511 | `29.001` | `15.508` | `46.5%` |
| F1 T511 | `23.213` | `16.550` | `28.7%` |
| CC T1023 | `168.034` | `129.019` | `23.2%` |

Spin-1 synthesis, with each cell showing `before → after (time reduction)`:

| Grid / bandlimit | STANDARD | GRAD_ONLY | DERIV1 |
| --- | ---: | ---: | ---: |
| GL T255 | `8.945 → 5.601 (37.4%)` | `9.264 → 5.739 (38.0%)` | `9.267 → 5.728 (38.2%)` |
| CC T255 | `9.869 → 6.579 (33.3%)` | `10.029 → 6.628 (33.9%)` | `10.148 → 6.862 (32.4%)` |
| F1 T255 | `9.287 → 5.784 (37.7%)` | `9.578 → 5.853 (38.9%)` | `10.024 → 5.932 (40.8%)` |
| GL T511 | `31.038 → 29.414 (5.2%)` | `31.877 → 25.620 (19.6%)` | `32.745 → 25.984 (20.6%)` |
| CC T511 | `38.473 → 29.059 (24.5%)` | `39.399 → 27.335 (30.6%)` | `39.879 → 28.864 (27.6%)` |
| F1 T511 | `31.788 → 27.151 (14.6%)` | `31.991 → 26.146 (18.3%)` | `32.424 → 30.019 (7.4%)` |
| CC T1023 | `222.325 → 183.792 (17.3%)` | `226.359 → 185.691 (18.0%)` | `231.731 → 189.735 (18.1%)` |

The two T255/T511 process ranges were separated for the cases in these tables. At T1023 CC, all three spin-1 mode ranges were separated across three pairs. One-thread controls showed no repeatable change; the executed algorithm remains serial in that setting. T1023 GL/F1 spin-1 results do not support splitting groups and are excluded from the policy. The extra worker scratch increased T511 workspace construction by about `0.2–0.8 ms`; it added no full spectrum or map. Process maximum-RSS comparisons were noisy, with observed differences within roughly `0.4 MiB` at T511.

Analysis uses at most the current Rayon pool size even when a workspace has more scratch slots for finer synthesis tasks. T511/T1023 analysis controls on GL/CC/F1 stayed within about `2%` between revisions; no analysis gain is claimed. A workspace created in a 16-thread pool and reused in an 8-thread pool passed the coefficient-equivalence regression.

Candidate output comparisons against DUCC `0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` covered scalar and spin-1 synthesis on GL/CC/F1 at T255/T511 and T1023 CC, including STANDARD, GRAD_ONLY and DERIV1, plus STANDARD analysis. Maximum relative L2 error was `1.73e-13`; spin-1 mode errors were at most `1.54e-13`. Existing adjoint and one/eight-thread equivalence tests passed. Release tests also passed for the ignored scalar T511/T1023/T2047 all-grid checks, spin-1/2 T511 all-grid checks, spin-1/2 T1023 checks, spin-2 T1023 synthesis, and spin-3/4 regressions. The tested change is limited to work partitioning; recurrence, polar cutoffs, FFT work and numerical conventions are unchanged.

Sol's read-only review found no source safety blocker and confirmed disjoint worker scratch and output ownership. It noted that 32-lane tasks can change fast/fallback selection and suggested a cross-pool synthesis regression. The description now states that limitation, and `synthesis_workspace_from_larger_pool_is_safe_in_smaller_pool` verifies numerical equivalence when an eight-thread workspace is used in a two-thread pool.

### T127 synthesis grouping refinement (2026-10-13)

The T127 GL/F1 policy previously emitted two 64-ring tasks for an eight-thread pool; CC emitted three. A second screening reduced the single-block group size to 32 rings, then 16 rings for grids with `128 <= ntheta < 256`. At T127 this creates eight tasks on GL/F1 and nine on CC. The Legendre kernels are unchanged. Shorter calls can change which existing SIMD fast/fallback path accepts a batch; each lane retains its cutoff and scaled fallback. Workspace storage remains bounded: GL/F1 reuse the same eight recurrence workspaces, while CC adds one workspace (about 22 KiB of numeric scratch for this T127 plan). No map or full spectrum is duplicated.

For the 16-versus-32-ring comparison, three paired process runs used `examples/m1b_bench.rs`, reused native plans/workspaces, f64 full-bandlimit T127 transforms, one warm-up and five timed samples per operation. Runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`; Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo release `opt-level=3`, 16 codegen units, LTO off, with runtime AVX2/FMA dispatch and no `target-cpu=native`. Each table cell is the median paired eight-thread time in milliseconds and the median paired reduction:

| Grid | Scalar STANDARD | Spin-1 STANDARD | Spin-1 GRAD_ONLY | Spin-1 DERIV1 |
| --- | ---: | ---: | ---: | ---: |
| GL | `0.705 → 0.501 (28.9%)` | `1.338 → 1.037 (22.5%)` | `1.310 → 0.988 (25.6%)` | `1.352 → 1.022 (24.3%)` |
| CC | `0.695 → 0.507 (28.5%)` | `1.756 → 1.437 (18.2%)` | `1.789 → 1.456 (18.6%)` | `1.820 → 1.498 (17.7%)` |
| F1 | `0.761 → 0.546 (28.1%)` | `1.460 → 1.100 (23.6%)` | `1.478 → 1.066 (27.6%)` | `1.498 → 1.108 (24.7%)` |

One-thread synthesis controls changed by `-0.5%` to `+2.9%`. STANDARD analysis controls showed no repeatable gain; the CC spin-1 eight-thread result varied across pairs and is not claimed as an analysis improvement. Release Python-plan comparisons against `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` covered scalar and spin-1 STANDARD synthesis/analysis and spin-1 GRAD_ONLY/DERIV1 synthesis at T127 on GL/CC/F1 with eight threads. Maximum relative-L2 error was `1.90e-14`; maximum absolute error was `6.36e-14`. The candidate passed `cargo fmt --all -- --check` and `cargo test --release --workspace --all-features` (117 Rust tests and 10 f32 tests passed; 8 high-bandlimit tests were ignored). Sol's focused read-only review found no safety or numerical blocker, recommended not claiming an analysis gain, and suggested high-order cross-pool coverage. The low-degree workspace-reuse test remains supplemented by the dense T127 DUCC comparisons and existing all-grid scalar-conversion regression; no dedicated high-order cross-pool test was added. No FFT code changed.

### T63 synthesis grouping and fast-path screening (2026-10-13)

For T63, the eight-thread schedule previously made one task on GL/F1 and two on CC. The policy now uses 16-ring tasks when `ntheta >= 64` and the existing work-group condition is met. This yields four tasks on GL/F1 and five on CC; the workspace still has eight recurrence workers because analysis already requires eight. A three-process screening compared 8-ring and 16-ring tasks. The 8-ring result varied by grid and operation, with paired changes from `-14.5%` to `+35.2%`; it was not retained.

The accepted 16-ring policy was compared with the prior 64-ring grouping in six alternating process pairs using `examples/m1b_bench.rs`. Each native f64 T63 operation reused its plan/workspace, warmed once and took five timed samples. The comparison used CPUs 0–7 on an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`, Rust `1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off, and runtime AVX2/FMA dispatch without `target-cpu=native`. Entries are the median one/eight-thread milliseconds; the parenthesized value is the median paired eight-thread reduction:

| Grid | Scalar STANDARD | Spin-1 STANDARD | Spin-1 GRAD_ONLY | Spin-1 DERIV1 |
| --- | ---: | ---: | ---: | ---: |
| GL | `0.170/0.192 → 0.172/0.130 (32.9%)` | `0.379/0.364 → 0.361/0.241 (34.4%)` | `0.359/0.380 → 0.375/0.269 (31.0%)` | `0.363/0.390 → 0.367/0.280 (29.9%)` |
| CC | `0.179/0.211 → 0.180/0.134 (35.9%)` | `0.648/0.509 → 0.645/0.376 (27.4%)` | `0.647/0.509 → 0.649/0.380 (25.5%)` | `0.667/0.524 → 0.663/0.401 (25.2%)` |
| F1 | `0.179/0.173 → 0.197/0.135 (25.9%)` | `0.356/0.359 → 0.365/0.277 (24.6%)` | `0.356/0.364 → 0.355/0.246 (31.1%)` | `0.381/0.367 → 0.366/0.260 (28.4%)` |

Every paired eight-thread synthesis comparison favored 16-ring tasks; reductions ranged from `19.7%` to `47.9%`. One-thread ranges overlapped with mixed signs, so no serial change is claimed. Analysis ranges also overlap, and no analysis improvement is claimed. A release Python-plan comparison with `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) at eight threads covered scalar and spin-1 STANDARD synthesis/analysis and spin-1 GRAD_ONLY/DERIV1 synthesis on GL/CC/F1 with `phi0=0.37`; maximum relative-L2 error was `1.20e-14`, maximum absolute error was `6.12e-15`. Focused one/eight-thread scalar and spin-1 STANDARD, GRAD_ONLY and DERIV1 synthesis checks passed on all three grids. The full release all-feature suite passed (118 Rust tests and 10 f32 tests; 8 high-bandlimit tests ignored).

The cutoff and scaling data are unchanged by task sizing. The existing spin-1 STANDARD analysis profile reports 100% cutoff-active lane work, no mixed blocks, and no scaled lanes at T63; at T127 it reports 99.8% active work, 48 mixed GL blocks (46 CC/F1), and 10–13% scaled lanes in the paired fast path. Thus T63 has no inactive ring work to compact, while T127 compaction can remove little degree work and the earlier guarded synthesis continuation did not improve whole transforms. Those results are specific to STANDARD analysis and the recorded T127 mixed-block synthesis screen; no new compaction or scaled-to-normal continuation was added for scalar, STANDARD, GRAD_ONLY or DERIV1 paths. Sol's focused read-only review confirmed that the T63 groups use disjoint worker scratch and spectra, including CC's one-ring tail, and verified the reported 8-thread paired results. No FFT code changed.

### Spin-1 scalar-conversion eligibility below T63 (2026-10-13)

`SPIN1_FAST_MIN_LMAX` is 15 for f64 scalar-conversion synthesis and STANDARD analysis. This enables the existing scalar-conversion path for spin-1 STANDARD, GRAD_ONLY and DERIV1 synthesis at T15, T31 and T47. An eligibility-1 screen found no repeatable T1 improvement and mixed T7 results; those limits use the general recurrence. The threshold does not change f32 dispatch because f32 does not enable paired scalar kernels. At T14, the spin-1 and DERIV1 tests exercise the general-path side of the boundary; at T15, they compare scalar-conversion results against the general recurrence.

The native comparison used `examples/m1b_bench.rs`, full `mmax`, `phi0=0.37`, `nphi=2*lmax+3`, minimum exact-analysis ring counts, and reused f64 plans/workspaces. Each process warmed each operation once and took five timed samples; five alternating process pairs were compared. The one-thread and eight-thread columns show median process medians in milliseconds, before the threshold change at 63 and after lowering it to 15. Runs were pinned to CPUs 0–7 on an AMD Ryzen 9 5950X running Linux `7.0.14-20-pve`, with Rust `1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO disabled and runtime AVX2/FMA dispatch; `target-cpu=native` was not used. Plan and workspace construction were outside transform timings.

| Bandlimit | Grid | Spin-1 STANDARD synthesis, 1T/8T | Spin-1 GRAD_ONLY synthesis, 1T/8T | DERIV1 synthesis, 1T/8T |
| --- | --- | ---: | ---: | ---: |
| T15 | GL | `0.071/0.060 → 0.022/0.025` | `0.071/0.063 → 0.022/0.026` | `0.071/0.072 → 0.024/0.028` |
| T15 | CC | `0.079/0.065 → 0.046/0.047` | `0.073/0.064 → 0.037/0.045` | `0.063/0.064 → 0.038/0.048` |
| T15 | F1 | `0.071/0.061 → 0.020/0.029` | `0.069/0.061 → 0.023/0.027` | `0.072/0.073 → 0.024/0.027` |
| T31 | GL | `0.382/0.380 → 0.069/0.072` | `0.382/0.385 → 0.084/0.085` | `0.382/0.386 → 0.073/0.075` |
| T31 | CC | `0.396/0.398 → 0.139/0.148` | `0.401/0.403 → 0.138/0.140` | `0.397/0.459 → 0.144/0.147` |
| T31 | F1 | `0.387/0.385 → 0.085/0.073` | `0.382/0.384 → 0.071/0.073` | `0.385/0.387 → 0.076/0.075` |
| T47 | GL | `1.233/1.217 → 0.186/0.218` | `1.212/1.201 → 0.192/0.226` | `1.212/1.218 → 0.193/0.202` |
| T47 | CC | `1.237/1.262 → 0.348/0.349` | `1.282/1.262 → 0.347/0.352` | `1.280/1.286 → 0.362/0.370` |
| T47 | F1 | `1.204/1.189 → 0.186/0.227` | `1.201/1.203 → 0.188/0.228` | `1.213/1.199 → 0.192/0.198` |

The synthesis reductions across grid and mode cells were 39.7–71.8% at T15, 62.6–81.9% at T31 and 71.7–84.9% at T47 with one thread; with eight threads they were 25.0–63.0%, 57.1–81.4% and 71.2–84.4%, respectively. The T63 controls use the same fast path before and after the threshold change and showed no resolved change. STANDARD analysis also improved at T15/T31/T47:

| Bandlimit | Grid | 1 thread, before → after (ms) | 8 threads, before → after (ms) |
| --- | --- | ---: | ---: |
| T15 | GL | `0.053 → 0.025` | `0.027 → 0.019` |
| T15 | CC | `0.068 → 0.048` | `0.039 → 0.035` |
| T15 | F1 | `0.082 → 0.058` | `0.044 → 0.033` |
| T31 | GL | `0.336 → 0.083` | `0.101 → 0.042` |
| T31 | CC | `0.405 → 0.184` | `0.150 → 0.115` |
| T31 | F1 | `0.412 → 0.191` | `0.147 → 0.121` |
| T47 | GL | `1.045 → 0.202` | `0.258 → 0.118` |
| T47 | CC | `1.277 → 0.452` | `0.381 → 0.270` |
| T47 | F1 | `1.301 → 0.471` | `0.383 → 0.267` |

Median plan-construction times were `0.004/0.007/0.018 ms` before and after at T15/T31/T47. Eight-thread workspace-allocation medians changed from `0.027/0.028/0.038 ms` to `0.027/0.029/0.039 ms`. The four per-worker complex buffers add 1,088 bytes at T15; the eight-worker workspace adds 8.5 KiB. The transform does not allocate another map or spectrum.

The candidate Python extension matched `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` on T15/T31/T47 GL/CC/F1 with `phi0=0.37`. Comparisons covered spin-1 STANDARD synthesis/analysis and both adjoints, E-only and B-only synthesis, GRAD_ONLY synthesis and adjoint, and DERIV1 synthesis and adjoint. Dense coefficients were scaled by `1e-5`; analysis used deterministic finite maps. Every comparison passed `rtol=atol=3e-12`; the largest absolute error was `1.010e-10`, and the maximum normalized L2 error `||actual-reference||₂ / max(||reference||₂, 1)` was `1.788e-14`. The extended fast-versus-general regressions cover T14/T15/T31/T47 for synthesis, STANDARD analysis and adjoint identities, and DERIV1 on all three grids. The T511 all-grid spin check and T1023 sparse-synthesis and exact-analysis DUCC checks passed after the change. The focused read-only Sol review found no numerical or memory blocker; it confirmed the `lmax+1` scalar recurrence row is allocated, `l<spin` coefficients stay excluded, and f32 does not use this path. No FFT code changed.

### Spin analysis factor preparation reordering (2026-10-13, rejected)

A candidate moved `fill_spin_factors()` after the partial paired-scalar analysis pass. The paired pass overwrites these factor arrays, so the candidate removed one spin-factor preparation per order before rebuilding factors for the polar recurrence. It changed no recurrence or accumulation arithmetic. Two alternating baseline/candidate process pairs used `examples/m1b_bench.rs` on f64 spin-1 STANDARD analysis at T255, T511 and T1023 on CC/F1, with a T511 GL control. Plans and workspaces were reused; each process warmed once and timed five calls. Runs used CPUs 0–7 on the Ryzen 9 5950X / Linux `7.0.14-20-pve`, Rust `1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off and runtime AVX2/FMA dispatch. The medians below are computed from two process medians, in milliseconds; changes are candidate relative to baseline.

| Grid / bandlimit | Baseline 1T/8T | Candidate 1T/8T | Change 1T/8T |
| --- | ---: | ---: | ---: |
| CC T255 | `22.402/9.098` | `23.308/9.024` | `+4.0%/-0.8%` |
| CC T511 | `115.318/42.738` | `118.684/41.776` | `+2.9%/-2.3%` |
| CC T1023 | `780.994/256.520` | `821.461/277.094` | `+5.2%/+8.0%` |
| F1 T255 | `22.921/9.305` | `23.162/9.595` | `+1.0%/+3.1%` |
| F1 T511 | `125.103/45.670` | `127.818/47.700` | `+2.2%/+4.4%` |
| F1 T1023 | `776.008/279.298` | `815.582/286.052` | `+5.1%/+2.4%` |
| GL T511 control | `90.707/24.339` | `91.155/25.474` | `+0.5%/+4.7%` |

Process ranges overlapped in most comparisons, and the candidate was slower in both thread counts for CC T1023 and all F1 bandlimits. The T511 GL control showed no improvement. The focused spin-1 fast-versus-general analysis and adjoint regression passed with the candidate, but the whole-transform measurements do not support retaining it. The source uses the previous factor order.

### Paired scalar analysis FMA accumulation (2026-10-13, rejected)

Callgrind attributed a large share of scalar and spin-1 analysis instructions to `accumulate_scalar_analysis_pair()`. Two local AVX2/FMA variants replaced the packed multiply/add accumulation with fused multiply-adds. One also precomputed `basis*cos(theta)`; the other kept the original multiplication order for the opposite-parity term. Neither source change was retained.

The precomputed-product variant reduced Callgrind instructions for GL T127 spin-1 analysis from `679,269,937` to `658,287,072` (`-3.1%`) and improved scalar GL analysis at T511/T1023, but regressed spin-1 one-thread analysis by `3.4–9.0%` at the measured T511/T1023 grids. Its spin-1 eight-thread results were mixed. The order-preserving FMA variant reduced scalar GL T127 instructions from `248,121,377` to `244,318,326` (`-1.5%`). Its scalar T511/T1023 elapsed times appeared `7–9%` lower across two process pairs, but the host load average was `7–14` on a 16-core CPU, and the spin-1 results included large outliers and mixed changes. Those timings do not establish a reproducible whole-transform gain or a safe scalar-only specialization. Callgrind counts describe executed instructions and are not cycle or elapsed-time measurements.

The accumulator and scaled fallback remain unchanged. No additional dispatch parameter or duplicated reducer was added to distinguish scalar analysis from spin-conversion analysis. Reconsider this only with a quieter, paired whole-transform comparison that includes DUCC, adjoint checks and high-band validation; do not infer a speedup from instruction reduction alone. No FFT code changed.

### Spin-1 safe-lane transition split (2026-10-13)

At T255, a temporary test-only path census showed that the existing `sin(theta) >= 0.01` transition guard kept active GL/F1 lanes on scaled recurrence after the normal fused block had declined. The rejected general-path work was 12.2–13.8% of active degree-lane work at one thread and 6.1–6.9% at eight threads. CC had no comparable rejected work. The census was removed after collection. T63 and T127 remain below the transition gate and do not execute this change.

The candidate packs active lanes with `sin(theta) >= 0.01` into existing worker scratch, runs the unchanged guarded transition on those lanes and scatters their spectra to the original rings. Active polar lanes remain in separate existing scratch and use the scaled paired fallback. If the safe-lane transition declines or fails its finite-output preflight, no spectrum is written by the helper and the original fallback handles all active lanes. The cutoff, transition threshold, arithmetic and polar contribution are unchanged. No worker workspace or map/spectrum allocation was added.

Native whole-transform measurements used `examples/m1b_bench.rs`, f64 spin-1 synthesis with reused plans and workspaces, `phi0=0.37`, full `mmax`, `nphi=2*lmax+3`, one warm-up and five timed calls per process. Baseline commit was `277675f`; the candidate was the current source before this documentation and test-only strengthening. T255/T511 use three alternating baseline/candidate process pairs; T1023 DERIV1 uses two pairs. Runs were pinned to CPUs 8–15 on an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`. Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo release `opt-level=3`, 16 codegen units, LTO off, runtime AVX2/FMA dispatch; `target-cpu=native` was not enabled. Values are median process medians in milliseconds, one/eight threads:

| Bandlimit | Grid | Mode | 1 thread, before → after | 8 threads, before → after |
| ---: | --- | --- | ---: | ---: |
| T255 | GL | STANDARD | `18.608 → 14.638` | `5.550 → 4.521` |
| T255 | GL | GRAD_ONLY | `19.145 → 14.827` | `5.708 → 4.615` |
| T255 | GL | DERIV1 | `19.451 → 15.027` | `5.783 → 4.722` |
| T255 | CC | STANDARD | `21.657 → 21.964` | `6.676 → 6.704` |
| T255 | CC | GRAD_ONLY | `22.079 → 22.057` | `6.744 → 6.754` |
| T255 | CC | DERIV1 | `22.322 → 22.286` | `6.900 → 6.970` |
| T255 | F1 | STANDARD | `19.733 → 16.197` | `5.790 → 4.911` |
| T255 | F1 | GRAD_ONLY | `20.386 → 16.330` | `5.858 → 4.958` |
| T255 | F1 | DERIV1 | `20.454 → 16.456` | `6.060 → 5.153` |
| T511 | GL | STANDARD | `89.757 → 78.034` | `25.075 → 22.481` |
| T511 | GL | GRAD_ONLY | `92.448 → 79.971` | `30.957 → 23.383` |
| T511 | GL | DERIV1 | `93.278 → 80.056` | `27.136 → 23.698` |
| T511 | CC | STANDARD | `94.138 → 83.142` | `32.209 → 26.431` |
| T511 | CC | GRAD_ONLY | `98.750 → 83.913` | `39.775 → 26.593` |
| T511 | CC | DERIV1 | `95.943 → 83.983` | `33.167 → 27.167` |
| T511 | F1 | STANDARD | `90.880 → 74.569` | `25.132 → 22.899` |
| T511 | F1 | GRAD_ONLY | `92.774 → 76.203` | `25.903 → 21.471` |
| T511 | F1 | DERIV1 | `94.519 → 76.777` | `26.679 → 22.655` |
| T1023 | GL | DERIV1 | `614.157 → 575.724` | `187.734 → 165.756` |
| T1023 | CC | DERIV1 | `626.725 → 585.871` | `191.232 → 172.130` |
| T1023 | F1 | DERIV1 | `623.362 → 576.612` | `189.265 → 166.351` |

T255 GL and F1 improvements were 17.9–22.7% at one thread and 15.0–19.1% at eight; the CC control stayed within measurement noise. T511 one-thread improvements were 11.7–18.8%. Eight-thread medians improved, though some process ranges overlapped and the baseline contained scheduling outliers. T1023 DERIV1 improved 6.3–7.5% at one thread and 10.0–12.1% at eight across the two paired runs. A fresh three-pair T255 rerun reproduced GL STANDARD (`18.601/5.747 → 14.640/4.575 ms`) and F1 DERIV1 (`20.378/6.103 → 16.452/5.096 ms`); CC STANDARD remained a nonsplitting control (`21.880/6.677 → 21.469/6.657 ms`).

The rebuilt Python plan matched installed `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` on T255 GL/CC/F1 for STANDARD, GRAD_ONLY and DERIV1 synthesis. It used Python `3.11.15`, NumPy `2.4.6`, matching coefficients and `phi0`, with one process per setting and three groups of five timed samples. This run checks numerical agreement, not before/after performance. Maximum absolute and relative-L2 errors were `1.095e-12` and `3.490e-14`. The ignored release DUCC tests `m3_spin_t511_all_grids_matches_ducc_and_roundtrips` and `m3_spin_t1023_sparse_synthesis_matches_ducc` passed; standard spin adjoint and spin-3/4 regression tests also passed. The focused regression proves that the T255 GL/F1 safe transition succeeds with a positive polar fallback count, checks safe outputs and the complete split block against the general recurrence, and keeps the CC nonsplitting control. Sol's strictly read-only review found no numerical, indexing, ownership or race blocker; it requested explicit proof of successful split use, which the regression now asserts. The T255 probe's first block is cutoff-active; the full-grid comparison separately covers cutoff-excluded rings. Formatting, workspace Clippy, full all-feature and no-default-feature Rust tests, and the Python suite passed. No FFT code changed.

### Scalar synthesis continuation gate at T511 (rejected, 2026-10-13)

A trial lowered `SCALAR_TRANSITION_MIN_LMAX` from T1023 to T511, enabling the existing guarded paired scaled-to-normal continuation for scalar f64 synthesis at T511. The recurrence and its extreme-coefficient fallback were unchanged. The baseline was commit `065f0a5`; the candidate differed only by the gate. Three alternating process pairs per T511 grid and two pairs for the T255 GL control used `examples/m1b_bench.rs`, reused plans/workspaces, one warm-up and five timed calls. Runs were pinned to CPUs 8–15 on a Ryzen 9 5950X / Linux `7.0.14-20-pve`, with Rust `1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off and runtime AVX2/FMA dispatch without `target-cpu=native`. Values are median process medians in milliseconds, one/eight threads:

| Grid / bandlimit | Existing T1023 gate | Trial T511 gate | Change |
| --- | ---: | ---: | ---: |
| GL T511 | `68.569/16.247` | `68.086/16.649` | `-0.7%/+2.5%` |
| CC T511 | `67.470/22.701` | `67.042/23.022` | `-0.6%/+1.4%` |
| F1 T511 | `69.473/16.972` | `69.224/17.019` | `-0.4%/+0.3%` |
| GL T255 control | `13.835/3.564` | `13.816/3.625` | `-0.1%/+1.7%` |

The T511 one-thread differences are within measurement variability, while all eight-thread medians are slightly slower. The T255 control is unchanged-path noise. The release scalar DUCC/analytic check through T2047 passed with the lower gate, but there is no performance case to retain it. `SCALAR_TRANSITION_MIN_LMAX` remains 1023. No source change from this trial was kept and no FFT code changed.

### Paired scalar synthesis coefficient preparation (2026-10-13)

Callgrind on f64 spin-1 GL GRAD_ONLY synthesis at T255 attributed `960,674,688` instruction references (24.95% of the baseline program total) to `FusedSynthesisBlock`. The block repeated the DUCC paired-degree coefficient transform for every latitude-vector chunk even though the transformed even/odd coefficients depend only on order and degree. Preparing them once per order for scalar synthesis, and once per converted channel for spin-1 synthesis, reduced Callgrind program references from `3,850,340,884` to `3,198,422,898` (16.9%). Callgrind counts are instruction references, not cycles or elapsed time.

The implementation reuses per-worker complex scratch, keeps the paired recurrence and summation order unchanged, and leaves the scaled fallback and polar cutoff unchanged. It adds no full map or spectrum. Plans below T15 allocate four short scalar coefficient scratch vectors for this path; at T14 this is at most 1 KiB per worker. No FFT code changed.

Native measurements used `examples/m1b_bench.rs`, reused f64 plans and workspaces, full `mmax`, `phi0=0.37`, `nphi=2*lmax+3`, one warm-up and five timed calls per process. Baseline was `dcb562f`; the candidate was the prepared-coefficient implementation. Runs used CPUs 0–7 on an AMD Ryzen 9 5950X / Linux `7.0.14-20-pve`, Rust `1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off and runtime AVX2/FMA dispatch without `target-cpu=native`. T511 results are medians of five alternating baseline/candidate process pairs; T1023 results use two pairs. Values are milliseconds, one/eight threads:

| Grid / bandlimit | Operation | Baseline 1T/8T | Candidate 1T/8T |
| --- | --- | ---: | ---: |
| GL T511 | Scalar synthesis | `66.285/16.510` | `58.183/15.385` |
| GL T511 | Spin-1 STANDARD synthesis | `76.759/22.425` | `61.916/21.224` |
| GL T511 | Spin-1 GRAD_ONLY synthesis | `77.362/22.322` | `61.810/21.143` |
| GL T511 | Spin-1 DERIV1 synthesis | `77.864/23.740` | `64.018/23.185` |
| CC T511 | Scalar synthesis | `64.457/22.630` | `57.005/22.450` |
| CC T511 | Spin-1 STANDARD synthesis | `79.952/25.916` | `63.940/25.369` |
| CC T511 | Spin-1 GRAD_ONLY synthesis | `80.069/25.977` | `64.868/25.188` |
| CC T511 | Spin-1 DERIV1 synthesis | `80.792/26.510` | `65.356/25.933` |
| F1 T511 | Scalar synthesis | `67.770/16.253` | `59.105/15.662` |
| F1 T511 | Spin-1 STANDARD synthesis | `72.327/21.435` | `57.111/21.475` |
| F1 T511 | Spin-1 GRAD_ONLY synthesis | `72.558/21.069` | `58.024/20.572` |
| F1 T511 | Spin-1 DERIV1 synthesis | `75.392/23.188` | `59.755/20.862` |
| GL T1023 | Scalar synthesis | `445.576/130.630` | `382.815/129.604` |
| GL T1023 | Spin-1 STANDARD synthesis | `550.211/160.302` | `440.740/154.616` |

At GL T511, one-thread time fell by 12.2% for scalar synthesis and 17.8–19.3% for the three spin-1 modes. Eight-thread improvements were 6.8% for scalar and 2.3–5.7% for spin 1. At GL T1023, one-thread time fell by 14.9% for scalar and 20.2% for spin-1 STANDARD; eight-thread time was essentially unchanged for scalar and fell 3.2% for spin 1. CC/F1 T511 one-thread gains were 11.6–21.0%; their eight-thread gains ranged from no resolved change to 10.0%, with F1 STANDARD unchanged. The CC/F1 and T1023 candidate runs used the same prepared kernel before a small control-flow cleanup that removed an unreachable production fallback; the final-source GL T511 rerun reproduced the scalar, STANDARD and GRAD_ONLY gains. DERIV1's one-thread gain reproduced, while its eight-thread samples remained more variable. The effect is strongest at one thread because coefficient preparation removes repeated work within each vector block while the eight-thread transform is increasingly limited by parallel overhead and memory effects.

The focused coefficient test compares each prepared even/odd pair with the existing degree transform, including the sectoral span and the shortest `lmax=m+1` and `lmax=m+2` tails. On final source, `cargo fmt --all -- --check`, workspace Clippy with warnings denied, all-feature tests (120 passed, 8 ignored), no-default-feature tests (110 passed, 7 ignored), and `uv run pytest python/tests` (8 passed) succeeded. Release DUCC and analytic checks passed for scalar through T2047 and spin 1 through T2047; adjoint and spin-3/4 regression tests also passed. A strictly read-only Sol review found no numerical or ownership blocker and requested the three short spans; the regression now includes them. The f32 implementation declines this f64-only helper, and spin-2 remains unchanged.

### Scalar and spin-1 analysis active-lane compaction control (2026-10-13)

A temporary census counted degree-weighted ring pairs admitted by the current polar-order cutoff. The active fractions for scalar and spin-1 work were 100% at T63, 99.75–99.83% at T127, 95.0–95.3% at T255, 88.45–88.69% at T511 and 83.57–83.72% at T1023 across GL, CC and F1. At T255, about 18% of block-degree workload fell in mixed 64-lane blocks, while only about 5% of individual lane work was inactive. At T511, mixed blocks accounted for about 15.7–15.9% of block-degree workload and fully excluded blocks for 4.7–4.9%; at T1023 these were about 10% and 11.6–11.8%. North/south pair activity is mirrored on these symmetric grids, with the equatorial ring handled as a singleton. These counts motivated a direct compaction control rather than a cutoff-mask-only experiment.

The control changed only the scalar paired-analysis and spin-1 scalar-conversion-analysis compaction gates to disable packing; cutoff masks, recurrence arithmetic and outputs remained unchanged. Native measurements used `examples/m1b_bench.rs`, reused f64 plans/workspaces, one warm-up and five calls per process, pinned to CPUs 0–7 on the Ryzen 9 5950X / Linux `7.0.14-20-pve`. Rust was `1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off, runtime AVX2/FMA dispatch and no `target-cpu=native`. Three alternating process pairs were used per case. Values are median process medians in milliseconds, one/eight threads:

| Operation | Grid / bandlimit | Compaction on | Compaction off | Improvement with compaction |
| --- | --- | ---: | ---: | ---: |
| Scalar analysis | GL T511 | `32.002/9.641` | `57.546/16.747` | `44.4%/42.4%` |
| Spin-1 STANDARD analysis | GL T511 | `92.694/23.908` | `185.429/50.733` | `50.0%/53.3%` |
| Scalar analysis | GL T1023 | `240.565/81.134` | `343.652/90.042` | `30.0%/9.9%` |
| Spin-1 STANDARD analysis | GL T1023 | `677.833/190.255` | `1059.947/240.362` | `36.1%/20.8%` |
| Scalar analysis | CC T511 | `40.465/16.728` | `66.902/24.643` | `39.5%/32.1%` |
| Spin-1 STANDARD analysis | CC T511 | `121.150/40.478` | `214.491/67.531` | `43.5%/40.1%` |
| Scalar analysis | F1 T511 | `41.068/17.510` | `66.876/24.896` | `38.6%/29.7%` |
| Spin-1 STANDARD analysis | F1 T511 | `120.983/41.034` | `213.997/68.241` | `43.5%/39.9%` |

The largest effect is not explained by skipped lane count alone. Temporary T511 GL callback counters showed scaled lane-pair steps falling from 16.0% to 4.2% for scalar analysis and from 16.1% to 4.4% for spin-1 analysis when compaction was enabled; the corresponding normal-path work increased. Thus compaction also improves SIMD path eligibility. The control added no workspace or memory traffic to the production path. DUCC-reference scalar analysis and spin-1 fast/general plus adjoint focused tests passed with compaction disabled; no production source was changed. Compaction remains enabled. The measured gain is consistent across both analysis operations and grids, so no alternative compaction batch size is warranted by this evidence. No FFT code changed.

### Spin-1 transition threshold at T63 (screened, 2026-10-13)

A temporary path counter tested the existing scaled-to-normal transition with its `lmax` gate lowered from 191 to 63. It counted zero transition-helper calls for STANDARD, GRAD_ONLY or DERIV1 synthesis on GL, CC and F1 at T63. The safe-lane split was also ineligible: every endpoint in the scalar-conversion fast segments had `sin(theta) >= 0.01`. The counter and test were removed, and the threshold remains 191. Three alternating native process pairs showed overlapping, noisy whole-transform timings; because the candidate did not enter the transition path, those timings provide no evidence of a speed change. The earlier T127 all-grid/mode screen is recorded with the spin-1 safe-lane transition split and likewise does not support lowering the gate. No source change was retained and no FFT code changed.

### Spin-1 paired-channel synthesis recurrence (2026-10-13)

The earlier paired scalar-conversion path advanced and accumulated the Q and U channels in separate SIMD calls. This candidate prepares both channels' even/odd degree coefficients once per order, advances their common scalar recurrence once per latitude block, and keeps four real/imaginary SIMD accumulators live together. It is selected only for f64 spin-1 synthesis when every lane has `sin(theta) >= 0.01`, no ring is excluded by the order cutoff, and the existing paired coefficient preparation succeeds. Other cases retain the established scalar-conversion or scaled recurrence path. The path serves STANDARD and GRAD_ONLY synthesis; DERIV1 is scalar synthesis and is not affected. It adds two complex coefficient scratch arrays per worker (16,416 bytes at T511; 131,328 bytes for eight workers) and no map or spectrum allocation.

Native matched comparisons used the release `m1b_bench` executable, reusable plans/workspaces, `phi0=0.37`, `nphi=2*lmax+3`, one warm-up and five timed samples per process, pinned to CPUs 0–7 on the Ryzen 9 5950X / Linux `7.0.14-20-pve`. Rust was `1.99.0 (b940084d7 2026-09-28)`, release `opt-level=3`, 16 codegen units, LTO off, runtime AVX2/FMA dispatch and no `target-cpu=native`. The baseline executable came from `a67ee09`; the candidate was built from the then-current worktree. Each value below is the median of three alternating baseline/candidate process pairs, milliseconds at one/eight threads:

| Grid / bandlimit | Operation | Baseline 1T/8T | Candidate 1T/8T | Change 1T/8T |
| --- | --- | ---: | ---: | ---: |
| GL T511 | Spin-1 STANDARD synthesis | `62.192/21.549` | `59.033/20.757` | `-5.1%/-3.7%` |
| GL T511 | Spin-1 GRAD_ONLY synthesis | `64.176/21.726` | `59.958/21.536` | `-6.6%/-0.9%` |
| CC T511 | Spin-1 STANDARD synthesis | `65.100/26.013` | `61.487/25.570` | `-5.6%/-1.7%` |
| CC T511 | Spin-1 GRAD_ONLY synthesis | `66.443/26.389` | `62.990/26.000` | `-5.2%/-1.5%` |
| F1 T511 | Spin-1 STANDARD synthesis | `58.601/20.190` | `54.333/19.829` | `-7.3%/-1.8%` |
| F1 T511 | Spin-1 GRAD_ONLY synthesis | `58.333/22.257` | `54.891/19.645` | `-5.9%/-11.7%` |

The one-thread result improved in every valid GL/CC/F1 case. Eight-thread changes were smaller, with the F1 GRAD_ONLY series showing substantial process variability; only its median is reported and no precise scaling claim is made. The native harness rejects spin-1 DERIV1, correctly, because DERIV1 is a scalar derivative operation; it was mistakenly included in one screening loop and its panics are excluded. Plan construction and workspace-allocation samples overlapped between baseline and candidate (plan about `0.13–0.17 ms`, workspace about `7.4–8.3 ms`); they do not explain the execution change.

On final source, `cargo test -p rustsht --lib spin1_scalar_` passed seven focused tests, including comparison to the general channel recurrence and all-grid scalar-conversion coverage. `cargo test --release -p rustsht --lib m3_spin_t511_all_grids_matches_ducc_and_roundtrips -- --ignored` passed the opt-in T511 GL/CC/F1 spin-1/2 DUCC and roundtrip check with the candidate active. Existing focused adjoint checks are unchanged by this synthesis-only path. The DUCC test emits no aggregate error statistic, so this entry records fixture/test acceptance rather than inventing a measured max error. Keep this result as a bounded f64 spin-1 synthesis optimization; analysis, f32, DERIV1, and spin 2 are unchanged. No FFT code changed.

### Bounded scalar GL Legendre table compared with Torch-harmonics (2026-10-13)

Torch-harmonics PR [#322](https://github.com/NVIDIA/torch-harmonics/pull/322), inspected at head `37751d3e90b8f12fe9439c52847f4c5d70d5a627` (base `97f7d2756d2d2d3b974d8ee31ca04e032fdf1d05`), returns rectangular order-degree-latitude Legendre tensors and masks unsupported triangular modes rather than packing them. Its single-process scalar transform stores the full table; inverse synthesis transposes the table to make degree contiguous. The folded CC/F1 analysis path allocates a full output tensor and builds it in blocks of at most `2^20` float64 basis values (8 MiB). Those blocks bound intermediate basis storage, not the final projection. The regular Legendre cache uses the helper's 20-entry default; the folded projection cache holds two entries. Distributed transforms can store local order/latitude shards. Torch-harmonics was not installed in the benchmark environment, so its execution time was not measured.

The Rust experiment stores only `m <= l <= lmax` and the northern half of each GL grid. Latitude is contiguous within each `(m,l)` pair. Synthesis reflects basis values with `(-1)^(l+m)`; analysis applies that parity while summing north and south Fourier samples. It uses the existing order-major worker spectrum and adds no transform-sized workspace. The f64 GL table is limited to `72 <= lmax <= 255`, `mmax >= ceil(3*lmax/4)`, exact GL sampling and at most 34 MiB. f32, spin plans, CC/F1 and larger tables use recurrence.

For full triangular support and `ntheta=lmax+1`, the Rust table occupies `8*ceil(ntheta/2)*((lmax+1)*(lmax+2)/2)` bytes. A dense float64 Torch table with the same full dimensions occupies `8*(lmax+1)^3` bytes, or twice that for the vector table. The table payloads below are exact element counts; T360 and above are temporary measurements with the production gate raised and are disabled in the committed implementation.

| Bandlimit | Rust north-triangular table | Torch scalar table | Torch two-row vector table |
| --- | ---: | ---: | ---: |
| T72 | 0.762 MiB (eligible) | 2.97 MiB | 5.93 MiB |
| T127 | 4.03 MiB | 16 MiB | 32 MiB |
| T255 | 32.13 MiB | 128 MiB | 256 MiB |
| T360 | 90.23 MiB (temporary) | 358.9 MiB | 717.9 MiB |
| T511 | 256.5 MiB (temporary) | 1 GiB | 2 GiB |
| T720 | 716.87 MiB (temporary) | 2.79 GiB | 5.59 GiB |

The table experiment used `examples/m1b_bench.rs`, f64 full-bandlimit GL transforms, `phi0=0.37`, `nphi=2*lmax+3`, one warm-up and five timed calls per process, reused native plans and workspaces, and one/eight-thread Rayon pools. Three process runs were used for T127/T255 and the temporary T360/T511/T720 screen. Baseline measurements came from commit `7c7e963`; runs were pinned to CPUs 0–7 on the AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`. Rust was `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, default 16 codegen units, LTO off, runtime AVX2/FMA dispatch and no `target-cpu=native`. Values are median process medians in milliseconds, one/eight threads:

| Bandlimit / plan | Setup | Scalar synthesis recurrence → table | Scalar analysis recurrence → table |
| --- | ---: | ---: | ---: |
| T127, production | `0.018 → 2.402 ms` | `1.884/0.542 → 0.962/0.343` | `1.114/0.579 → 0.702/0.347` |
| T255, production | `0.017 → 33.238 ms` | `13.579/4.018 → 6.574/1.681` | `6.553/2.244 → 5.011/1.565` |
| T360, temporary | `0.034 → 89.6 ms` | `26.145/6.918 → 16.184/5.132` | `15.558/5.712 → 11.997/5.368` |
| T511, temporary | `0.035 → 251.2 ms` | `60.979/15.962 → 43.239/12.873` | `36.171/10.142 → 32.731/14.435` |
| T720, temporary | `0.091 → 678.9 ms` | `153.981/43.642 → 115.706/34.477` | `97.507/27.644 → 89.412/37.918` |

The production table reduced T127 scalar synthesis by 49%/37% and analysis by 37%/38%. At T255 it reduced synthesis by 52%/58% and analysis by 24%/30%. Setup therefore amortizes after about 3–12 calls at T127 for an individual operation and 5–49 calls at T255; the slower analysis cases require the most reuse. The T255 process peak was 41,496 KiB with a 32.13 MiB table payload. One-shot procedural calls are poor table users: the Python procedural benchmark measured T255 scalar calls at about 38 ms versus 6.2 ms through a reused Python plan, because each procedural call rebuilds the table. The Python plan also allocates a Rust workspace per method call; its input copies were below 0.02 ms at T255.

Temporary tables at T360–T720 improved synthesis but gave mixed analysis results. At T511, analysis regressed by about 9% with one thread and 42% with eight. At T720, analysis improved by about 8% with one thread but regressed by about 37% with eight. Table construction reached 679 ms at T720. `/usr/bin/time -v` measured a process peak of 786,656 KiB with the T720 table. These measurements do not justify the additional memory or the multithreaded analysis regression, so the production cap stays at 34 MiB/T255.

Python plan comparisons used the installed `ducc0==0.41.0`, Python `3.11.15`, NumPy `2.4.6`, matching coefficients/maps and five timed calls per sample group. Repeated eight-thread runs had large scheduling/allocation outliers at some low bandlimits; the native reusable-workspace measurements above determine acceptance. At GL T127, Python-plan scalar synthesis/analysis medians were `0.921/0.629 ms` versus DUCC `0.728/0.670 ms` at one thread. At T255 they were `6.304/4.446 ms` versus `1.915/1.329 ms`. At T720, where recurrence is active, scalar timings were `149.967/93.233 ms` versus `24.223/14.870 ms`. Spin-1 at T720 measured `145.424/222.770 ms` versus `48.525/31.584 ms`. The maximum relative-L2 error across the tested GL Python comparisons was `9.123e-14`; table-versus-DUCC errors were T127 `1.070e-14/1.114e-14` and T255 `2.001e-14/2.277e-14` for synthesis/analysis. The T127/T255 table-vs-recurrence regression compares synthesis, analysis and both adjoints below `2e-12`; extreme `1e300` inputs select the scaled recurrence and match it exactly.

The installed DUCC wheel was compared with a local source build at the same pinned DUCC revision. The source CMake defaults use C++17, `-O3 -ffast-math -march=native`, IPO/LTO and stripped symbols. The installed wheel and source-built DUCC were close at T511/T720; the source build measured scalar GL synthesis/analysis at T511 `10.127/6.599 ms` (1T) and `3.091/1.153 ms` (8T), and T720 `23.931/15.076 ms` (1T) and `6.584/2.299 ms` (8T). The Rust release build retained portable runtime dispatch and strict floating-point semantics, so compiler flags do not make the two builds identical. `perf_event_paranoid=4` blocked hardware event profiling; the comparison reports elapsed time and memory rather than treating instruction counts as cycles. No FFT implementation or scheduling changed.

The independent read-only Sol review found no correctness blocker. It confirmed north/south parity, the equatorial singleton, all four operator weightings, and disjoint order/workspace ownership. It requested a more precise underflow guard and an asymmetric mirrored-cutoff regression. The table builder now rejects any active zero basis value except an exact equatorial parity zero; the new test checks an excluded northern ring and active reflected southern ring with nonzero `phi0`. The tests lock eligibility at T72, T127 and T255, and exclude T360+, f32, spin plans, CC and F1. Full all-feature Rust tests passed (125 plus 10 f32; 8 ignored), no-default-feature tests passed (115 plus 10 f32; 7 ignored), workspace Clippy with warnings denied passed, and `uv run pytest python/tests` passed (8 tests). Release DUCC/analytic checks passed for scalar through T2047 on GL/CC/F1, spin 1/2 T511 on all grids, and spin 1/2 T1023 exact analysis.

The complete tables, procedure, high-bandlimit screen, and code review disposition are in [the precomputation report](../reports/legendre-precomputation-2026-10-13.md). No FFT code changed.

### Torch-harmonics storage and Python workspace follow-up (2026-10-13)

Rechecked the source at torch-harmonics PR [#322](https://github.com/NVIDIA/torch-harmonics/pull/322), head 37751d3e90b8f12fe9439c52847f4c5d70d5a627 (open at inspection). Standard single-process transforms store rectangular order-degree-latitude tables with unsupported triangular modes masked to zero; synthesis makes degree contiguous by transposing the table. The folded CC/F1 analysis path computes order/degree blocks of at most 2^20 float64 basis values (8 MiB), then stores a full projection. The regular Legendre cache uses the helper's default maximum of 20 entries; the folded projection cache holds two. Distributed transforms can build local order and latitude shards. At T720 extents (721,721,721), the float64 payload is 2.79 GiB scalar or 5.59 GiB for two vector rows. Packing triangular support and using north/south parity gives a Rust scalar table of 716.87 MiB, one quarter of the dense payload. If both vector rows admit valid spin-reflection reconstruction, the two-row packed payload would be about 1.40 GiB; this is an untested shape estimate. On the reported 12,227 MiB GPU, the dense scalar and vector tables alone occupy about 23% and 47% of device memory. These estimates exclude other live tensors. PyTorch/torch-harmonics was not installed, so no direct Torch allocation, OOM reproduction, or runtime comparison was possible. The existing Rust table's T127/T255 gains and T511/T720 memory and analysis tradeoffs are the measured comparison; details are in the report.

A temporary plan-local `Mutex<Option<ShtWorkspace>>` experiment passed the candidate's focused, full Rust and nine Python tests, including concurrent calls on one plan. Its T511 GL spin-1 Python-plan medians changed from `132.955/290.418 ms` to `128.867/204.866 ms` (one-thread synthesis/analysis), and from `164.283/194.699 ms` to `170.243/196.993 ms` (eight threads). Host load exceeded 42 with unrelated compiler processes on the 32 logical CPUs; the one-thread analysis ranges overlapped (`202.807–325.046 ms` baseline, `196.628–208.499 ms` candidate), and eight-thread calls did not improve. A single 15.369 ms native workspace-allocation sample does not establish whole-transform savings. The candidate was reverted because a reproducible end-to-end gain was not established; Python plan methods retain per-call workspace creation. No API or numerical source change was committed.

### T72 compact scalar GL table gate expansion (2026-10-13)

The existing north-hemisphere triangular f64 table was enabled at T72 after measuring it against the recurrence implementation. The implementation change lowers the minimum table bandlimit from 111 to 72; all other GL, spin, precision, sampling, truncation, 34 MiB and exceptional-input guards remain. The T72 payload is 799,496 bytes (0.762 MiB), and plan construction changed from `0.037` to `0.653 ms`.

The native comparison used five alternating baseline/candidate process pairs on the AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`, with Rust 1.99.0, release `opt-level=3`, 16 codegen units, LTO off, AVX2/FMA runtime dispatch and no `target-cpu=native`. Geometry was f64 GL `73x147`, full `mmax=72`, `phi0=0.37`, reused native plan/workspace. A temporary local benchmark-harness edit increased timed repetitions from five to 25 per process and was restored after measurement. One-thread processes were pinned to CPU 30; eight-thread processes were pinned to CPUs 0–7. The host had unrelated compiler work on most CPUs. Values below are medians across the five process pairs in milliseconds:

| Operation | Recurrence 1T / 8T | Compact table 1T / 8T | Change 1T / 8T |
| --- | ---: | ---: | ---: |
| Synthesis | `0.401 / 0.522` | `0.353 / 0.352` | `-12.0% / -32.6%` |
| Analysis | `0.462 / 0.450` | `0.266 / 0.266` | `-42.4% / -40.9%` |

The setup increment repays after about 13/4 synthesis calls or 4/4 analysis calls at one/eight threads. A repeated S/A pair repays it after about three one-thread iterations or two eight-thread iterations. The short T72 Python-plan cross-check used `examples/python_perf_baseline.py`, Python 3.11.15, NumPy 2.4.6 and `ducc0==0.41.0`, with three process groups of five calls. Errors were maximum absolute/RMS/relative-L2 `7.709e-17 / 9.134e-18 / 5.409e-15` for synthesis and `5.221e-15 / 2.579e-16 / 7.052e-15` for analysis. The Python process timing at eight threads varied substantially and is not the native speedup evidence.

Focused `cargo test -p rustsht --lib precomputed_scalar` passed five tests. The T72 table-versus-recurrence case checks synthesis, analysis, both adjoints and both adjoint inner-product identities. Full validation passed: format check; Clippy with all targets/features and warnings denied; all-feature tests (125 Rust and 10 f32 passed; 8 ignored); no-default-feature tests (115 Rust and 10 f32 passed; 7 ignored); and `uv run pytest python/tests` (8 passed). The rebuilt Python plan compared T72 GL scalar synthesis and analysis directly with DUCC as described above. This retains the bounded table already in the design; no dense matrix, dependency or FFT change was added. The source and tests were committed as `d7f3dea` and pushed to `main`. The read-only Sol review found no numerical or bounds blocker and verified the table-size and break-even calculations. It flagged that the earlier report sentence describing three process runs did not cover T72; the report now identifies the T72 evidence separately as five alternating process pairs with 25 timed samples per process. No code change was requested from review.

### T511 spin-2 analysis Callgrind profile (2026-10-13)

Callgrind 3.26.0 ran `taskset -c 30 valgrind --tool=callgrind --callgrind-out-file=/tmp/rustsht-spin2-t511-analysis.callgrind target/release/examples/m1b_bench 2 511 1 analysis GL STANDARD` on the Ryzen 9 5950X. The workload was f64, GL `512x1025`, STANDARD analysis, and one-thread execution. The Rust 1.99.0 release benchmark took five samples per analysis path. Callgrind recorded 45.92 billion instruction references over the process; these are instruction counts, not cycles or native elapsed time. The paired scalar recurrence and analysis accumulator accounted for about 88.2% of references, and RustFFT forward transforms accounted for 2.4%. The first profile part recorded 90.7 million calls to `multiply_complex_scaled_product`. Normal-range and scaled-fallback work were not counted separately. Valgrind emitted a `brk segment overflow` warning but exited successfully and wrote both profile parts. The measured instruction distribution and limitations are detailed in [the precomputation report](../reports/legendre-precomputation-2026-10-13.md#t511-spin-2-analysis-instruction-profile). This profile does not justify a dense-table change or an FFT optimization.

### Safe borrowed Python inputs (2026-10-13)

`PyReadonlyArrayDyn::as_slice()` now supplies the C-contiguous NumPy input directly to Rust operations. The `PyReadonlyArrayDyn` value stays alive while `Python::detach` runs, and only the safe borrowed slice crosses into the detached closure. No raw pointer cast or numerical-kernel copy was added. Input shape, dtype extraction and standard-layout checks are unchanged. The Python API documents that callers must not mutate the same storage through another alias until the operation returns. The tests cover read-only arrays and simultaneous read-only calls sharing one array; they do not attempt the prohibited concurrent mutation.

The comparison used `examples/python_perf_baseline.py`, T511 GL f64 (`512x1025`, `phi0=0.37`), Python 3.11.15, NumPy 2.4.6, DUCC 0.41.0 and the Rust 1.99.0 release extension on an AMD Ryzen 9 5950X. One-thread runs were pinned to CPU 30 and eight-thread runs to CPUs 0–7. Each process group warmed the operation and took five timed samples; five process groups were collected for both the copied-input baseline from commit `7b6acec` and the borrowed-input candidate. Plan methods reuse the plan but still allocate `ShtWorkspace` and output arrays per call.

| Operation | Rust 1T copied → borrowed | DUCC 1T | Rust 8T copied → borrowed | DUCC 8T |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | `60.243 → 59.899 ms` | `10.188 ms` | `15.858 → 15.641 ms` | `2.819 ms` |
| Scalar analysis | `33.820 → 33.306 ms` | `6.652 ms` | `9.934 → 9.872 ms` | `1.005 ms` |
| Spin-1 synthesis | `62.118 → 60.490 ms` | `21.552 ms` | `23.348 → 23.456 ms` | `4.395 ms` |
| Spin-1 analysis | `96.010 → 96.519 ms` | `13.182 ms` | `25.580 → 24.563 ms` | `1.964 ms` |

The process-level ranges overlap for most operations; the eight-thread spin-1 analysis medians differ by 4.0%, but the baseline and candidate ranges are separated by only 0.037 ms. No repeatable whole-transform speedup is established. Same-sized NumPy copy checks took about 0.05/0.075 ms for scalar coefficients/maps and 0.07/0.14 ms for spin-1 coefficients/maps; these checks estimate copy scale but do not time Rust's old `to_vec()` call exactly. The numerical errors against DUCC were unchanged: scalar synthesis/analysis relative L2 `6.632e-14/3.484e-14`, spin-1 `6.697e-14/5.128e-14`. The binding no longer allocates a second input vector; at T511 this avoids about 6.0 MiB for a scalar call and 12.0 MiB for a spin-1 call. It does not reduce the remaining DUCC execution gap. Formatting, workspace Clippy, all-feature Rust tests (125 plus 10 f32 passed, 8 ignored), no-default-feature Rust tests (115 plus 10 f32 passed, 7 ignored) and all nine Python tests passed. No workspace-reuse or output-buffer API changed.

### T511 scalar analysis Callgrind profile (2026-10-13)

Callgrind 3.26.0 ran `taskset -c 30 valgrind --tool=callgrind --callgrind-out-file=/tmp/rustsht-scalar-t511-analysis.callgrind target/release/examples/m1b_bench 0 511 1 analysis GL STANDARD` on the Ryzen 9 5950X. The workload was f64, GL `512x1025`, STANDARD analysis, and one-thread execution. Callgrind collected 8.02 billion instruction references over the process; these are not cycles or native elapsed time. The paired recurrence visitor accounted for 86.6% of references, with its recurrence vectorizer at 46.3% exclusive cost. The analysis accumulator vectorizer accounted for 11.6%; scaled multiplication helpers accounted for about 9.0% combined. Valgrind emitted a `brk segment overflow` warning but exited successfully and wrote the profile. The profile and its limits are recorded in [the precomputation report](../reports/legendre-precomputation-2026-10-13.md#t511-scalar-analysis-instruction-profile). These counts do not support extending the full table past T255; the T720 eight-thread analysis screen regressed 37% with 716.9 MiB of table memory.

### Rejected single-state recurrence range check (2026-10-13)

A candidate replaced the per-degree check of both recurrence states with a check of only the newly computed state, on the assumption that the other state had already been checked. A focused invariant test disproved that assumption: the established predicate accepts a pair when its maximum magnitude is in `[SMALL, BIG]`, so a subnormal new value can remain in range while its partner is at `BIG`. Checking the subnormal value alone would trigger scaling and change the recurrence's binary exponent path. No performance measurement was made, and the candidate was removed before transform validation. Keep the paired-state check unless an alternative is shown equivalent for near-zero degree pairs and SIMD tails.

### SIMD paired-state range check (2026-10-13)

The scalar paired recurrence checks whether any latitude lane's two recurrence states leave the ordinary-number interval before deciding whether to rescale. Replacing its scalar lane scan with a Pulp vector scan removes repeated scalar comparisons in scalar and spin-1/2 scalar-conversion analysis. The SIMD predicate preserves the pairwise maximum rule, handles NaNs with `f64::max` semantics, and uses the scalar predicate for the non-vector tail. It does not change recurrence values or accumulation order.

The benchmark used the reused-workspace native `m1b_bench` release example with f64 GL `T511` and `T1023`, deterministic inputs, five timed samples per process, and five alternating baseline/candidate process pairs. Baseline was commit `b20f7c2`; candidate was the working diff. Both were built with Rust `1.99.0`, release `opt-level=3`, default 16 codegen units, LTO off, portable Pulp runtime dispatch (AVX2/FMA on the Ryzen 9 5950X), and no `target-cpu=native`. Processes were pinned to CPUs 0–7 on Linux `7.0.14-20-pve`. Values are medians of the five process medians; ranges are the minimum and maximum process medians, in milliseconds:

| Case | Baseline 1T | Candidate 1T | Change | Baseline 8T | Candidate 8T | Change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar analysis T511 | `31.574 [31.408–32.479]` | `26.070 [25.797–26.414]` | `-17.4%` | `9.412 [9.275–9.869]` | `8.681 [8.472–8.967]` | `-7.8%` |
| Scalar analysis T1023 | `236.031 [232.663–239.510]` | `191.275 [189.024–191.831]` | `-19.0%` | `70.278 [69.872–71.803]` | `64.420 [63.784–64.719]` | `-8.3%` |
| Spin-1 analysis T511 | `93.459 [90.956–94.067]` | `75.354 [74.984–76.292]` | `-19.4%` | `24.061 [23.718–24.854]` | `21.830 [21.748–22.145]` | `-9.3%` |

The synthesis controls did not show a material change: scalar synthesis T511 shifted by `-1.6%/-2.5%` at one/eight threads, and spin-1 synthesis shifted by `0.0%/+1.2%`; process ranges overlapped. The performance effect is therefore concentrated in the range-check use within analysis paths. No direct Torch-harmonics execution comparison was possible in this environment; table memory, setup and reuse trade-offs are documented in [the Legendre precomputation report](../reports/legendre-precomputation-2026-10-13.md).

The focused SIMD test compares scalar and dispatched predicates over vector blocks and tails, including mixed valid/failing lanes at both a packed-vector index and the scalar tail, signed finite bounds, zero, NaN and infinity. The read-only Sol review found no correctness blocker; it confirmed exceptional-value and boundary behavior and suggested the mixed-lane case now covered by the test. `cargo test --release -p rustsht --lib ducc` passed 30 tests; release scalar T511/T1023/T2047 GL/CC/F1 and spin-1/2 T511 GL/CC/F1 DUCC regressions passed. Full checks passed: formatting; workspace Clippy with warnings denied; all-feature tests (126 Rust and 10 f32 passed, 8 ignored); no-default-feature tests (116 Rust and 10 f32 passed, 7 ignored); and Python tests (9 passed). The DUCC high-band fixtures report pass/fail rather than aggregate max-error values, so no additional error statistic is claimed. No FFT, public API, table, or numerical tolerance changed.

### Rejected two-accumulator analysis experiment

An f64 scalar paired-analysis candidate alternated vector blocks across two accumulators per channel to shorten accumulation dependency chains. Five alternating native process pairs measured GL STANDARD analysis with reused plans/workspaces, one warm-up and five timed samples per process, on the Ryzen 9 5950X with Rust 1.99.0 release settings and portable AVX2/FMA dispatch. T511 scalar analysis changed from `26.169` to `27.467 ms` at one thread and `8.697` to `8.659 ms` at eight threads; T1023 scalar analysis changed from `190.877` to `200.935 ms` and `63.973` to `64.252 ms`; spin-1 T511 analysis changed from `75.860` to `82.040 ms` and `21.675` to `22.189 ms`. The candidate code body nearly doubled in size, with no YMM stack spills visible in the vector body. Focused scalar-reference, extreme-finite-input, spin-1 and DUCC tests passed, but the one-thread regressions and neutral/slower eight-thread measurements did not justify the change. The candidate was reverted. Full ranges and method details are in the [precomputation report](../reports/legendre-precomputation-2026-10-13.md#rejected-two-accumulator-analysis-experiment).

### Mixed-exponent SIMD analysis accumulation (2026-10-13)

The paired scalar analysis reducer previously used SIMD accumulation only when every latitude lane had exponent zero and normal-range basis values. If one lane needed binary scaling, the callback multiplied all lanes with scalar scaled helpers. The earlier T511 Callgrind profile assigned 11.6% of process instruction references to the analysis accumulator vectorizer and about 9.0% to scaled complex multiplication helpers. The new branch SIMD-accumulates exponent-zero lanes whose basis is zero or in `[SMALL, BIG]`; lanes with a nonzero exponent, subnormal or out-of-range basis, NaN, or infinity use the existing scaled helper. `analysis_products_safe` still gates all direct products and reductions. The all-normal path is unchanged, and the candidate adds no workspace arrays or transform-sized storage.

Native comparisons used `examples/m1b_bench.rs` with f64 GL STANDARD analysis, full `mmax`, `phi0=0.37`, `nphi=2*lmax+3`, reused plans and workspaces, one warm-up and five timed calls per process. The baseline binary came from `0cdfbcf`; the candidate was built from the final worktree. Three alternating baseline/candidate process pairs ran on CPUs 0–7 of an AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`. Rust was `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch (AVX2/FMA), and no `target-cpu=native`. Values are medians of process medians with the min–max range in milliseconds:

| Operation | Bandlimit | Baseline 1T | Candidate 1T | Change | Baseline 8T | Candidate 8T | Change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar analysis | T255 | `4.771 [4.710–4.906]` | `4.238 [4.230–4.397]` | `-11.2%` | `1.436 [1.420–1.437]` | `1.472 [1.322–1.510]` | `+2.5%` |
| Scalar analysis | T511 | `26.228 [25.818–26.537]` | `23.747 [23.667–23.897]` | `-9.5%` | `8.473 [8.385–8.690]` | `7.505 [7.167–7.562]` | `-11.4%` |
| Scalar analysis | T1023 | `189.855 [189.804–196.505]` | `179.946 [178.149–180.466]` | `-5.2%` | `64.650 [63.751–64.863]` | `60.932 [60.908–62.077]` | `-5.8%` |
| Spin-1 STANDARD analysis | T255 | `14.824 [14.807–15.066]` | `11.687 [11.514–11.751]` | `-21.2%` | `5.447 [5.274–5.448]` | `3.648 [3.616–3.714]` | `-33.0%` |
| Spin-1 STANDARD analysis | T511 | `77.061 [76.703–77.666]` | `66.074 [66.066–66.125]` | `-14.3%` | `22.081 [21.903–22.458]` | `17.132 [17.131–17.370]` | `-22.4%` |
| Spin-1 STANDARD analysis | T1023 | `531.046 [529.801–537.344]` | `493.537 [492.631–500.033]` | `-7.1%` | `152.413 [152.265–153.234]` | `138.055 [137.907–138.901]` | `-9.4%` |

Scalar T255 eight-thread ranges overlap; the measured `2.5%` regression is within run variability and does not establish an effect. The other table rows show separated baseline/candidate ranges. The speedup is from reused-workspace native execution; no reduction in Python boundary or setup cost is attributed to it.

The final release Python extension was compared with installed `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Three run groups each took five timed calls for T511 GL f64 plan analysis. The plan method allocates its workspace per call. Rust/DUCC medians in milliseconds were scalar `24.125/6.496` at one thread and `7.783/1.003` at eight threads; spin 1 `65.218/12.604` and `17.744/1.924`. The scalar eight-thread Rust samples had a `2.930 ms` run-group spread; this Python result is not used to estimate the native kernel gain. Maximum absolute, RMS, and relative-L2 errors were `1.238e-14`, `1.779e-16`, and `3.484e-14` for scalar analysis, and `1.788e-14`, `2.619e-16`, and `5.128e-14` for spin-1 analysis. Rust still takes about 2.5–9.2 times longer in these Python-plan comparisons.

Focused tests cover extreme finite products that overflow before longitude scaling, mixed exponent-zero and scaled lanes with vector and scalar tails, and subnormal bases that must use the scaled path. The read-only Sol review found no correctness blocker and asked for the full all-features suite after the final inactive-basis mask; that suite passed after the mask. Final checks passed: `cargo fmt --all -- --check`, workspace Clippy with warnings denied, all-feature tests (128 Rust and 10 f32 passed, 8 ignored), no-default-feature tests (118 Rust and 10 f32 passed, 7 ignored), `uv run pytest python/tests` (9 passed), release DUCC tests (30 passed, 7 ignored), T2047 scalar GL/CC/F1 DUCC and analytic checks, and T1023 spin exact analysis against DUCC. No numerical tolerance or summation contract was weakened. Full measurements and review notes are in [the precomputation report](../reports/legendre-precomputation-2026-10-13.md#mixed-exponent-simd-analysis-accumulation).

### T511 spin-1 analysis profile and worker-granularity screen (2026-10-13)

With hardware counters unavailable (`perf_event_paranoid=4`), Callgrind 3.26.0 profiled the one-thread f64 T127 GL spin-1 STANDARD analysis benchmark using `taskset -c 30 valgrind --tool=callgrind --callgrind-out-file=/tmp/rustsht-4c732e5-spin1-t127-analysis.callgrind target/release/examples/m1b_bench 1 127 1 analysis GL STANDARD`. The complete benchmark process recorded 451,137,951 instruction references; the benchmark warms and measures two Rayon pool configurations, so these counts are not a single transform and are not cycles. `analyze_spin_order_group` accounted for 86.62% inclusive references, `analyze_spin_scalar_fast_order` for 86.53%, and RustFFT forward transforms for 9.29%. Spin analysis's scalar-conversion recurrence remains the dominant measured region; FFT remains outside this optimization phase.

Temporary leaf timing around spin-1 T511 analysis showed eight recursive leaves each carrying an estimated 7.35–7.64 million degree-lane operations. Their elapsed times ranged from 7.38 to 10.46 ms, with all leaves starting within 0.35 ms and finishing between 7.66 and 10.64 ms. This points to moderate cost variation despite near-equal coefficient-work weights, rather than a branch that waited to start. Timing hooks were removed after the measurement.

A controlled scheduler candidate used all 16 recurrence workspaces already allocated by the T511 synthesis-oriented workspace instead of limiting analysis recursion to the eight Rayon workers. This doubled the number of independent order tasks without increasing workspace allocation. Three alternating process pairs used the release `m1b_bench` with one warm-up and five timed calls, on the Ryzen 9 5950X with affinity to CPUs 0–31. Results are median process medians with ranges across pairs, in milliseconds:

| Analysis | Threads | Current schedule | 2x task candidate | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar T511 | 1 | `24.283 [24.000–24.562]` | `23.684 [23.564–23.726]` | `-2.5%` |
| Scalar T511 | 8 | `7.562 [7.443–7.597]` | `7.712 [7.674–7.961]` | `+2.0%` |
| Spin-1 T511 | 1 | `66.695 [65.734–67.497]` | `65.591 [65.401–66.465]` | `-1.7%` |
| Spin-1 T511 | 8 | `17.744 [17.722–20.961]` | `17.753 [17.151–18.652]` | `+0.1%` |

The one-thread spin difference is not meaningful because that path is serial. At eight threads, scalar analysis was slower and spin-1 analysis was unchanged within run spread. Peak RSS ranges also overlapped. The candidate passed focused scheduling tests, then was reverted; no dynamic task-granularity change was accepted. A separate current-source thread-count screen gave T511 scalar analysis medians of 23.736/14.624/10.357/7.715/7.638/7.077 ms and spin-1 medians of 66.764/37.857/26.124/17.617/16.368/13.769 ms for 1/2/4/8/16/32 threads, respectively. These are scaling screens, not paired before/after measurements.

The fast-versus-general screen below confirms the scalar-conversion route is much cheaper through the tested low and high bands. Active-work leaf timing, path-status counters and the twofold-task experiment are now recorded; they do not support another scheduler change. Detailed comparative matrix-storage results and Torch-harmonics limitations remain in [the Legendre precomputation report](../reports/legendre-precomputation-2026-10-13.md).

### Spin-1 analysis fast-path comparison and conversion-factor experiment (2026-10-13)

A temporary build disabled only the spin-1 scalar-conversion fast analysis path; synthesis, spin-2 dispatch and the production source remained unchanged. Three alternating process pairs compared this binary with the current fast-path binary at f64 GL T63/T127/T255/T511 using `examples/m1b_bench.rs`. Each process used one warm-up and five timed samples for its one-thread and eight-thread operations, was pinned to CPUs 0–7 on the Ryzen 9 5950X, and used the Rust 1.99.0 release build with portable Pulp runtime dispatch. Values are medians of the three process medians and ranges in milliseconds:

| Bandlimit | Fast path 1T | General path 1T | Fast path 8T | General path 8T |
| --- | ---: | ---: | ---: | ---: |
| T63 | `0.376 [0.351–0.464]` | `2.353 [2.331–2.452]` | `0.208 [0.188–0.218]` | `0.445 [0.438–0.499]` |
| T127 | `2.177 [2.015–2.289]` | `17.911 [17.898–17.995]` | `0.958 [0.933–0.969]` | `3.012 [2.979–3.261]` |
| T255 | `11.250 [11.141–11.310]` | `140.362 [140.107–140.937]` | `3.714 [3.638–3.716]` | `24.610 [24.303–24.636]` |
| T511 | `64.722 [64.678–65.032]` | `1083.668 [1082.033–1085.000]` | `17.272 [17.141–17.349]` | `201.072 [200.925–202.373]` |

The fast path was 2.1–16.7 times faster at one thread and 2.1–11.6 times faster at eight threads. This confirms that the existing spin-1 conversion eligibility from its low-bandlimit gate is valuable; no cutoff change is needed. The production general recurrence remains as the polar and unsupported-domain fallback. The fast/general all-grid analysis-and-adjoint test passed through T255, and the release T511 GL/CC/F1 DUCC/roundtrip test passed.

A separate private experiment replaced two square-root expressions in `scalar_to_spin1` with the order's already-prepared scalar `epsilon` factors. Disassembly of that helper reduced its body from 225 to 209 instructions and scalar `sqrtsd`/`divsd` instructions from 3/3 to 1/1. Three paired process runs at the same GL workloads did not establish a whole-transform gain:

| Bandlimit | Baseline 1T | Candidate 1T | Baseline 8T | Candidate 8T |
| --- | ---: | ---: | ---: | ---: |
| T63 | `0.351 [0.344–0.364]` | `0.346 [0.329–0.384]` | `0.234 [0.219–0.234]` | `0.233 [0.208–0.255]` |
| T127 | `2.035 [2.008–2.084]` | `1.991 [1.976–2.194]` | `0.957 [0.929–0.973]` | `0.988 [0.964–1.011]` |
| T255 | `11.295 [11.230–11.376]` | `11.231 [11.026–11.472]` | `3.656 [3.603–3.690]` | `3.708 [3.615–3.841]` |
| T511 | `65.860 [65.425–66.209]` | `64.752 [64.562–65.808]` | `16.856 [16.796–17.337]` | `17.640 [16.315–20.247]` |

Ranges overlap at each bandlimit, and the candidate is slower at T127/T255 eight-thread medians and noisy at T511 eight threads. It passed the spin-1 recurrence equivalence, all-grid fast/general analysis and adjoint, DUCC T511 all-grid, and analytic checks, but source changes were reverted because the instruction reduction did not improve whole-transform timing beyond measurement variability. It used no additional memory. The helper-level assembly change does not justify keeping a candidate on its own.

### Bounded spin-1 GL analysis table (2026-10-14)

Commit `a151ab1` adds a lazy, compact scalar-basis table for repeated spin-1 STANDARD exact analysis on f64 GL T72/T127. It extends the existing triangular north-hemisphere table through degree `lmax+1`, uses mirrored degree parity, and keeps the general scaled recurrence for other grids, operations, inputs above the safe scale, and larger bands. The table payload is 0.783 MiB at T72 and 4.094 MiB at T127. Spin table construction is lazy: Rust plan construction remained 0.018–0.029 ms, while the first native analysis includes table setup.

Three alternating process pairs compared commit `552fd83` recurrence with the candidate using `m1b_bench`, f64 GL, full `mmax`, `phi0=0.37`, a reused native plan/workspace, one warm-up and five timed samples per process, and one/eight Rayon threads. The Ryzen 9 5950X processes were pinned to CPUs 0–7; rustc 1.99.0 release used `opt-level=3`, 16 codegen units, no LTO or native CPU target, and portable Pulp runtime dispatch (AVX2/FMA). Values are median milliseconds with process ranges:

| Bandlimit | Recurrence 1T / 8T | Table 1T / 8T | Improvement 1T / 8T |
| --- | ---: | ---: | ---: |
| T72 | `0.438 (0.429–0.444) / 0.210 (0.205–0.211)` | `0.278 (0.263–0.285) / 0.145 (0.127–0.147)` | `36.5% / 31.0%` |
| T127 | `2.023 (1.977–2.025) / 0.940 (0.899–0.952)` | `1.074 (1.068–1.078) / 0.510 (0.510–0.545)` | `46.9% / 45.7%` |

First-call analysis medians were recurrence/table `0.507/1.070 ms` at T72 and `2.279/5.222 ms` at T127 with one thread. The fresh-plan eight-thread measurements were `0.257/1.013 ms` and `1.084/4.430 ms`; these include a separate prior cold one-thread call in the same process and are approximate. From the first-call surcharge and steady-state savings, setup repays after about five total analyses at T72/T127 one thread and thirteen/nine at eight threads. Peak RSS increased about 732 KiB/3,968 KiB at T72/T127; these process deltas are approximate, while the table payload is the exact size. The direct north/south parity regression was added after an independent review found cancellation in an iterative reconstruction; the corrected sums were rebenchmarked and retained.

The rebuilt Python plan was compared against installed `ducc0==0.41.0` on the same deterministic maps. Relative-L2 coefficient errors were `5.070e-15` at T72 and `1.006e-14` at T127; maximum absolute errors were `2.434e-15` and `6.999e-15`. Python-plan latency was Rust/DUCC `0.278/0.288 ms` (T72 1T), `0.311/0.407 ms` (T72 8T), `1.129/1.361 ms` (T127 1T), and `3.929/2.705 ms` (T127 8T). The Python binding allocates a workspace per call, and its T127 eight-thread results vary more than the native benchmark; these timings validate the interface and numerical output, not the native performance acceptance. Torch-harmonics was not installed, so no Torch GPU timing or allocation was performed. The compact table is a bounded symmetry-reduced specialization, not a second dense or blocked Torch-style matrix; storage and source comparisons remain in [the report](../reports/legendre-precomputation-2026-10-13.md#torch-harmonics-storage-and-rust-table-layout).

After the parity fix, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (131 Rust and 10 f32 passed; 8 ignored), `cargo test -p rustsht --no-default-features` (121 Rust and 10 f32 passed; 7 ignored), and `uv run pytest python/tests` (9 passed) succeeded. The opt-in release `m3_spin_t511_all_grids_matches_ducc_and_roundtrips` check also passed for spins 1/2 on GL/CC/F1. The focused table tests cover concurrent first use, GL T72/T127 selection, T255 recurrence fallback, fast/general and adjoint equivalence, finite `1e150` fallback, parity cancellation, and asymmetric polar cutoffs. No FFT, public API or Torch implementation changed.

### Python caller-provided plan outputs (2026-10-14)

`PythonShtPlan.synthesis_into(alm, out)` and `analysis_into(map, out)` now write scalar and spin STANDARD results into caller-owned NumPy arrays. They validate exact shape, dtype, C-contiguity and writeability, reject overlapping buffers, and call the existing Rust plan kernels while the GIL is released. Input/output NumPy owners remain alive for the call; callers must not access either buffer through another alias until return. Allocating methods call the same dispatch helpers and continue transferring their owned result vectors to NumPy without a final data copy. This API avoids allocating a fresh result array for repeated plan calls; it does not change the numerical kernels.

Focused tests compare allocating and caller-output methods exactly for scalar and spin-1 GL, CC and F1 plans with nonzero `phi0`, and check shape, layout, dtype, read-only and overlap errors. `cargo check -p rustsht-python`, `cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test --workspace --all-features` (131 Rust tests and 10 f32 tests passed; 8 ignored), and `uv run pytest python/tests` (16 passed) succeeded. The release extension also passed T511/T720 scalar and spin-1 equality checks against allocating plan methods at one and eight threads.

The release comparison used Python 3.11.15, NumPy 2.4.6, Rust 1.99.0, the Ryzen 9 5950X, T511/T720 GL f64, `phi0=0.37`, identical deterministic inputs, warm reused plans, and one/eight Rayon threads. One-thread runs were pinned to CPU 30; eight-thread runs to CPUs 0–7. Each cell used three process runs with one warm-up and seven alternating timed pairs; values below are medians of process medians in milliseconds. The process-to-process variation was several percent for some spin cases.

| Bandlimit | Operation | Threads | Allocating | Caller output |
| --- | --- | ---: | ---: | ---: |
| T511 | Scalar synthesis | 1 / 8 | `54.659 / 14.736` | `54.647 / 14.452` |
| T511 | Scalar analysis | 1 / 8 | `24.857 / 7.466` | `24.586 / 7.446` |
| T511 | Spin-1 synthesis | 1 / 8 | `61.236 / 22.657` | `60.750 / 21.314` |
| T511 | Spin-1 analysis | 1 / 8 | `68.109 / 17.661` | `67.990 / 17.433` |
| T720 | Scalar synthesis | 1 / 8 | `135.636 / 40.820` | `135.435 / 39.488` |
| T720 | Scalar analysis | 1 / 8 | `65.853 / 20.337` | `65.803 / 20.183` |
| T720 | Spin-1 synthesis | 1 / 8 | `141.547 / 54.725` | `139.814 / 53.367` |
| T720 | Spin-1 analysis | 1 / 8 | `177.731 / 44.647` | `177.100 / 44.408` |

The timing changes range from negligible to a few percent and do not establish a whole-transform speedup: allocating methods already transfer their Rust-owned vector without copying its contents. Caller-provided output avoids repeated result allocation and supports buffer reuse. It avoids approximately 7.94 MiB per scalar map result or 15.88 MiB per spin-1 map result at T720, and 3.97 MiB or 7.94 MiB per scalar or spin coefficient result, respectively. No separate peak-RSS measurement was collected. These timings are RustSHT API comparisons, not DUCC parity measurements; numerical agreement with DUCC is covered by the existing core tests and the new methods are exactly equal to the allocating methods.

### Python plan workspace reuse (2026-10-14)

`PythonShtPlan` lazily creates and reuses one `ShtWorkspace`. The workspace lock is acquired after releasing the GIL and held through the Rust operation, so concurrent calls through one plan serialize without holding the GIL. Separate plan instances have independent workspaces. A poisoned lock discards the scratch and retries initialization. Procedural functions remain one-shot and create a plan and workspace per call. No numerical kernel or public method signature changed; each live plan now retains its workspace memory.

The comparison used a temporary source export of commit `0c15ffb` for the allocating-workspace baseline and the candidate Python binding rebuilt from the current source. Both used Python 3.11.15, NumPy 2.4.6, installed `ducc0==0.41.0`, f64 GL spin-1 STANDARD analysis, deterministic inputs, reused plan objects, `phi0=0.37`, and CPU affinity to CPUs 0–7 on an AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`. Rust was `1.99.0 (b940084d7 2026-09-28)`, LLVM 23.1.1, release `opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch (AVX2/FMA), with no native CPU target. Five process groups each used five timed calls. Values are medians of process medians and ranges in milliseconds:

| Bandlimit | Threads | Fresh workspace per call | Reused plan workspace | Median change |
| --- | ---: | ---: | ---: | ---: |
| T72 | 1 | `0.268 [0.267–0.273]` | `0.262 [0.261–0.263]` | `-2.2%` |
| T127 | 1 | `1.137 [1.135–1.170]` | `1.115 [1.111–1.119]` | `-1.9%` |
| T72 | 8 | `0.150 [0.142–0.306]` | `0.120 [0.117–0.284]` | ranges overlap; no gain established |
| T127 | 8 | `1.307 [1.156–1.432]` | `1.241 [1.145–4.002]` | ranges overlap; no gain established |

The independent native benchmark reported workspace construction at approximately `0.010/0.029 ms` for T72 and `0.018/0.201 ms` for T127 at one/eight threads (five calls in one process). These figures are small relative to execution and are not combined with the Python timing as if they were the same boundary. One-thread Python-plan analysis showed small, separated improvements; the eight-thread measurements were noisy and do not support a speedup claim. Relative to installed DUCC, maximum absolute, RMS and relative-L2 analysis errors remained `2.434e-15`, `1.854e-16`, `5.070e-15` at T72 and `6.999e-15`, `2.108e-16`, `1.006e-14` at T127. The feature adds one retained workspace per plan; peak RSS was not separately measured.

Validation passed: workspace-wide Clippy with warnings denied, all-feature tests (131 Rust and 10 f32 passed; 8 ignored), no-default-feature tests (121 Rust and 10 f32 passed; 7 ignored), `uv run pytest python/tests` (9 passed), and formatting. Existing concurrent same-plan Python tests passed. The read-only Sol review found no actionable safety or benchmark issue and agreed that the one-thread results support only small gains, with no eight-thread claim. Detailed bounded Legendre-table versus Torch-harmonics storage measurements remain in [the precomputation report](../reports/legendre-precomputation-2026-10-13.md#torch-harmonics-storage-and-rust-table-layout).

### T255 spin-1 GL analysis table

The compact north-hemisphere table now also supports spin-1 STANDARD exact analysis through T255 when the table fits its existing 34 MiB cap. The scalar conversion requires degree `lmax+1`; at T255 this is GL degree 256 on 256 rings. For `m=0`, `P_256` is zero at every GL quadrature node, so the table records that exact null mode as zero instead of rejecting the table when the floating recurrence reaches a zero at one node. Other unrepresentable contributing entries still reject table construction and retain scaled recurrence. The T255 table contains 4,243,456 f64 values (`33,947,648` bytes, `32.375 MiB`). One table is retained per eligible plan after its first exact analysis.

The baseline was commit `41b3a94` (T255 remained on spin-1 scalar-conversion recurrence); the candidate was the current working source. Measurements used `examples/m1b_bench.rs`, f64 GL `256x513`, `phi0=0.37`, reusable native plans/workspaces, five timed operations per process after a warm-up, three process runs, Rust `1.99.0 (b940084d7 2026-09-28)` / LLVM `23.1.1`, Linux `7.0.14-20-pve`, release `opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch (AVX2/FMA), no `target-cpu=native`, and task affinity to CPUs 0–7 on the Ryzen 9 5950X. Values are medians and ranges of process medians in milliseconds:

| Path | Baseline 1T | Table 1T | Baseline 8T | Table 8T |
| --- | ---: | ---: | ---: | ---: |
| Warm analysis | `11.280 [11.233–11.568]` | `7.766 [7.745–8.507]` | `3.633 [3.585–3.643]` | `2.679 [2.619–3.224]` |
| First analysis | `11.588` | `39.067` | `3.527` | `34.865` |

Warm native analysis improved by `31.2%` with one thread and `26.2%` with eight threads; the process ranges are separated at both counts. Lazy table setup took about `31.3 ms` in the one-thread call and `32.2 ms` in the eight-thread call. With the measured steady-state savings, the table repays after approximately 9 total T255 analyses at one thread or 34 at eight threads. The measured peak RSS of the benchmark process rose from `13,268 KiB` to `79,624 KiB`; this harness briefly holds both its one-thread plan and a second cold eight-thread plan, so the `66,356 KiB` difference corresponds to approximately two table payloads. A single retained table adds `32.375 MiB` per plan.

The rebuilt Python plan gave RustSHT/DUCC medians of `12.270/3.250 ms` before and `8.101/3.087 ms` after at one thread, using three process medians of five samples. At eight threads the medians were `5.347/0.510 ms` before and `2.807/0.513 ms` after; RustSHT's process ranges were wide (`3.582–9.147 ms` and `2.796–9.995 ms`), so the Python data do not establish a multicore speedup. The native benchmark provides the matched eight-thread acceptance result. The DUCC reference was installed `ducc0==0.41.0` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Python one-thread errors changed from `7.745e-15` maximum absolute, `2.314e-16` RMS and `2.267e-14` relative L2 to `7.050e-15`, `1.722e-16` and `1.688e-14`; the candidate outputs agree more closely at these measures. Rust analysis also matches the scalar-conversion recurrence within `3e-10` relative-to-unit scale, and the static T255 DUCC coefficients plus the existing adjoint identity pass with the table enabled.

`cargo test -p rustsht spin_t255_gl_matches_ducc_samples_and_coefficients` and the focused table-extension test passed. The full workspace run also passed 131 Rust tests and 10 f32 tests (8 ignored); no-default-feature tests passed 121 Rust and 10 f32 tests (7 ignored); `uv run pytest python/tests` passed 16 tests. Formatting, workspace Clippy with warnings denied, and the opt-in release scalar T511/T1023/T2047 all-grid and spin-1/2 T511 all-grid tests passed. The strictly read-only Sol review found no numerical or bounds blocker, verified the T255 table payload and break-even estimates, and requested correction of stale scope/report statements; those statements are updated here and in [D23](00-scope-and-decisions.md#approved-decisions). The optimization is retained for repeated exact GL analyses through T255, subject to the 34 MiB cap. Its first-call setup and per-plan storage make it a poor choice for one-shot analysis; T255 plans that are reused fewer than the measured break-even count spend more total time than recurrence. No synthesis, CC/F1, f32, FFT or spin-2 path changed.

### Spin-1 GL table-backed synthesis (2026-10-13)

The existing north-hemisphere triangular `ScalarLegendreTable` now also backs f64 spin-1 STANDARD synthesis on exact GL grids. After `spin1_to_scalar` prepares the two scalar coefficient streams, each eligible degree accumulates directly from the cached table; southern values use `(-1)^(l+m)`, the equatorial lane is used once, and each ring retains its `1/sin(theta)` factor and order cutoff. The table is lazy and shared with spin-1 exact analysis. Synthesis enables it only for the current single-thread STANDARD call with E and B inputs, `127 <= lmax <= 191`, GL sampling, all ring sines at least `0.01`, and table-safe converted coefficients. The operation's eligibility flag is carried to the block kernel so a table previously built by analysis cannot enable GRAD_ONLY or partial-polar blocks. CC/F1, f32, multi-threaded execution, and unsafe or ineligible blocks retain the recurrence. No additional table representation or workspace was added.

The table payload is `4,292,608 bytes` (`4.09 MiB`) at T127 and `14,376,960 bytes` (`13.71 MiB`) at T191. T72 does not meet the synthesis bandlimit gate. The regression test compares table and recurrence outputs and checks the synthesis adjoint at T72, T127, T128 and T191; T128 uses 129 latitude rings and 258 longitudes, covering the equatorial singleton and even-longitude path. Its T255 case preloads the analysis table and exercises the synthesis gate above the measured-benefit ceiling. All cases use nonzero `phi0=0.37`, with sample and normalized-adjoint tolerances of `3e-11` and `2e-11`. Python comparisons with `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` measured:

| Band | Python plan Rust / DUCC synthesis | Maximum absolute error | RMS error | Relative L2 error |
| --- | ---: | ---: | ---: | ---: |
| T127 | `2.644 / 1.523 ms` | `1.757e-15` | `1.323e-16` | `4.344e-14` |
| T191 | `7.150 / 1.992 ms` | `2.733e-15` | `1.954e-16` | `4.278e-14` |

Python plan calls allocate a Rust workspace per operation; these values establish DUCC agreement, not native-kernel speedup. Native steady-state comparisons used release `examples/m1b_bench.rs`, reused plan/workspace, f64 GL STANDARD, full `mmax`, `phi0=0.37`, one warm-up and five timed samples per process. The baseline was commit `0f760cf625de3de3af1813876880096e96ef869b`; the candidate used the then-current worktree. Both used Rust `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO off, Pulp runtime AVX2/FMA dispatch, and no `target-cpu=native`. The CPU was an AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`; processes were pinned to CPUs 0–7. Values are medians and ranges of five process medians, in milliseconds:

| Band | Recurrence 1T | Table 1T | Change | Recurrence 8T | Table 8T | Change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T127 | `2.867 [2.833–2.955]` | `2.400 [2.369–2.459]` | `-16.3%` | `0.940 [0.910–1.001]` | `0.943 [0.902–0.969]` | `+0.3%` |
| T191 | `9.856 [9.702–9.955]` | `7.150 [7.035–7.275]` | `-27.5%` | `no material change` | `no material change` | `within run spread` |

For T191 eight-thread synthesis, 11 process runs per variant measured medians of `2.696 ms` for recurrence and `2.747 ms` for the candidate; ranges overlap broadly (`2.572–2.806` and `2.567–2.811 ms`), so no change is established. A temporary native first-call harness, using the same deterministic complex coefficients and reused workspace, measured recurrence/table first-call medians of `3.111/6.405 ms` at T127 and `10.342/20.584 ms` at T191. The corresponding warmed medians were `2.946/2.429 ms` and `10.154/7.107 ms`. The first synthesis adds about `3.294/10.242 ms`; the table repays that cost after about eight/five total calls at T127/T191 in this synthesis-only workload. If exact analysis already initialized the shared table on the same plan, synthesis has no additional table-build cost. The temporary harness was removed after measurement.

A follow-up compared T255 table synthesis with the recurrence at commit `0f760cf`, using the same release benchmark, affinity and five-sample process procedure. Across three alternating process pairs, recurrence/table medians were `11.945/12.265 ms` at one thread (`+2.7%`) and `4.387/4.447 ms` at eight threads (`+1.4%`). One-thread process ranges were narrowly separated (`11.878–12.056` and `12.106–12.298 ms`); the eight-thread ranges overlapped. The 32.375 MiB table therefore had no measured whole-transform benefit at T255. Synthesis eligibility is capped at T191; the independent T255 exact-analysis table path is unchanged.

The strictly read-only Sol review found no numerical, indexing, safety or performance-justification blocker. It identified that cached table availability could bypass the intended current-operation gate and requested an eligible odd-latitude/even-longitude test plus first-call timing; the operation gate was passed explicitly and the T128 case and amortization measurements were added. Full all-feature tests passed (132 Rust and 10 f32; 8 ignored), no-default tests passed (122 Rust and 10 f32; 7 ignored), Python tests passed (16), and formatting and workspace Clippy passed. Warm synthesis improves at one thread on T127/T191 but not at eight threads; the specialization is limited to repeated, single-threaded GL wind transforms through T191. DUCC remains about `1.6x` and `3.4x` faster than the warmed Python-plan method at T127 and T191 respectively. No FFT, analysis, adjoint, GRAD_ONLY, DERIV1, CC/F1, f32 or spin-2 algorithm changed.

### North/south paired spin-1 GL analysis (2026-10-13)

The spin-1 STANDARD exact-analysis scalar-conversion path now processes mirrored GL rings as one parity pair. It applies each ring's Fourier phase and its own quadrature, sine and longitude normalization before forming even/odd inputs for the scalar recurrence. The equatorial ring is processed once. Each ring's order cutoff is applied independently; inactive members contribute zero, and mixed active pairs remain valid. Partially active 64-pair blocks are compacted into the current worker's `RecurrenceWorkspace`; no workspace field, heap allocation or full-sized table was added. The original general spin recurrence remains the fallback when a paired block cannot use the normal-range scalar reducer. CC/F1 and other spin modes keep their existing paths. No FFT implementation changed.

Native f64 GL spin-1 STANDARD analysis was measured with `examples/m1b_bench.rs`, reusing a plan and workspace, `phi0=0.37`, full `mmax`, deterministic inputs, one warm-up and five timed calls per process. Three process runs were collected for each revision; the baseline was `39d52239317380c41dd64fe046d7be091555f01a`, and the candidate was based on that source plus the paired path. Rust was `1.99.0 (b940084d7 2026-09-28)` / LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO disabled, portable Pulp AVX2/FMA runtime dispatch, without `target-cpu=native`. Measurements ran on an AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`, pinned to CPUs 0–7. Values are medians and ranges of process medians in milliseconds:

| Bandlimit | Threads | Baseline | Paired path | Change |
| --- | ---: | ---: | ---: | ---: |
| T511 | 1 | `63.769 [63.616–63.860]` | `39.104 [38.479–39.343]` | `-38.7%` |
| T511 | 8 | `17.293 [16.966–17.328]` | `13.596 [12.411–15.001]` | `-21.4%` |
| T720 | 1 | `171.492 [168.778–173.569]` | `103.966 [103.732–104.239]` | `-39.4%` |
| T720 | 8 | `43.739 [43.395–43.918]` | `35.095 [34.407–39.763]` | `-19.8%` |

These are whole analysis times, including longitude FFT and surrounding transform work. The measurement confirms a net gain but does not isolate the Legendre-stage contribution. No hardware instruction counters were collected. The implementation reuses existing worker scratch; peak RSS was not measured separately.

The rebuilt Python plan was compared with installed `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, using matching f64 GL inputs, full bandlimits, `phi0=0.37`, and explicit one/eight thread settings. These are steady-state plan-method timings, with three process runs and five timed calls after warm-up; values are median milliseconds:

| Bandlimit | Threads | RustSHT | DUCC0 | Ratio |
| --- | ---: | ---: | ---: | ---: |
| T511 | 1 | `38.869` | `12.850` | `3.02x` |
| T511 | 8 | `12.546` | `1.989` | `6.31x` |
| T720 | 1 | `106.661` | `30.701` | `3.47x` |
| T720 | 8 | `37.025` | `4.461` | `8.30x` |

Maximum absolute / RMS / relative-L2 coefficient errors against DUCC were `1.401e-14 / 1.910e-16 / 3.740e-14` at T511 and `1.391e-14 / 2.200e-16 / 6.061e-14` at T720. The paired path also passed the full-latitude recurrence comparison, asymmetric cutoff and equatorial cases, an adjoint identity, and an extreme finite-input case that must select the existing fallback. The opt-in release T511 all-grid spin test and T1023 exact-analysis spin test passed; CC/F1 continue through their existing paths. The read-only Sol review of the focused diff found no actionable mathematical, cutoff, scratch-ownership, thread-safety or benchmark issue.

### Scalar synthesis mixed-range SIMD lane split (2026-10-13)

Callgrind on the preceding f64 T511 GL scalar-synthesis build attributed about `74%` of instruction references to the paired degree callback fallback and `19.8%` to the fused prepared SIMD kernel. `multiply_complex_scaled_product` accounted for `18.9%` exclusive and `25.0%` inclusive references. The fallback ran for an entire fully active latitude block when only some sectoral initial values required binary scaling. The new path first preserves the existing all-normal fused case. For a fully cutoff-active block with at least four ordinary initial lanes and at least one scaled lane, it compacts ordinary lanes into the existing worker buffers and evaluates them with the register-resident fused recurrence. It compacts the remaining lanes and advances them with the existing binary-scaled recurrence and contribution helpers. If there are fewer than four ordinary lanes, all lanes retain the existing fallback. Blocks with mixed polar cutoffs also retain their existing active-ring path. This changes no cutoff or recurrence scaling rule and adds no workspace allocation.

Native f64 scalar STANDARD synthesis used `examples/m1b_bench.rs`, reusable plans/workspaces, `nphi=2*lmax+3`, one warm-up and five timed calls per process. The baseline binary was built from `1102dbe3acf3a1535ec7c5bfad7f09e15666456c`; the candidate included the lane split. Each result is the median of process medians from paired baseline/candidate runs. Rust was `1.99.0 (b940084d7 2026-09-28)` / LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch (AVX2/FMA on this host), without `target-cpu=native`. Runs used an AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`, pinned to CPUs 0–7. Times are milliseconds at one/eight threads:

| Grid / bandlimit | Baseline | Lane split | Change |
| --- | ---: | ---: | ---: |
| GL T511 | `56.296/15.289` | `43.903/12.412` | `-22.0%/-18.8%` |
| CC T511 | `57.891/22.256` | `45.540/16.671` | `-21.3%/-25.1%` |
| F1 T511 | `59.801/16.176` | `45.999/12.934` | `-23.1%/-20.0%` |
| GL T720 | `140.566/41.011` | `104.690/35.390` | `-25.5%/-13.7%` |
| GL T1023 | `383.313/128.006` | `338.148/112.997` | `-11.8%/-11.7%` |

T511 GL used five paired process runs; the baseline/candidate ranges were `55.916–57.096 / 14.598–15.706 ms` and `43.455–44.461 / 12.161–12.882 ms`. T511 CC/F1 used three pairs; T720 used three pairs; T1023 used three pairs. The T511 and T720 Python-plan comparisons were rebuilt from the candidate and run against installed `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Rust/DUCC medians were `45.170/10.348 ms` (T511 1T), `12.829/1.289 ms` (T511 8T), `109.770/24.663 ms` (T720 1T), and `35.983/2.870 ms` (T720 8T). Python output-map errors were max absolute/RMS/relative L2 `3.495e-14/7.845e-16/6.632e-14` at T511 and `7.790e-14/1.423e-15/8.545e-14` at T720. Input copying remained below `0.12 ms` and output allocation below `0.002 ms` in these calls; neither explains the native improvement.

The matched T511 Callgrind harness fell from `14,387,331,653` to `11,170,026,565` instruction references (`-22.4%`). This is a simulated instruction count, not cycles or elapsed time. The candidate's fused `FusedSynthesisBlock` accounted for `21.7%`; `multiply_complex_scaled_product` no longer appeared among functions above 1% of references. The benchmark process includes repeated transforms and two one-thread Rayon pools, so these profiles establish the changed instruction distribution rather than per-call cycle counts. The split uses existing per-worker recurrence and compaction scratch, so workspace memory is unchanged.

The focused mixed-initial-lane test compares the partitioned result with the binary-scaled reference and verifies that the input contains both exponent-zero and exponent-scaled sectoral states. `cargo test --workspace --all-features` passed 135 Rust tests and 10 f32 tests (8 ignored); `cargo test -p rustsht --no-default-features` passed 125 Rust tests and 10 f32 tests (7 ignored); `uv run pytest python/tests` passed 16 tests; formatting and workspace Clippy with warnings denied passed. The opt-in release scalar DUCC/analytic check through T2047 passed on GL, CC and F1. Its maximum DUCC middle-row sample error was `2.083e-13` and maximum analytic map error was `9.958e-12`; scalar analysis coefficient errors were at most `4.175e-14`. The standard suite retained scalar synthesis adjoint checks and spin-3/4 regressions. Sol's read-only review found no actionable mathematical, cutoff, indexing, scratch ownership, memory safety or race issue. No FFT, spin-1 or spin-2 kernel changed.

`cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test --workspace --all-features` (134 Rust tests and 10 f32 tests passed; 8 ignored), `cargo test -p rustsht --no-default-features` (124 Rust tests and 10 f32 tests passed; 7 ignored), and `uv run pytest python/tests` (16 passed) passed. The one/eight-thread native gains justify retaining the paired GL analysis path. DUCC remains substantially faster, particularly at eight threads; further work should target the remaining recurrence and multicore gap rather than attribute this gain to binding or FFT changes.

## Adaptive Legendre strategy selection (2026-10-13)

The recurrence-versus-compact-table investigation and its committed raw measurements are in [`reports/adaptive-legendre-strategy-2026-10-14.md`](../reports/adaptive-legendre-strategy-2026-10-14.md). The internal selector is deterministic, chooses at transform time using the executing Rayon pool, and leaves the compact north-hemisphere table lazy under the existing 34 MiB cap. Scalar and spin-1 GL STANDARD strategies are operation-specific; f32, CC/F1, over-resolved GL, gradient/derivative modes, and unsupported bands retain recurrence/specialized kernels.

Current policy avoids the measured one-thread scalar synthesis table regressions below T127, scalar T255 full-analysis table regressions at one and sixteen-plus threads, spin-1 T72 one-thread analysis without a repeatable gain, and spin-1 full-mmax T255 analysis tables at eight-plus threads. Table creation remains first-use cost; the report gives reuse break-even and total times for representative transform counts. At the time of this initial selector report there was no caller-specific reuse hint or public strategy API. The 2026-10-14 follow-up below adds an optional estimate without hidden calibration. The exact-minimum GL measurements do not represent over-resolved atmospheric grids; full-grid T63/T85 controls remain recurrence-only.

The table-vs-recurrence regression suite covers all four scalar operators, DUCC/analytic comparisons, adjoint identities, equatorial and asymmetric cutoff cases, odd/even longitude counts, extreme inputs, f32 recurrence fallback, and concurrent/cross-pool reuse. Final checks passed: fmt, all-feature clippy, 141 all-feature Rust tests plus 10 f32 tests, 126 no-default Rust tests plus 10 f32 tests, and 16 Python tests. The general suites mark eight all-feature and seven no-default high-band tests ignored. Separate release runs passed scalar T511/T1023/T2047 across GL/CC/F1, spin T511 all grids, and spin T1023 exact analysis; other high-spin opt-in cases remain unrun. The T319–T1023 native recurrence screen is recorded in the report.

A temporary first-call leaf timer sampled spin-1 GL analysis at T511 and T720 with 8, 16 and 32 pinned Rayon threads. Estimated active degree-lanes per leaf ranged by 2–7% at 8–16 threads and 11–20% at 32 threads; measured leaf durations varied by roughly 1.7–2.8x. The probe timed one instrumented transform per process on a shared host, so it could not separate kernel cost from run-to-run worker and frequency variation. It did not establish a whole-transform scheduling gain, and the temporary instrumentation was removed without changing the weighted split. The existing multi-process q1–q16 scaling data remain the performance evidence for this scheduler.

## FFT-friendly longitude-count suggestion (2026-10-14)

The native GL screen compared the existing `nphi=2*lmax+3` benchmark default, the minimum odd sampling count and the smallest count with prime factors no larger than seven. At full `mmax`, the selected count reduced one-thread scalar and spin-1 STANDARD times by 6–28% from T63 through T1023, except T72 where the existing count 147 was already selected. At T511 the one-thread reductions were 6–10%; at T720 they were 8–13%. Eight-thread reductions were larger in many cases, though several shared-host samples had high variability. Reduced `mmax=floor(lmax/2)` screens at T72–T720 showed 7–38% one-thread and 17–58% eight-thread reductions against the existing full-`lmax` benchmark count. T720 used the refined smooth count 1458 rather than 1470.

`GridSpec::fft_friendly_nphi(mmax)` returns the measured smooth count as an explicit suggestion. `GridSpec::new` still uses the caller's exact `nphi`. The suggestion excludes nonlinear oversampling requirements. Python plan comparisons against installed DUCC 0.41.0 at T127/T511 used matched counts and reported maximum relative-L2 errors no larger than `6.7e-14`; T720 errors were below `9.2e-14`. The T127/T511 selected counts improved both RustSHT and DUCC one-thread times; T720 one-process comparisons showed the same direction. The q8 Python timings were too variable to use for selection.

Evidence, raw process samples, memory, setup costs, exact commands and grid shapes are in [`reports/fft-friendly-longitude-selection-2026-10-14.md`](../reports/fft-friendly-longitude-selection-2026-10-14.md) and its [`data`](../reports/data/fft-friendly-nphi-2026-10-14/) directory. This study changes no FFT implementation or transform kernel. Checks passed: formatting, workspace Clippy with warnings denied, 142 all-feature Rust tests and 10 f32 tests (8 ignored), 127 no-default Rust tests and 10 f32 tests (7 ignored), and 16 Python tests.

## Independent-field batching study (2026-10-14)

The read-only SHTns audit and RustSHT batch measurements are recorded in [`batched-transform-study-2026-10-14.md`](../reports/batched-transform-study-2026-10-14.md), with process samples and summaries under [`reports/data/batched-transform-2026-10-14`](../reports/data/batched-transform-2026-10-14/). On the AMD Ryzen 9 5950X, Rust 1.99.0 release build, GL f64 T127/T511, field-parallel execution with one-thread workspaces improved eight-field warm throughput by 2.0–3.4x against sequential workspace reuse at q=8. Two T511 fields lost to sequential execution; four fields generally helped. Pool-sized workspaces showed no consistent benefit and used more memory. T511 q=8 eight-field one-thread workspaces increased RSS by about 65 MiB; the workload peak was 197–262 MiB including field buffers. Native single-field Rust/DUCC ratios ranged from 2.4–4.5x at one thread and 1.6–7.4x at eight threads, depending on operation. Python-plan comparisons against installed DUCC 0.41.0 reported relative-L2 errors below `6.7e-14` for tested T127/T511 scalar and spin-1 operations.

A temporary two-field scalar synthesis recurrence-sharing prototype matched both ordinary Rust outputs but was about 9–11% slower at q=1 and only 0–8% faster with overlapping process variability at q=8. It was removed. No batch API or production transform change was justified; applications can run independent calls with separate workspaces through Rayon. FFT engine work remains deferred.

## Scalar north/south-paired synthesis experiment (2026-10-14)

The test-only T511 GL prototype reused the two-channel paired-degree kernel to synthesize eligible northern rings and reconstruct their southern partners through scalar parity. It preserved exact-grid outputs to relative-L2 `1.274e-14` at q1 and `1.936e-14` at q8; an over-resolved `513x1022` control exercised the equatorial singleton and even-longitude Nyquist case at `1.309e-14`. The fully active normal-range blocks covered only 14.1%/21.2% of the estimated active recurrence updates at q1/q8. Across three process medians the full transform was 10.4% slower at q1 and 6.2% slower at q8. Repeated per-block factor preparation and the two-channel scatter outweighed the recurrence work saved, so the prototype was removed. No production scalar or spin kernel changed. Raw results and the decision are in [`hemisphere-paired-synthesis-2026-10-14.md`](../reports/hemisphere-paired-synthesis-2026-10-14.md) and its [`data`](../reports/data/hemisphere-paired-synthesis-2026-10-14/) directory.

## Spin-1 direct recurrence comparison (2026-10-14)

The current scalar-conversion path was compared with the existing general spin recurrence for f64 GL STANDARD synthesis and analysis at T63, T72, T127, T255, T511 and T720, with one and eight threads. The over-resolved `ntheta=lmax+2` grids kept the compact scalar table out of both paths. At T511, three process medians gave scalar-conversion/general-recurrence times of `58.275/1156.332 ms` synthesis and `41.700/1070.976 ms` analysis at one thread; at eight threads they were `24.506/167.693 ms` and `13.149/198.434 ms`. At T720 the scalar-conversion path was 11.9–28.8 times faster. Cross-path relative-L2 errors were at most `9.55e-14` across tested operations and bands.

This comparison rejects the existing general recurrence as a replacement for scalar conversion. A separate test-only direct fused spin-1 recurrence prototype was 5.1–12.0 times slower than scalar conversion in paired f64 GL STANDARD synthesis at T127/T255/T511 and q1/q8, despite cross-path relative-L2 errors below `4.1e-13` and synthesis-adjoint errors below `3.2e-14`. Per-degree recurrence-safety reductions and repeated spin-factor preparation accompanied the fused loop; hardware counters were unavailable to attribute the slowdown. The prototype and benchmark controls were removed. The q8 general analysis path scaled better than scalar conversion at T511 but was still 15.1 times slower, so future work should profile the fast analysis path's multicore execution rather than fuse the slower general path without a work-reduction mechanism. Exact samples, dimensions, commands, source baseline and DUCC context are in [`spin1-direct-vs-scalar-conversion-2026-10-14.md`](../reports/spin1-direct-vs-scalar-conversion-2026-10-14.md) and its [`data`](../reports/data/spin1-direct-vs-scalar-conversion-2026-10-14/) directory.

### Spin-1 T511 analysis scaling and strategy re-audit (2026-10-14)

The T511 q1/q2/q4/q8/q16 native and installed DUCC 0.41.0 measurements, raw samples, and a temporary per-order scheduling-weight experiment are documented in [`spin1-analysis-thread-scaling-2026-10-14.md`](../reports/spin1-analysis-thread-scaling-2026-10-14.md). RustSHT scales `3.18x` from one to eight threads, versus `6.72x` for DUCC; its q8 latency is `12.378 ms` and q16 latency is `12.604 ms`. The q8 per-order-overhead scheduling candidate did not beat the existing weighted split beyond measurement variability and was removed. No production scheduling change was accepted.

The adaptive Legendre report already contains the current selector measurements and implementation. Source history confirms no Legendre recurrence or selector changes after `db3d230`; the paired spin-1 analysis improvement was included in the strategy study. The spin-1 analysis scaling sweep does not affect the compact table's bandlimit eligibility, so existing operation/thread-specific table policy remains unchanged. `sht_bench` was inspected read-only at `0d5462232c24ad926cc5a2e8bf653b131c93445f`, and SHTns at `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`; the strategy report records the transferable operation-specific selection idea and rejects hidden plan-time calibration.

### GL spin-1 analysis ring-scale hoist experiment (2026-10-14)

A prototype cached `(ring_weight / sin(theta)) / nphi` once per GL spin-1 plan and reused it across orders in the paired STANDARD analysis path. The arithmetic retained the previous conversion and division order. Paired native process measurements at T255/T511/T720 showed no repeatable q8 improvement; q1 was 1.3% slower at T511 and 1.6% slower at T720. The additional plan vector used 5.6 KiB at T720 and added several microseconds to plan construction. Focused recurrence, extreme-input fallback, static T255 DUCC and opt-in T511 all-grid spin tests passed, but there was no whole-transform gain, so the production prototype was removed. The detailed table and raw samples are in [`the experiment report`](../reports/spin-analysis-ring-scale-hoist-2026-10-14.md).

### Scalar paired-recurrence lane normalization fast path (2026-10-14)

The current scalar paired recurrence previously normalized every lane on every step after any lane required binary scaling. It now bypasses normalization when a lane has exponent zero and both values are already in the established normal range; all other lanes retain the scaled helper. On T511 f64 GL scalar analysis, five paired process runs improved the median by 3.33% at one thread and 3.32% at eight threads. Callgrind instruction references fell 3.34% for the same q1 case. Other T511 synthesis/spin-1 gains were smaller and some q8 measurements overlapped noise; no broad speedup is claimed. DUCC 0.41.0 coefficient comparisons, adjoints, extreme finite fallbacks, scalar high-band tests through T2047, spin-1/2 T511 all-grid checks, spin-3/4 shared-kernel regressions, all-feature/no-default Rust tests, Clippy, formatting and Python tests passed. The full timing, error and environment record is [`the report`](../reports/scaled-lane-normalization-2026-10-14.md), with paired native and DUCC samples under [`reports/data/scaled-lane-normalization-2026-10-14`](../reports/data/scaled-lane-normalization-2026-10-14/).

### Opposite-parity cosine hoist experiment (2026-10-14)

A temporary scalar and spin-1 analysis path preweighted odd-parity Fourier components by the fixed north-ring cosine once per ring pair, with a range guard against subnormal products. At T511, scalar analysis was 1.26% slower at one thread and flat at eight; spin-1 changes were also flat. T720 results were within run variation. Focused DUCC, recurrence-equivalence, adjoint and scaled-lane tests passed, but the block preparation and guard overhead removed the expected benefit, so the prototype was reverted. No source, workspace or API change remains. Raw paired runs and profile context are in [`the experiment report`](../reports/opposite-cosine-hoist-2026-10-14.md) and [`its data directory`](../reports/data/opposite-cosine-hoist-2026-10-14/).

### Paired recurrence range-check fusion experiment (2026-10-14)

A prototype fused the paired scalar recurrence's normal-range predicate into its vector update to avoid rereading the updated lane buffers. Five alternating T511 GL spin-1 analysis process pairs produced median changes of `-0.35%` at one thread and `-1.25%` at eight; both paired ranges crossed zero. Fifteen focused recurrence tests passed, and T511 DUCC coefficient error remained `3.740e-14` relative L2. Callgrind did not isolate the predicate's memory cost. The prototype was removed because the whole-transform change was within measurement variability. See [`the report`](../reports/paired-recurrence-range-check-2026-10-14.md) and [`raw samples`](../reports/data/paired-recurrence-range-check-2026-10-14/paired-process-samples.txt).

### Two-way SIMD accumulation for paired analysis (2026-10-14)

The f64 normal-range paired analysis reducer now updates two independent partial sums per complex component while processing two AVX2 latitude vectors per loop iteration. The AVX2 loop showed no vector-register spills. Native q1 medians improved by `1.55%` for spin-1 T511, `1.68%` for spin-1 T720, and `2.00%` for scalar T511. q8 measurements were inconclusive and are not claimed as gains. Callgrind instruction references for q1 T511 spin-1 analysis fell `1.74%`. DUCC coefficient errors, the spin-1/2 all-grid T511/T1023 regressions, focused fallback/tail tests, full Rust suites, Clippy, formatting and Python tests passed. No workspace memory was added. Full measurements and raw process samples are in [`the report`](../reports/paired-analysis-two-way-unroll-2026-10-14.md) and [`its data file`](../reports/data/paired-analysis-two-way-unroll-2026-10-14/paired-process-samples.txt).

## Parallel spin-1 synthesis coefficient preparation (2026-10-14)

Commit `c6b238d` adds lazy per-workspace coefficient scratch for f64 spin-1 GL STANDARD synthesis. At eight threads the selector builds the two scalar-conversion coefficient streams once per order in parallel and reuses them across latitude groups. The deterministic gate is `127 <= lmax <= 720`, q8-or-higher execution, multiple groups, and a payload at or below 17 MiB; all other cases retain recurrence conversion. It does not cache across calls because coefficient inputs can change.

Five paired native process runs found q8 warm reductions of 10.9% at T127, 17.8% at T255 and 11.2% at T720. Eight T511 pairs showed a median paired reduction of 8.8%, with process medians `20.895 -> 20.178 ms`; half-mmax T511 improved `20.142 -> 18.453 ms`. T720 q1 was unchanged, q4 gained only about 1.6% warm with several milliseconds of first-call cost, and q16 had a smaller positive median change. q4 remains on the old path. A forced T1023 full/half-mmax cache showed no repeatable gain and required up to 32 MiB, so the selected range ends at T720.

At T720 the cache payload is `15.886 MiB`; measured peak RSS increased about `16.2 MiB`. Median first call was `57.853/59.920 ms` (recurrence/cache), while warm calls were `51.346/46.441 ms`. Total-time estimates break even after approximately two calls, with shared-host variation suggesting a conservative two-to-three-call amortization. Focused cached/uncached component errors were at most `9.216e-19`. Release DUCC tests passed for spin 1/2 through T511 on GL/CC/F1, spin 1 through T1023 sparse synthesis, and spins 1–4 at T2047 sparse synthesis. Full all-feature tests (144 Rust and 10 f32 passed; 8 ignored), no-default tests (128 Rust and 10 f32 passed; 7 ignored), Clippy and formatting passed. The complete timing table, process medians, memory, commands and error figures are in [`the report`](../reports/spin1-parallel-coefficient-cache-2026-10-14.md).

The read-only Sol High review found no blocker in `c6b238d`. Its suggestion for an exact per-order cache-row check is covered in `spin1_parallel_coefficient_cache_matches_uncached_synthesis`; the existing transform comparison still checks grouped output. A post-cache Callgrind run counted 8.121 billion instruction references for T511 q8 spin-1 synthesis and 5.046 billion for analysis. The fused synthesis vectorizer accounted for 28.3% of exclusive synthesis references; the paired analysis accumulator accounted for 24.5% of analysis references. Callgrind counts are not elapsed time. See [`the cache report`](../reports/spin1-parallel-coefficient-cache-2026-10-14.md) for the profile conditions and limits.

### Paired analysis FMA experiment (2026-10-14)

A temporary Pulp FMA rewrite reduced AVX2 vector multiply/add instructions in the normal-range paired analysis reducer from 20 to 12 and reduced T511 q8 spin-1 Callgrind instruction references by 3.62%. Whole-transform q8 spin-1 analysis was 2.84% slower across five alternating process pairs; scalar q8 was nominally 1.34% faster but noisy. The independent q1 screen showed spin-1 0.9% slower. The candidate passed focused recurrence and DUCC tests but did not meet performance acceptance and was reverted. Callgrind instruction references did not predict wall time. See [`the experiment record`](../reports/spin1-analysis-thread-scaling-2026-10-14.md#paired-analysis-fma-experiment) and [`raw paired samples`](../reports/data/paired-analysis-fma-2026-10-14/paired-process-samples.txt).

### Spin-1 order-major analysis layout experiment (2026-10-14)

A lazy q8 GL scratch transpose of the two spin-1 Fourier spectra into order-major latitude rows passed the T511 eight-thread DUCC/round-trip regression, but made warm q8 analysis 3.4% slower at T511 and 3.6% slower at T720. First-call medians increased 32.6% and 21.2%, respectively; the retained scratch used 8 MiB and 15.864 MiB. The transpose was removed. The scalar path's order-major layout remains in place because it reuses existing workspace storage and had measured whole-transform gains. See [`the spin-1 layout report`](../reports/spin1-analysis-thread-scaling-2026-10-14.md#spin-1-order-major-analysis-layout-experiment) and [`raw samples`](../reports/data/spin1-order-major-analysis-2026-10-14/process-samples.txt).

### Spin-1 q8 64-ring synthesis schedule control (2026-10-14)

A spin-1-only T511 q8 schedule reduced the cached coefficient-row copies by grouping 64 rather than 32 latitude rings, with eight rather than sixteen groups. Five paired processes were consistently slower; the median rose from `19.943` to `22.818 ms` (`+14.4%`). First-call time also increased. The candidate passed the q8 T511 DUCC/round-trip regression but was reverted because the reduced scheduling granularity outweighed the saved copies. See [`the group-size report`](../reports/latitude-group-size-16-2026-10-14.md#spin-1-q8-64-ring-group-control) and [`raw samples`](../reports/data/spin1-ring64-synthesis-2026-10-14/ring64-process-samples.txt).

### Release LTO and codegen-unit screen (2026-10-14)

Thin LTO with the default 16 codegen units improved T511 q1 scalar analysis by a repeatable 5.3%, but the confirming q8 spin-1 synthesis median was 6.1% slower and q8 analysis was flat. At T720 q8, spin-1 synthesis was unchanged. One codegen unit without LTO slowed q8 spin-1 synthesis by 23%; combining one codegen unit with thin LTO also regressed it. The repository release profile was left unchanged. The thin-LTO build passed scalar T511/T1023/T2047 GL/CC/F1 DUCC/analytic tests and spin-1/2 T511 all-grid DUCC checks. See [`the compiler-profile report`](../reports/release-profile-screen-2026-10-14.md) and [`raw process samples`](../reports/data/release-profile-screen-2026-10-14/process-samples.txt).

### Spin-1 cached-row direct-read experiment (2026-10-14)

A temporary q8 synthesis path read cached spin-1 scalar coefficient rows directly for fully active paired blocks at T640 and above. At T720, ten alternating native GL process pairs gave a median paired change of `-0.42%`; the candidate was faster in six pairs, with paired changes from `-3.96%` to `+6.19%`. At T511, where direct reads were disabled, five pairs showed no repeatable shared-path effect. The direct-read path passed exact block comparison and T511 all-grid DUCC/round-trip checks, but the whole-transform gain was within process variability, so the source prototype was removed. Full conditions, errors and raw samples are in [`the experiment report`](../reports/spin1-paired-coefficient-preparation-2026-10-14.md) and [`its data`](../reports/data/spin1-paired-coefficient-preparation-2026-10-14/final-alternating-process-samples.txt).

### Spin-1 paired synthesis coefficient iteration (2026-10-14)

The fused spin-1 synthesis loop now slices its six coefficient rows once and traverses them together, removing repeated per-row bounds checks from the degree loop. Five alternating native process pairs improved q1 by 11.04% at T511 and 12.06% at T720; q8 improved 7.64% at T511, while T720 q8 was modest and noisy at 2.95%. Callgrind instruction references fell 9.49% for q1 T511, but are not cycles. The read-only Sol High review found no correctness or ownership blocker and confirmed the unchanged accumulation order and bounded tails. DUCC, adjoint, T511 all-grid, T2047 sparse synthesis, full Rust, Clippy, format and Python checks passed. Details, Python-plan/DUCC comparisons, raw samples, and environment are in [`the report`](../reports/spin1-paired-zip-iteration-2026-10-14.md) and [`its data`](../reports/data/spin1-paired-zip-iteration-2026-10-14/).

### Scalar paired synthesis iterator experiment (2026-10-14)

A temporary zipped traversal removed four coefficient-slice bounds checks from the scalar AVX2 pair loop. Five paired processes improved q1 synthesis by 4.57% at T511 and 3.67% at T720. q8 T511 was flat; ten paired T720 q8 runs were 3.39% slower at the median and favored the candidate in only two pairs. The iterator change was removed rather than add a thread-count-specific duplicate kernel. DUCC synthesis, scalar adjoints and release T511/T1023/T2047 GL/CC/F1 tests passed. The report includes raw timings, assembly evidence and the rejection rationale: [`scalar paired synthesis iterator experiment`](../reports/scalar-paired-zip-iteration-2026-10-14.md).

### Spin-1 synthesis table policy refresh (2026-10-14)

After the accepted paired synthesis loop change, current-source q1 table/recurrence pairs showed the synthesis table faster at T127 (+4.47%), T175 (+8.11%) and T191 (+14.00%), but slower at T144 (-16.36%) and T159 (-1.80%). The deterministic q1 gate now retains T127 and `175 <= lmax <= 191`; bands between T128 and T174 use recurrence unless another path applies. Analysis tables continued to win at T85/T127/T159/T191. The 34 MiB cap, lazy construction, precision and grid guards are unchanged. Direct DUCC 0.41.0 map errors at the five synthesis bands were at most `4.344e-14` relative L2. The table/recurrence/adjoint test, T511 all-grid spin regressions, T2047 sparse spin-1–4 regressions, full Rust suites, Clippy, format and Python tests passed. Cold-call and warm timings, break-even estimates, table bytes, Python/DUCC results and raw alternating samples are in [`the selector refresh`](../reports/adaptive-legendre-refresh-2026-10-14.md) and [`its data`](../reports/data/adaptive-legendre-refresh-2026-10-14/).

The Sol High read-only review found one low-severity setup issue: an unsafe spin-1 synthesis input could build the table before the per-order range check selected recurrence. A preflight now rejects first-use table construction when any E/B component exceeds one quarter of the table input limit; safe calls retain the per-order check, and an existing table skips the additional scan. A focused `1e150` coefficient test confirms recurrence-equivalent finite output and an uninitialized table. Three paired T127/T175/T191 q1 runs found no warm or first-call regression beyond process variability. Unsafe first calls avoid the corresponding `4.094`, `10.576` or `13.711 MiB` table payload. The other selector, numerical, race and workspace checks found no blocker. After this fix, all-feature tests passed (145 Rust and 10 f32; 8 ignored), no-default-feature tests passed (129 Rust and 10 f32; 7 ignored), Clippy and formatting passed, and the T2047 sparse spin 1–4 GL/CC/F1 release test passed. Details and raw timings are in [`the selector refresh`](../reports/adaptive-legendre-refresh-2026-10-14.md#spin-1-synthesis-table-range-preflight).

### Operation-specific high-band synthesis grouping (2026-10-14)

A temporary per-group timer found T720 q8 scalar synthesis polar groups took about `14–20 ms`, versus `4.3–6.2 ms` for central/equatorial groups; the active degree-lane estimate did not predict this imbalance. A 32-ring policy improved paired native GL STANDARD medians by 13–25% for scalar q4 at T511/T640/T720, 16–24% for scalar q8/q16 at T640/T720, and 7–8% for spin-1 q8/q16 at T640/T720. It is limited to full-band f64 exact GL within T511–T720 and selected executing pool sizes. Spin-1 q4, GRAD_ONLY and DERIV1 remain on their prior groups after the finer policy regressed or failed to help those controls. Additional recurrence scratch is grown lazily in the executing workspace; one T720 q16 RSS pair showed +832 KiB. Focused T511 scalar and T720 spin-1 adjoint identities passed; T511–T2047 scalar DUCC/analytic release checks across GL/CC/F1 passed. The full all-feature suite passed (148 Rust plus 10 f32; 8 ignored), no-default suite passed (129 Rust plus 10 f32; 7 ignored), workspace Clippy, formatting and 16 Python tests passed. Current Rust/DUCC q8 context remains 10.6x for T720 scalar and 6.8x for spin 1. Full samples, method and limitations are in [`the group policy report`](../reports/fine-synthesis-latitude-groups-2026-10-14.md) and [`its data`](../reports/data/fine-synthesis-latitude-groups-2026-10-14/).

### Spin-1 analysis dual-channel dispatch experiment (2026-10-14)

A Callgrind profile of current T511 q8 spin-1 analysis attributed 20.0% of inclusive references to `AnalysisPairAccumulation` and 8.1% to its mixed-range variant. A prototype used one Pulp dispatch for the Q and U reductions, preserving separate sequential channel passes. Five paired native runs at T511/T720 showed q8 changes of `+0.24%/-0.93%`, with both ranges crossing zero; q1 changes were `-1.94%/-0.95%`. Callgrind references fell only 0.98% q8. The prototype was removed. Its first `.map` version lowered to a per-degree `Array::Drain` path and was 7–15x slower; explicit calls removed that compiler-generated cost but did not establish a whole-transform gain. Focused reducer, GL/CC/F1 adjoint, T511 DUCC and T1023 exact-analysis DUCC tests passed. See [`the experiment report`](../reports/spin1-analysis-channel-dispatch-2026-10-14.md) and [`paired samples`](../reports/data/spin1-analysis-channel-dispatch-2026-10-14/process-medians.csv).

### Spin-1 mirrored-ring synthesis (2026-10-14)

For f64 spin-1 STANDARD synthesis, a north/south paired scalar recurrence now runs on full-band exact GL plans from T319 through T511 when the executing Rayon pool has four to eight workers. It converts E/B coefficients to scalar streams, accumulates even/odd degrees together across mirrored rings, and reconstructs the reflected contribution. The path checks both ring cutoffs and normal-range safety before using the fused kernel; excluded, polar or scaled blocks retain the existing recurrence. It adds no persistent plan or workspace memory and leaves the equator on the existing path.

Five release processes each ran seven alternating whole-transform pairs at T319 and T511, plus thread-count and T720 controls. At T319, paired-sample medians improved `22.71%` at q4 and `12.57%` at q8; all 35 paired samples favored the candidate at each pool size. At T511 the q4 median improved `15.04%` (33/35 samples faster), while q8 improved `4.09%` (29/35); q1, q2, q16 and T720 q8 remain unselected. Relative-L2 errors against the general recurrence were below `1.8e-15` on the selected cases. The T511 high-order sparse field matched stored DUCC 0.41.0 samples, and a T319 adjoint identity and `1e300` scaled-fallback regression passed. The final focused test also covers an odd-latitude equator, a partial block, and Nyquist order. Full Rust, no-default-feature, Clippy, format and Python checks passed. The exact grids, raw timing vectors, memory, DUCC reference and review findings are in [`the report`](../reports/spin1-hemisphere-pair-synthesis-2026-10-14.md) and [`its data`](../reports/data/spin1-hemisphere-pairing-2026-10-14/).

### Spin-1 analysis order-leaf overpartitioning (2026-10-14, rejected)

A temporary policy requested up to twice the weighted analysis leaf count for exact GL spin-1 STANDARD analysis at T511+ and q8+, bounded by existing worker scratch. Five paired T511 q8 processes were `3.19%` slower at the median; T511 q16 was mixed, and T720 q8 was `11.55%` slower. The q4 control was unchanged within noise. Current weighted splitting remains. A refreshed Callgrind run counted `5.019` billion instruction references for T511 q8 analysis; the Pulp paired visitor took `24.62%` exclusive references. AVX2 disassembly showed eight accumulator chains and no YMM spills in that loop, so this profile does not justify more scheduling leaves. The candidate passed the opt-in T511 spin DUCC/roundtrip test on GL/CC/F1 before removal. Full samples and the rejection rationale are in [`the experiment report`](../reports/spin1-analysis-leaf-overpartition-2026-10-14.md) and [`its sample data`](../reports/data/spin1-analysis-thread-scaling-2026-10-14/more-leaves-paired-samples.txt).

The adaptive Legendre selector was checked at `3bb4fd9`. Later kernel changes occur outside its table-eligible bands or add a safe-input preflight without changing ordinary table arithmetic. The measured deterministic policy is unchanged; the initial report recorded cold-call costs and break-even counts instead of hidden calibration. A caller estimate was added in the 2026-10-14 follow-up below. See [`the selector study`](../reports/adaptive-legendre-strategy-2026-10-14.md#latest-head-selector-check).

### Over-resolved GL Legendre strategy follow-up (2026-10-14)

Forced recurrence/table comparisons on the current recurrence baseline covered scalar and spin-1 STANDARD synthesis/analysis at q1/q8 for minimum and atmospheric T63/T72/T85 grids, plus atmospheric T127/T191 controls and reduced-mmax cases. A q2/q4/q16 sweep covered representative scalar synthesis/analysis and spin-1 analysis cases at T63/T85/T127/T191. Over-resolved tables are bounded to at most twice the minimum GL latitude count; automatic analysis requires full mmax and lmax <=127. Scalar T63 analysis uses recurrence at q4+ after neutral/slower paired results; scalar synthesis requires a multi-thread pool. The 34 MiB cap is unchanged. Tables remain lazy. T191 analysis tables regressed; T127 q8 analysis improved, and q8 synthesis gains varied by band. The report records first-use cost and reuse break-even, including the weak T85 synthesis case. The Sol High read-only architecture review found no correctness or safety blocker and prompted the oversampling bound. All-feature Rust tests passed (151 Rust plus 10 f32; 9 ignored), no-default tests passed (129 plus 10 f32; 7 ignored); Python, Clippy and formatting checks are recorded with the final commit. See [`the over-resolved strategy report`](../reports/overresolved-gl-legendre-table-2026-10-14.md) and [`raw process samples`](../reports/data/overresolved-gl-legendre-table-2026-10-14/).

### Caller-provided Legendre reuse estimates (2026-10-14)

The additive `LegendreReuseEstimate` constructor lets applications with known STANDARD table-eligible reuse estimate coefficient-to-map and map-to-coefficient calls separately. `ShtPlan::new` and its automatic policy are unchanged. Selection checks the executing Rayon pool at operation time; cold estimates are accepted only for measured exact cells. The north-hemisphere table remains lazy and below the 34 MiB cap. Counts include only calls that can reach that table path: for spin 1, adjoint synthesis cannot use the map-to-coefficient table and must not contribute to its estimate. A positive estimate can use a shared table after another eligible operation has paid setup; `Some(0)` continues to force recurrence.

Four paired current-source comparisons measured first-call and warm recurrence/table costs. Scalar T127 exact GL synthesis q8 crossed at 86 total calls and selects at 90; scalar T85 atmospheric GL synthesis q8 crossed at 67 and selects at 70; spin-1 T159 exact GL synthesis q1 crossed at 31 and selects at 32; spin-1 T159 atmospheric GL analysis q4 crossed at 27 and selects at 30. Raw samples, process ranges, total times for up to 1,000 calls, payload estimates and commands are in [`the reuse-estimate report`](../reports/legendre-reuse-estimate-2026-10-14.md) and [`its raw data`](../reports/data/legendre-reuse-estimate-2026-10-14/).

The first Sol High review identified execution-direction naming and break-even counts missing the first-call `+1`; both were corrected. Its read-only follow-up then found the spin-1 adjoint-synthesis eligibility mismatch and incomplete Rust docs for an already-initialized table. The estimate contract and docs now cover those cases, with a focused regression for spin-1 adjoint synthesis. The reviewer confirmed that neither finding remains. Numerical kernels and tolerances are unchanged. Validation passed: `cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test --workspace --all-features` (160 Rust and 10 f32 tests, 9 ignored), `cargo test -p rustsht --no-default-features` (132 Rust and 10 f32 tests, 7 ignored), and `uv run pytest python/tests` (20 passed). All-feature tests include existing spin-3/4 regressions.

### Current scalar paired-analysis profile and unfinished fusion candidate (2026-10-14)

At `5d3ed6b`, a current-source T127/T511 scalar GL analysis screen and Callgrind profiles confirmed that paired recurrence and accumulation dominate the remaining analysis work. T511 q1/q8 reused-workspace native medians were `22.788/6.959 ms`; DUCC context remains the earlier Python-plan comparison (`6.470/1.074 ms`, with RustSHT `23.058/7.344 ms`), so the boundaries are not identical. The AVX2 disassembly showed the paired recurrence writing degree state to worker scratch and calling its visitor once per degree pair; analysis then dispatches a separate reducer and performs a horizontal reduction. The retained reducer already uses two partial chains and eight YMM accumulators. FMA and deeper-unroll variants remain rejected by earlier whole-transform measurements.

A scalar-only register-resident recurrence/accumulation prototype was considered but not implemented. It needs staged per-block coefficient output so a recurrence that exits normal range can discard the partial result and use the existing scaled path. No source behavior changed. The current profile, raw baseline samples, exact reproduction commands and unresolved implementation conditions are in [`the profile note`](../reports/current-scalar-analysis-kernel-profile-2026-10-14.md) and [`its data`](../reports/data/current-scalar-analysis-profile-2026-10-14/).

### Three-chain SIMD paired-analysis accumulation (2026-10-14, rejected)

A temporary third independent reduction chain shortened the AVX2 dependency chain but increased the live accumulator count from eight to twelve. In five alternating ABBA process blocks, q1 spin-1 STANDARD analysis regressed by 7.21% at T511 and 6.80% at T720. At q8, T511 was within noise (+0.36% median); T720 regressed 2.71%. The candidate passed seven focused normal/scaled/extreme-input reducer tests and the opt-in T511 spin-1/2 DUCC 0.41.0 GL/CC/F1 analysis, synthesis and round-trip test. AVX2 disassembly showed a larger stack frame and stack-mediated vector traffic, plus multiply-high division for three-element chunk lengths. The two-chain implementation remains. See [`the experiment report`](../reports/paired-analysis-three-chain-2026-10-14.md) and [`paired raw process samples`](../reports/data/paired-analysis-three-chain-2026-10-14/).

### T159 over-resolved analysis table selector (2026-10-14)

Commit `b7401fa` extends the over-resolved analysis-table ceiling to T159. Paired recurrence/table runs at f64 GL `320x640`, `lmax=mmax=159`, `phi0=0.37` found repeatable analysis gains for scalar q1/q2/q4/q8 and spin-1 q2/q4/q8/q16. Scalar q16 was noisy and spin-1 q1 was neutral, so those cases retain recurrence. T191 remains recurrence and the synthesis policy is unchanged. The existing triangular tables use `15.72 MiB` (scalar) and `15.92 MiB` (spin 1), within the 34 MiB cap. First use adds about `14.5–15.0 ms`, requiring approximately `29–156` repeated analyses in the selected cells to amortize. Native scalar table/recurrence comparisons cover all four operators; spin-1 q8 analysis matches the general recurrence and its adjoint identity. Installed DUCC 0.41.0 q1/q8 Python comparisons at the same grid pass the existing `2e-12` tolerance for analysis and its adjoint, with relative-L2 errors `1.0–1.6e-14`. Python plan q1 medians were `2.225/4.145 ms` for Rust scalar/spin-1 and `0.973/2.216 ms` for DUCC; q8 Rust plan timing was highly variable, so it did not inform the selector. An attempted all-operator T159 comparison exposed a pointwise synthesis error above that test tolerance for both the recurrence baseline and the existing q8 scalar table path; the analysis-only regression isolates the changed selector without altering synthesis behavior or tolerances. Full details, exact paired ranges, cold costs, RSS and raw samples are in [`the T159 evaluation`](../reports/t159-overresolved-analysis-2026-10-14.md) and [`its data directory`](../reports/data/overresolved-t159-analysis-2026-10-14/).

Validation on the accepted source: `cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test --workspace --all-features` (151 passed, 9 ignored; 10 f32 passed), `cargo test -p rustsht --no-default-features` (129 passed, 7 ignored; 10 f32 passed), and `uv run pytest python/tests` (20 passed). The release scalar T511/T1023/T2047 all-grid DUCC and analytic check and the release spin-1/2 T511 GL/CC/F1 DUCC/round-trip check passed. Focused T159 selector, table-versus-recurrence, general spin recurrence, adjoint, and q1/q8 DUCC analysis tests passed. No tolerance or FFT implementation changed.

### Fused Q/U spin-1 analysis reduction (2026-10-14)

Commit `612a90e` baseline profiling identified the two scalar-conversion channel reducers as prominent work in spin-1 GL analysis. An f64 normal-range SIMD kernel now accumulates Q and U together and reuses the shared recurrence basis and cosine loads. Unsafe or scaled channels retain the prior independent fallback, and f32 uses its previous path. Five alternating native pairs at T511 improved by 4.79% q1 and 3.60% q8. At T720, q1 improved 3.67%; q8's 0.70% median gain was within the paired range. Candidate AVX2 assembly uses eight YMM accumulators without spills. T511/T720 DUCC 0.41.0 analysis errors were `3.740e-14`/`6.061e-14` relative L2. Full workspace, no-default, Clippy, format, Python, spin adjoint, T511 GL/CC/F1 and T1023 exact-analysis checks passed. The overflow fallback test remains in place; no FFT or public API changed. See [`the experiment report`](../reports/spin1-dual-channel-analysis-2026-10-14.md) and [`paired samples`](../reports/data/current-kernel-baseline-2026-10-14/spin1-dual-analysis-paired/).

### DUCC0 source-build comparison (2026-10-14)

The pinned DUCC revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` built with its default native flags (`-O3 -ffast-math -march=native`, CMake IPO/LTO) was compared with installed `ducc0==0.41.0` at T511/T720 for scalar and spin-1 STANDARD synthesis/analysis, q1/q8. Five alternating process pairs with five recorded calls per process found no repeatable source-build advantage: paired medians were generally within 1%, and all five-pair ranges crossed zero. A first T720 q8 spin-1 synthesis screen favored source by about 5–7%, but the raw-sample repeat was neutral (`+0.1%`, source vs installed). The source revision's project metadata reports version 0.41.1; the installed wheel and source build both identify GCC 15.2.0. Numerical comparisons against the source build had relative-L2 errors from `3.486e-14` to `9.123e-14`. RustSHT's multi-fold gap is not explained by a consistent installed-wheel optimization deficit. See [`the build comparison`](../reports/ducc-source-build-screen-2026-10-14.md) and [`raw samples`](../reports/data/ducc-source-build-screen-2026-10-14/).

### Spin-1 staged synthesis coefficient factorization (2026-10-14)

Commit `d19af76` was the native baseline. Combining each even/odd transformed coefficient before its scaled Legendre multiply reduced q1 whole-transform medians by 12–14% at T511, T720 and T1023. The q8 improvement was neutral at T511 and T720 but 10.19% at T1023. Five-operation Callgrind references at T511 fell 11.4%; this is an instruction count, not a cycle measurement. Installed DUCC 0.41.0 Python-plan comparisons at T511/T720 agreed with relative-L2 errors from `4.715e-14` to `9.123e-14`. A read-only Sol High review found no blocker and suggested an exact cancellation test, which now exercises both scalar and dispatched paths. The temporary scalar transition-threshold change was rejected because paired whole-transform timings were neutral or slower. Full details, DUCC timings, raw samples, reviewer disposition and validation are in [`the factorization report`](../reports/spin1-staged-synthesis-factorization-2026-10-14.md) and [`its data`](../reports/data/current-kernel-baseline-2026-10-14/).

### Latest-HEAD adaptive selector recheck (2026-10-14)

The broad selector study predated the fused Q/U spin-1 analysis reducer and staged synthesis factorization. A temporary same-binary override compared automatic, forced recurrence and forced compact-table execution at the affected GL cells. The policy now uses recurrence for scalar T127 q1 synthesis, adds the measured T159 q1 spin-1 synthesis table point while retaining recurrence at T144, and uses recurrence for full-mmax T255 spin-1 analysis at q2 and above. The 34 MiB cap and lazy table construction are unchanged. DUCC numerical comparisons reported relative-L2 errors from `1.283e-14` to `5.210e-14`; full Rust, no-default-feature and Python suites passed. Detailed method, paired results, setup/memory costs, commands and raw samples are in [`the latest-HEAD results`](../reports/adaptive-legendre-latest-head-results-2026-10-14.md), with the preceding [source audit and experiment plan](../reports/adaptive-legendre-latest-head-recheck-2026-10-14.md).

### Paired scalar and spin-1 synthesis overflow guards (2026-10-15)

Paired scalar coefficient preparation now rejects a non-finite transformed pair before its recurrence consumer runs. Scalar synthesis falls back to the unpaired scaled recurrence using the original packed coefficients at every bandlimit. The public T1 GL case with `a10=1.1e308` remains finite and matches `±a10/sqrt(4π)` within `2e-14` relative error. Preparation tests cover ordinary values, the smallest positive subnormal and the overflowing T1 pair; the existing T1023 transition test covers subnormal and extreme finite inputs.

Spin-1 synthesis now checks both converted scalar rows before selecting the scalar recurrence. A non-finite conversion uses the general spin recurrence. Separate Q/U paired coefficient preparation stages both channels before either spectrum is updated; if either channel is unsafe, both use the unpaired scaled recurrence. Tests compare the conversion-overflow and paired-coefficient-overflow paths with the general recurrence, and public synthesis of `1e307` coefficients returns finite maps. The existing scalar-conversion comparison passes on GL, CC and F1 through T224.

`cargo fmt --all -- --check`, workspace Clippy with warnings denied, `cargo test -p rustsht --no-default-features` (137 unit and 10 f32 tests passed; 7 ignored), and `cargo test --workspace --all-features` (165 unit and 10 f32 tests passed; 9 ignored) passed. No Python bindings changed, so Python tests were not run. Performance screening is outstanding: the host load average was 8.13 when checked, and no exclusive benchmark window was established.

### Workspace execution-capacity migration (2026-10-15)

`ShtWorkspace` now reports its requested execution capacity and allocated recurrence-worker count. `ShtPlan::prepare_workspace` changes capacity explicitly and reserves recurrence scratch for regular analysis/synthesis groups and applicable fine or spin-1 hemisphere groups. Parallel builds accept capacities from 1 through 64; non-parallel builds accept 1. Each operation limits active Rayon work to the smaller of the workspace capacity and active pool size. Reducing capacity retains scratch. Transform dispatch no longer grows recurrence scratch in response to a pool migration. A capacity-1 workspace remains suitable for outer parallelism over independent fields.

A test-only observer recorded `rayon::current_thread_index()` at scalar synthesis group entry for recurrence-only f64 GL T1023 (`lmax=mmax=1023`, `ntheta=1024`, `nphi=2048`). The default workspace migrations measured: q1→q8 `1` active worker / `1` scratch slot; q8→q1 `1/8`; q1→q16 `1/1`; q16→q8 `8/16`; q3→q8 `3/3`. Explicitly preparing the q1 workspace for capacity 8 before q8 execution measured `8` active workers / `8` scratch slots and matched a fresh q8 workspace within `1e-13` relative L2. Thus the executing pool alone does not expand a workspace, while explicit preparation makes higher capacity available.

Adapted q1→q4 workspaces were compared with fresh q4 workspaces for scalar and spin-1 `S`, `A`, `S*` and `A*` on GL, CC and F1, with operation switching on the same workspace. All comparisons passed at relative L2 `1e-13`; real-field adjoint identities passed at `2e-10`. Reducing capacity to 1 retained allocated scratch and returned equivalent serial synthesis. Invalid capacity requests left the previous capacity unchanged. Existing hemisphere-selector and synthesis regressions passed after capacity-aware selection. The selector regression also checks q8 T319 workspace allocations for spins 0, 2, 3 and 4; each reserves the 10 regular groups and not the 16 spin-1 hemisphere groups.

The parallel-independent-field control at q8/B8 T511 was screened with the existing `m5_batch_bench` and three release processes per operation. Results and raw samples are in [`the control report`](../reports/workspace-capacity-control-2026-10-15.md). Current process-median medians were `49.300/27.533 ms` scalar synthesis/analysis and `60.203/42.966 ms` spin-1 synthesis/analysis; the 2026-10-14 study reported `48.827/29.529` and `68.666/44.976 ms`. This cross-revision screen showed no clear regression, but it is not a paired baseline/candidate acceptance measurement. Further performance acceptance remains outstanding.

### Paired workspace-capacity batch comparison (2026-10-15)

Three alternating process pairs compared parent `4f54934` and capacity-control `b6f2415` for q8/B8 T511 GL f64 scalar and spin-1 STANDARD synthesis and analysis. Each process collected five warm batch samples. Capacity-1 workspaces inside the q8 outer pool reduced spin-1 synthesis by 35.11% (paired range `-36.63%` to `-34.51%`) and changed scalar synthesis by `+2.02%` (all pairs `+1.88%` to `+3.38%`). Scalar analysis varied widely; spin-1 analysis was inconclusive. Pool-sized workspaces showed a repeatable 10.70% reduction for spin-1 synthesis; scalar changes and spin-1 analysis remained within variable ranges. RSS during workspace allocation changed by less than 0.3 MiB in the measured cases. This is a same-host scheduling screen, not numerical accuracy validation or broad performance acceptance. The complete table, environment, command and raw samples are in [`the paired comparison report`](../reports/workspace-capacity-paired-2026-10-15.md) and its [`data`](../reports/data/workspace-capacity-paired-2026-10-15/).

Final checks passed: `cargo fmt --all -- --check`; workspace Clippy with warnings denied; `cargo test --workspace --all-features` (168 unit and 10 f32 tests passed; 9 ignored); `cargo test -p rustsht --no-default-features` (137 unit and 10 f32 tests passed; 7 ignored); and `uv run pytest python/tests` (20 passed).

### Fused scalar recurrence and accumulation for analysis (M4, 2026-10-15)

Commit `f972cd7` adds an f64 scalar paired-analysis kernel that advances the recurrence and accumulates each degree pair inside one SIMD dispatch. It stages block contributions in existing worker scratch and commits only when the complete block stays in the normal numeric range; unsafe or scaled blocks retain the established fallback. Commit `7c79dd6` gates the path at `lmax >= 255` after forced-recurrence T127 q8 regressed 7.47% before the gate. The gated T127 q8 control was neutral within its paired range.

Three alternating release-process pairs on f64 GL STANDARD analysis found T255 q1 median improvement of 8.95% (all pairs faster) and T511 improvements of 12.21% at q1 and 5.51% at q8 (all pairs faster). T255 q8's 3.69% median improvement was inconclusive because its paired range crossed zero. T127 automatic measurements followed the compact-table path and are reported only as a selector control. A separate T511 peak-RSS sample measured 28,396/40,224 KiB at baseline q1/q8 and 28,352/41,388 KiB at candidate q1/q8; the one-process q8 increase is descriptive and does not establish a repeatable memory change.

The fused recurrence test compares SIMD lanes and a tail against the prior paired recurrence and checks scaled-sectoral fallback. The T511 GL sectoral-field test compares stored DUCC 0.41.0 samples with an independent closed form and recovers the packed coefficient vector. DUCC source revision: `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Validation passed: formatting, workspace Clippy with warnings denied, 171 all-feature Rust tests plus 10 f32 tests (9 ignored), and 140 no-default-feature Rust tests plus 10 f32 tests (7 ignored). The opt-in M3 all-grid high-bandlimit test and Python suite were not run; no bindings changed. Exact conditions, paired ranges, RSS sample and raw process data are in [`the report`](../reports/fused-scalar-analysis-2026-10-15.md) and [`its data`](../reports/data/fused-scalar-analysis-2026-10-15/).

### Fused spin-2 scalar-conversion analysis (M4, 2026-10-15)

The f64 GL spin-2 fast-analysis path now advances its paired scalar recurrence and accumulates both converted channels in one SIMD dispatch. It stages each block in existing worker scratch and commits only when both channel products and all recurrence lanes remain normal; otherwise the established paired scaled path handles the block. The selection remains inside the existing exact-GL spin-2 path at `lmax >= 500`, with the `sin(theta) >= 0.01` polar range and per-ring cutoff behavior unchanged.

Three alternating release-process pairs at T511 and T720 improved f64 GL STANDARD analysis by 7.36%/8.99% at q1 and 2.59%/3.32% at q8, respectively; every paired result favored the candidate. A single T720 peak-RSS sample measured `51,524/52,396 KiB` baseline q1/q8 and `52,744/53,220 KiB` candidate; the small increase is descriptive and not a repeated memory result. The two-channel tail/reference tests, scaled-sectoral decline, unsafe-channel fallback against the general recurrence, and release T511 spin-1/2 DUCC 0.41.0 GL/CC/F1 roundtrip test passed. DUCC source revision: `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

Final checks passed: formatting, workspace Clippy with warnings denied, 174 all-feature Rust tests plus 10 f32 tests (9 ignored), and 143 no-default-feature Rust tests plus 10 f32 tests (7 ignored). The opt-in T1023 spin-2 tests and Python suite were not run; no bindings changed. Full timing conditions, process ranges, RSS and raw samples are in [`the report`](../reports/fused-spin2-analysis-2026-10-15.md) and [`its data`](../reports/data/spin2-fused-analysis-2026-10-15/).

### Fused spin-1 analysis recurrence and reduction (M4, 2026-10-15)

The f64 GL spin-1 analysis path now advances the scalar-conversion recurrence and accumulates Q and U in one SIMD dispatch. Both coefficient rows are staged in existing recurrence-worker scratch and committed only after both channels complete in the normal range. Unsafe products and scaled recurrence blocks retain the paired fallback. The selector requires `lmax >= 255` and single-thread execution; workspace allocation and public APIs are unchanged.

Five alternating whole-analysis process pairs at T511 and T1023 improved q1 medians by 9.65% and 9.61%, respectively, with every pair faster. T63 and T127 were below the fusion threshold. An ungated q8/q16 screen had paired ranges crossing zero at both degrees, so fusion is disabled for parallel execution. Five-pair q8/q16 controls after the gate also had ranges crossing zero. Median process peak RSS changed from 27,448 to 27,268 KiB at T511 q1 and from 101,168 to 101,212 KiB at T1023 q1; no consistent memory increase was measured.

Two regressions check atomic staging and one-channel unsafe fallback against the general spin recurrence. Paired and full-latitude recurrence comparison passes. Spin-1 analysis/general-recurrence comparisons and adjoint identities pass through T255 on GL, CC and F1. Release DUCC 0.41.0 checks pass at T511 on all three grids and exact T1023 analysis on all three grids, using source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. A read-only Sol High review found no correctness findings. The workspace-wide tests and Clippy remain scheduled with the final E milestone validation; no Python bindings changed. Timings, CPU/compiler configuration, exact commands, RSS, and raw samples are in [the report](../reports/fused-spin1-analysis-2026-10-15.md) and [its data](../reports/data/spin1-fused-analysis-2026-10-15/).

### Active order-work prefix for analysis scheduling (E, 2026-10-15)

Parallel scalar analysis at `lmax >= 511` and spin analysis at `lmax >= 255` now build a checked `u128` order-work prefix from the prepared ring cutoffs. The bounded workspace array has `mmax+2` entries. Building it uses one cutoff histogram pass over rings and one pass over orders; recursive splits read prefix differences instead of rescanning all rings. The existing ring-scan midpoint remains as a test reference. Prefix construction occurs only after analysis selects the parallel order-group path, and the recursive split points and strict tie behavior are unchanged.

Tests compare every subinterval's work and midpoint against the scan reference for `lmax=0..16`, truncated `mmax`, empty and inactive cutoffs, fully active rings, asymmetric limits and cutoffs above `mmax`. Length and arithmetic overflow paths are checked. The prefix tests and scalar q1/q8 schedule comparison at T255/T1023 pass; the T511 spin-1/2 GL/CC/F1 DUCC 0.41.0 release check passes with four Rayon threads. `cargo check -p rustsht --no-default-features` passes. Capacity-proportional worker splitting is implemented and measured in the next E increment.

### Capacity-proportional analysis order splitting (E, 2026-10-15)

Scalar and spin analysis now divide active order work according to the worker budget assigned to each child. Equal-capacity branches preserve the existing midpoint; unequal-capacity branches compare boundaries against the exact rational work target, including its fractional part. A review found that comparing only to the floored target could select a worse split; both scalar and active-prefix helpers now have a regression for that case. The 64-bit-only overflow assertion and parallel-only Rayon test also have target/feature gates. A read-only Sol High follow-up found no remaining correctness findings.

The cross-pool q1 reference test passes for q2/q3/q4/q5/q6/q8/q16. Release DUCC and analytic validation passes for scalar T511/T1023/T2047 on GL/CC/F1 with five Rayon threads, spin-1/2 exact T1023 analysis on GL/CC/F1 with five threads, and spin-1 q1/q5 T2047 agreement plus the real-field analysis-adjoint identity. DUCC fixtures use version 0.41.0, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

Five-pair f64 GL STANDARD whole-analysis measurements on full-mmax T511/T1023 show repeatable gains at q3 and q6 for scalar and spin 1. q5 spin-1 improved in both tested degrees; scalar q5 T1023 improved in a five-pair repeat, while scalar q5 T511 ranges crossed zero across ten pairs and remains inconclusive. q2/q4/q8/q16 retain their original split points by equality tests. The candidate adds no workspace data beyond the E1 order-work prefix; measured process peak RSS changes were within a few hundred KiB. The report records paired ranges, controls, host load, CPU/compiler, thread affinity and limitations; raw samples and commands are in [`the capacity-split report`](../reports/analysis-capacity-split-2026-10-15.md) and [`its data`](../reports/data/analysis-capacity-split-2026-10-15/).

Final checks pass: formatting; workspace Clippy with warnings denied; 181 all-feature Rust tests plus 10 f32 tests (10 ignored); and 144 no-default-feature Rust tests plus 10 f32 tests (7 ignored). Python tests were not run because bindings did not change.

### Scalar and spin-1 analysis leaf profile (E, 2026-10-15)

A test-only release profile recorded per-leaf time, order range, active ring/order pairs, estimated work, recurrence block class, worker index, and start/end offsets for scalar and spin-1 GL STANDARD analysis at T511/T1023 with q3/q5/q6/q8/q16. Estimated leaf work was within 2.3% at q6, while the median slowest-to-fastest leaf-time ratio reached 1.9 for T511 and 1.5 for T1023. At q6 spin-1 T1023, leaves with 74.6–75.6 million estimated work took 27.7–40.9 ms. Longer leaves also contained more mixed cutoff blocks and more orders; the profile does not separate their costs.

All q3–q8 samples reached q concurrent leaf intervals and recorded every worker ID. At q16, one scalar T511 sample used 15 worker IDs and reached 14 concurrent intervals. Rayon steal-event counts were not exposed by the worker-index probe. The first leaf began about 40 ms into scalar T1023 analysis and 74–76 ms into spin-1 T1023 analysis; the profile did not split this time among Fourier transforms, weighting, and other preparation.

No scheduling change follows from this profile. The earlier two-leaf-per-worker spin-1 experiment regressed T511 q8 and T720 q8, as recorded in [`the overpartition report`](../reports/spin1-analysis-leaf-overpartition-2026-10-14.md). A dynamic queue would need a new paired benchmark against the current scheduler and that rejected candidate. The profile, raw data, method, and validation are in [`the leaf-profile report`](../reports/analysis-leaf-imbalance-2026-10-15.md) and [`its data`](../reports/data/analysis-leaf-profile-2026-10-15/).

### Current-source analysis phase profile (E, 2026-10-15)

A test-only release profile split scalar and spin-1 GL STANDARD analysis into forward longitude FFTs, latitude setup, scalar order-spectrum preparation, coefficient initialization, order-work prefix construction, and recurrence with accumulation. At q6 T1023, serial row FFTs took 39.6 ms of 75.5 ms scalar analysis and 79.7 ms of 139.1 ms spin-1 analysis. Recurrence and accumulation took 33.9 ms and 52.3 ms. At q1, recurrence and accumulation remained the larger phase: 128.0 ms of 171.2 ms for scalar and 190.5 ms of 269.5 ms for spin 1.

The profile times RustFFT calls at the SHT row boundary and does not change their implementation. It covers GL scalar and spin-1 STANDARD at T511/T1023 and q1/q6, with five successive samples per case in one unpinned process. The raw timing, phase definitions, ranges, and limitations are in [`the analysis phase report`](../reports/analysis-phase-profile-2026-10-15.md) and [`its data`](../reports/data/analysis-phase-profile-2026-10-15/).

The row-scheduling experiment is complete; its measurements and validation are recorded below.

### Parallel SHT row FFT scheduling (E, 2026-10-15)

Scalar and spin analysis now schedule independent longitude rows across the prepared workspace capacity for grids with at least 128 latitude rows and 257 longitude samples. Each Rayon task owns a private root `RealFftWorkspace` and writes a disjoint spectrum chunk. Capacity-one execution retains the original serial row loops, including the interleaved Q/U row order. `prepare_workspace` allocates up to `min(execution_capacity, ntheta)` row-FFT workspaces for eligible grids; smaller grids allocate none. Reducing capacity retains the allocated scratch. The RustFFT call, packing, scaling and dependency are unchanged.

Five alternating process pairs at q6 improved full-mmax f64 GL STANDARD scalar analysis by 26.1% at T511 and 36.1% at T1023; spin-1 improved by 36.3% and 43.9%. Every paired result favored the candidate. Five-pair q6 controls at T127 improved scalar by 29.3% and spin 1 by 44.9%. q1 controls at T511/T1023 had median changes from +0.2% to +1.8%, with ranges crossing zero. Peak RSS increased by medians of 200–912 KiB at q6 T511/T1023; exact row-FFT vector payload is 196,800 bytes at T511 and 393,408 bytes at T1023 for six workers. Capacity-one workspaces allocate no additional row-FFT payload.

The threshold/pool-migration test confirms q1-to-q4 preparation, worker scratch sizing and coefficient agreement. The cross-pool analysis test passes for q1/q2/q3/q4/q5/q6/q8/q16, and existing prepared-workspace analysis tests cover GL, CC and F1. Release scalar DUCC 0.41.0 and analytic checks pass through T2047 on all three grids; spin-1/2 exact T1023 DUCC checks pass on all three grids. Final package/workspace checks and remaining no-default results are recorded in the report. f32 performance, CC/F1 performance, and other q>1 benchmark cells remain unmeasured. See [`the row-FFT report`](../reports/row-fft-parallelism-2026-10-15.md) and [`its raw data`](../reports/data/row-fft-parallelism-2026-10-15/).

### Conditional order-owned scalar synthesis (F/M4, 2026-10-15)

The order-owned scalar GL T511/T1023 prototype was tested and removed. Five alternating whole-transform process pairs showed the T511 q8 candidate 32.65% slower than commit `8ccc0d5` and the T1023 q8 candidate 1.87% slower. Both q1 controls were also slower. All five paired changes were positive in each case, so the prototype did not pass the performance gate. The host load average was nonzero during this screen; the raw data and load record are in [the report](../reports/order-owned-synthesis-2026-10-15.md) and [its data directory](../reports/data/order-owned-synthesis-2026-10-15/).

Candidate correctness checks covered high-order coefficient placement, q5 remainder partitions, odd/even longitude counts and the scaled fallback destination. The all-feature and no-default-feature suites and workspace Clippy passed with the prototype present. No order-owned synthesis code or candidate-specific test remains in the source tree. f32, CC/F1 and other q>1 performance coverage remains outstanding.

The opt-in T1023 GL spin-2 synthesis comparison passed for q1 and q8 with the polar-threshold case enabled. This check compares thread counts; it does not add a DUCC reference or a performance result.

### Current f32 spin-1 GL T511 screen (M4, 2026-10-15)

Five release processes using `examples/m4_f32_bench.rs` measured median q1/q8 times of 993.391/146.682 ms for synthesis and 1014.694/179.030 ms for analysis. Median process peak RSS was 15,312 KiB. Compared with the two-run averages recorded on 2026-10-12, current times are lower by 15.0%–39.8%; the runs were not paired and do not isolate the effect of a source change. This screen did not rerun DUCC and does not change the performance-acceptance status. See [the report](../reports/f32-spin1-current-screen-2026-10-15.md) and [raw process data](../reports/data/f32-rowfft-check-2026-10-15/).

### f32 spin-1 analysis SIMD accumulation (M4, 2026-10-15)

At commit `82605f6`, the f32 spin-analysis visitor used Pulp vector arithmetic when each lane was active for the order, the Fourier inputs and recurrence bases were normal, and both recurrence exponent arrays were zero. Scaled, inactive, zero, subnormal and exceptional lanes used scalar accumulation. The path used existing workspace arrays and included a scalar SIMD tail. Internal unit tests covered 64- and 61-lane blocks, E-only and E/B output, scalar/runtime dispatch, scaled exponents, cutoff lanes and zero Fourier input. A T511 f32 GL analysis test compared selected low-, middle- and maximum-degree E/B coefficients with `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; it passed in debug and release mode.

Two screens of five alternating process pairs each at GL T511 measured analysis improvements of 14.1–14.4% at q1 and 8.6–10.1% at q8. Each process warmed once and measured five transform calls; q1 and q8 synthesis measurements were retained as controls. Candidate q1 synthesis was 2.2–2.6% slower, while q8 synthesis paired changes were noisy. Combined synthesis-plus-analysis time improved 5.8% at q1 in both screens; q8 combined changes were small and paired ranges crossed zero. The change adds no workspace arrays; process peak RSS median increased by 20 KiB in the first screen and 72 KiB in the repeat. The host was a shared Ryzen 9 5950X using Rust 1.99.0, LLVM 23.1.1 and runtime AVX2/FMA dispatch. No current DUCC timing was collected, so the screen does not claim a new RustSHT/DUCC ratio. Workspace all-feature and no-default-feature suites, formatting, Clippy and the release T511 DUCC check passed. See [the report](../reports/f32-spin1-analysis-simd-2026-10-15.md) and [raw paired data](../reports/data/f32-spin1-analysis-simd-2026-10-15/).

### f32 spin-1 zero-lane SIMD eligibility (M4, 2026-10-15)

The analysis accumulation guard now accepts exact positive and negative zero in Fourier inputs and recurrence bases. Scaled, inactive, subnormal and exceptional lanes still use scalar accumulation. Two five-pair T511 GL process screens measured q1 analysis improvements of 2.4–2.9%; q8 medians improved by 1.0–2.5%, with one repeat pair 16.62% slower. Combined q1 synthesis-plus-analysis time decreased by 2.74% and 3.05%. The synthesis-only q1 control also improved, so the combined movement is an executable-level observation rather than an analysis-only attribution. The zero-lane and subnormal predicates, release DUCC 0.41.0 comparison, all-feature/no-default Rust tests, formatting and Clippy passed. No new DUCC timing was collected. See [the report](../reports/f32-spin1-zero-lane-simd-2026-10-15.md) and [raw paired data](../reports/data/f32-spin1-zero-lane-simd-2026-10-15/).

### f32 spin recurrence normalization range-check screen (M4, 2026-10-15)

Callgrind on one q1 f32 T511 GL spin-1 analysis at commit `82605f6` attributed 27.46% of instruction references to lane normalization and 19.09% to recurrence updates. A Pulp SIMD range-check prototype was screened and removed: five paired processes regressed synthesis by 66–72% and analysis by 67–82% across q1/q8, with all paired cells slower. Focused normalization/recurrence and T511 debug DUCC checks passed with the prototype present; candidate code and tests were removed. No repeat was run after the large consistent regressions. See [the report](../reports/f32-spin1-normalization-range-check-2026-10-15.md), [Callgrind profile](../reports/data/f32-spin1-analysis-profile-2026-10-15/), and [paired data](../reports/data/f32-spin1-normalization-simd-2026-10-15/).

### Scalar analysis recurrence/accumulation loop fusion screen (M4, 2026-10-15)

A prototype combined f64 scalar analysis accumulation and recurrence updates in one latitude-vector loop. AVX2 code reused basis and cosine vectors without vector-register spills. Two five-pair process screens at q1/q8 and T511/T1023 did not establish a repeatable whole-transform gain: q1 T1023 changed direction between screens, and every q8 paired range crossed zero. The prototype was removed. A 21-lane internal test remains to cover an odd SIMD-vector remainder and scalar tail in the accepted path. All-feature and no-default-feature suites, workspace Clippy, formatting, and the opt-in release scalar T511/T1023/T2047 GL/CC/F1 DUCC and analytic check passed with the prototype present. See [the screen report](../reports/scalar-analysis-loop-fusion-screen-2026-10-15.md) and [its raw data](../reports/data/fused-scalar-analysis-loop-fusion-2026-10-15/).

### f32 spin-1 interleaved recurrence normalization (M4, 2026-10-15)

An interleaved plus/minus lane-normalization loop passed a bitwise comparison with the two-pass scalar reference and the release T511 DUCC analysis check. One five-pair instrumented whole-transform screen did not show an analysis gain beyond measurement variability; summed synthesis-plus-analysis time increased by 0.39% at q1 and 0.98% at q8. The kernel change was removed. `examples/m4_f32_bench.rs` now records each inner call time as well as the median; the same instrumentation was used for both binaries. See [the screen report](../reports/f32-spin1-normalize-interleave-screen-2026-10-15.md) and [its raw data](../reports/data/f32-spin1-normalize-interleave-2026-10-15/).

### f64 spin-1 T511 DUCC timing refresh (M4, 2026-10-15)

Two follow-up five-pair screens used q1 on CPU 0 and q8 on CPUs 8–15. Their paired RustSHT/DUCC ratios were 1.30/1.77x and 1.31/1.82x at q1/q8; the second screen ranges were 1.29–1.33x and 1.73–1.89x. The first corrected screen on CPUs 0–7 had wide q8 variation (2.02–4.15x). Follow-up load averages were below 3.1, and the CPUs were not isolated. DUCC's timings remain different from the October 14 thread-scaling report and closer to a separate October 12 comparison. These measurements describe the recorded affinities and native/Python call boundaries; they do not replace the earlier gap table. See [the comparison report](../reports/spin1-analysis-ducc-refresh-2026-10-15.md) and [raw data](../reports/data/spin1-analysis-ducc-refresh-2026-10-15/).

### f64 scalar T511 DUCC timing refresh (M4, 2026-10-15)

Five alternating native/Python process pairs measured scalar GL STANDARD analysis at q1/q8. RustSHT/DUCC paired median ratios were 1.65x and 2.17x; all pairs favored DUCC. One DUCC q1 result and one RustSHT q8 result widened the paired ranges. The CPU set was not isolated, and DUCC medians were higher than an earlier Python-plan comparison. Treat these as current shared-host observations; the earlier gap figures remain separate. See [the comparison report](../reports/scalar-analysis-ducc-refresh-2026-10-15.md) and [raw data](../reports/data/scalar-analysis-ducc-refresh-2026-10-15/).

### f64 T1023 scalar and spin-1 DUCC timing refresh (M4, 2026-10-15)

Five alternating pairs measured GL STANDARD analysis at q1/q8. Spin-1 RustSHT/DUCC ratios were 0.97x and 1.14x, with narrow ranges. Scalar ratios were 1.32x and 2.13x; the scalar screen coincided with rising host load and remains provisional. q1 used CPU 0 and q8 used CPUs 8–15; the CPUs were not isolated. See [the report](../reports/t1023-analysis-ducc-refresh-2026-10-15.md) and [raw data](../reports/data/sht-t1023-ducc-refresh-2026-10-15/).

### Current-source scalar/spin-1 analysis phase profile (M4, 2026-10-15)

The release diagnostic measured T1023 GL STANDARD analysis at q1/q6 with the current parallel row-FFT path. Median recurrence and accumulation times were 116.038/24.190 ms for scalar and 174.656/40.564 ms for spin 1; median forward-FFT times were 35.728/7.017 ms and 72.083/14.098 ms, respectively. Recurrence and accumulation remain the largest measured phase. This unpinned phase profile is diagnostic and does not establish a paired performance change. See [the report](../reports/current-analysis-phase-profile-2026-10-15.md) and [raw data](../reports/data/current-analysis-phase-profile-2026-10-15/).

### f64 scalar T1023 DUCC timing follow-up (M4, 2026-10-15)

Five alternating fresh-process pairs measured GL STANDARD analysis at q1/q8. RustSHT/DUCC paired median ratios were `1.32x` and `1.67x`, with pair ranges `1.31–1.33x` and `1.62–1.77x`. These are lower than the prior provisional q8 ratio of `2.13x`; both implementations ran faster in this collection window. Load averages stayed near 2.6–3.7, but an unrelated unpinned Python process was active, so these remain shared-host observations. See [the report](../reports/t1023-scalar-ducc-followup-2026-10-15.md) and [raw samples](../reports/data/sht-t1023-scalar-ducc-followup-2026-10-15/).

### f32 spin-1 analysis overflow fallback (M4, 2026-10-15)

A review of the SIMD accumulator found that normal finite Fourier inputs and recurrence bases did not bound per-lane partial sums. Sixteen active lanes alternating `+f32::MAX` and `-f32::MAX`, with equal `plus` and `minus` bases, cancel in scalar order but can overflow when same-sign values accumulate in one SIMD lane. The SIMD path now checks its four output components for finiteness and recomputes nonfinite results in scalar order. This adds no workspace storage and avoids scanning the lanes a second time before each degree. The regression test passes through the private accumulator boundary; no public-transform reproducer is claimed.

Five alternating release-process pairs compared the guard with commit `a763ad8` at f32 spin-1 GL T511, q1 and q8, using `examples/m4_f32_bench.rs` (one warmup and five samples per operation). On the shared Ryzen 9 5950X host, the paired median analysis times changed by `-2.30%` at q1 and `-2.25%` at q8; q8 ranged from `-4.52%` to `+2.78%`. Synthesis control medians changed by `+0.02%` at q1 and `+0.58%` at q8. These are noisy no-regression observations, not claimed speedups. Both binaries estimate 12.13 MiB for core numeric arrays; process peak RSS was not collected. The raw inner samples, binary hashes, affinity and load record are in [the paired output](../reports/data/f32-simd-overflow-guard-2026-10-15/paired-process-output.txt), and the methodology and summaries are in [the report](../reports/f32-spin1-simd-overflow-fallback-2026-10-15.md). Workspace all-feature/no-default tests, formatting and Clippy pass with the fallback.

### Scalar analysis basis-cosine reuse (M4, 2026-10-15, rejected)

The candidate rewrote opposite-parity products from `(spectrum * basis) * cos(theta)` to `spectrum * (basis * cos(theta))`, sharing the latter product between real and imaginary components. Focused paired-analysis and fused-block tests passed, as did the release T511 GL DUCC/analytic test. Five alternating native T511 process pairs measured q1 at a median `0.94%` slower, with every pair slower; q8's paired median changed by `-1.39%` but its range was `-5.40%` to `+40.99%` due to one large slow pair. The candidate was removed. No workspace or FFT code changed. The raw measurements and exact patch are in [the report](../reports/scalar-analysis-basis-cosine-screen-2026-10-15.md) and [its data directory](../reports/data/scalar-analysis-basis-cosine-screen-2026-10-15/).

### Fused scalar analysis recurrence range checks (M4, 2026-10-15)

The f64 scalar fused analysis block now checks each computed recurrence value before it becomes the next current value. This removes a separate reread of both recurrence buffers after each update. A range failure declines the fused block; its staged coefficients are committed only on success, and the caller reruns the established scaled path. Recurrence equations, accumulation order and workspace size are unchanged.

Five alternating release-process pairs at T511/T1023 show q1 medians 6.16%/5.10% faster, with all five pairs favoring the candidate at each size. q8 medians were 3.05%/3.21% faster, but both paired ranges crossed zero and remain inconclusive. A fresh T1023 comparison measured RustSHT/DUCC 0.41.0 ratios of 1.25x at q1 and 1.56x at q8; this is a current-source comparison, not an isolated measurement of the change's effect on the ratio. At T1023, median peak RSS changed by -140 KiB at q1 and -368 KiB at q8. The report records the shared-host method, ranges, pinned DUCC revision, and raw data in [the scalar range-check report](../reports/scalar-analysis-range-check-fusion-2026-10-15.md) and [its data](../reports/data/scalar-analysis-range-check-fusion-2026-10-15/).

The late recurrence range-failure regression, all-feature and no-default Rust suites, workspace Clippy, formatting, and the release scalar T511/T1023/T2047 GL/CC/F1 DUCC and analytic check pass. Python tests were not run because bindings did not change. Evidence covers f64 scalar GL STANDARD performance at q1/q8 and T511/T1023; q8 performance remains inconclusive, and spin-1 performance was not measured for this change.

### Fused scalar analysis SIMD range-check reduction (M4, 2026-10-15, rejected)

A candidate accumulated the per-vector minimum range result and horizontally reduced once after each recurrence update, instead of reducing each SIMD vector group separately. Its focused regression covered a first-vector failure, a later-vector failure, and a scalar-tail failure. Five alternating source pairs at T511/T1023 improved q1 by 6.2–6.6%; all five T511 q8 pairs favored the candidate by a median 5.6%. T1023 q8 results were unstable: an initial five-pair screen had a +3.59% median change, a repeat had -4.93%, and the pooled ten-pair median was +2.40% with a -13.76% to +17.46% range. The candidate was removed because the high-band q8 result did not establish a repeatable gain. Formatting and the focused range regression passed; full suites and Clippy were not run for this rejected candidate. See [the screen report](../reports/scalar-analysis-range-check-min-aggregation-screen-2026-10-15.md) and [its data](../reports/data/scalar-analysis-range-check-min-aggregation-screen-2026-10-15/).

### Fused scalar analysis cosine-square reuse (M4, 2026-10-15, rejected)

A candidate precomputed `cos(theta)^2` once per active latitude block and reused the same f64 values for each degree recurrence update. Five paired q1 process comparisons at T511/T1023 favored the candidate by medians of 1.04%/1.86%, with every pair faster. q8 results were noisy and did not establish a gain: across ten pairs, the median paired change was -1.78% at T511 and +0.45% at T1023, with ranges spanning -21% to +26% and -17% to +30%. The candidate was removed. Three focused fused-block tests and formatting passed; full suites and Clippy were not run for this rejected candidate. See [the screen report](../reports/scalar-analysis-cosine-square-hoist-screen-2026-10-15.md) and [its data](../reports/data/scalar-analysis-cosine-square-hoist-screen-2026-10-15/).

### f64 spin-1 analysis recurrence range checks (M4, 2026-10-15)

The fused Q/U scalar-conversion path now checks each recurrence value as it is computed, rather than rereading both recurrence buffers in a separate range pass. A declined block discards staged coefficients and uses the established scaled fallback. Five paired q1 comparisons improved by 3.24% at T511 and 2.60% at T1023, with every pair faster. Ten-pair q8 medians were near neutral at T511 (-0.11%) and slightly faster at T1023 (-0.88%); both ranges cross zero. At T1023, a fresh DUCC 0.41.0 comparison measured current RustSHT/DUCC ratios of 0.94x at q1 and 1.14x at q8. The report records the outlying DUCC q1 run, call boundaries, numerical checks and raw data in [the spin-1 report](../reports/spin1-analysis-range-check-fusion-2026-10-15.md) and [its benchmark data](../reports/data/spin1-analysis-range-check-fusion-screen-2026-10-15/).

The late-range-failure regression, all-feature and no-default suites, workspace Clippy, formatting, and release DUCC/analytic spin-1/2 checks at T511 and T1023 on GL/CC/F1 pass. No workspace storage or Python binding changed. Evidence covers f64 spin-1 GL STANDARD analysis at q1/q8 and T511/T1023; q8 changes remain within measurement variation.

### Fused scalar analysis range-mask reduction (M4, 2026-10-15)

The f64 fused scalar analysis block now combines recurrence validity masks across SIMD vector groups and performs one horizontal reduction per recurrence update. Scalar-tail checks and transactional fallback are unchanged. Five paired q1 processes improved by 5.08% at T511 and 4.91% at T1023; all ten pairs favored the candidate. q8 medians improved by 2.63% at T511 across 20 pairs and 2.48% at T1023 across 10 pairs, but both ranges crossed zero. Callgrind references for the T511 q8 process fell by 3.20%; this is instruction-count evidence, not elapsed time. A current T1023 comparison measured RustSHT/DUCC 0.41.0 ratios of 1.16x at q1 and 1.63x at q8; the q8 RustSHT times varied across processes. Formatting, workspace Clippy, all-feature and no-default suites, release fused-block tests and scalar T511/T1023/T2047 GL/CC/F1 DUCC and analytic checks passed. Python tests were not run because bindings did not change. See [the range-mask report](../reports/scalar-analysis-range-mask-reduction-2026-10-15.md) and [its benchmark data](../reports/data/scalar-analysis-range-mask-reduction-2026-10-15/).

### T1023 f32 scalar and spin-1 reference check (M4, 2026-10-16)

An opt-in release integration test compares scalar and spin-1 STANDARD synthesis and analysis against `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, on GL `1024×2048`, full T1023, `phi0=0.37`, and one thread. The deterministic float32 maps and complex64 coefficients cover eight map samples per output channel and nine low, interior, and high harmonic modes per analysis channel. The test also checks sums and Euclidean norms of the complete outputs. It passes selected-value absolute bounds of `2.5e-5` for maps and `2e-6` for coefficients, sum bound `1e-3`, and norm bound `3e-4`. Full-array relative-L2 errors are `4.87e-5`/`1.67e-5` for scalar synthesis/analysis and `3.84e-5`/`2.07e-5`/`2.56e-5` for spin-1 synthesis channel 0/analysis channel 0/analysis channel 1, with spin-1 synthesis channel 1 at `4.19e-5`. No T1023 f32 acceptance threshold has been established; these results characterize accuracy without closing that decision. The reference generator, comparison script, JSON results, command, and checksums are in [the report](../reports/f32-t1023-ducc-validation-2026-10-16.md) and [its data directory](../reports/data/f32-t1023-validation-2026-10-16/).

The test can optionally dump complete RustSHT arrays under `target/` for comparison with DUCC. The full-array relative-L2 values are reported in the extension report; generated arrays are not committed. The no-default-feature test initially attempted to compile the Rayon-only `m1_f32_atmospheric_bench` example. Its Cargo target now declares the existing `parallel` feature requirement, consistent with the other parallel benchmark examples. Workspace Clippy, the all-feature suite, the no-default-feature suite, and the focused T1023 reference test pass. The shared `/tmp` filesystem was full during one attempt, so successful workspace commands used a temporary directory under `target/`.

### Guarded scalar f32 normalization skip at T1023 (E, 2026-10-16)

The scalar f32 recurrence skips repeated lane-by-lane normalization when its recurrence pairs remain in the normal range with zero binary exponents. A Pulp SIMD range check calls the established normalizer when any lane leaves that range. Compile-time specialization leaves the original per-degree path in use for every truncation except the measured T1023 case; there is no workspace or public API change.

Three alternating native process pairs on GL `1024×2048`, full T1023, and `phi0=0.37` reduced scalar STANDARD synthesis and analysis by 4.39% and 4.49% at one thread and 2.95% and 1.10% at eight threads. Every paired comparison favored the candidate. The T720 disabled-path control did not regress; its paired changes ranged from −1.04% to −0.23%. An intermediate per-degree runtime gate regressed T720 q8 synthesis and analysis by 3.20% and 2.63%; that implementation was replaced by the const-generic dispatch.

The final candidate's six complete scalar and spin-1 f32 arrays are bitwise identical to baseline outputs. The T1023 scalar and spin-1 STANDARD outputs match DUCC 0.41.0 with full-array relative L2 errors from `1.67e-5` to `4.87e-5`; no T1023 f32 acceptance threshold is established. Boundary tests cover SIMD lanes, a scalar tail, range endpoints, exceptional values, and exponent state. Formatting, workspace Clippy, all-feature tests (188 unit and 11 f32 passed; 13 unit and 1 f32 ignored), no-default tests (150 unit and 11 f32 passed; 7 unit and 1 f32 ignored), and the opt-in DUCC test pass. Python tests were not run because bindings did not change. Exact timings, candidate hashes, comparisons, commands, and logs are in [the report](../reports/f32-scalar-normalization-skip-2026-10-16.md) and [its data directory](../reports/data/f32-scalar-normalization-skip-2026-10-16/).
