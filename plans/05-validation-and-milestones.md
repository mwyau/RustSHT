# Verification, benchmarks and milestones

## Validation contract

A transform is not validated by a single roundtrip. For each supported geometry **GL, CC, F1**, spin, dtype, operator and grid sampling, compare to pinned DUCC output **and** at least one independent mathematical check.

- **Synthesis**: constants, isolated normalized `Y_lm`, pure sectoral/near-sectoral modes, mixed random coefficients and high-`m` polar cases.
- **Analysis**: recover coefficients of independently synthesized bandlimited fields; compare to direct quadrature for sufficiently small GL problems; test CC/F1 high-bandlimit cases that ordinary low-resolution quadrature would not recover.
- **Adjoints**: verify separately `<S a, x>_map = <a,S* x>_alm` and `<A x,a>_alm = <x,A* a>_map` with the real harmonic inner product (factor two for `m>0` modes) and documented map dot product.
- **Spin/vector**: spin=0,1,2 standard cases, E/B pure components, geographic `(-v,u)` convention, analytic derivative identities. Spin 3 and 4 pass all-grid full-operation checks at `lmax=spin`, sparse synthesis checks through T2047, and selected exact-analysis coefficient checks through T1023; spin 5 and above remain outside the operation contract pending separate verification.
- **Derivative modes**: `GRAD_ONLY` equals STANDARD synthesis when curl coefficients are zero; check `DERIV1` signs/scaling and the corresponding **adjoint-synthesis** modes. `analysis_2d` remains STANDARD-only.
- **Grid geometry**: validate nodes, `4*pi` summed weights, CC poles, F1 half-grid offset, GL ordering and longitude `phi0` shift. For exact analysis, require `lmax<=ntheta-1` (GL/F1), `lmax<=ntheta-2` (CC), and `mmax<=floor((nphi-1)/2)`; test both `nphi=2*mmax+1` and `2*mmax+2`.
- **Layout and safety**: `mmax<lmax`, rejection of nonreal input `m=0` coefficients and nonzero `l<spin` coefficients, odd/even longitude, contiguous/strided and batched inputs where supported, aliasing, invalid sizes and integer overflow.
- **Threading and SIMD**: scalar and vectorized kernels agree within established tolerances; results are independent of thread count within documented floating-point reproducibility limits. Use absolute and norm-relative errors with a sensible near-zero floor; report worst error and RMS/distribution, not just roundtrip l2. Define acceptance thresholds *before* evaluating benchmarks. Distinguish algorithmic error from known floating-point order differences; never relax a tolerance merely to hide failures.

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
| CI | T127, T255 | All supported spins/operations on available grids |
| High | T511, T1023 | Stable recurrence, high-spin and CC/F1 analysis |
| Extended | T2047, T4095 | High-bandlimit comparisons on suitably provisioned hosts |
| Stress | T8191+ | Explicit opt-in, track peak memory and numerical behavior |

Stress tiers are **not release promises**. Publish a tested maximum for each `(grid, dtype, spin/mode, operation)` rather than asserting a uniform numerical maximum from one case. Test minimum permitted ring count and oversampled ring count separately.

## Milestones (approved D11 ordering)

**M0 — Project setup:** GPL license, citations, AGENTS.md, plans, Cargo scaffold and CI configuration. No SHT code yet.

**M1 — Complete GL operation family**

- **M1a:** Core Rust harmonic/grid specs, packed layout, faithful DUCC GL iterative/Bogaert node/weight algorithms, RustFFT plus a private real-input adapter, stable scalar f64 synthesis/analysis, scalar reference, `pulp` SIMD recurrence, coarse Rayon scheduling and initial release-mode profiling.
- **M1b:** GL spin=1/2 STANDARD synthesis/analysis, vector E/B and applicable GRAD_ONLY/DERIV1 synthesis on the shared DUCC-derived scaled SIMD recurrence. Design for higher spins; record validated range.
- **M1c:** GL adjoint synthesis and adjoint analysis; GRAD_ONLY/DERIV1 adjoint synthesis; inner-product proofs/tests, GL full-operation parity and high-`L` accuracy gate.

**M2 — Same engine, additional grids**

- **M2a:** CC geometry and analysis/adjoint-analysis prepared-CC resampling; test every GL-supported operator on CC, including near-minimal ring counts.
- **M2b:** F1 staggered geometry and its resampling/adjoint; repeat the full operation matrix.

**M3 — High-bandlimit acceptance:** Complete all-grid f64 parity, analytic/DUCC comparisons and actual tested degree/spin limits at T511/T1023 and opt-in T2047+. Fix critical numerical defects before focusing on speed.

**M4 — Performance tuning and dtype:** Tune the SIMD, coarse threading and scratch layout implemented during M1; implement and verify f32 and report reproducible DUCC benchmarks.

**M5 — Python binding:** Optional PyO3/NumPy numerical API; plan-oriented plus procedural functions, packaging, GIL release, shape/dtype tests and wheel license review.

**M6 — spharmgrid:** Optional RustSHT backend matching its scalar/derivative/vector adapter, with independent parity, thread behavior and packaging review. Keep DUCC default until tested.

## Long autonomous implementation sessions

Start from current `main` on a **new feature branch**, using [00](00-scope-and-decisions.md) without routine human checkpoints:

1. Establish normalization/phase and implement **M1a**: DUCC-derived GL geometry, scalar f64 S/A, public plan, RustFFT with verified in-tree real-input adapter, reusable scratch, scalar-reference tests, `pulp` SIMD recurrence, coarse Rayon scheduling and initial release benchmarks.
2. Implement **M1b**: GL spin 1/2 STANDARD, E/B and applicable derivative/gradient synthesis; then **M1c**: STANDARD adjoints and derivative/gradient adjoints, with independent identities.
3. Once GL tests pass, complete **M2a CC** and **M2b F1** on the same recurrence engine, including CC/F1 prepared-CC meridional resampling and its adjoint.
4. Run affordable **M3** high-degree checks. If possible continue with **M4** f32 and tuning existing SIMD/thread scheduling, comparing AVX2 against AVX-512 on suitable hosts; proceed to **M5** Python functions/plan and **M6** spharmgrid adapter only when earlier work is verified.
5. Commit after each independently functioning, tested increment, then move on. Run relevant format, lint and numerical tests before commits, plus full suites periodically. Do not add unfinished placeholders, weaken tests, create temporary GitHub Actions jobs or commit large generated fixtures. Skip resource-intensive optional stress/benchmark tests with an explicit note rather than blocking other work. End with branch and commit list, operators/grids/spins/degrees tested, reference revision, command results, blockers and skipped checks. Push only when explicitly authorized for that session; do not merge into `main`.

## Required evidence for each merged numerical milestone

- A mapped algorithm/reference source and source-provenance note if any code is adapted.
- Analytic and DUCC-backed tests demonstrating the claimed operator and grid.
- Explicit tested precision/spin/bandlimit/shape matrix with failure cases.
- Profiling only when performance is claimed; commit the benchmark methodology and environment, not large unexplained generated fixtures. The milestone ordering is fixed by D11; exact spin range, release tolerances, threading defaults and high-`L` performance acceptance remain open in [00](00-scope-and-decisions.md).

## Recorded implementation and numerical checks

### GL geometry (2026-10-12)

`GridSpec` now constructs north-to-south GL colatitudes and ring weights for `ntheta >= 1`. The implementation uses the DUCC Newton iteration through 100 rings and its modified Bogaert/FastGL approximations above 100 rings. The Rust Newton path uses f64; DUCC uses C++ `long double` for this branch. The comparison executable compiled `ducc0/math/gl_integrator.cc` from revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` with g++ 15.2.0 and `-O2`. It compared every colatitude and `2π`-scaled ring weight for `ntheta = 1, 2, 3, 99, 100, 101, 127, 255, 1024` (1,712 values per quantity). The largest absolute colatitude difference was `1.946e-15` rad. The largest absolute ring-weight difference was `7.416e-16`; the largest relative ring-weight difference was `1.607e-13`, at the small endpoint weights for `ntheta=100`. The outputs for `ntheta >= 101` matched at the printed f64 precision. Independent checks cover closed-form `ntheta=1,2,3` rules, all polynomial moments through degree `2*ntheta-1` for `ntheta=1..16`, symmetry and `4π` weight sums at `ntheta=99,100,101,127,255,1024`, and invalid dimensions and `phi0`. On an x86-64 CPU with rustc 1.99.0, `cargo fmt --all -- --check`, `cargo test --workspace --all-features` (6 passed), and `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass. No transform operator or SIMD recurrence has been validated yet.

