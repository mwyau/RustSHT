# Numerical specification: GL, CC, F1

## Operators and their relationships

Define **synthesis** `S: alm -> map`, **analysis** `A: map -> alm`, **adjoint synthesis** `S*: map -> alm`, and **adjoint analysis** `A*: alm -> map`. For an admissible bandlimited field, `A(S(a)) ≈ a`; in general `S(A(map)) != map` for arbitrary sampled maps. The two adjoints must satisfy their separate inner-product identities and **must not** be implemented by aliasing analysis and synthesis.

DUCC uses a real harmonic coefficient domain stored as complex nonnegative-`m` coefficients. For the standard Euclidean map inner product, the matching harmonic real inner product counts `m>0` real and imaginary degrees of freedom with factor **2** (equivalently `sqrt(2)`-scaled real/imag packing); `m=0` is real. Match DUCC's convention and phase before applying any weights. Quadrature weights belong to `A` (and to `A*` according to its definition), **not automatically to `S*`**.

DUCC 2D `analysis_2d` and `adjoint_analysis_2d` use **STANDARD** semantics. `synthesis_2d` and `adjoint_synthesis_2d` also provide `GRAD_ONLY` and `DERIV1`. Do not invent derivative-specific `analysis_2d` modes to fill an artificial matrix.

## Numerical convention checkpoint

Before implementing the first recurrence, establish scalar `Y_lm` normalization and phase, Fourier signs/scaling, negative-`m` reality relationship, packed indexing and both inner products. Verify against published mathematics, independent low-degree fields and the pinned DUCC revision, rather than relying on roundtrips. For spin, establish spin-weighted and E/B conventions, angular derivative signs and mode relationships before coding the recurrence.

For uniform longitude rings, DUCC's `get_gridweights` gives latitude-ring weights `w_i` that sum to `4*pi`; each pixel has integration weight `w_i/nphi`. FFT normalization is separate. DUCC `ringfactor` is fixed to one in the initial RustSHT API and parity tests.


## GL first: complete operation family (approved D11)

Work on one grid with a shared, grid-independent recurrence/FFT driver:

1. Spin-0 scalar `S/A`, normalization, phase, packed coefficients, GL nodes and weights.
2. Spin-1 and spin-2 STANDARD `S/A` including vector E/B, then test higher spins separately before promising their limits.
3. `GRAD_ONLY` and `DERIV1` synthesis and adjoint synthesis where valid.
4. Both STANDARD `S*` and `A*` for scalar/spin; verify adjoint identities on the real harmonic domain.
5. Gate GL completion on this entire validated matrix, including high-degree numerical checks.

Then add **CC**, then **F1**, reusing the transform drivers and introducing only the required ring locations, quadrature and spectral meridional resampling paths. Every supported operation gets a fresh grid-specific parity test.

## Grid and recoverability contract

With `Nθ = ntheta`, `Nφ = nphi` and `lmax=L`, exact analysis in DUCC's rectangular 2D interface requires:

| Geometry | Colatitude sample `theta_i` (north to south) | Maximum safe `L` |
| --- | --- | --- |
| GL | `acos(x_i)` at GL nodes, increasing theta | `Nθ - 1` |
| CC | `i*pi/(Nθ-1)`, including both poles | `Nθ - 2` |
| F1 | `(i+1/2)*pi/Nθ`, excluding poles | `Nθ - 1` |

For alias-free **analysis**, `mmax <= floor((Nφ-1)/2)`, in addition to `0 <= mmax <= lmax`. Thus `Nφ=2*mmax+1` is the minimum odd longitude count and `2*mmax+2` the smallest even one. Test both. Report errors for invalid exact-analysis input rather than silently returning aliased coefficients. Some **synthesis** configurations have different longitude/degree constraints; do not apply exact-analysis limits to all operations indiscriminately.

The `get_gridweights` values from DUCC include azimuthal integration (`2*pi` times latitude-rule weights): for a full sphere their sum should be `4*pi`. Test symmetry, positivity where expected, pole behavior and exact low-degree moments. Compare nodes, weights and `maximum_safe_l` separately.

DUCC's **CC/F1 high-bandlimit analysis is not simple quadrature on the original latitude rings**. Its `analysis_2d` computes longitude Fourier modes, resamples their meridional profiles to a prepared CC grid (`resample_to_prepared_CC`) using pole/reflection parity, zero padding and convolution with suitable weights, and then performs the Legendre adjoint. `adjoint_analysis_2d` reverses that path through `resample_from_prepared_CC`. Model these as an explicit analysis-side operator and its adjoint. Do **not** replace this by simple CC/F1 weights or assume it follows directly from the 2023 general-NUFFT paper.

## Storage and coordinate conventions

- Real fields, with `a_lm` stored for `m=0..mmax` and each `l=m..lmax`; `nalm = (mmax+1)*(lmax+1) - mmax*(mmax+1)/2`. Use checked arithmetic.
- Harmonic modes with `l < spin` must be zero and input `m=0` coefficients real. Reject forbidden nonzero input rather than silently discarding it.
- In STANDARD mode: spin 0 uses one real map and one coefficient channel; spin>0 uses two real map and two coefficient channels. `GRAD_ONLY` uses one coefficient channel and two map channels for spin>0.
- `DERIV1` is the dedicated spin-1 derivative path; verify its normalization `sqrt(l*(l+1))` and angular outputs `d/dtheta`, `(1/sin(theta))*d/dphi` against analytic fields.
- Geographic east/north wind `(u,v)` maps to DUCC's southward-theta/eastward-phi components `(-v,u)`. Reverse this conversion on output.
- Angles are radians; `phi_j = phi0 + 2*pi*j/Nφ` eastward, and latitude rings proceed north to south. Validate nonzero `phi0`, both latitude parities, odd/even `Nφ`, and CC pole handling.
- Preserve f64/complex128 accuracy first; f32/complex64 later. The high-bandlimit recurrence should not allocate an `O(L^3)` full harmonic basis.

## High-degree recurrence and failure modes

DUCC's `sht_inner_loop.h` uses rescaled normalized spin-0 and spin-weighted recurrences, polar cutoffs, equatorial reflection parity, vector batches and finite workspace blocks. Match these *numerical properties* in Rust; naive factorial evaluation or unscaled associated Legendre recurrences are not acceptable substitutes.

Test sectoral and near-sectoral harmonics, `m=0` and `m=L`, `l=spin`, north/south polar behavior, tiny and dominant coefficients, and mixed-degree stress. Establish explicitly which spin/precision/ring layout combinations are recoverable; do not extrapolate from a low-L roundtrip.

Reference: DUCC `sht.cc` (`maximum_safe_l`, `get_gridweights`, `analysis_2d`, `adjoint_analysis_2d`, `resample_*_prepared_CC`), `sht_inner_loop.h`, `python/sht_pymod.cc`. Pinned baseline and sources are in [01](01-ducc-provenance-and-license.md).
