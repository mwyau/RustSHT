# Verification, benchmarks and milestones

## Validation contract

A transform is not validated by a single roundtrip. For each supported geometry **GL, CC, F1**, spin, dtype, operator and grid sampling, compare to pinned DUCC output **and** at least one independent mathematical check.

- **Synthesis**: constants, isolated normalized `Y_lm`, pure sectoral/near-sectoral modes, mixed random coefficients and high-`m` polar cases.
- **Analysis**: recover coefficients of independently synthesized bandlimited fields; compare to direct quadrature for sufficiently small GL problems; test CC/F1 high-bandlimit cases that ordinary low-resolution quadrature would not recover.
- **Adjoints**: verify separately `<S a, x>_map = <a,S* x>_alm` and `<A x,a>_alm = <x,A* a>_map` with the real harmonic inner product (factor two for `m>0` modes) and documented map dot product.
- **Spin/vector**: spin=0,1,2 standard cases, E/B pure components, geographic `(-v,u)` convention, analytic derivative identities. Evaluate spin 3/4 before setting the upper-spin contract.
- **Derivative modes**: `GRAD_ONLY` equals STANDARD synthesis when curl coefficients are zero; check `DERIV1` signs/scaling and the corresponding **adjoint-synthesis** modes. `analysis_2d` remains STANDARD-only.
- **Grid geometry**: validate nodes, `4*pi` summed weights, CC poles, F1 half-grid offset, GL ordering and longitude `phi0` shift. For exact analysis, require `lmax<=ntheta-1` (GL/F1), `lmax<=ntheta-2` (CC), and `mmax<=floor((nphi-1)/2)`.
- **Layout and safety**: `mmax<lmax`, `m=0` real constraint, `l<spin`, odd/even longitude, contiguous/strided and batched inputs where supported, aliasing, invalid sizes and integer overflow.
- **Threading and SIMD**: scalar and vectorized kernels agree within established tolerances; results are independent of thread count within documented floating-point reproducibility limits. Use absolute and norm-relative errors with a sensible near-zero floor; report worst error and RMS/distribution, not just roundtrip l2. Define acceptance thresholds *before* evaluating benchmarks. Distinguish algorithmic error from known floating-point order differences; never relax a tolerance merely to hide failures.

**Source caution:** Use the mathematical cases in DUCC's Python test suite as a guide, but do not copy `python/test/test_sht.py` verbatim: its header is GPL-3.0-or-later. See the [license plan](01-ducc-provenance-and-license.md).

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

- **M1a:** Core Rust harmonic/grid specs, packed layout, RustFFT interface, GL nodes/weights, stable scalar f64 synthesis and analysis.
- **M1b:** GL spin=1/2 STANDARD synthesis/analysis, vector E/B and applicable GRAD_ONLY/DERIV1 synthesis. Design for higher spins; record actual validated range.
- **M1c:** GL adjoint synthesis and adjoint analysis; GRAD_ONLY/DERIV1 adjoint synthesis; inner-product proofs/tests, GL full-operation parity and high-`L` accuracy gate.

**M2 — Same engine, additional grids**

- **M2a:** CC geometry and analysis/adjoint-analysis prepared-CC resampling; test every GL-supported operator on CC, including near-minimal ring counts.
- **M2b:** F1 staggered geometry and its resampling/adjoint; repeat the full operation matrix.

**M3 — High-bandlimit acceptance:** Complete all-grid f64 parity, analytic/DUCC comparisons and actual tested degree/spin limits at T511/T1023 and opt-in T2047+. Fix critical numerical defects before focusing on speed.

**M4 — Optimization and dtype:** Stable-Rust multiarch SIMD dispatch, parallelism, scratch reuse, f32 implementation/verification and reproducible DUCC benchmarks.

**M5 — Python binding:** Optional PyO3/NumPy numerical API; plan-oriented plus procedural functions, packaging, GIL release, shape/dtype tests and wheel license review.

**M6 — spharmgrid:** Optional RustSHT backend matching its scalar/derivative/vector adapter, with independent parity, thread behavior and packaging review. Keep DUCC default until tested.

## Required evidence for each merged numerical milestone

- A mapped algorithm/reference source and source-provenance note if any code is adapted.
- Analytic and DUCC-backed tests demonstrating the claimed operator and grid.
- Explicit tested precision/spin/bandlimit/shape matrix with failure cases.
- Profiling only when performance is claimed; commit the benchmark methodology and environment, not large unexplained generated fixtures. The milestone ordering is fixed by D11; exact spin range, release tolerances, threading defaults and high-`L` performance acceptance remain open in [00](00-scope-and-decisions.md).
