# M3 prime and Bluestein checkpoint

Reference revision: `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

## Planner paths

The complex planner keeps DUCC's threshold: odd prime factors below 110 use
the paired `cfftpg`-style combine; factors from 110 use Bluestein. The real
planner keeps its threshold of 135 for `rfftpg` and `rfftpblue`. The real
planner also implements DUCC's scalar `rfftp_complexify` condition for even
lengths above 1,000 when the length is above 10,000 or factorization contains
a factor greater than five. The separate DUCC selection condition that
requires vectorization is not active in this scalar plan.

| Length or factor | Complex path | Real path | Bluestein convolution length |
| --- | --- | --- | ---: |
| 13, 17, 31, 41, 53, 107, 109 | `cfftpg` | `rfftpg` | — |
| 113, 127, 131 | Bluestein | `rfftpg` | 225, 256, 264 |
| 137 | Bluestein | Bluestein | 275 |
| 241 | Bluestein | Bluestein | 484 |
| 257 | Bluestein | Bluestein | 525 |
| 683 | Bluestein | Bluestein | 1,372 |
| 1,013 | Bluestein | Bluestein | 2,025 |
| 1,025 = 5² × 41 | General-radix stages | General-radix stages | — |
| 2,049 = 3 × 683 | General radix and Bluestein at 683 | General radix and Bluestein at 683 | 1,372 |
| 4,097 = 17 × 241 | General radix and Bluestein at 241 | General radix and Bluestein at 241 | 484 |

Bluestein plans precompute chirps and the convolution kernel spectrum. They
reuse a nested complex plan and caller-owned convolution scratch during
execution. DUCC stores its kernel spectrum scaled by `1/M`. Rust stores the
unscaled spectrum for ordinary amplitudes and conditionally applies scaling
for overflow or underflow headroom. The real Bluestein pass uses the common
complex Bluestein plan and reconstructs the child frequencies required by the
public half-spectrum.

## Correctness evidence

The f64 direct O(N²) tests check forward and inverse transforms for complex
lengths 107, 109, 113, 137, and 241 and real lengths 127, 131, 137, and 241.
The f32 direct-DFT tests cover complex length 137, real lengths 137 and 1,026,
and larger metric cases. f64 tests compare nonterminal Bluestein stages at
lengths 113² and 137² against RustFFT. f32 and f64 regression tests cover SHT
longitude lengths 1,025, 2,049, and 4,097, near-maximum finite values, minimum
subnormals, cancellation, and independently supplied inverse spectra. The
tests reject nonfinite outputs.

The pinned DUCC comparisons checked 202,347 output records (404,694 scalar
components) over f64 C2C, R2C, and C2R lengths through 10,008. With DUCC
vectorization disabled, the maximum absolute error was `1.819e-12` and the
maximum scaled error was `6.821e-13`. With the native build and DUCC
vectorization enabled, those values were `9.095e-13` and `6.821e-13`.
RustFFT maximum absolute error was `3.297e-12`. The current run outputs are
`/tmp/m3-final-compare-portable.txt` and
`/tmp/m3-final-compare-native-vectorized.txt` in the measurement environment.

## Performance records

`benches/results/m3-prime-bluestein.csv` is the per-operation table. For each
tested length it records C2C forward and inverse, R2C, C2R, one-transform and
32-transform timings, plan construction, workspace allocation, scratch
bytes, kernel and packing phases, whole-call times, and RustSHT-FFT/RustFFT/DUCC
ratios. Direct DUCC scratch is labeled `kernel_scratch_bytes`. Its real adapter
also allocates one packed working row of `N` doubles per batch row; that size
is recorded separately as `adapter_working_bytes`.

`benches/results/m3-bluestein-phases.csv` splits a benchmark-only complex
Bluestein wrapper into chirp packing, forward convolution FFT, spectral
product, inverse convolution FFT, and final chirp. Its
`prototype_operation_ns` field measures that instrumented wrapper, not
`ComplexFftPlan::forward`. In the current two pinned runs, the convolution FFT
phases dominate the prototype for all five profiled lengths. This establishes
where to profile next; it is not a production public-call comparison.

Measurements use warm plans, seven samples per process, two process runs, and
one execution thread pinned to logical CPU 30 on an AMD Ryzen 9 5950X. The
host was shared with the Legendre workload, so timings are provisional. Exact
compiler flags, ratios, plan and workspace data, and the smooth-length
checkpoint are in `m3-scalar-performance.md`.