### Packed harmonics and real FFT adapter (2026-10-12)

`HarmonicSpec` stores inclusive degree/order bounds and nonnegative spin, computes a checked order-major packed length, and provides safe `(l,m)` indexing. Its input validator checks the packed length, real `m=0` coefficients, and zero coefficients below the spin floor. Public coefficients use a direct `num-complex` dependency and the crate's `Complex64` alias. The private FFT adapter uses a half-length complex RustFFT with pair packing and Hermitian reconstruction for even longitude counts; odd counts use a full-complex RustFFT. Forward spectra use the unnormalized DFT convention. Inverse execution divides by the transform length. Plans hold the RustFFT plans and even-length twiddles; each caller-owned workspace holds transform and scratch buffers. FFT comparisons use the full-complex RustFFT at lengths `1,2,3,4,5,6,7,8,31,32,33`, covering impulses, constants, cosine modes, a mixed row, phase-shifted modes, real DC/Nyquist edges and inverse scaling. Arbitrary valid half-spectra are also checked against a direct Hermitian inverse sum. The harmonic metadata tests cover triangular and rectangular layouts, invalid bounds, overflow, coefficient lengths, `m=0` reality and the spin floor. On the x86-64 CPU with rustc 1.99.0, all 15 Rust unit tests, the workspace clippy command with warnings denied, formatting check, and `uv run pytest python/tests` (2 passed) pass. These checks validate the metadata and FFT helper only; no SHT transform operator or SIMD recurrence has been validated.

### Scalar GL synthesis and analysis (2026-10-12)

`ShtPlan` now combines GL geometry, packed scalar harmonics, the real FFT adapter and sectoral normalization metadata. `ShtWorkspace` holds the per-ring Fourier spectra, longitude row, FFT scratch and a reusable epsilon array. The transform fills epsilon factors once per order and reuses them across latitude rings. Scalar synthesis allows the real Nyquist order on even `nphi` and folds its positive/negative pair into the real Nyquist bin. Exact scalar GL analysis requires `ntheta >= lmax+1` and `mmax <= floor((nphi-1)/2)`. Both operations reject nonzero spin until spin recurrences are implemented. The scalar kernel uses DUCC's sectoral normalization, normalized three-term epsilon recurrence, shared binary rescaling and polar order cutoff, adapted from `sht_inner_loop.h` at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Independent closed forms cover `Y00`, `Y10`, `Y20`, `Y21`, and `Y22`; analytic synthesis and exact analysis use nonzero `phi0`, `mmax < lmax`, and both minimum odd (`nphi=5`) and minimum even (`nphi=6`) longitude counts for `mmax=2`. An even-longitude synthesis test covers the real Nyquist order. Static transform references generated with DUCC package `ducc0==0.41.0` cover scalar synthesis and analysis at `lmax=mmax=2`, `ntheta=3`, `nphi=5`, `phi0=0.37`, and one thread. On an x86-64 CPU with rustc 1.99.0 release builds, full 256-by-513 T255 synthesis maps at `phi0=0.37` were compared against `ducc0==0.41.0` with one thread. A sectoral field (`a_255,255=1`) had maximum absolute error `1.710e-14`, relative L2 error `3.412e-15`, and RMS error `1.088e-15`. A mixed field populated each packed coefficient as `10^-5 * (((7l+11m) mod 19)-9 + i*(((13l+5m) mod 23)-11))`, with the imaginary part set to zero for `m=0`; it had maximum absolute error `5.558e-15`, relative L2 error `3.102e-14`, and RMS error `1.835e-16`. Analysis of the map `map[i,j] = sin(0.019i + 0.037j) + 0.3cos(0.071i - 0.013j)` produced maximum absolute coefficient error `8.876e-15`, relative error `2.229e-14` in the real harmonic inner product, and coefficient RMS error `2.275e-16`. Unit tests retain selected DUCC samples and coefficients for these cases. The tests cover f64, spin zero, GL, scalar S/A through T255; adjoints, multithread scheduling, spins and other grids remain unvalidated. The 30-test workspace suite, formatting check and clippy with warnings denied pass on Rust 1.89.0 and 1.99.0; `uv run pytest python/tests` passes 2 tests.

### Latitude-blocked SIMD recurrence (2026-10-12)

The scalar GL synthesis and analysis operations dispatch their normalized epsilon recurrences through `pulp 0.22.3`. Each order traverses contiguous latitude lanes in blocks of 64 rings, with per-ring binary exponents retained for high-degree scaling. The scalar recurrence remains the fallback. Workspace scratch holds three recurrence buffers and exponents for at most one block, plus cached latitude sines and cosines and the per-order polar cutoff mask for the full grid. The crate enables pulp's AVX2/FMA and AVX-512 backends for runtime CPU detection. This host is an x86-64 CPU with AVX2/FMA and no AVX-512, so AVX2/FMA and scalar fallback paths were executed; AVX-512 and AArch64 NEON were not executed. The recurrence tests compare the scalar reference, pulp scalar fallback and runtime-selected backend for orders 0 through 8 and degree 32 over 11 latitudes, then compare order 255 through degree 768 over 11 near-polar and interior latitudes. A separate test forces both binary rescaling directions in independent lanes. These cases cover vector blocks, tail lanes and recurrence scratch reused across ring blocks. Analytic checks and DUCC 0.41.0 static references exercise the dispatched transform path through T255. The complete Rust workspace has 32 passing tests; format, clippy with warnings denied, and Python import tests pass on Rust 1.89.0 and 1.99.0. The pre-Rayon benchmark mode measured T511 scalar GL synthesis and analysis on an x86-64 CPU with rustc 1.99.0, release profile, AVX2/FMA runtime dispatch and one thread. Each operation had one warmup followed by five timed samples; each reported sample is the median. Across three runs, the 64-ring-block synthesis medians were 1.592 s, 1.588 s and 1.611 s, and analysis medians were 2.282 s, 2.278 s and 2.295 s. With a single 512-ring block, the synthesis medians were 1.533 s, 1.532 s and 1.541 s, and analysis medians were 2.529 s, 2.541 s and 2.564 s. Comparing the medians of these run medians, 64-ring blocks reduced analysis time by 10.2% and increased synthesis time by 3.8%. The estimated numeric-array working set was 12.05 MiB, including input/output coefficients, map, Fourier spectra, grid arrays and workspace arrays, and excluding RustFFT scratch and executable/runtime allocations. This single-host measurement supports the block choice for analysis but does not establish a general speedup; the synthesis cost remains a tuning target.

### Coarse Rayon scheduling (2026-10-12)

With the `parallel` feature, synthesis partitions contiguous latitude groups across Rayon tasks. Analysis partitions disjoint order-major coefficient ranges across tasks. Each task uses a recurrence scratch set owned by `ShtWorkspace`; the serial path uses one set. The workspace sizes its worker groups using the active Rayon pool at construction. The real FFT row loop remains serial. The 33-test workspace suite now includes one-thread versus two-thread comparisons for analytic synthesis and analysis on 131 latitude rings. DUCC 0.41.0 reference checks run through the parallel dispatch path on the available global Rayon pool. `cargo test -p rustsht --no-default-features` passes all 32 non-Rayon tests. The x86-64 CPU has host topology omitted; AVX2/FMA is available and AVX-512 is not. AVX2/FMA and scalar fallback paths passed recurrence checks. AVX-512 has not been executed. On Rust 1.89.0 and 1.99.0, the workspace tests, formatting check and Clippy with warnings denied pass; `uv run pytest python/tests` passes two tests. The T511 harness measured one-thread and eight-thread runs with rustc 1.99.0, release profile, f64 and AVX2/FMA runtime dispatch. Each operation had one warmup and five timed samples per run. Across two runs, synthesis medians were 1.555 s and 1.564 s with one thread, and 0.227 s and 0.234 s with eight threads. Analysis medians were 2.276 s and 2.283 s with one thread, and 0.556 s and 0.563 s with eight threads. Ratios of the average run medians give 6.8x synthesis and 4.1x analysis speedups. Estimated numeric-array storage was 12.10 MiB for the eight-worker workspace, excluding RustFFT scratch and executable/runtime allocations. This timing covers the recurrence scheduling benefit; the forward and inverse FFT loops are still serial, and no explicit scalar-versus-AVX2 or AVX-512 timing was recorded. Analysis now splits order ranges recursively with `rayon::join`, which removes the previous per-call job descriptor vector while retaining disjoint coefficient and recurrence scratch slices. A one-run T511 confirmation after this change measured synthesis at 1.561 s with one thread and 0.230 s with eight, and analysis at 2.301 s with one thread and 0.556 s with eight. These medians are not included in the two-run summary above.

