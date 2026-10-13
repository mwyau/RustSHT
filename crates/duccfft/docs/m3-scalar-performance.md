# M3 scalar performance checkpoint

Date: 2026-10-13. The implementation measured here is source commit
`0fa6b54` (`feat/duccfft`). Benchmark metadata and refreshed results are in
commit `d613dd6`. The pinned DUCC reference is
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

M3 adds the complex and real general-prime passes, Bluestein plans, the
DUCC-conditioned real complexify plan, a complex multipass executor, and a
finite-input inverse-normalization fast path. Correctness checks pass, but
smooth real transforms and several C2C cases still have multi-fold timing
gaps. This checkpoint records those gaps before SIMD work.

## Measurement method

The host is an AMD Ryzen 9 5950X. Timed processes were pinned to logical CPU
30, used one execution thread, warmed reusable plans, and reported the median
of seven samples. Each table row combines two process-level medians. The host
also ran the Legendre agent's workload, so all timings are provisional. Inputs
are deterministic trigonometric sequences from the benchmark harness. Inverse
real tests use independently supplied spectra. Batch timing measures 32 rows;
`*_batch_ns` stores total time for the batch, while `*_public_ns` stores time
per transform.

Rust uses 1.99.0 (`b940084d7`, LLVM 23.1.1), Cargo 1.99.0, the default
optimized `bench` profile, no custom `RUSTFLAGS`, no `target-cpu`, and no LTO.
RustFFT 6.4.1 uses its default runtime planner in the main comparison. A
separate `FftPlannerScalar` run records a scalar RustFFT control. The current
smooth table includes both. Default RustFFT may select CPU-specific kernels at
runtime, so its results describe the library's default behavior on this CPU,
not a scalar-only comparison.

DUCC uses g++ 15.2.0 with two separate builds:

- Portable target: `-std=c++20 -O3 -DNDEBUG -pthread`.
- Native target: `-std=c++20 -O3 -DNDEBUG -march=native -pthread`.

Both timing builds pass `vectorize=false` to DUCC. The native-target column
therefore measures the scalar DUCC planner compiled for the host ISA. Separate
numerical comparisons also ran a native DUCC build with `vectorize=true`; those
results are recorded in `m3-prime-bluestein.md`, not mixed into the timing
table. DUCC's real measurements include conversion to or from the public
half-spectrum. Its kernel scratch and real-adapter working storage are
reported separately.

`benches/results/m3-smooth-performance.csv` has the per-run single and batch
timings, plan construction, workspace allocation, scratch bytes, packing,
kernel phases, and ratios for every required smooth length and operation.
`m3-rustfft-scalar.csv` records the scalar RustFFT control. The full prime and
Bluestein table is in `m3-prime-bluestein.csv`. All tables separate C2C
forward, C2C inverse, R2C, and C2R.

## Smooth-radix timings

Each timing cell gives `DUCCFFT / RustFFT default / DUCC portable / DUCC
native` in microseconds. The parenthesized ratios give `DUCCFFT / RustFFT`,
`DUCCFFT / DUCC portable`, and `DUCCFFT / DUCC native` in that order. Values
are whole-public-operation timings, including inverse normalization and real
packing or unpacking.

| N | C2C forward, µs (ratios) | C2C inverse, µs (ratios) |
| ---: | --- | --- |
| 64 | 0.336 / 0.075 / 0.160 / 0.160 (4.44 / 2.10 / 2.10) | 0.411 / 0.090 / 0.166 / 0.150 (4.57 / 2.48 / 2.73) |
| 128 | 0.732 / 0.125 / 0.336 / 0.300 (5.85 / 2.18 / 2.43) | 0.937 / 0.160 / 0.391 / 0.311 (5.86 / 2.40 / 3.01) |
| 256 | 1.528 / 0.345 / 0.651 / 0.566 (4.42 / 2.35 / 2.70) | 1.889 / 0.411 / 0.751 / 0.586 (4.60 / 2.51 / 3.22) |
| 512 | 3.267 / 0.631 / 1.438 / 1.413 (5.18 / 2.27 / 2.31) | 3.978 / 0.791 / 1.618 / 1.448 (5.03 / 2.46 / 2.75) |
| 1,024 | 7.820 / 1.493 / 3.401 / 3.046 (5.24 / 2.30 / 2.57) | 9.398 / 1.728 / 3.822 / 3.256 (5.44 / 2.46 / 2.89) |
| 2,048 | 17.814 / 3.662 / 8.511 / 8.110 (4.86 / 2.09 / 2.20) | 20.759 / 4.143 / 8.796 / 7.915 (5.01 / 2.36 / 2.62) |
| 4,096 | 40.031 / 8.371 / 21.029 / 20.574 (4.78 / 1.90 / 1.95) | 46.051 / 9.523 / 21.285 / 20.163 (4.84 / 2.16 / 2.28) |

