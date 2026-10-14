# Scope and decisions

Updated 2026-10-12 (America/New_York). Implementation status and validation evidence are recorded in [05](05-validation-and-milestones.md).

## Objective

Implement high-bandlimit SHT algorithms used by DUCC in an independent **RustSHT** library, with native Rust and Python APIs. Support **GL, CC and F1** regular isolatitude grids with scalar, vector/spin-weighted, derivative, synthesis, analysis and relevant adjoint operations, aiming for DUCC-equivalent numerical results and competitive performance. An eventual optional backend for [spharmgrid](https://github.com/mwyau/spharmgrid) is a downstream use case, not a dependency. Copyright for original Rust code: **Albert M. W. Yau** and contributors. License: **GPL-2.0-or-later**. Cite Martin Reinecke and collaborators; retain third-party notices for any source expression actually copied, adapted or translated. See [reference and licensing](01-ducc-provenance-and-license.md).

## Approved decisions

| ID | Decision | Consequence |
| --- | --- | --- |
| D11 | **Feature-first / GL first**, approved 2026-10-11 | Complete scalar, spin/vector, applicable derivative modes, synthesis/analysis and both adjoint families on GL before bringing the *same engine* to CC and F1. Incremental GL substeps are allowed. |
| D12 | **DUCC as algorithmic/numerical reference**, approved 2026-10-11 | RustSHT is not DUCC's upstream/downstream repository or an official DUCC release. Original Rust code and adapted source expression have different provenance obligations. |
| D13 | **GL/CC/F1 parity is the release objective**, reaffirmed 2026-10-11 | All three grids must pass the same supported operator/spin test matrix and explicit high-bandlimit checks before claiming full scope. |
| D14 | **Rust 1.99.0 stable toolchain**, approved 2026-10-12 | Use the pinned stable toolchain for development, CI, benchmarks and builds. Avoid nightly-only features without a demonstrated need; preserve runtime CPU dispatch and a portable fallback. |
| D15 | **Long autonomous implementation sessions**, approved 2026-10-11 | Work on a new feature branch through as many independently verified milestones as possible, preserving GL-first ordering. |
| D22 | **Scalar and low-spin performance priority**, approved 2026-10-12 | Optimize scalar and spin-1 transforms first, then spin 2. Preserve spin-3/4 support and regression coverage without expanding those implementations; do not add spin 5+ without new authorization. |
| D23 | **Bounded GL Legendre tables**, approved 2026-10-13 | Use a north-hemisphere, triangular f64 table for scalar GL plans from T72 through T255, and for spin-1 STANDARD exact GL analysis through T255, when `mmax >= 0.75*lmax`, `ntheta == lmax+1`, and each table fits within 34 MiB. Spin-1 conversion stores through degree `lmax+1`. Keep recurrence for f32, spin synthesis, other spins, CC/F1, lower truncations and larger tables. Reusable-plan measurements and Torch-harmonics storage comparisons are recorded in [05](05-validation-and-milestones.md). The T72 scalar gate expansion is commit `d7f3dea`; the T255 spin-1 extension is recorded in [05](05-validation-and-milestones.md). |

### D11 rationale

A scalar-only implementation on all grids would defer spin recurrence, component count, derivative and adjoint design until late. Instead, build those shared mathematical interfaces on GL, validate them, then introduce grid-specific geometry and meridional analysis paths for CC and F1. The GL feature gate and the later all-grid high-bandlimit gate are separate.

### D12 rationale

Implement the algorithms in Rust using published literature and DUCC for behavioral comparison. DUCC source-code copying or direct translation, if it occurs, requires preserving its actual notices and tracking the original version; citing an algorithm does not itself transfer source copyright.

### D22 rationale

Scalar transforms support atmospheric scalar fields. Spin 1 supports horizontal vector winds, gradients, E/B decomposition, divergence and vorticity workflows. Spin 2 remains an important symmetric trace-free tensor operation and follows scalar and spin-1 optimization. The required parity set is spins 0–2. The current API and tested operation limit includes spin 4, but new spin-3/4 specialization and high-bandlimit expansion are deferred. Keep their existing implementation and regression tests, including when shared kernels change. Spin 5 and above require new authorization.

## Working engineering defaults (not yet final)

- Rust 2024 with Rust **1.99.0** pinned for development and CI. Port DUCC's numerical SHT algorithms in Rust; use RustFFT for complex FFTs plus a small private real-input FFT adapter for longitude rows. No RealFFT dependency or custom FFT butterflies. AVX-512 requires runtime CPU detection.
- Implement **f64/complex128** first, with a direct `num-complex` dependency for the public `Complex64` alias; design for f32/complex64. Both precisions are in the eventual feature matrix.
- Reusable plans and scratch; implement a scalar reference and `pulp` SIMD recurrence over latitude lanes during GL M1, with optional Rayon for coarse independent work and runtime dispatch on x86-64 and AArch64.
- Python bindings via a separate optional layer; keep spharmgrid decoupled.
- Avoid unbounded dense harmonic matrices. Symmetry-reduced scalar GL synthesis/analysis and spin-1 exact-analysis tables are permitted only within the measured 34 MiB cap and bandlimits in D23; recurrence remains the general method.

## Selected implementation defaults (2026-10-11)

These choices guide implementation without routine confirmation. Revise them when implementation evidence, tests or a concrete downstream need supports a change; record why. The D02 operation limit was set to spin 4 on 2026-10-12 after spin 3 and 4 passed analytic and DUCC checks for the full operation family on GL, CC and F1 at `lmax=spin`. Later checks cover sparse STANDARD synthesis through T2047 and selected exact-analysis coefficients through T1023 on all three grids. The supporting measurements are recorded in [05](05-validation-and-milestones.md). This operation limit does not change the D22 optimization order or authorize spin 5+.

| ID | Selected choice | Consequence |
| --- | --- | --- |
| D01 | Idiomatic Rust `ShtPlan`; DUCC-familiar Python functions | Plan contains grid, bandlimits and spin; use explicit operation/mode-specific methods. No mode in `HarmonicSpec` or public kernel trait hierarchy. |
| D02 | Required parity spins: 0, 1 and 2; current operation limit is spin 4 | Spin 3 and 4 passed all-grid operation checks at `lmax=spin`, sparse synthesis through T2047 and selected exact-analysis coefficient checks through T1023; spin 5 and above remain unsupported and require new authorization. |
| D03 | f64 first, f32 after f64 all-grid correctness | Verify f32 separately, particularly at high bandlimits. |
| D04 | Private real-FFT adapter over RustFFT | For even longitude lengths pack pairs of real samples into a half-length complex FFT, then reconstruct the nonnegative Hermitian spectrum. Invert for synthesis. Use full-length complex RustFFT for odd lengths initially. No RealFFT dependency or FFT butterflies. |
| D05 | Independent GPL-2.0-or-later Rust/Python packages | Keep spharmgrid independent, with a separately packaged optional backend; review combined distribution before release. |
| D06 | Accuracy before speed | Through GL scalar T255, use provisional relative L2 `<=5e-11` for independent analytic and pinned-DUCC comparisons, and adjoint normalized residual `<=1e-10`; define absolute near-zero checks. Set distinct high-L/spin/f32 criteria from evidence. |
| D07 | Contiguous into-buffer Rust methods, Python API after native parity | Reuse `ShtWorkspace`; add procedural Python functions and a reusable plan after verified Rust operations. |
| D08 | Theta/phi vector components in core | Convert geographic winds `(u,v) -> (-v,u)` in spharmgrid only. |
| D09 | Rectangular GL, CC and F1 only | Arbitrary rings, other geometries, NUFFT and GPU are outside current scope. |
| D10 | `pulp` for SIMD during GL M1 | Preserve scalar fallback and dispatch AVX2/FMA, AVX-512 (`x86-v4`) and AArch64 NEON at runtime. Use raw `std::arch` only for measured missing operations. No global `target-cpu=native` in portable builds. |
| D16 | Unit `ringfactor` in initial API | Compare with DUCC's default scaling of one; defer arbitrary per-ring scaling. |
| D17 | Strict input validation | Reject nonreal input `m=0` coefficients and nonzero `l<spin` coefficients rather than discarding them. |
| D18 | Incremental autonomous implementation | Work on a new branch, commit independently tested steps and continue; disclose unaffordable stress tests or blocked stages without weakening tests. |
| D19 | Faithful DUCC numerical methods | Port DUCC GL geometry, CC/F1 quadrature and resampling, scaled scalar/spin recurrences, normalization, cutoffs and adjoints from the pinned GitHub revision. Preserve source notices where directly translated. |
| D20 | Small runtime dependency set | Use `rustfft` for FFTs, `pulp` for SIMD, `num-complex` for public complex types and optional `rayon` for threading. No `gauss-quad`, `realfft`, `ndarray` or general matrix library in the numerical core. |
| D21 | SIMD and parallelism during M1 | Establish a scalar reference, then implement SIMD latitude-lane blocking and coarse scheduling before CC/F1. M4 tunes those kernels and adds f32. |

## Numerical port and dependency policy

Source repository: [mreineck/ducc](https://github.com/mreineck/ducc), pinned at `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Port the relevant numerical algorithms into Rust: `src/ducc0/math/gl_integrator.{h,cc}` uses iterative Legendre roots for `ntheta<=100` and the Bogaert/FastGL method for larger grids; `src/ducc0/sht/sht.cc` defines CC/F1 quadrature, prepared-CC resampling and operator weights; `sht_inner_loop.h` defines scaled scalar/spin recurrences. Preserve material numerical behaviors rather than substituting a generic quadrature or dense matrix routine. C++ `long double` in the small-GL algorithm may not have an exact stable Rust f64 equivalent; assess observed node/weight differences and adjust only with evidence. External crates are limited to general-purpose FFT, SIMD, complex storage and threading. A private even-length real-FFT packing helper over RustFFT is permitted; do not port PocketFFT butterflies. Use `pulp` to express DUCC's vectorized recurrences without duplicating large raw-intrinsic kernels; use `std::arch` only for measured gaps. Keep packed coefficients and blocked contiguous workspaces, not `ndarray` in the hot path. Any direct source translation must retain applicable DUCC/FastGL copyright and license notices, with the original file and revision recorded. Numerical tests are written independently.

## Evidence-dependent choices

Higher-spin maximum, high-bandlimit and f32 tolerances, SIMD speedups, final allocation guarantees, batching/strides, thread defaults and safe concurrency behavior need measured evidence. Copied or translated DUCC source needs separate copyright and license review.

## Excluded from initial parity scope

General irregular-point NUFFT synthesis, HEALPix or arbitrary ring geometry, spin on non-isolatitude samples, GPU kernels, full spherical coefficient rotation, and xarray/geophysical utilities. The 2023 paper discusses **general-grid NUFFT operations**, which motivates later extensions but does not itself establish the GL/CC/F1 exact-analysis contract. Success requires independently verified Rust SHT kernels, documented tested limits, Python access and an optional spharmgrid adapter—not merely DUCC-matching roundtrips.