### GL spin and derivative synthesis (2026-10-12)

Spin-1 and spin-2 GL plans now provide STANDARD E/B synthesis and analysis. The shared spin recurrence also supports synthesis with B set to zero (`GRAD_ONLY`) and scalar angular-derivative synthesis (`DERIV1`). The recurrence coefficients, half-angle initialization and normalization are adapted from DUCC's `sht_inner_loop.h` at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; the Rust implementation uses per-lane binary scaling and caller-owned latitude-block scratch. The public numeric contract remains spin 1/2. Spin 3 and higher, adjoints and non-GL grids remain unimplemented. The static DUCC fixtures use `ducc0==0.41.0`. For spin 1 and 2, synthesis and analysis samples on GL 3x7 at `phi0=0.37` match the reference within `5e-14` for map samples and `8e-13` for selected E/B coefficients. Analytic checks cover the lowest-degree pure E modes, gradient equivalence with STANDARD synthesis at B=0, and the spin-1 scalar derivative identity. GL T255 tests cover mixed E/B synthesis samples, sectoral modes and selected analysis coefficients for both spins; the retained fixture tolerances are `2e-12` for synthesis samples, `3e-11` for sectoral samples, and `2e-10` for selected analysis coefficients. Exact analysis tests include minimum odd/even longitude lengths and rectangular `mmax < lmax` layouts. A one-thread/two-thread comparison covers synthesis and analysis on GL 131x11. The 41-test Rust workspace passes with Rust 1.89.0 and 1.99.0; Clippy with warnings denied passes on both, format checks pass, and `uv run pytest python/tests` passes two tests. Current evidence is f64, GL, spin 1 and 2, STANDARD synthesis/analysis and gradient/derivative synthesis through T255; it does not yet include an adjoint identity or full coefficient/map error distribution, which remain M1c validation. Adding spin scratch also increases each workspace's numeric-array estimate. A single T511 run on the x86-64 CPU with rustc 1.99.0, release profile, f64 and AVX2/FMA runtime dispatch measured synthesis at 1.572 s with one thread and 0.232 s with eight, and analysis at 2.264 s with one thread and 0.546 s with eight; each value is the median of five timed samples after one warmup. The harness estimates 16.23 MiB for the eight-worker numeric arrays, including a second Fourier spectrum and per-worker positive/negative spin recurrences and factors; RustFFT scratch and executable/runtime allocations remain excluded. This one-run measurement records the current workspace footprint and does not replace the two-run M1a timing summary above.

### GL adjoints (2026-10-12)

Scalar GL plans now provide `S*` and `A*`. Spin-1/2 plans provide STANDARD `S*` and `A*`, and the gradient-only and scalar DERIV1 synthesis modes have their own adjoint-synthesis operations. Exact analysis and its adjoint enforce `ntheta >= lmax+1` and `mmax <= (nphi-1)/2`. Synthesis and its adjoints accept `mmax <= nphi/2`, including even-longitude Nyquist. The `S*` map product is an unweighted pixel sum and the real harmonic product weights `m>0` by two; `A*` scales synthesized samples on each GL ring by `ring_weight/nphi`. Static DUCC fixtures use `ducc0==0.41.0` and the pinned transform implementation at revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Scalar `S*`/`A*`, spin-1/2 STANDARD `S*`/`A*`, GRAD_ONLY `S*`, and DERIV1 `S*` are checked at GL 3x7 with `phi0=0.37`; scalar, spin and gradient STANDARD `S*` are also checked at GL 3x4 with the Nyquist order and nonzero `phi0`. The tests check separate adjoint identities rather than roundtrips. T255 identities cover scalar `S*`, `A*` and DERIV1 `S*`, and spin-1/2 STANDARD `S*`, `A*` and GRAD_ONLY `S*`; the scalar identity tolerance is `3e-10` relative to `max(|reference|,1)`, and the spin identity tolerance is `2e-8` absolute. At M1c, `cargo test --workspace --all-features` passed 47 tests with rustc 1.99.0 and 1.89.0; `cargo test -p rustsht --no-default-features` passed 45 tests. Formatting and Clippy with warnings denied passed on both compiler versions, and `uv run pytest python/tests` passed two tests.

### CC geometry and prepared-grid analysis (2026-10-12)

`GridSpec` constructs CC colatitudes at `theta_i = i*pi/(ntheta-1)` with both poles and the FFT-based Waldvogel weights. `ShtPlan` prepares the CC resampling grids and FFTs. Exact analysis maps longitude Fourier modes to the prepared CC grid, applies the CC meridional weights and projects with the scalar or spin recurrence. `A*` synthesizes on that prepared grid and applies `resample_from_prepared_CC`; `S*` continues to use the unweighted grid projection. Exact CC analysis requires `ntheta >= lmax+2` and `mmax <= floor((nphi-1)/2)`. The prepared ring count follows DUCC's `good_size_complex(lmax+1)+1` rule. The translated code in `grid.rs` and `meridional.rs` cites DUCC revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; the FFT-size helper uses the GPL-2.0-or-later option of the DUCC FFT header's dual license. Static geometry references from `ducc0==0.41.0` cover CC weights at `ntheta=2..5`; independent moments through degree `ntheta-1` and `4*pi` sums pass for `ntheta=2..20`. Scalar and spin-1/2 `S`, `A`, `S*` and `A*` references cover the minimum CC ring count at `4x5` with `phi0=0.37`, including adjoint identities. Scalar and spin analysis also cover the minimum even longitude count (`nphi=6`), and scalar and spin `A`/`A*` cover oversampled CC rings at `7x5`. Scalar DERIV1 and spin-1/2 GRAD_ONLY synthesis and their adjoints have static DUCC references and adjoint identity checks at `4x5`. The scalar T255 sectoral field at minimum `ntheta=257`, `nphi=513` agrees with its closed-form map within `4e-12` at every sample; exact analysis recovers every packed coefficient within `2e-9`. The M2a tests cover f64, scalar and spin 1/2, CC, scalar T255 and low-degree derivative and spin modes through T2. F1, f32 and spin 3+ are not implemented. On an x86-64 CPU, `cargo test --workspace --all-features` passed 57 tests with rustc 1.99.0 and Rust 1.89.0; `cargo test -p rustsht --no-default-features` passed 55 tests with rustc 1.99.0. Formatting and Clippy with warnings denied passed on both compiler versions, and `uv run pytest python/tests` passed two tests.

### F1 geometry and prepared-grid analysis (2026-10-12)

`GridSpec` constructs pole-excluded F1 colatitudes at `theta_i = (i+1/2)*pi/ntheta` and Waldvogel weights. The prepared-CC operator handles F1's reflected Fourier profiles with the half-ring phase shift, truncation or padding to the prepared CC grid, and the matching adjoint resampling. Exact F1 analysis requires `ntheta >= lmax+1`; longitude limits match GL and CC. The implementation and quadrature translations cite DUCC revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Static `ducc0==0.41.0` references cover scalar and spin-1/2 `S`, `A`, `S*`, `A*`, scalar DERIV1 and spin GRAD_ONLY at the minimum F1 ring count (`3x5`). Adjoint identities are checked for every operation family. Minimum even longitude tests (`3x6`) cover scalar and both spins; oversampled rings (`7x5`) cover scalar and both spins. F1 weights match DUCC at `ntheta=1..6`; independent moments through degree `ntheta-1` and `4*pi` sums pass for `ntheta=1..20`. At the scalar T255 minimum grid (`256x513`), synthesis agrees with the sectoral closed form at every sample within `4e-12`, and analysis recovers all packed coefficients within `2e-9`. The M2b validation domain is f64, GL/CC/F1, scalar and spin 1/2, scalar T255, and low-degree derivative/gradient modes through T2. Spin 3+ and f32 remain unimplemented. On an x86-64 CPU, `cargo test --workspace --all-features` passed 65 tests with rustc 1.99.0 and Rust 1.89.0; `cargo test -p rustsht --no-default-features` passed 63 tests with rustc 1.99.0. Formatting and Clippy with warnings denied passed on both compiler versions, and `uv run pytest python/tests` passed two tests.

