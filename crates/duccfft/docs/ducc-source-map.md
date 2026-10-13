# DUCC 1D FFT source map

Reference revision: `c1ed01b8fda69d2a378190f0915f1a19762dddbf` from
`https://github.com/mreineck/ducc`.

The Rust port covers the one-dimensional pass planners and kernels in
`src/ducc0/fft/fft.h` and `src/ducc0/fft/fft1d_impl.h`. The N-dimensional
executor in `fftnd_impl.h`, DCT/DST wrappers, Hartley transforms, and unrelated
DUCC subsystems are outside the port.

## Plan and execution structure

`fft.h` defines `cfftpass<Tfs>` and `rfftpass<Tfs>`. Each pass reports its
scratch size and whether it needs a separate copy buffer; `exec()` returns the
buffer holding the result. The multipass plans alternate input and copy
buffers between passes. `pocketfft_c<Tfs>` and `pocketfft_r<Tfs>` build those
plans for a one-dimensional length. Their `exec` wrappers apply the caller's
scale factor after the pass tree. The pass kernels themselves leave forward
and inverse results unnormalized. Complex multipasses execute passes in
construction order for both directions. A real forward multipass executes
passes in reverse construction order; a real inverse multipass executes them
in construction order.

`cfftpass::factorize()` removes factors of 8 and 4, then places a factor 2 at
the front, followed by ascending odd factors. `rfftpass::factorize()` removes
factors of 4 and places a factor 2 at the front before odd factors. These
different decompositions feed different kernel sets.

## Complex transforms

`cfftpass<Tfs>::make_pass()` selects the complex plan:

| Factor case | DUCC component |
| --- | --- |
| 2, 3, 4, 5, 7, 8, 11 | `cfftp2`, `cfftp3`, `cfftp4`, `cfftp5`, `cfftp7`, `cfftp8`, `cfftp11` |
| One odd prime factor from 13 through 109 | `cfftpg` |
| One odd prime factor of 110 or greater | `cfftpblue` |
| More than one factor | `cfft_multipass` |
| Optional large aligned 1D vector case | `cfftp_vecpass` |

`cfft_multipass` creates one pass per factor while the product is at most
10,000. Above that threshold, it sorts prime factors descending and groups
them into two approximately balanced products before creating two passes.
`cfftp_vecpass` is considered when vectorization is enabled, the 1D length is
greater than 300 and at most 100,000, and the length is divisible by the
four-lane vector width selected in `make_pass()`. The complex and real planners
must remain separate; their radix sets and selection conditions differ.

`cfftpg` is the odd general-prime pass. `cfftpblue` handles larger odd prime
factors with a Bluestein convolution. Its subtransform length is
`good_size_cmplx(2*ip-1)`, where `good_size_cmplx` selects a size composed of
2, 3, 5, 7, and 11. The chirp is derived from roots of unity at length `2*ip`;
DUCC transforms a symmetric, zero-padded chirp and stores its spectrum with a
`1/ip2` factor. Execution applies the chirp, forward convolution FFT, spectral
product, inverse FFT, and output chirp.

## Real transforms

`rfftpass<Tfs>::make_pass()` selects `rfftp2`, `rfftp3`, `rfftp4`, or
`rfftp5` for those exact radices. A single odd prime factor below 135 uses
`rfftpg`; a factor of 135 or greater uses `rfftpblue`. Composite lengths use
`rfft_multipass`.

For even lengths greater than 1,000, DUCC can select `rfftp_complexify` before
the real factor tree. It does so when vectorization is enabled and the length
is divisible by eight, when the length exceeds 10,000, or when factorization
contains a factor greater than five. `rfftp_complexify` packs even and odd
real samples into an `N/2` complex transform and separates or combines the
Hermitian modes around that transform.

`rfftpblue` constructs a complex plan of length `ip`. That plan selects complex
Bluestein for this odd prime factor and uses a convolution length of
`good_size_cmplx(2*ip-1)`. The real and complex Bluestein paths therefore share
complex kernels while keeping separate top-level real passes.

DUCC's in-place real format stores DC at element 0, then each positive
frequency's real and imaginary components at `2*k-1` and `2*k`. Even-length
Nyquist occupies element `N-1` as a real value. The public Rust API will use
`Complex<T>` values for bins `0..=floor(N/2)` and convert at the boundary.

## Roots, arithmetic, and scratch

`src/ducc0/math/unity_roots.h` defines `UnityRoots`. `RootCalc` forms its initial
angle from a long-double pi constant, then stores the angle in `Thigh`. For
f32 and f64 transforms, `Thigh` is double; the quadrant-reduced trigonometric
arguments and `sin`/`cos` calls therefore use double precision. `UnityRoots`
stores two smaller root tables, combines them on lookup, and conjugates roots
above the half-turn. Kernels use conjugation and
`special_mul<fwd>` to select the negative-exponent forward transform or
positive-exponent inverse transform. `src/ducc0/math/cmplx.h` defines the
internal pair representation and these sign-dependent complex products.

