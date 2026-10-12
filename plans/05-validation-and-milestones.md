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

**M5 — Python binding:** The PyO3/NumPy plan and procedural APIs are implemented. Complete safe borrowed inputs, reusable workspace and caller-provided output work, then maintain packaging, GIL, shape/dtype and wheel-license checks.

**M6 — spharmgrid:** Optional RustSHT backend matching its scalar/derivative/vector adapter, with independent parity, thread behavior and packaging review. Keep DUCC default until tested.

## Long autonomous implementation sessions

For a new implementation session, start from `main` on a feature branch. Continue an active session on its existing branch. Use [00](00-scope-and-decisions.md) without routine human checkpoints. M1 and M2 operation work and the recorded M3 checks are implemented; M4 performance tuning remains active, while the M5 Python API exists and has additional performance work outstanding.

1. Continue scalar and spin-1 performance work, then optimize spin 2. Use pinned-DUCC comparisons, independent analytic checks and the spin-3/4 regression suite for shared-kernel changes.
2. Complete remaining M4 validation and f32 checks at affordable bandlimits. Compare AVX2/FMA with AVX-512 only on hardware that executes the relevant ISA.
3. Complete the remaining M5 Python input, workspace and output-reuse work, then proceed to the optional M6 spharmgrid adapter when numerical behavior and packaging are verified.
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

With the `parallel` feature, synthesis partitions contiguous latitude groups across Rayon tasks. Analysis partitions disjoint order-major coefficient ranges across tasks. Each task uses a recurrence scratch set owned by `ShtWorkspace`; the serial path uses one set. The workspace sizes its worker groups using the active Rayon pool at construction. The real FFT row loop remains serial.

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