### High-bandlimit f64 checks (2026-10-12)

Opt-in release tests extend scalar sectoral synthesis and exact analysis to T511, T1023 and T2047 on GL, CC and F1. The tests use `nphi=2*lmax+3`, minimum exact-analysis ring counts, and compare every synthesized pixel with the closed-form sectoral field. Five middle-ring values per grid and degree are compared with static outputs from `ducc0==0.41.0`. Analysis checks every packed coefficient against the input sectoral coefficient. The maximum errors across the three grids were:

| Degree | DUCC sample max absolute error | Analytic map max absolute error | Analytic map RMS error | Analysis coefficient max absolute error | Analysis coefficient RMS error |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | `5.773e-15` | `1.742e-12` | `5.489e-14` | `1.288e-14` | `3.556e-17` |
| T1023 | `1.257e-14` | `4.132e-12` | `1.070e-13` | `1.199e-14` | `1.657e-17` |
| T2047 | `2.083e-13` | `9.958e-12` | `2.241e-13` | `4.197e-14` | `2.898e-17` |

A second opt-in release test covers spin-1 and spin-2 synthesis and analysis on all three grids at T511 with minimum exact-analysis sampling. Static DUCC references cover five middle-ring samples for each map component and selected coefficients from independently generated maps; analysis of the synthesized sectoral fields checks every E/B coefficient. Across the six `(grid, spin)` cases, the largest DUCC synthesis-sample error was `8.837e-14` (largest case RMS `5.464e-14`), the largest selected DUCC analysis-coefficient error was `2.533e-17` (largest case RMS `1.446e-17`), and the largest synthesis-analysis coefficient error was `3.575e-14` (largest case RMS `6.976e-17`). A third opt-in release test covers sparse spin-1 and spin-2 synthesis at T1023 on all three grids with 65 rings and `nphi=2049`. Static `ducc0==0.41.0` fixtures compare five middle-ring samples for each map component. The largest fixture error across the six `(grid, spin)` cases is `2.154e-13` (largest case RMS `1.359e-13`). STANDARD synthesis with B set to zero agrees with GRAD_ONLY synthesis at every map sample. The separate exact-sampling test below covers T1023 spin analysis. A fourth opt-in release test covers spin-1 and spin-2 synthesis and exact analysis at T1023 on GL/F1 with 1024 rings and CC with 1025 rings; each uses `nphi=2049`. Static DUCC fixtures compare five middle-ring samples per component and selected coefficients from independently generated maps. The largest DUCC synthesis-sample error across the six `(grid, spin)` cases is `3.020e-13` (largest case RMS `1.901e-13`), the largest selected analysis-coefficient error is `9.022e-18` (largest case RMS `4.533e-18`), and the largest synthesis-analysis coefficient error is `6.839e-14` (largest case RMS `6.677e-17`). Analysis of each synthesized high-degree mode checks every E and B coefficient. An opt-in release test checks sparse spin-1 through spin-4 synthesis at T2047 on GL, CC and F1 with 65 rings and `nphi=4097`. A single E-sectoral coefficient is synthesized with zero B coefficients. Every map sample is compared with the closed form for the normalized sectoral spin harmonics. Across grids, the maximum DUCC sample error, analytic map error and analytic RMS error for each spin are:

| Spin | DUCC sample max absolute error | Analytic map max absolute error | Analytic map RMS max |
| ---: | ---: | ---: | ---: |
| 1 | `2.146e-13` | `1.344e-11` | `5.274e-13` |
| 2 | `2.112e-13` | `1.236e-11` | `3.883e-13` |
| 3 | `2.169e-13` | `1.218e-11` | `4.259e-13` |
| 4 | `2.096e-13` | `1.013e-11` | `2.122e-13` |

Five middle-ring samples per component are compared with static `ducc0==0.41.0` references. The three middle rings lie at the equator. The expanded test ran in 33.80 s in release mode on an x86-64 CPU with rustc 1.99.0 and AVX2/FMA runtime dispatch; DUCC used one thread. A routine test validates spin 3 and 4 on all three grids at `lmax=mmax=spin`, with minimum exact-analysis latitude counts and `nphi=2*lmax+3`. Synthesis of the `a^E_(spin,0)=1` field matches its analytic form, `q(theta)=-sqrt((2spin+1) binom(2spin,spin)/(4*pi)) (sin(theta)/2)^spin`, with a zero second component at every sample. DUCC 0.41.0 references cover three synthesis samples per grid/spin and selected coefficients from arbitrary-map analysis. Both spins pass STANDARD `S/A/S*/A*`, `GRAD_ONLY` synthesis and its adjoint identity on GL, CC and F1. Across the six cases, maximum DUCC synthesis-sample error is `2.776e-16`, maximum analytic-map error is `1.665e-16` (largest RMS `7.905e-17`), maximum selected DUCC analysis-coefficient error is `1.776e-15`, maximum `A(S)` coefficient error is `1.157e-15`, and the largest absolute adjoint inner-product residual is `3.331e-16`. This full-operation check covers spin 3/4 through `lmax=spin`; spin 5 and above remain rejected. An opt-in release check extends spin-3/4 STANDARD synthesis and exact analysis to T511 on GL, CC and F1, using minimum exact-analysis latitude counts and `nphi=1025`. Sectoral synthesis is compared at five middle-ring samples with static `ducc0==0.41.0` references and at every pixel with the closed-form Wigner-d expression. Analysis of deterministic arbitrary maps is compared with DUCC for three selected E/B mode pairs per case. Across six cases, the maximum DUCC synthesis-sample error is `6.528e-14`, the maximum analytic-map error is `2.169e-12` (largest RMS `9.632e-14`), and the maximum selected DUCC analysis-coefficient error is `2.273e-17`. The test ran in 3.63 s in release mode on the x86-64 CPU with rustc 1.99.0 and AVX2/FMA runtime dispatch; DUCC used one thread. An additional opt-in test checks spin-3/4 exact analysis at T1023 on GL, CC and F1 using the minimum exact-analysis latitude counts and `nphi=2049`. Analysis of deterministic arbitrary maps is compared with DUCC for the lowest `m=0` coefficient and the sectoral `(lmax,mmax)` coefficient in both E and B. The largest absolute coefficient error across six cases is `8.882e-18`. This test checks selected output coefficients and does not claim a full coefficient comparison at T1023. The checks ran with rustc 1.99.0 in release mode on an x86-64 CPU with AVX2/FMA runtime dispatch; DUCC reference outputs use one thread. The Rust tests use the global Rayon pool. The high-bandlimit tests are ignored by the routine suite and run explicitly with `cargo test --release --lib transform::tests::m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_t511_all_grids_matches_ducc_and_roundtrips -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_t1023_sparse_synthesis_matches_ducc -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_t1023_exact_analysis_matches_ducc -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_three_and_four_t1023_exact_analysis_matches_ducc -- --ignored --exact --nocapture`, `cargo test --release --lib spin_transform::tests::m3_spin_t2047_sparse_synthesis_matches_ducc_and_sectoral_formula -- --ignored --exact --nocapture`, and `cargo test --release --lib spin_transform::tests::m3_spin_three_and_four_t511_all_grids_match_ducc_and_analytic_fields -- --ignored --exact --nocapture`. All seven checks pass. The spin-3/4 operation check runs in the routine suite as `cargo test --lib spin_transform::tests::spin_three_and_four_match_ducc_analytic_fields_and_adjoint_identities -- --exact --nocapture`. Validation does not cover spin 5+, f32, spin-3/4 synthesis above T2047, their exact analysis above T1023, or their adjoints above `lmax=spin`.

### Latitude block-size tuning (M4, 2026-10-12)

The T511 scalar GL benchmark compared 16, 32, 64 and 128-ring blocks on an x86-64 CPU with host topology omitted, AVX2/FMA and no AVX-512. It used rustc 1.99.0, the release profile, one warmup, five timed samples per operation and one/eight-thread Rayon pools. The table reports per-run medians in milliseconds; 32-ring results average three run medians, 64-ring results average two, and 16/128 are single exploratory runs.

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

