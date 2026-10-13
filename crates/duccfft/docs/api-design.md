# API design

The public API will expose immutable `ComplexFftPlan<T>` and `RealFftPlan<T>`
values for `f32` and `f64`. Each plan owns its factorization, twiddles, chirp
data, and nested plans. A caller creates one mutable workspace per concurrent
execution. Repeated execution will not allocate. The planned operations are:

- `ComplexFftPlan<T>::forward` and `inverse` on an in-place `&mut [Complex<T>]`.
- `RealFftPlan<T>::forward` from `&[T]` to the ordinary half-spectrum
  `&mut [Complex<T>]` of length `n / 2 + 1`.
- `RealFftPlan<T>::inverse` from that half-spectrum to `&mut [T]`. Forward transforms will compute the unnormalized negative-exponent DFT. Inverse
transforms will include a `1/n` scale. The real inverse will reject nonzero
imaginary parts at DC and, for even `n`, Nyquist. The `n == 1` transform is
valid; zero is rejected by the plan constructor. The public inverse remains
normalized; there is no scaling option in the initial API. Operations report
invalid lengths and mismatched buffers through `FftError`. The plan and workspace types will use concrete fields and checked constructors. Invalid lengths, arithmetic overflow, allocation failure, and mismatched
buffers will return errors. Public buffer formats will not expose DUCC's
internal FFTPACK real packing. Batch execution will use contiguous rows, because RustSHT stores longitude
rows contiguously. The batch API and SIMD layout will be added after scalar
execution is measured. The crate will not depend on RustSHT, Rayon, or a
runtime FFT backend.