| N | R2C, µs (ratios) | C2R, µs (ratios) |
| ---: | --- | --- |
| 64 | 1.458 / 0.141 / 0.141 / 0.131 (10.34 / 10.38 / 11.17) | 1.377 / 0.111 / 0.140 / 0.146 (12.47 / 9.84 / 9.43) |
| 128 | 3.722 / 0.261 / 0.251 / 0.251 (14.26 / 14.86 / 14.83) | 3.272 / 0.205 / 0.281 / 0.246 (15.92 / 11.64 / 13.30) |
| 256 | 7.104 / 0.501 / 0.511 / 0.506 (14.18 / 13.90 / 14.04) | 6.162 / 0.391 / 0.511 / 0.486 (15.76 / 12.06 / 12.68) |
| 512 | 17.468 / 1.012 / 1.072 / 1.127 (17.26 / 16.29 / 15.50) | 14.432 / 0.817 / 1.157 / 0.972 (17.68 / 12.47 / 14.86) |
| 1,024 | 38.758 / 2.149 / 2.220 / 2.514 (18.04 / 17.46 / 15.41) | 27.362 / 1.853 / 2.244 / 2.029 (14.76 / 12.19 / 13.49) |
| 2,048 | 80.321 / 4.439 / 4.864 / 5.295 (18.10 / 16.51 / 15.17) | 63.480 / 3.637 / 5.285 / 4.559 (17.45 / 12.01 / 13.92) |
| 4,096 | 155.719 / 9.498 / 11.326 / 12.092 (16.40 / 13.75 / 12.88) | 122.245 / 7.809 / 11.687 / 10.600 (15.65 / 10.46 / 11.53) |

The scalar RustFFT control narrows the C2C ratio: at lengths 64–4,096,
DUCCFFT is 1.5–2.0× slower than scalar RustFFT. The real ratio remains
8.3–10.7× for R2C and 7.6–9.1× for C2R. The real execution structure is a
larger remaining scalar gap than fixed-radix C2C.

## General-prime and Bluestein results

The planner keeps DUCC's separate thresholds: complex factors below 110 use
`cfftpg`, complex factors from 110 use `cfftpblue`, real factors below 135 use
`rfftpg`, and real factors from 135 use `rfftpblue`. This gives separate paths
for 113, 127, and 131. Bluestein convolution lengths are 275 for 137, 484 for
241, 525 for 257, 1,372 for 683, and 2,025 for 1,013. The measured SHT sizes
use no Bluestein at 1,025 = 5² × 41, use M=1,372 at 2,049 = 3 × 683, and use
M=484 at 4,097 = 17 × 241. Planner selections and all operation timings are
listed in `m3-prime-bluestein.md` and `m3-prime-bluestein.csv`.

Selected whole-call timings are below. Each cell gives DUCCFFT / RustFFT /
DUCC portable / DUCC native in microseconds.

| Length and operation | Timing, µs |
| --- | --- |
| C2C forward, 683 | 32.01 / 10.94 / 15.05 / 11.09 |
| C2C forward, 1,013 | 59.16 / 12.92 / 22.71 / 11.34 |
| R2C, 1,025 | 32.74 / 6.02 / 4.85 / 3.56 |
| C2R, 1,025 | 49.08 / 6.07 / 4.58 / 3.25 |
| R2C, 2,049 | 117.82 / 34.66 / 48.51 / 37.52 |
| C2R, 2,049 | 115.52 / 34.80 / 49.51 / 37.67 |
| R2C, 4,097 | 237.11 / 56.98 / 98.36 / 69.57 |
| C2R, 4,097 | 245.64 / 67.53 / 100.51 / 70.64 |

The prime table records plan construction and reusable workspace allocation
separately. For example, the C2C length-1,013 plan takes about 83.5 µs to
construct, its workspace takes 2.1 µs, and it uses 113,584 scratch bytes.
At length 2,049 the C2C plan takes about 70.3 µs to construct and uses 98,768
scratch bytes. Direct DUCC scratch excludes adapter storage in its own column;
the real adapter's working bytes are recorded separately.

The Bluestein phase prototype measures chirp packing, forward convolution FFT,
spectral multiplication, inverse convolution FFT, and final chirp. At p=1,013
the two convolution FFTs take about 52.6 µs of a 55.2 µs prototype operation.
This is a phase prototype, not a production public-call measurement. It shows
that convolution FFT execution is the next phase to optimize within
Bluestein.