Stage timers at T511 showed the Legendre recurrence took 1537 ms in synthesis and 2153 ms in analysis; FFT stages took 5.1 ms and 3.1 ms, respectively. The first change makes `scale_binary` adjust stored exponent bits when both the input and result are normal numbers, retaining the staged multiplication path for subnormal, overflow and underflow boundaries. A test compares its result bit-for-bit against the former staged path across signed zero, normal and subnormal values, and exponents from -4097 to 4097. The release harnesses used rustc 1.99.0, f64, T511 GL `512 x 1025`, one warmup and five samples per operation, with one-worker and eight-worker Rayon pools. Before and exponent-path values are single-run medians; guarded direct-multiply values average two run medians. Timings are in milliseconds on the x86-64 CPU with AVX2/FMA and no AVX-512:

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


The common-case scalar measurements average three per-run medians after one warmup and five timed samples. They use the same rustc 1.99.0 release build and x86-64 CPU host as the earlier T511 measurements:

| Operation | Threads | Guarded multiply, earlier order | Common-case path | Change from earlier order |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 667.130 | 461.915 | -30.76% |
| Scalar synthesis | 8 | 103.903 | 73.677 | -29.09% |
| Scalar analysis | 1 | 806.614 | 551.584 | -31.62% |
| Scalar analysis | 8 | 216.405 | 147.511 | -31.84% |


| Operation | Threads | Common-case path | Shared factor check | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 461.915 ms | 450.753 ms | -2.42% |
| Scalar synthesis | 8 | 73.677 ms | 72.680 ms | -1.35% |
| Scalar analysis | 1 | 551.584 ms | 543.562 ms | -1.45% |
| Scalar analysis | 8 | 147.511 ms | 146.592 ms | -0.62% |

The T511 timings use the same rustc 1.99.0 release harness, CPU, one warmup and five timed samples per operation recorded above. These timings are for the implementation that passed all seven high-bandlimit release checks against `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`), covering scalar and spin transforms through T2047 and analytic recurrence checks. An independent Python comparison used the rebuilt editable extension, `ducc0==0.41.0`, deterministic T511 coefficients and seeded random analysis maps on the same GL shape. Each operation had one warmup and five timed samples; RustSHT and DUCC used the same one- or eight-thread setting. With one thread, scalar synthesis and analysis took 471.541 ms and 552.281 ms in RustSHT versus 12.043 ms and 12.240 ms in DUCC; spin-1 took 1362.377 ms and 1389.893 ms versus 25.781 ms and 25.992 ms. With eight threads, scalar took 79.025 ms and 146.604 ms versus 1.871 ms and 1.773 ms; spin-1 took 206.881 ms and 358.471 ms versus 3.742 ms and 3.722 ms. Scalar synthesis map maximum absolute/RMS errors were `2.627e-14`/`7.674e-16`; analysis coefficient errors were `1.436e-15`/`2.179e-16`. Spin-1 map errors were `3.054e-14`/`7.729e-16`, and analysis coefficient errors were `1.659e-15`/`2.410e-16`. Across these operations and thread counts, RustSHT is 39–96 times slower than DUCC; M4 performance acceptance remains open.

### Fused spin recurrence step (M4, 2026-10-12)

`recurrence_step` advances each spin basis with `next = (a * cos(theta) - b_eff) * current - previous`, where `b_eff` has a branch-specific sign. The implementation now expresses both stages as fused multiply-add operations through `pulp::Simd::mul_add_f64s` and `f64::mul_add`. This reduces spin-1 T511 GL runtime in every measured operation relative to commit `d4ec0c4`. Each value is an average of three per-run medians from the release `m1b_bench` harness, with one warmup and five timed samples per operation, using rustc 1.99.0, f64, an x86-64 CPU with AVX2/FMA and one- or eight-thread Rayon pools:

| Operation | Threads | Before FMA | FMA | Change |
| --- | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | 1371.815 ms | 1335.613 ms | -2.64% |
| Spin-1 synthesis | 8 | 203.655 ms | 198.386 ms | -2.59% |
| Spin-1 analysis | 1 | 1412.318 ms | 1372.486 ms | -2.82% |
| Spin-1 analysis | 8 | 354.049 ms | 343.635 ms | -2.94% |

All seven release-mode high-bandlimit checks pass against `ducc0==0.41.0` at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; they cover scalar and spin transforms through T2047, including analytic recurrence checks. The rebuilt Python extension also passes all eight DUCC comparison tests.

### f32 plan, workspace and transform support (M4, 2026-10-12)

`ShtPlan<T = f64>` and `ShtWorkspace<T = f64>` support f32 and f64. `Complex32` and `Complex64` identify the matching coefficient types. `GridSpec` stores coordinates and quadrature weights as f64; plan construction converts the numerical metadata to the selected arithmetic type. The FFT adapter and CC/F1 resampler execute in that type. The f64 and f32 recurrences use runtime-dispatched `pulp` SIMD over 64-ring latitude blocks, with scalar fallback. Binary scaling and recurrence initialization remain lane-wise. The Rust integration tests use `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) and float32/complex64 inputs. Static scalar synthesis and analysis references cover T2 on GL `3x5`, CC `4x5` and F1 `3x5`, all with `phi0=0.37`. A spin-1 GL `2x3` synthesis reference covers T1 at `phi0=0.23`. A spin-1 sparse GL synthesis case compares selected map samples at T127. Independent f32 analytic checks cover scalar `m=0` fields on all three grids, `Y_11` derivatives on GL, and a T127 scalar sectoral field. Scalar `S*` and `A*` identities pass on GL/CC/F1; spin-1 through spin-4 STANDARD `S`, `A`, `S*`, `A*`, GRAD_ONLY synthesis and its adjoint pass at `lmax=spin` with exact-analysis minimum latitude counts and minimum odd longitude counts. DERIV1 synthesis and its adjoint pass at T2. The FFT helper compares f32 spectra and roundtrips with full-complex RustFFT at lengths 1, 2, 3, 4, 5, 8, 31, 32 and 33. The T127 f32 cases cover scalar and spin-1 GL synthesis; they do not test T127 analysis or other grids. The initial release benchmark, before f32 SIMD, used `examples/m4_f32_bench.rs` on an x86-64 CPU (host topology omitted), x86_64 AVX2/FMA, rustc 1.99.0, f32 spin-1, GL `512x1025`, T511 and `phi0=0.37`. RustSHT reuses its plan and workspace. `ducc0==0.41.0` with NumPy 2.4.6 uses `synthesis_2d` and `analysis_2d` on the same dtype, grid, coefficients, and thread counts. Each operation has one warmup and five timed samples; the table reports each median.

| Operation | Threads | RustSHT | DUCC | RustSHT / DUCC |
| --- | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | 1340.185 ms | 27.495 ms | 48.7x |
| Spin-1 synthesis | 8 | 197.964 ms | 3.795 ms | 52.2x |
| Spin-1 analysis | 1 | 1405.229 ms | 25.061 ms | 56.1x |
| Spin-1 analysis | 8 | 330.506 ms | 3.522 ms | 93.8x |

The initial RustSHT medians show 6.8x synthesis and 4.3x analysis speedups from one to eight threads. The estimated numeric arrays use 12.13 MiB and exclude RustFFT plan and execution scratch, allocator metadata, and process memory. The scalar-lane implementation took 48.7–93.8 times longer than DUCC in these measurements. These initial one-host measurements do not establish a performance acceptance result.

### f32 latitude recurrence SIMD (M4, 2026-10-12)

The scalar and spin f32 recurrence kernels dispatch through `pulp::Arch` and operate on 64-ring blocks. Recurrence arithmetic uses f32 SIMD; initial half-angle or sectoral powers and binary rescaling remain per lane. `Arch::Scalar` provides the portable fallback. Unit checks compare scalar fallback with `Arch::new()` across three blocks and a one-ring tail, near-polar latitudes, scalar orders through T127, and spin 1–4 through T96. The integration suite runs the runtime-selected path and retains DUCC 0.41.0, analytic and T127 checks. On this x86-64 CPU host, `Arch::new()` selects AVX2/FMA; AVX-512 is unavailable. The release benchmark repeats the initial RustSHT setup on rustc 1.99.0. Both the earlier scalar-lane and current SIMD RustSHT runs use `examples/m4_f32_bench.rs`, which reuses the plan and workspace. The DUCC measurements use `ducc0==0.41.0`, NumPy 2.4.6, float32 maps, complex64 coefficients, and the same grid and coefficients; each operation has one warmup and five timed samples. Values are medians in milliseconds.

| Operation | Threads | Scalar-lane RustSHT | SIMD RustSHT | SIMD reduction | DUCC | SIMD RustSHT / DUCC |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | 1340.185 | 1177.619 | 12.1% | 27.464 | 42.9x |
| Spin-1 synthesis | 8 | 197.964 | 181.242 | 8.4% | 6.598 | 27.5x |
| Spin-1 analysis | 1 | 1405.229 | 1281.740 | 8.8% | 25.292 | 50.7x |
| Spin-1 analysis | 8 | 330.506 | 297.164 | 10.1% | 3.464 | 85.8x |

The SIMD run gives 6.5x synthesis and 4.3x analysis speedups from one to eight RustSHT threads. The estimated numeric-array footprint remains 12.13 MiB, excluding RustFFT plan and execution scratch, allocator metadata, and process memory. SIMD reduces the scalar-lane medians by 8.4–12.1%; RustSHT remains 27–86 times slower than DUCC in these measurements. f32 performance acceptance and memory-traffic optimization remain open.

### Reuse Fourier scale constants in transform loops (M4, 2026-10-12)

Scalar and spin synthesis now convert the longitude count to the transform precision outside the order and ring loops. Scalar and spin analysis reuse that conversion in their coefficient loops. Spin synthesis also computes its Nyquist scale once per ring group. These changes retain the same multiplication and division order.


| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 1174.303 | 1168.563 | -0.49% |
| Synthesis | 8 | 184.243 | 178.572 | -3.08% |
| Analysis | 1 | 1286.292 | 1273.005 | -1.03% |
| Analysis | 8 | 295.586 | 297.207 | +0.55% |


### f32 normal binary scaling fast path (M4, 2026-10-12)

The f32 scalar recurrence now adjusts the exponent bits directly when multiplying a normal value by a power of two produces another normal value. Subnormal, overflow, and extreme-exponent results use the general binary scaler. The fast path is used by both scalar recurrence multiply helpers; zero, nonfinite, and boundary products retain the general or split-binary paths. Unit tests compare normal-value scaling with staged multiplication and compare the optimized complex multiply with a split-binary reference. The cases include signed zero, normal and subnormal inputs and outputs, infinities, NaN, and exponents from -4097 to 4097. Existing f32 integration checks compare transform results with `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) and analytic fields.


| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 756.300 | 728.566 | -3.7% |
| Synthesis | 8 | 143.088 | 138.013 | -3.5% |
| Analysis | 1 | 887.067 | 842.292 | -5.0% |
| Analysis | 8 | 195.777 | 186.292 | -4.8% |

The two-run timing averages are lower in all four cases. They measure one host and workload; they do not establish a general speedup or close the f32 performance gap with DUCC. The benchmark did not collect DUCC timings. On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 73 library unit tests and 10 f32 integration tests; seven opt-in high-bandlimit tests are ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 71 library unit tests and 10 f32 integration tests, with the same seven tests ignored. Workspace Clippy with warnings denied passes on both compilers. Formatting and `git diff --check` pass. The Python binding tests were not run because this change does not modify the bindings.

### Skip duplicate scaling at f32 recurrence boundaries (M4, 2026-10-12)

`scale_normal_binary` now returns a value only when exponent adjustment keeps the product normal. For subnormal, overflow and extreme-exponent results it returns `None`, allowing the caller to use the split-binary path without first calculating and discarding a result from the general scaler. This preserves the split path's rounding at subnormal boundaries. The bitwise unit tests compare fast-path results with staged power-of-two scaling and compare the complex multiply with the split-binary reference across signed zero, normal and subnormal operands, infinities, NaN, and exponents from -4097 to 4097. They cover the underflow tie where staged scaling rounds to the minimum normal value while exponent adjustment requires the split path. The existing f32 integration suite compares transforms with `ducc0==0.41.0` (source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) and analytic fields. The T511 scalar benchmark used f32, GL `512x1025`, one or eight Rayon threads, one warmup and five timed samples per run. The plan and workspace were reused. Each table value averages two per-run medians in milliseconds. The comparison used the same temporary harness and compiler at `3124301` and the working tree with the boundary fallback change. The estimated core numeric-array footprint was 8.13 MiB.

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 728.566 | 617.665 | -15.2% |
| Synthesis | 8 | 138.013 | 112.768 | -18.3% |
| Analysis | 1 | 842.292 | 735.521 | -12.7% |
| Analysis | 8 | 186.292 | 162.783 | -12.6% |

These measurements cover one host, dtype, grid and bandlimit. No DUCC timing was collected for this change. On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 73 library unit tests and 10 f32 integration tests; seven opt-in high-bandlimit tests are ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 71 library unit tests and 10 f32 integration tests, with the same seven tests ignored. Workspace Clippy with warnings denied passes on both compilers. Formatting and `git diff --check` pass. The Python binding tests were not run because this change does not modify the bindings.

### f32 recurrence normalization magnitude checks (M4, 2026-10-12)

`normalize_pair_lanes` now selects the larger absolute recurrence value and checks its scale thresholds using the unsigned magnitude bits. For finite IEEE-754 values, unsigned magnitude order matches numeric order; the current recurrence supplies finite pairs, and debug builds assert that constraint. The rescaling operations and exponent updates are unchanged. The T511 scalar benchmark used f32, GL `512x1025`, one or eight Rayon threads, one warmup and five timed samples per run. The plan and workspace were reused. Each table value averages two per-run medians in milliseconds; the estimated numeric-array footprint was 8.13 MiB.

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 617.665 | 591.653 | -4.2% |
| Synthesis | 8 | 112.768 | 110.297 | -2.2% |
| Analysis | 1 | 735.521 | 720.346 | -2.1% |
| Analysis | 8 | 162.783 | 160.041 | -1.7% |

These measurements cover one host, dtype, grid and bandlimit. No DUCC timing was collected for this change. The unit test compares the bit-based normalization with the previous float-magnitude algorithm at zero, subnormal, `SMALL` and `BIG` thresholds, and large finite values. On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 74 library unit tests and 10 f32 integration tests; seven opt-in high-bandlimit tests are ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 72 library unit tests and 10 f32 integration tests, with the same seven ignored. Workspace Clippy with warnings denied passes on both compilers. Formatting and `git diff --check` pass. Python binding tests were not run because this change does not modify the bindings.

### Spin f32 recurrence normalization magnitude checks (M4, 2026-10-12)

The spin f32 `normalize_pair_lanes` function now selects the larger absolute recurrence value and checks its scale thresholds using unsigned magnitude bits. For finite IEEE-754 values, unsigned magnitude order matches numeric order; the recurrence supplies finite pairs, and debug builds assert that constraint. Rescaling and exponent updates are unchanged. A unit test compares the bit-based implementation with the former float-magnitude algorithm at signed zero, subnormal values, one-ULP neighbors of `SMALL` and `BIG`, and large finite values. The T511 spin benchmark used f32, spin 1, GL `512x1025`, one or eight Rayon threads, one warmup, and five timed samples per run. The plan and workspace were reused. The before measurements used `564e76c`; the after measurements used the working tree with this change. Each table value averages two per-run medians in milliseconds. The estimated core numeric-array footprint was 12.13 MiB.

| Operation | Threads | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 1164.235 | 1132.685 | -2.7% |
| Synthesis | 8 | 180.424 | 175.707 | -2.6% |
| Analysis | 1 | 1271.015 | 1241.490 | -2.3% |
| Analysis | 8 | 293.103 | 286.721 | -2.2% |

The four two-run timing averages are lower for this workload. They measure one host, dtype, grid and bandlimit; no DUCC timing was collected for this change. On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 75 library unit tests and 10 f32 integration tests; seven opt-in high-bandlimit tests are ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 73 library unit tests and 10 f32 integration tests, with the same seven ignored. Workspace Clippy with warnings denied passes on both compilers. Formatting and `git diff --check` pass. Python binding tests were not run because this change does not modify the bindings.


### f32 phase and pole edge cases (M4, 2026-10-12)

