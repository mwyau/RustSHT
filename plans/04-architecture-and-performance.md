# Numerical architecture and performance

## Public/private boundary

Keep `ShtPlan` immutable with its FFT plans, precomputed recurrence coefficients and selected dispatch function. `ShtWorkspace` owns mutable scratch per invocation or worker; no global mutable state. Prefer internal concrete kernel implementations and `pub(crate)` helpers over public abstraction traits. The workspace already has one native numerical crate and one Python binding crate. Implement Python numerical bindings after native correctness; do not add further crates without a concrete need. The native API uses borrowed slices with checked shapes and `Result`-returning operations, and does not know about Xarray or geographic wind units. See [03](03-api-and-python.md) for the proposed modules.

## Shared engine and grid-specific stages

Build one harmonic kernel family for spin-0 and DUCC-supported spin-positive recurrences, synthesis/adjoint synthesis, and analysis/adjoint analysis. The harmonic core receives ring coordinates and prepared Fourier modes, not hard-coded GL/CC/F1 choices. Grid-specific code handles nodes, quadrature and any meridional preparation/reconstruction.

**GL is the first full feature gate** (D11). Only after scalar/spin/vector, derivative and adjoint tests pass on GL should CC and F1 integrate through the same core. Preserve vectorization opportunities in the interface; don't build and then discard a scalar-only `Y_lm` matrix implementation. For GL analysis, weight Fourier modes according to the GL latitude quadrature. For CC/F1 analysis, implement the distinct prepared-CC meridional resampling, parity, padding and weighting stages, with a matching adjoint. Their FFT/convolution costs belong in performance accounting; RustFFT will serve both longitude FFTs and any needed latitude FFTs.

## FFT choice and measurement

Start with **RustFFT**, converting real longitude rows into complex FFT buffers and keeping the nonnegative Fourier modes with the correct normalization and `phi0` phase. Synthesis explicitly restores the conjugate-symmetric spectrum. This baseline may use roughly twice the FFT-stage work of a packed real-input method for even lengths; measure before introducing `realfft` or custom packing. Keep the FFT backend internal. Reuse planned FFTs and explicit scratch buffers, including meridional resampling transforms. Investigate the cost of transpose/packing and per-worker FFT scratch separately from FFT arithmetic. DUCC FFT porting is not an initial task.

## Stable SIMD and dispatch

- Portable reference kernel with ordinary Rust loops and compiler auto-vectorization.
- x86-64: conditional AVX2/FMA and AVX-512 kernels with a portable baseline. Rust 1.89 stabilizes AVX-512 target features and many intrinsics; every specialized kernel must check its required CPU feature subsets.
- AArch64: NEON plus portable fallback for other targets.
- Detect CPU features once at plan creation; select a kernel outside hot loops. Isolate unsafe intrinsic calls in small, feature-checked boundaries; do not require AVX-512 on every x86-64 machine.
- Start with `std::arch` if exact layout/instruction control is needed; compare `pulp` only after an accurate reference path exists.
- Portable distributed wheels must not require `-C target-cpu=native`; use runtime dispatch for supported variants. SIMD across independent latitude rings (or independent batch elements) while respecting the recurrence in degree `l`. Reproduce the required rescaling, polar cutoff and reflection behavior before comparing peak vector instruction throughput.

## Threading, memory and allocation

- Rayon remains an **optional** dependency; benchmark scheduling over independent `m` blocks, rings or batches, and avoid parallelizing many short FFTs unnecessarily.
- Establish per-worker scratch lifetime and bound worst-case allocations from `ncomp`, `lmax`, `mmax`, `ntheta`, `nphi` and batch.
- Use `checked_mul`/`checked_add` for `nalm`, map dimensions and buffer index offsets.
- Avoid an `O(L^3)` dense basis cache. At `L=8191`, one full triangular complex128 coefficient array already occupies roughly **512 MiB**; one `8192 x 16384` float64 map is **1 GiB**, before copies or scratch.
- Keep the no-allocation hot path measurable and differentiate initialization, warmed execution, internal packing and Python conversion.

## Benchmark method

Compare RustSHT with the pinned DUCC CPU implementation on identical hardware, data, precision, `lmax/mmax`, grid/longitude dimensions, spin, mode, threads and output accuracy. Report:

- FFT-only, meridional resample-only, Legendre-only and full SHT stage times.
- Plan construction versus steady-state repeated execution.
- One-thread and multi-thread scaling; AVX2/AVX-512/NEON dispatch selected.
- CPU type, compiler flags, memory/RSS, correctness tolerance and run-to-run spread.
- True minimum odd `Nφ=2*mmax+1`, smallest even `Nφ=2*mmax+2`, and FFT-friendly alternatives; compare correctness and performance separately. A within-2x DUCC throughput ratio is an **initial optimization checkpoint**, not a feature acceptance requirement or an achieved result. Never trade correct high-bandlimit results for that ratio. Dependencies: `rustfft`; optional `rayon`. Add PyO3/NumPy/maturin in the Python layer; keep `realfft`, `ndarray`, `pulp` and other runtime crates optional until measured need.