## Execution structure and generated code

The pre-M3 C2C implementation recursively transformed strided subsequences
and combined each level into workspace. Its pinned factorization is
`[2, 8, 8, 8]` at N=1,024 and `[8, 8, 8, 8]` at N=4,096. Counting the root,
internal calls, and leaves gives 1,171 recursive calls at 1,024 and 4,681 at
4,096. The multipass executor visits one pass descriptor per radix stage:
four passes at both lengths. Pass descriptors carry `l1`, `ido`, radix,
and stage-local twiddles. Their input and output indices follow DUCC's
`cfft_multipass` `l1`/`ido` arrangement, with contiguous `ido` spans and
twiddles applied after each fixed butterfly.

At N=1,024, moving from the pre-M3 recursive C2C code to the multipass code
reduced forward time from 19.672 to 7.820 µs and inverse time from 23.960 to
9.398 µs, about 2.5× in both directions. At N=4,096 the measured gains were
1.75× forward and 1.89× inverse. The comparison changes recursion, pass
indexing, and buffer movement together; it does not isolate each cause.

The first multipass is in-place because its `l1=1` input and output locations
coincide after gathering the radix values. Later passes alternate between the
caller array and workspace output. The final result is copied back only when
it ends in workspace. A temporary all-out-of-place first-pass comparison was
neutral at N=1,024 and slower by about 13–17% at N=2,048 and 4,096. A separate
copy profile measured 0.12 µs at N=1,024 and 0.91 µs at N=4,096; these are
about 1.5% and 2.3% of C2C forward time. The measured schedule therefore
keeps the in-place first pass.

Stage-local twiddles avoid root-index modulus in the C2C butterflies. The
plan stores these twiddles as well as the full root table. The `ido == 1`
pass skips the runtime complex multiply because its stage roots are unity.
Two-process measurements show a small or neutral timing change from this
guard, so it is retained for the exact arithmetic simplification and match to
DUCC's no-twiddle loop.

The release `x86_64-unknown-linux-gnu` binary was inspected with `nm` and
`objdump`. The f64 pass code contains SSE2 `addpd`, `subpd`, and `mulpd` as
well as scalar `mulsd`; it has no AVX instructions. These packed operations
mostly handle the real and imaginary pair of a complex value rather than
independent butterflies. The large pass symbol also contains cold generic
radix branches and bounds-check failure blocks. The inspection did not show
that those branches execute in the smooth cases, and it did not establish a
register-spill cost. RustFFT's default planner has runtime SIMD kernels; the
Rust scalar control does not.

The real executor still uses a recursive pass tree. For N=1,024 its factor
tree has the same 1,365 recursion nodes as the four-radix C2C tree. Callgrind
with debug line information measured 44.0 million instruction references for
100 normal C2R operations at N=1,024. Recursive execution accounts for 41.1
million inclusive references (93.4%); `small_radix_dft_all` accounts for 7.20
million (16.4%). The root multiplication and output-scale expression accounts
for 2.46 million references (5.6%). These inclusive costs overlap.

For high-range spectra made from `MAX/8` components, Callgrind measured 57.9
million instruction references, 31.6% above the normal case. The recursive
function accounts for 55.1 million inclusive references (95.0%), and the
small-radix kernel for 8.68 million (15.0%). Source-line counts show the
additional prescale, residual construction, correction transforms, and root
products inside the recursive function. These operations are compiled into
the same function, so Callgrind does not assign each a separate wall-time
share. Across two uninstrumented process runs, the high-range C2R input was
14.8%, 7.1%, and 12.2% slower than the normal input at lengths 1,024, 2,048,
and 4,096, respectively.

## Inverse residual correction

For finite inverse inputs with maximum component at or below
`MAX/(4*N)`, complex and real plans defer `1/N` and use the ordinary
butterfly path. This preserves headroom for the reviewed mixed-radix sums and
root products and skips residual construction in ordinary inputs. Inputs
outside the bound, including nonfinite inputs, use conditional stage
prescaling and residual correction. The bound is conservative and is not a
claim of bitwise equivalence to scaling each stage.

The historical 1,025 result was 79.2 µs R2C and 221.3 µs C2R; the current
result is 32.7 and 49.1 µs. At 2,049, the earlier 1.91 ms and 6.77 ms results
are now 117.8 and 115.5 µs. The 2,049 gain mostly comes from replacing the
quadratic 683-point combine with Bluestein. The 1,025 C2R/R2C ratio narrowed
from 2.8× to 1.5× after both the inverse fast path and M3 general-prime work.
At smooth length 1,024, C2R is faster than R2C (27.4 versus 38.8 µs). These
whole-operation measurements do not assign the historical gains to a single
change.