Scalar and spin transforms retain `phi0` as f64 while forming each order phase, then convert the sine and cosine to the selected transform type. If `m*phi0` overflows f64, phase setup derives the integer-order phase by exponentiating `e^(i phi0)`. Grid and prepared CC colatitudes remain f64 through sine/cosine evaluation; the exact `0` and `pi` endpoints produce exact zero sine values before conversion to f32. Regression tests synthesize scalar and spin fields at `phi0=f64::MAX`, requiring finite f32/f64 outputs that agree within `3e-6`; an independent scalar `Y_22` check compares the order-two phase with the square of `e^(i phi0)`. A CC scalar `Y_11` test requires every f32 sample at the south pole to be exactly zero. On rustc 1.99.0 and 1.89.0, `cargo test --workspace --all-features` passes 71 unit tests and 10 f32 integration tests, with seven opt-in high-bandlimit tests ignored. On rustc 1.99.0, `cargo test -p rustsht --no-default-features` passes 69 unit tests and 10 f32 integration tests, with the same seven tests ignored. `cargo test --test f32`, Clippy with warnings denied on both compilers, formatting checks and `uv run pytest python/tests` pass; pytest reports eight tests. These edge-case tests do not extend the high-bandlimit f32 validation domain or address the open f32 SIMD performance gap.

### Python plan binding (M5, 2026-10-12)

The PyO3 extension exposes `rustsht.ShtPlan` and top-level procedural functions for GL, CC and F1 grids. They accept `float64` maps and `complex128` coefficients in component-first NumPy arrays. The plan methods and procedural functions cover scalar and spin STANDARD synthesis, analysis and both adjoints, spin gradient synthesis and its adjoint, and scalar DERIV1 synthesis and its adjoint. `get_gridweights` returns the grid ring weights. Inputs must be C-contiguous; each call copies its inputs into Rust-owned vectors, releases the GIL for workspace creation and transform execution, and returns a new NumPy array. The package includes the PEP 561 marker and `__init__.pyi` stub. Python tests convert between RustSHT's order-major coefficient packing and DUCC's default alm layout. Plan and procedural scalar S, A, S* and A* are compared with `ducc0==0.41.0` on GL, CC and F1; the procedural ring weights are compared on each grid. Spin-2 STANDARD S, A, S* and A*, GRAD_ONLY S and S*, and scalar DERIV1 S and S* are compared through both interfaces on GL. The tests use `phi0 != 0`, check the scalar real-harmonic adjoint identity, and reject incorrect shapes, dtypes and Fortran-contiguous arrays. The largest comparison tolerance is `2e-12`; DUCC runs with one thread. The pinned DUCC source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. On an x86-64 CPU, `cargo fmt --all -- --check`, workspace Clippy with warnings denied and `cargo test --workspace --all-features` pass on rustc 1.89.0 and 1.99.0. Each workspace test run passes 68 tests with seven high-bandlimit tests ignored. The Rust 1.99.0 no-default-features suite passes 66 tests with the same seven tests ignored. `uv run pytest python/tests` passes eight tests with Python 3.11.15, NumPy 2.4.6 and `ducc0==0.41.0`. It validates f64/complex128 inputs and the listed plan and procedural operations against the editable package. `uv build` creates a CPython 3.11 abi3 Linux x86_64 wheel and source archive. Both include the GPL license, `LICENSE-THIRD-PARTY`, the typing stub and PEP 561 marker; the wheel metadata declares NumPy and GPL-2.0-or-later. The third-party notice records selected license texts for 42 dependencies in the Linux x86_64 extension build graph. A clean Python 3.11.15 environment installs the wheel, imports it and passes a GL grid-weight smoke check. The full numerical suite was run against the editable package, not the installed wheel. f32/complex64 and the spharmgrid adapter are not covered.

### Performance baseline before Fourier-weight hoisting (M4/M5, 2026-10-12)

Measurements used an x86-64 CPU (host topology omitted), Rust 1.99.0 release builds, x86_64 AVX2/FMA dispatch and no AVX-512. Python measurements used Python 3.11.15, NumPy 2.4.6 and `ducc0==0.41.0` from source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The editable extension was rebuilt with `maturin develop --release`; its build ID matched `target/release/lib_rustsht.so`. RustSHT and DUCC used the same `phi0=0.37`, grid, coefficients, map values and thread counts. For each Python run, `RAYON_NUM_THREADS` was set to the requested thread count before process startup; the harness checks this setting before importing NumPy, DUCC or RustSHT. `examples/python_perf_baseline.py` records the Python plan, procedural and DUCC measurements. Each timed operation had one warmup and five samples per run, with two runs. Tables report the median of the run medians and their spread. The native benchmark reused one plan and workspace. Python plan methods reuse the plan but still copy inputs and allocate a workspace and output for each call; procedural functions construct a plan and workspace per call. At T511 GL `512x1025`, f64, scalar and spin 1, the native Rust results were:

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

The scalar synthesis map error against DUCC had maximum absolute error `2.627e-14`, RMS error `7.674e-16` and relative L2 error `6.487e-14`; scalar analysis coefficient errors were `1.010e-14`, `2.281e-16` and `4.467e-14`. Spin-1 synthesis errors were `2.363e-14`, `8.126e-16` and `6.675e-14`; analysis errors were `1.463e-14`, `2.298e-16` and `4.500e-14`. Python plan construction took about `0.036 ms`; a separate five-sample copy check measured coefficient copies at `0.04–0.10 ms` and map copies at `0.09–0.21 ms`; output allocation took `0.001 ms`. A native T511 workspace took `3.68–3.93 ms` at the median of eleven measurements. These binding and allocation costs are small relative to transform execution. Representative Python plan timings on CC and F1 used T255, f64, one thread, `nphi=513`, and exact-analysis latitude counts (CC `257`, F1 `256`). Values are median and spread in milliseconds for the two runs:

| Grid | Spin | Synthesis RustSHT / DUCC | Analysis RustSHT / DUCC |
| --- | ---: | ---: | ---: |
| CC | 0 | 68.384 / 1.973 (0.254 / 0.088) | 73.545 / 5.246 (0.723 / 0.056) |
| CC | 1 | 189.635 / 7.948 (1.508 / 0.708) | 195.422 / 14.742 (0.272 / 0.144) |
| F1 | 0 | 68.052 / 1.949 (0.320 / 0.034) | 73.331 / 5.309 (0.363 / 0.116) |
| F1 | 1 | 186.212 / 5.835 (0.112 / 0.013) | 195.379 / 14.113 (1.147 / 0.214) |

Maximum absolute, RMS and relative L2 errors on CC were `3.490e-15`, `1.648e-16`, `2.793e-14` for scalar synthesis; `1.169e-14`, `2.425e-16`, `2.376e-14` for scalar analysis; `4.338e-15`, `1.426e-16`, `2.349e-14` for spin-1 synthesis; and `9.907e-15`, `1.782e-16`, `1.747e-14` for spin-1 analysis. The corresponding F1 errors were `4.625e-15`, `1.727e-16`, `2.922e-14`; `1.185e-14`, `2.448e-16`, `2.399e-14`; `5.627e-15`, `1.516e-16`, `2.492e-14`; and `9.921e-15`, `1.785e-16`, `1.749e-14`.


### Analysis Fourier-weight hoisting (M4/M5, 2026-10-12)

Scalar and spin analysis now prepare the order phase and applicable ring scaling once per `(m, ring)` and reuse those Fourier values for all degree updates. Each recurrence worker uses three 64-lane complex buffers; scalar analysis uses one and spin analysis uses two. Ring blocks and degree accumulation order are unchanged. GL weights and longitude normalization remain in the analysis projection. CC/F1 prepared-grid weighting and Fourier normalization remain in the resampling stage. Focused Rust checks passed for scalar and spin DUCC references and adjoint identities on GL, CC and F1. f32 scalar and spin all-grid DUCC/analytic checks and adjoint identities also passed. The T511 Python comparisons after rebuilding the extension retained the baseline error values at printed precision: scalar synthesis/analysis relative L2 errors were `6.487e-14`/`4.467e-14`, and spin-1 errors were `6.675e-14`/`4.500e-14`. Native release comparisons used rustc 1.99.0, AVX2/FMA dispatch, f64, GL `512x1025`, T511, `phi0=0.37`, one warmup and five timed samples per operation on an x86-64 CPU. Plans and workspaces were reused. The before values are the two run medians reported in the baseline; three runs were made after the change. Synthesis is included to show the unaffected path.

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


Python comparisons used Python 3.11.15, NumPy 2.4.6, `ducc0==0.41.0`, the rebuilt editable extension and the same T511 grid/data/thread settings as the baseline. Each median is the median of two run medians; each run had one warmup and five timed samples. Plan methods, procedural functions and DUCC were measured separately. The before values below are plan medians; their spreads and procedural timings are in the baseline table above.

