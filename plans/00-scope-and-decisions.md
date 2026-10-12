# Scope and decisions

Updated 2026-10-11 (America/New_York). Planning only; there is no working SHT implementation yet.

## Objective

Implement high-bandlimit SHT algorithms used by DUCC in an independent **RustSHT** library, with native Rust and Python APIs. Support **GL, CC and F1** regular isolatitude grids with scalar, vector/spin-weighted, derivative, synthesis, analysis and relevant adjoint operations, aiming for DUCC-equivalent numerical results and competitive performance. An eventual optional backend for [spharmgrid](https://github.com/mwyau/spharmgrid) is a downstream use case, not a dependency.

Copyright for original Rust code: **Albert M. W. Yau** and contributors. License: **GPL-2.0-or-later**. Cite Martin Reinecke and collaborators; retain third-party notices for any source expression actually copied, adapted or translated. See [reference and licensing](01-ducc-provenance-and-license.md).

## Approved decisions

| ID | Decision | Consequence |
| --- | --- | --- |
| D11 | **Feature-first / GL first**, approved 2026-10-11 | Complete scalar, spin/vector, applicable derivative modes, synthesis/analysis and both adjoint families on GL before bringing the *same engine* to CC and F1. Incremental GL substeps are allowed. |
| D12 | **DUCC as algorithmic/numerical reference**, approved 2026-10-11 | RustSHT is not DUCC's upstream/downstream repository or an official DUCC release. Original Rust code and adapted source expression have different provenance obligations. |
| D13 | **GL/CC/F1 parity is the release objective**, reaffirmed 2026-10-11 | All three grids must pass the same supported operator/spin test matrix and explicit high-bandlimit checks before claiming full scope. |

### D11 rationale

A scalar-only implementation on all grids would defer spin recurrence, component count, derivative and adjoint design until late. Instead, build those shared mathematical interfaces on GL, validate them, then introduce grid-specific geometry and meridional analysis paths for CC and F1. The GL feature gate and the later all-grid high-bandlimit gate are separate.

### D12 rationale

Implement the algorithms in Rust using published literature and DUCC for behavioral comparison. DUCC source-code copying or direct translation, if it occurs, requires preserving its actual notices and tracking the original version; citing an algorithm does not itself transfer source copyright.

## Working engineering defaults (not yet final)

- Stable Rust 2024; use RustFFT for longitude transforms. No RealFFT dependency or DUCC FFT port until measurement supports it.
- Implement **f64/complex128** first, but design for f32/complex64. Both precisions are in the eventual feature matrix.
- Reusable plans and scratch buffers; optional Rayon; CPU runtime dispatch for x86-64 and AArch64 after scalar/reference correctness.
- Python bindings via a separate optional layer; keep spharmgrid decoupled.
- Avoid large precomputed dense harmonic matrices and hidden high-bandlimit caps.

## Open questions

| ID | Question | Current proposal |
| --- | --- | --- |
| D01 | Rust API vs exact DUCC API shape | Idiomatic `ShtPlan` plus familiar procedural Python functions; mathematical parity is required, identical syntax is not |
| D02 | Positive-spin support | Establish spin 0, 1, 2 parity; test spin 3/4 and determine the maximum supported range from DUCC behavior and memory/performance |
| D03 | f32 timeline | f64 first; f32 required before all-feature release |
| D04 | Real-input longitude FFT | RustFFT complex baseline; real-packing optimization if FFT fraction justifies it |
| D05 | GPL packaging alongside BSD spharmgrid | Separate optional Python backend, review how actual wheels and redistributions are combined |
| D06 | Accuracy/performance thresholds | Define per dtype/spin/operator/grid tolerance against analytic and DUCC references; benchmark T255–T8191 as resources permit |
| D07 | Python API | Expose both into-buffer operations and convenience functions after core Rust proof |
| D08 | Wind/vector API | Low-level DUCC spherical components plus explicit geographic east/north wrappers |
| D09 | Nonrectangular grids and arbitrary ring sampling | Out of current scope; target rectangular GL/CC/F1 and document the boundary |
| D10 | Multiarch | Portable fallback, x86-64 AVX2/FMA/AVX-512 and AArch64 NEON, dispatched at plan creation |

Record new owner decisions here with date, evidence and consequences. Do not convert proposals into approved requirements without review.

## Excluded from initial parity scope

General irregular-point NUFFT synthesis, HEALPix or arbitrary ring geometry, spin on non-isolatitude samples, GPU kernels, full spherical coefficient rotation, and xarray/geophysical utilities. The 2023 paper discusses **general-grid NUFFT operations**, which motivates later extensions but does not itself establish the GL/CC/F1 exact-analysis contract.

Success requires independently verified Rust SHT kernels, documented tested limits, Python access and an optional spharmgrid adapter—not merely DUCC-matching roundtrips.