The inverse tests cover normal random inputs, cancellation, mixed dynamic
range, maximum finite values, minimum normal and subnormal values, direct
inverse DFTs, pinned DUCC comparisons, and f32/f64. The existing extreme-value
regressions pass. No residual correction is performed for eligible ordinary
finite values; the fallback preserves the existing high-range objective.

## f32 error summary

The long f32 metric tests compare f32 outputs with independent f64 direct-DFT
results for inputs first rounded to f32. `scaled max` is maximum absolute
error divided by `max(maximum reference magnitude, 1)`. The measured cases
contain no nonfinite outputs.

| Operation | Lengths | Maximum absolute error | Maximum RMS error | Maximum relative L2 error | Maximum scaled max error | Nonfinite outputs |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| C2C forward | 1,024, 127, 137, 4,097 | 1.537e-4 | 1.484e-5 | 2.297e-7 | 1.164e-7 | 0 |
| C2C inverse | 1,024, 127, 137, 4,097 | 8.453e-8 | 1.871e-8 | 2.275e-7 | 8.453e-8 | 0 |
| R2C | 1,024, 137, 1,025, 4,097 | 1.069e-4 | 1.037e-5 | 2.246e-7 | 6.906e-8 | 0 |
| C2R | 1,024, 137, 1,025, 4,097 | 4.984e-8 | 9.028e-9 | 1.828e-7 | 4.984e-8 | 0 |

The f32 direct-DFT tests also cover lengths 1–64. General-prime direct tests
cover f32 complex length 137 and real length 137; the real complexify planner
test covers length 1,026 in f32 and f64. The f64 threshold probes cover all
listed prime cases. The test subsets are documented separately in
`m3-prime-bluestein.md`.

## Compiler configuration probes

Native CPU targeting and LTO were tested separately at lengths 1,024 and
4,096, with two process runs per configuration. `m3-compiler-config-probes.csv`
records DUCCFFT and RustFFT times for both batch counts and setup costs.

`-C target-cpu=native` improved C2R by about 16% at both sizes. It changed
most other DUCCFFT timings by less than 12% and made 4,096 C2C about 4–7%
slower in these samples. Enabling LTO with one codegen unit changed the
measured DUCCFFT timings by at most about 15% at 1,024 and by at most 3% at
4,096; the direction varied by operation. Neither build setting closes the
smooth real gap. These two-process measurements are provisional and are not
combined with the portable timing table.

## Architecture decision and remaining work

The recursive C2C engine was replaced with the DUCC `l1`/`ido` multipass
schedule. The buffer experiment supports the in-place first pass, and the
copyback profile shows that final copying is a small share of total time.
This structure is simpler to map to DUCC's vector-friendly pass dimensions
than the previous recursive tree. C2C scalar execution is still 4.4–5.9×
slower than RustFFT's default planner and 1.9–2.5× slower than portable DUCC
over the measured smooth lengths.

The real pass tree remains recursive and is 10–18× slower than RustFFT's
default planner at these smooth lengths. A complete forced-complexify
prototype over the current complex multipass plan ran 5.4–9.4× faster for R2C
and 5.4–6.5× faster for C2R at lengths 512–4,096. This prototype omits the
production extreme-value path and does not justify changing DUCC's planner
thresholds. It supports a targeted real multipass experiment before real SIMD
work.

The next scalar work should prototype DUCC's real pass loop structure on a
representative smooth size and compare it with the recursive real path and the
bounded complexify result. C2C can proceed to M4 SIMD and batched execution
using the measured multipass structure. The real kernel should enter SIMD
work after the scalar schedule comparison. General-prime kernels also use
less complementary-output reuse than DUCC, and the large-length balanced
packet grouping used by DUCC above 10,000 is not implemented.

## Reviewer conclusions

The read-only R3A review found no new numerical defect. It confirmed that C2C
pass indexing and buffer ownership match the pinned DUCC scalar multipass
layout, and that the in-place first pass is supported by the buffer schedule
data. The reviewer identified the real multipass gap as the next scalar
priority. The review also confirmed that default RustFFT runtime SIMD,
portable DUCC, and native-target DUCC require separate labels and that the
shared-host results remain provisional.

The review caught and this branch fixed an overstatement of f32 prime-test
coverage in `m3-prime-bluestein.md`. The R3B review of this checkpoint is
pending.