| Operation | Threads | Plan before (ms) | Plan after (median; spread ms) | Procedural after (median; spread ms) | DUCC after (median; spread ms) | Plan change | Plan / DUCC after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar analysis | 1 | 559.307 | 481.628; 1.848 | 483.365; 2.510 | 12.383; 0.114 | -13.9% | 38.9x |
| Scalar analysis | 8 | 152.961 | 129.949; 1.506 | 132.104; 2.210 | 1.871; 0.129 | -15.1% | 69.4x |
| Spin-1 analysis | 1 | 1497.584 | 1349.172; 9.721 | 1352.689; 3.281 | 25.653; 0.205 | -9.9% | 52.6x |
| Spin-1 analysis | 8 | 379.849 | 347.557; 4.466 | 341.697; 2.273 | 3.622; 0.001 | -8.5% | 96.0x |

This change reduces native analysis time and total profiled instruction references without changing the DUCC error at the reported precision. T511 GL remains 39–96 times slower than DUCC in these Python plan measurements. The new buffers add `3 * 64 * sizeof(Complex64)` bytes per recurrence worker (`0.0234 MiB` for eight workers); the estimated T511 eight-worker numeric-array footprints become 16.25 MiB for scalar and 24.26 MiB for spin 1. A separate eleven-sample native measurement found T511 scalar plan construction at `0.0334 ms` median (0.1092 ms spread) and workspace allocation at `3.6745 ms` (0.9984 ms spread). The Python harness measured plan construction at `0.035–0.039 ms`, input copies at `0.052–0.203 ms` and output allocation at `0.001–0.002 ms`; these costs remain small relative to transform execution. No FFT or meridional-resampling implementation changed.

### Spin synthesis phase and FFT scaling hoist (M4/M5, 2026-10-12)

Spin synthesis now accumulates unphased Fourier values across degree for each order and ring, then applies `exp(i*m*phi0)` and the longitude FFT scale once to each accumulated spectrum value. The existing q/u spectrum buffers are updated in place; this adds no full-sized workspace allocation. For even-longitude Nyquist, the accumulated value is phased first, then folded to `2*nphi*real` with zero imaginary part, preserving the previous component and FFT conventions. This changes floating-point operation order in the degree sum. Focused checks passed for spin STANDARD synthesis and its adjoint, `A*`, GRAD_ONLY, DERIV1 and their adjoints on GL, CC and F1, including even-longitude Nyquist, polar rings, and `phi0=0.37`. Direct f64 comparisons with `ducc0==0.41.0` at T2 covered E-only, B-only and mixed E/B coefficients, GL odd and even longitude counts, CC poles, GRAD_ONLY, DERIV1 and adjoint outputs; the largest absolute discrepancy was below `1.7e-15`. T511 Python spin-1 GL comparison with DUCC had maximum absolute error `2.365e-14`, RMS error `8.126e-16` and relative L2 error `6.675e-14`. f32 focused spin tests passed for all grids and spins 1–4, including adjoint and gradient checks; f32 GL DUCC synthesis and sparse T127 DUCC synthesis also passed. Release DUCC and analytic checks passed for spin 3/4 on GL, CC and F1 at T511, spin 1/2 at T1023, and sparse spin 1–4 synthesis at T2047. These tests retain the established high-bandlimit errors recorded above. Native release timings used rustc 1.99.0, f64, GL `512x1025`, T511, `phi0=0.37`, AVX2/FMA dispatch, one warmup and five timed samples per run on an x86-64 CPU. The same plan and workspace were reused. Each table entry gives the run medians, their median and range, in milliseconds:

| Threads | Before run medians (median; range) | After run medians (median; range) | Change |
| ---: | ---: | ---: | ---: |
| 1 | `1441.264, 1431.320, 1556.177` (`1441.264`; `124.857`) | `1291.352, 1282.280, 1290.691` (`1290.691`; `9.072`) | `-10.45%` |
| 8 | `215.173, 214.867, 211.066` (`214.867`; `4.107`) | `190.487, 191.759, 191.371` (`191.371`; `1.272`) | `-10.93%` |

The analysis operation is unchanged. Its after medians over the same three runs were `1282.264, 1281.893, 1285.030 ms` at one thread and `321.835, 322.025, 326.827 ms` at eight threads. The rebuilt Python extension used Python 3.11.15, NumPy 2.4.6 and `ducc0==0.41.0`. Both RustSHT and DUCC used identical T511 GL data, `phi0=0.37`, and one or eight threads. Each run had one warmup and five timed samples; the table reports the median of two run medians and their range. The Python plan before values are from the matched pre-hoist run after analysis Fourier-weight hoisting; procedural functions construct their plan/workspace on each call.

| Operation | Threads | Plan before (ms) | Plan after (median; range ms) | Procedural after (median; range ms) | DUCC after (median; range ms) | Plan change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Spin-1 synthesis | 1 | `1414.124` | `1295.828; 1.254` | `1291.631; 2.593` | `24.513; 0.049` | `-8.37%` |
| Spin-1 synthesis | 8 | `215.045` | `195.882; 3.562` | `199.649; 1.026` | `4.101; 0.589` | `-8.91%` |

Plan construction measured `0.035–0.041 ms`, input copies `0.104–0.213 ms`, and output allocation `0.001 ms` in these runs. The phase/scale hoist reduces native and Python plan synthesis time while retaining the DUCC error range; it does not close the remaining performance gap. The eight-thread DUCC spread reflects two run medians and should not be read as an optimization effect. Formatting, workspace Clippy with warnings denied, all-features tests and no-default-features tests passed with Rust 1.89.0 and 1.99.0. Each Rust test suite passed 75 tests with seven opt-in high-bandlimit checks ignored, plus all 10 f32 integration tests. `uv run pytest python/tests` passed all eight tests with Python 3.11.15.

### Scalar recurrence reciprocal-factor investigation (M4, 2026-10-12)

The current recurrence computes `(cos(theta)*current - epsilon[l]*previous)/epsilon[l+1]`. Three T511 release runs established the scalar GL baseline before this investigation on an x86-64 CPU, rustc 1.99.0, f64, AVX2/FMA runtime dispatch, `phi0=0.37`, and one/eight-thread pools. `examples/m1a_bench.rs` reused a plan and workspace, with one warmup and five samples per operation per run:

| Operation | One-thread run medians (median; range ms) | Eight-thread run medians (median; range ms) |
| --- | ---: | ---: |
| Synthesis | `525.214, 525.521, 524.876` (`525.214`; `0.645`) | `81.436, 85.797, 82.154` (`82.154`; `4.361`) |
| Analysis | `516.457, 517.436, 516.911` (`516.911`; `0.979`) | `137.004, 140.699, 136.425` (`137.004`; `4.274`) |

I tested reciprocal multiplication, a fused numerator with reciprocal multiplication, a compensated high/low reciprocal, and precomputed `a_lm=1/epsilon[l+1]`, `b_lm=epsilon[l]/epsilon[l+1]` with a fused update. The scalar-reference test compares the vector-lane recurrence with the retained division-based recurrence at a relative tolerance of `2e-13`, including near-polar and high-order lanes. Reciprocal multiplication first exceeded this limit at `(m,l,ring)=(0,20,15)` with `2.621e-13` relative error and at `(255,467,5)` with `2.654e-13`. The fused-numerator variant reached `5.449e-13` and `1.027e-12` at those tests; the compensated reciprocal reached `2.621e-13` and `5.045e-12`; and the `a,b` variant reached `1.062e-12` and `6.992e-13`. I did not relax the reference tolerance. The variants were rejected before timing because they failed this numerical acceptance check. The f32 `a,b` prototype passed the dispatched-versus-scalar-backend recurrence check and the existing scalar DUCC checks on GL, CC and F1; that does not override the f64 failure. DUCC's pinned [`Ylmgen::prepare` and scalar kernels](https://raw.githubusercontent.com/mreineck/ducc/c1ed01b8fda69d2a378190f0915f1a19762dddbf/src/ducc0/sht/sht_inner_loop.h) precompute paired-degree `alpha` and `coef` values, then use a two-step recurrence in the normal-range kernels. The simple one-step reciprocal substitutions tested here do not reproduce that method. No recurrence source or workspace change was retained, and no post-change timing is reported because every f64 candidate failed the existing scalar-reference check.
