# API design

The public API exposes immutable `ComplexFftPlan<T>` and `RealFftPlan<T>`
values for `f32` and `f64`. Current plans own their factorization and roots;
future prime and Bluestein passes may add chirp data and nested plans. A caller
creates one mutable workspace per concurrent execution. Repeated execution
does not allocate. The planned operations are:

- `ComplexFftPlan<T>::forward` and `inverse` on an in-place `&mut [Complex<T>]`.
- `RealFftPlan<T>::forward` from `&[T]` to the ordinary half-spectrum
  `&mut [Complex<T>]` of length `n / 2 + 1`.
- `RealFftPlan<T>::inverse` from that half-spectrum to `&mut [T]`. Forward transforms compute the unnormalized negative-exponent DFT. Inverse
transforms include a `1/n` scale. The real inverse ignores imaginary parts at
DC and, for even `n`, Nyquist. The `n == 1` transform is valid; zero is rejected
by the plan constructor. The public inverse remains normalized; there is no
scaling option in the initial API. Operations report invalid lengths,
allocation failures, and mismatched buffers through `FftError`. The plan and workspace types use concrete fields and checked constructors. Invalid lengths, scratch-size overflow, allocation failure, and mismatched
buffers return errors. Public buffer formats do not expose DUCC's internal
FFTPACK real packing. The current real implementation works directly with the
public half-spectrum and has fixed radix-2/3/4/5 butterflies. Larger factors
use a generic scalar combine; specialized general-prime and Bluestein passes
remain future work. Batch execution will use contiguous rows, because RustSHT stores longitude
rows contiguously. The batch API and SIMD layout will be added after scalar
execution is measured. The crate will not depend on RustSHT, Rayon, or a
runtime FFT backend.
