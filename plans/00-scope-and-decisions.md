# Scope and decisions

Updated 2026-10-11 (America/New_York). Planning only; there is no working SHT implementation yet.

## Objective

Implement high-bandlimit SHT algorithms used by DUCC in an independent **RustSHT** library, with native Rust and Python APIs. Support **GL, CC and F1** regular isolatitude grids with scalar, vector/spin-weighted, derivative, synthesis, analysis and relevant adjoint operations, aiming for DUCC-equivalent numerical results and competitive performance. An eventual optional backend for [spharmgrid](https://github.com/mwyau/spharmgrid) is a downstream use case, not a dependency. Copyright for original Rust code: **Albert M. W. Yau** and contributors. License: **GPL-2.0-or-later**. Cite Martin Reinecke and collaborators; retain third-party notices for any source expression actually copied, adapted or translated. See [reference and licensing](01-ducc-provenance-and-license.md).

## Approved decisions

| ID | Decision | Consequence |
| --- | --- | --- |
| D11 | **Feature-first / GL first**, approved 2026-10-11 | Complete scalar, spin/vector, applicable derivative modes, synthesis/analysis and both adjoint families on GL before bringing the *same engine* to CC and F1. Incremental GL substeps are allowed. |
| D12 | **DUCC as algorithmic/numerical reference**, approved 2026-10-11 | RustSHT is not DUCC's upstream/downstream repository or an official DUCC release. Original Rust code and adapted source expression have different provenance obligations. |
| D13 | **GL/CC/F1 parity is the release objective**, reaffirmed 2026-10-11 | All three grids must pass the same supported operator/spin test matrix and explicit high-bandlimit checks before claiming full scope. |
| D14 | **Rust 1.89 minimum**, approved 2026-10-11 | Stable AVX-512 target features and intrinsics, with runtime CPU checks and a portable fallback. |
| D15 | **Long autonomous implementation sessions**, approved 2026-10-11 | Work on a new feature branch through as many independently verified milestones as possible, preserving GL-first ordering. |

### D11 rationale

A scalar-only implementation on all grids would defer spin recurrence, component count, derivative and adjoint design until late. Instead, build those shared mathematical interfaces on GL, validate them, then introduce grid-specific geometry and meridional analysis paths for CC and F1. The GL feature gate and the later all-grid high-bandlimit gate are separate.

### D12 rationale

Implement the algorithms in Rust using published literature and DUCC for behavioral comparison. DUCC source-code copying or direct translation, if it occurs, requires preserving its actual notices and tracking the original version; citing an algorithm does not itself transfer source copyright.

## Working engineering defaults (not yet final)

- Rust 2024 with MSRV **1.89**; use RustFFT for longitude transforms. Rust 1.89 stabilizes AVX-512 target features and many intrinsics, but AVX-512 execution still requires runtime CPU detection. No RealFFT dependency or DUCC FFT port without measurement.
- Implement **f64/complex128** first, with a direct `num-complex` dependency for the public `Complex64` alias; design for f32/complex64. Both precisions are in the eventual feature matrix.
- Reusable plans and scratch buffers; optional Rayon; CPU runtime dispatch for x86-64 and AArch64 after scalar/reference correctness.
- Python bindings via a separate optional layer; keep spharmgrid decoupled.
- Avoid large precomputed dense harmonic matrices and hidden high-bandlimit caps.

## Selected implementation defaults (2026-10-11)

These choices guide implementation without routine confirmation. Revise them when implementation evidence, tests or a concrete downstream need supports a change; record why.

| ID | Selected choice | Consequence |
| --- | --- | --- |
| D01 | Idiomatic Rust `ShtPlan`; DUCC-familiar Python functions | Plan contains grid, bandlimits and spin; use explicit operation/mode-specific methods. No mode in `HarmonicSpec` or public kernel trait hierarchy. |
| D02 | Required parity spins: 0, 1 and 2 | Higher spins are optional and require separate analytic and DUCC verification. |
| D03 | f64 first, f32 after f64 all-grid correctness | Verify f32 separately, particularly at high bandlimits. |
| D04 | Complex RustFFT baseline | Reuse plans/scratch; real packing or `realfft` requires benchmark evidence. |
| D05 | Independent GPL-2.0-or-later Rust/Python packages | Keep spharmgrid independent, with a separately packaged optional backend; review combined distribution before release. |
| D06 | Accuracy before speed | Through GL scalar T255, use provisional relative L2 `<=5e-11` for independent analytic and pinned-DUCC comparisons, and adjoint normalized residual `<=1e-10`; define absolute near-zero checks. Set distinct high-L/spin/f32 criteria from evidence. |
| D07 | Contiguous into-buffer Rust methods, Python API after native parity | Reuse `ShtWorkspace`; add procedural Python functions and a reusable plan after verified Rust operations. |
| D08 | Theta/phi vector components in core | Convert geographic winds `(u,v) -> (-v,u)` in spharmgrid only. |
| D09 | Rectangular GL, CC and F1 only | Arbitrary rings, other geometries, NUFFT and GPU are outside current scope. |
| D10 | Runtime SIMD under stable Rust 1.89 | Scalar fallback; AVX2/FMA, AVX-512 and AArch64 NEON variants with exact feature checks, no global `target-cpu=native` in portable binaries. |
| D16 | Unit `ringfactor` in initial API | Compare with DUCC's default scaling of one; defer arbitrary per-ring scaling. |
| D17 | Strict input validation | Reject nonreal input `m=0` coefficients and nonzero `l<spin` coefficients rather than discarding them. |
| D18 | Incremental autonomous implementation | Work on a new branch, commit independently tested steps and continue; disclose unaffordable stress tests or blocked stages without weakening tests. |

## Evidence-dependent choices

Higher-spin maximum, high-bandlimit and f32 tolerances, SIMD speedups, final allocation guarantees, batching/strides, thread defaults and safe concurrency behavior need measured evidence. Copied or translated DUCC source needs separate copyright and license review.

## Excluded from initial parity scope

General irregular-point NUFFT synthesis, HEALPix or arbitrary ring geometry, spin on non-isolatitude samples, GPU kernels, full spherical coefficient rotation, and xarray/geophysical utilities. The 2023 paper discusses **general-grid NUFFT operations**, which motivates later extensions but does not itself establish the GL/CC/F1 exact-analysis contract. Success requires independently verified Rust SHT kernels, documented tested limits, Python access and an optional spharmgrid adapter—not merely DUCC-matching roundtrips.
