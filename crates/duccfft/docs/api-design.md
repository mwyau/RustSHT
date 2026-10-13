# API design

The public API exposes immutable `ComplexFftPlan<T>` and `RealFftPlan<T>`
values for `f32` and `f64`. A caller creates one mutable workspace per
concurrent execution. Plans and workspaces can be reused, and transform calls
do not allocate. The operations are:

- `ComplexFftPlan<T>::forward` and `inverse` on an in-place
  `&mut [Complex<T>]`.
- `RealFftPlan<T>::forward` from `&[T]` to the ordinary half-spectrum
  `&mut [Complex<T>]` of length `n / 2 + 1`.
- `RealFftPlan<T>::inverse` from that half-spectrum to `&mut [T]`. Forward transforms compute the unnormalized negative-exponent DFT. Inverse
transforms include a `1/n` scale. The real inverse ignores imaginary parts at
DC and, for even `n`, Nyquist. Length one is valid; zero is rejected. The
inverse API is always normalized and has no scale option. Operations report
invalid lengths, allocation failures, and mismatched buffers through
`FftError`. Plans own factorization, roots, fixed-radix data, prime chirps, and Bluestein
kernel spectra. Workspaces own output and radix scratch plus reusable nested
convolution storage. A workspace is mutable and cannot serve simultaneous
calls. An immutable plan can be shared across threads when each call has a
separate workspace. The complex planner uses the factor order and radix thresholds from DUCC's
`cfftpass`: fixed radices 2, 3, 4, 5, 7, 8, and 11; general odd-prime
combines below 110; and Bluestein at 110 and above. The real planner retains
its distinct factor order and threshold: fixed radices 2, 3, 4, and 5; paired
general-prime combines below 135; and Bluestein at 135 and above. The
real planner also uses `rfftp_complexify` for even lengths above 1,000 when
the length exceeds 10,000 or its factorization contains a factor above five. The vectorization-only DUCC selection case is not enabled in the scalar Rust
planner. See `ducc-source-map.md` for the source mapping and
`m3-scalar-performance.md` for the current validation and performance
evidence. The public real format uses `num_complex::Complex<T>` bins and does not expose
DUCC's internal FFTPACK packing. The crate depends on neither RustSHT nor a
runtime FFT backend, worker pool, or SIMD dependency. RustFFT is used only as
a development reference.
