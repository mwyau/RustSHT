# Numerical architecture and performance

## Public/private boundary

Keep `ShtPlan` immutable with its FFT plans, precomputed recurrence coefficients and selected dispatch function. `ShtWorkspace` owns mutable scratch per invocation or worker; no global mutable state. Prefer internal concrete kernel implementations and `pub(crate)` helpers over public abstraction traits. The workspace already has one native numerical crate and one Python binding crate. Implement Python numerical bindings after native correctness; do not add further crates without a concrete need. The native API uses borrowed slices with checked shapes and `Result`-returning operations, and does not know about Xarray or geographic wind units. See [03](03-api-and-python.md) for the proposed modules.

## Shared engine and grid-specific stages

Build one harmonic kernel family for spin-0 and DUCC-supported spin-positive recurrences, synthesis/adjoint synthesis, and analysis/adjoint analysis. The harmonic core receives ring coordinates and prepared Fourier modes, not hard-coded GL/CC/F1 choices. Grid-specific code handles nodes, quadrature and any meridional preparation/reconstruction.

**GL is the first full feature gate** (D11). Only after scalar/spin/vector, derivative and adjoint tests pass on GL should CC and F1 integrate through the same core. Preserve vectorization opportunities in the interface; don't build and then discard a scalar-only `Y_lm` matrix implementation.

For GL analysis, weight Fourier modes according to the GL latitude quadrature. For CC/F1 analysis, implement the distinct prepared-CC meridional resampling, parity, padding and weighting stages, with a matching adjoint. Their FFT/convolution costs belong in performance accounting; RustFFT will serve both longitude FFTs and any needed latitude FFTs.

## FFT: RustFFT plus a private real-input adapter

RustFFT is the **only FFT algorithm dependency**. Port DUCC's relevant real-transform packing strategy, not the PocketFFT implementation. Implement the narrow real-input helpers used for longitude rows:

- **Even `nphi`:** pack `x[2j] + i*x[2j+1]` into `nphi/2` complex samples. Run the planned half-length complex FFT; reconstruct `nphi/2+1` nonnegative Hermitian modes using cached twiddles. Invert these steps for synthesis. Handle correct scaling for the unnormalized RustFFT direction.
- **Odd `nphi`:** begin with a full-length complex RustFFT and extract/reconstruct conjugate frequencies. The highest positive mode is not a Nyquist mode; optimize only when profiling warrants it.
- Check length 1/2, DC, even-length real Nyquist mode, zero-padded/truncated `mmax`, `phi0` shifts, and inverse/adjoint scaling. Validate against a direct full-complex FFT, not solely with the adapter's own roundtrip.
- Cache FFT plans, twiddle constants and reusable worker scratch. Run RustFFT with caller-owned scratch, without allocation in steady execution. RustFFT handles FFT-specific CPU SIMD; avoid an FFT backend framework or hand-written butterfly kernels.

## DUCC recurrence with early SIMD

Port DUCC's normalized, scaled scalar and spin recurrence with its polar cutoffs, parity and sectoral handling. Establish an independently checked scalar reference, then vectorize the **same recurrence** across independent latitude rings using `pulp` during GL M1. Avoid rewriting the mathematical algorithm into a dense matrix multiplication.

- Iterate degree `l` sequentially while grouping latitude lanes and independent order `m` work into bounded, cache-friendly blocks. Prefer unit-stride SIMD access and reuse recurrence constants and buffers.
- Dispatch AVX2/FMA, AVX-512 (with `pulp`'s `x86-v4` feature) and AArch64 NEON only where CPU features are present. Keep a portable scalar fallback and test it explicitly.
- Preserve per-lane underflow/overflow rescaling, reflection parity and polar cutoffs. Use FMA where numerically appropriate and test floating-point order differences; no global fast-math.
- Use `std::arch` intrinsics only for an identified missing operation or measured improvement, confined to small runtime-checked boundaries. No unstable `std::simd`, `wide`, or SIMD trait hierarchy in the public API.

## Threading, memory and allocation

- Optional Rayon runs coarse independent `m` or ring-FFT blocks during GL M1. Avoid per-degree jobs, atomics on coefficients, nested parallel pools and shared mutable scratch.
- Each worker receives separate prepared scratch. Pack harmonic coefficients externally as before; internally use bounded contiguous Fourier and latitude-lane blocks. Record scratch size and copying cost.
- Use checked integer arithmetic for array sizes and offsets. Avoid dense `O(L^3)` bases and unnecessary full-map copies. At `L=8191`, a complex128 triangular harmonic array occupies about 512 MiB, and an `8192 x 16384` float64 map about 1 GiB, before scratch.
- Benchmark plan construction, FFT, recurrence, packing, memory and full repeated execution with one and multiple threads as soon as GL works.

## Benchmark method

Compare RustSHT with the pinned DUCC CPU implementation on identical hardware, data, precision, `lmax/mmax`, grid/longitude dimensions, spin, mode, threads and output accuracy. Report:

- FFT-only, meridional resample-only, Legendre-only and full SHT stage times.
- Plan construction versus steady-state repeated execution.
- One-thread and multi-thread scaling; AVX2/AVX-512/NEON dispatch selected.
- CPU type, compiler flags, memory/RSS, correctness tolerance and run-to-run spread.
- True minimum odd `Nφ=2*mmax+1`, smallest even `Nφ=2*mmax+2`, and FFT-friendly alternatives; compare correctness and performance separately.

Compare against the pinned DUCC implementation during GL M1, not only at M4. If slower by more than 2×, profile FFT, recurrence, packing and scheduling in the same session without weakening correctness. Benchmark AVX2 against AVX-512 where supported; a wider SIMD vector is not necessarily faster.

Core dependencies: `rustfft`, `num-complex`, `pulp` (enable `x86-v4` for AVX-512), and optional `rayon`. Avoid `gauss-quad`, `realfft`, `ndarray`, `faer`, `wide` or additional SIMD/FFT libraries without a concrete measured need. PyO3 remains in the existing Python member.
