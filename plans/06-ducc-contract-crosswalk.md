# DUCC operation and grid crosswalk

This document maps the GL/CC/F1 subset of the DUCC `ducc0` API to RustSHT operations. Reference: commit [`c1ed01b8fda69d2a378190f0915f1a19762dddbf`](https://github.com/mreineck/ducc/commit/c1ed01b8fda69d2a378190f0915f1a19762dddbf) (2026-10-09) in [mreineck/ducc](https://github.com/mreineck/ducc). Source comparisons define behavior; they do not prescribe the Rust code organization.

## Supported operators

| DUCC function | Numerical operator | Applicable modes |
| --- | --- | --- |
| `synthesis_2d` | S: harmonic coefficients → sampled map | STANDARD; GRAD_ONLY for spin>0; DERIV1 for spin-1 first derivatives |
| `adjoint_synthesis_2d` | S*: map → harmonic coefficients | Adjoint of the corresponding synthesis mode |
| `analysis_2d` | A: map → harmonic coefficients, left-inverse of S on valid bandlimited fields | STANDARD |
| `adjoint_analysis_2d` | A*: coefficients → map | STANDARD |
| `get_gridweights` | Integrated latitude/azimuthal quadrature weights | GL, CC, F1 |
| `maximum_safe_l` | Safe degree for exact analysis | Grid dependent |

`analysis_2d` and `adjoint_analysis_2d` use STANDARD in `sht.cc`; do not infer GRAD_ONLY or DERIV1 analysis by symmetry. Pseudo-analysis for arbitrary pixelizations is outside the initial scope.

## Exact-analysis limits

| Grid | North-to-south colatitude | Maximum safe `lmax` |
| --- | --- | --- |
| GL | Gauss–Legendre nodes `acos(x_i)` | `ntheta - 1` |
| CC | `theta_i = i*pi/(ntheta-1)` (includes poles) | `ntheta - 2` |
| F1 | `theta_i = (i+1/2)*pi/ntheta` | `ntheta - 1` |

The Python `analysis_2d` documentation additionally states `mmax <= floor((nphi-1)/2)`. Longitude samples are `phi0 + 2*pi*j/nphi`. Synthesis may admit other geometry/order combinations; validate each operation rather than requiring exact-analysis sampling universally.

`get_gridweights` already contains azimuthal `2*pi`; the ring weights sum to `4*pi` and the per-pixel weight for uniform longitude sampling is `w_i/nphi`. Check FFT normalization separately. The minimum longitude count for exact analysis is `2*mmax+1` (odd), or `2*mmax+2` for even sampling.

## Initial RustSHT subset

DUCC's rectangular operators accept per-ring `ringfactor`. The first RustSHT API uses only its default value of one; arbitrary ring scaling is deferred and not part of the initial parity claim. STANDARD analysis/adjoint analysis have no GRAD_ONLY or DERIV1 modes; these belong to synthesis and its adjoint.

## Harmonic representation

- Packed `alm`: for each nonnegative order `m=0..mmax`, degrees `l=m..lmax`. `nalm = (mmax+1)*(lmax+1) - mmax*(mmax+1)/2`.
- For real maps, `m=0` harmonic values are real. In the matching real harmonic inner product, `m>0` modes have factor **2**. DUCC's `python/test/test_sht.py` packs complex coefficients as scaled real/imag components for adjoint testing.
- Spin 0 STANDARD uses one map and one coefficient channel. Spin>0 STANDARD uses two of each; GRAD_ONLY/DERIV1 use one coefficient channel and two map channels. `l<spin` modes are excluded.
- spharmgrid wind conversion: geographic east/north `(u,v)` maps to spherical `(v_theta,v_phi)=(-v,u)`, because colatitude increases southward.
- DERIV1 uses spin-1 gradient conventions; the output represents angular `d/dtheta` and `(1/sin theta)*d/dphi` with the appropriate harmonic degree scaling.

## CC/F1 exact analysis and adjoint

In `src/ducc0/sht/sht.cc`, `analysis_2d` FFTs the longitude rows, then calls `resample_to_prepared_CC` on CC/F1 Fourier modes. That stage handles spin and longitude-order parity, the pole-inclusion flags, phase shifts for F1, Fourier padding/truncation, convolution and prepared-grid weighting. The Legendre transform then operates on a prepared CC ring lattice. `adjoint_analysis_2d` uses `resample_from_prepared_CC`, the corresponding adjoint path. Replacing this method with low-resolution direct quadrature violates exact-analysis recoverability at near-minimal ring counts.

GL analysis instead performs weighted longitude FFTs using GL quadrature and a Legendre projection. The CC/F1 resampling path uses FFTs in latitude as well as longitude; include both in memory and performance planning.

## Reference source map

| Topic | DUCC file |
| --- | --- |
| Transform signatures, spin/modes | `src/ducc0/sht/sht.h`, `python/sht_pymod.cc` |
| Safe grid degree and weights | `src/ducc0/sht/sht.cc` `maximum_safe_l`, `get_gridweights`; `src/ducc0/math/gl_integrator.{h,cc}` for GL nodes/weights |
| Scaled recurrences, polar cutoff, SIMD | `src/ducc0/sht/sht_inner_loop.h` |
| CC/F1 analysis and both adjoints | `src/ducc0/sht/sht.cc`, `sht_utils.h` |
| Operator definitions | `doc/sht.rst` |
| Numerical test cases | `python/test/test_sht.py` (**GPL-3.0-or-later header**; independently implement tests rather than copying source) |

## Literature

- [Reinecke & Seljebotn (2013)](https://doi.org/10.1051/0004-6361/201321494): libsharp, spin SHT and vectorized kernels.
- [Reinecke, Belkner & Carron (2023)](https://doi.org/10.1051/0004-6361/202346717): general-grid transforms, lensing and adjoints. This paper is not the sole source defining DUCC's CC/F1 exact-analysis method.