`pocketfft_c::bufsize()` accounts for copy storage, plan scratch, and a
32-complex-value alignment allowance for lengths divisible by 1,024.
`pocketfft_r::bufsize()`
accounts for copy storage and pass scratch. Bluestein plans add two convolution
arrays and the subplan workspace. Rust can replace aligned C++ containers and
virtual pass ownership with `Vec`-backed scratch and an enum plan tree, while
preserving the pass schedule, result-buffer alternation, scaling, and chirp
math. Checked size arithmetic is required in Rust.

## SIMD and threading

`src/ducc0/infra/simd.h` supplies DUCC's compile-time SIMD types. The complex
planner's optional `cfftp_vecpass` transposes a scalar pass into vector lanes,
executes a vector pass, and unpacks the result. `cfft_multipass` also has SIMD
paths for nontrivial `l1` or `ido` dimensions. These layouts guide later
intra-transform and cross-transform experiments; they are not required for the
first scalar implementation.

`src/ducc0/infra/threading.h` and `threading.cc` implement the scheduler used by
batched `l1`/`ido` paths. A direct 1D plan enters the top-level multipass with
`l1=ido=1`, but large complex factorizations can create nested multipasses with
nontrivial `l1` or `ido`; SIMD multipass execution can call `execStatic` there
and receives the caller's thread count. The reference harness explicitly passes
`nthreads=1`. The Rust crate will keep execution caller-scheduled and will not
include DUCC's worker pool; this is a deliberate departure from DUCC's nested
thread support. The nested `execStatic` route depends on the build having a
DUCC SIMD type for the scalar type; it is separate from the opt-in
`vectorize=true` pass-planner flag.

## DUCC SHT call sites

`src/ducc0/sht/sht.cc` uses `pocketfft_r<double>` for longitude rings in
`ringhelper::phase2ring`, `get_dh_weights`, and grid-weight routines. The ring
helper reuses a plan and buffer when `nphi` is unchanged. CC resampling builds
`pocketfft_c<T>` plans for reflected-latitude lengths in the input, prepared,
and weighted full-resolution grids. It schedules order ranges with
`execDynamic` and a chunk size of 64; each scheduler worker owns its profile
and scratch. The 1D plans are therefore reused across independent rows or
Fourier orders.

## Licensing and source retention

The listed FFT, complex arithmetic, roots, SIMD, alignment, and threading
files declare `BSD-3-Clause OR GPL-2.0-or-later`. The initial port will use the
BSD-3-Clause option. Any adapted code will preserve the applicable source
copyright and SPDX notices at file level, identify the pinned commit, and be
listed in `LICENSE-THIRD-PARTY`, which includes the full BSD conditions and
disclaimer. The C++ reference harness compiles the pinned source from an
external checkout and is not a runtime dependency.

## M1 Rust implementation status

`src/complex.rs` uses the complex factor order from `cfftpass::factorize()` in
`fft.h`. It executes those factors with an independent recursive
decimation-in-time schedule and plan-owned roots. Radices 2, 3, 4, 5, 7, 8,
and 11 use fixed butterflies corresponding to the pinned DUCC kernels. Other
radix leaves currently use a direct O(p^2) DFT. This fallback is temporary:
the DUCC `cfftpg` path for primes below 110 and `cfftpblue` path for larger
primes have not been ported. The current engine does not include
`cfftp_vecpass` or DUCC's nested scheduling.

The root table applies `RootCalc`'s octant reflection and evaluates f32/f64
roots with double-precision trigonometry; f32 stores the rounded f32 values.
It retains a full root table rather than DUCC's two-table reconstruction.
Plans retain that table and each workspace retains an output array and two
small-radix arrays. Execution performs no heap allocation. The inverse
distributes `1/n` across radix stages. It pre-scales a stage only when its
input magnitude approaches an overflow bound; otherwise it scales the
butterfly output. This avoids both avoidable overflow on large inputs and
underflow of small subnormal inputs. A correction transform recovers terms
lost while pre-scaling a group with a high dynamic range; a regression test
covers both f32 and f64. The independent direct-DFT tests cover lengths 1
through 64 in both precisions and compare inverse transforms against
independently supplied spectra. `benches/reference/compare.py` compares native
f64 C2C outputs against direct DUCC outputs at the benchmark lengths.

The `bench-native` output records plan construction, workspace allocation,
and the complete public C2C operation. It does not expose a separate
unnormalized butterfly-only measurement. Compare `public_operation_ns` with
other libraries' whole-call measurements; do not compare it with an
unnormalized kernel-only timing.
