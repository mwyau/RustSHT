# f32 real radix-four AVX2 kernels

## Change

The f32 real multipass executor uses AVX2 interiors for its forward and
inverse radix-four passes. Each loop handles four packed complex values with
256-bit vectors. The pass retains the existing `l1`/`ido` ordering, stage
twiddles, packed spectrum, and scalar tail. Runtime AVX2 detection selects the
kernels; other targets and CPUs retain the scalar pass. The kernels add no
execution scratch or allocation.

The equations follow the f64 interior kernels and DUCC's `rfftp4` pass in
`fft1d_impl.h` at
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The pass layout is described in
[`m4-real-multipass.md`](m4-real-multipass.md).

## Benchmark method

The public f32 R2C and C2R operations were compared with RustFFT 6.4.1 and a
RustSHT-style half-length complex adapter. The baseline production source is
`fc3c876bcf4a46c7f9eb04e07a5dbc6b5d594c01`; the AVX2 candidate production
source is `2ef4906c3a4c19fdad78cd2f071e6760ff145e92`. The candidate executable
was built at `a31dc46`, whose only change from the candidate production commit
is the benchmark adapter source. Both executables used
the exact benchmark source from `a31dc46` (SHA-256
`576e3d98c229eed581bafa8f0ade0f29d152084e88e7b7362486033add888ead`). That
file was copied into a detached baseline worktree. Both builds resolved the
same `Cargo.lock` (SHA-256
`e61d5628d09dc7c10e54e15cb5f3aeb5dd5c28392b44692ed153784ad8ca21ba`).

Each length and direction has six process samples and seven paired timing
samples per process. The RustFFT-first and DUCCFFT-first orders alternate
within each process; baseline and candidate processes were interleaved in
opposite order. The harness warms both plans and reuses workspaces. Each
timed operation includes public validation, real packing or unpacking, FFT
execution, and normalization. Input, spectrum, and output allocation are
outside the timer. Each sample repeats `max(262144 / n, 16)` transforms.
Plan and workspace setup medians use 20 constructions per process.

Runs were pinned to logical CPU 30 on an AMD Ryzen 9 5950X. Rust was
`rustc 1.99.0 (b940084d7, LLVM 23.1.1)` with Cargo 1.99.0 and the default
`bench` profile (`opt-level=3`). `RUSTFLAGS`, LTO, and `target-cpu` were
unset. AVX2 runtime dispatch was enabled. The paired run's one-minute load
average ranged from 3.47 to 3.60; the shared host was active, so absolute
times remain provisional. The candidate and baseline use the same RustFFT
adapter and compiler settings. This f32 experiment does not include a direct
C++ DUCC timing; the pinned DUCC comparisons for the f64 real path are in
[`m4-real-multipass.md`](m4-real-multipass.md).

Raw measurements contain 588 records per executable:
[`baseline samples`](../benches/results/m4-f32-real-avx2-baseline-process-samples.csv)
and
[`candidate samples`](../benches/results/m4-f32-real-avx2-candidate-process-samples.csv).
Their process, operation, length, and pair keys match. Checksums are stable
within f32 rounding. The harness compares each final DUCCFFT output with the
RustFFT adapter output: relative L2 error and
`max_abs_error / max(L2_norm(reference), 1)` must each be below `2e-5`.
This benchmark comparison metric differs from the independent DFT test
metric below.

## Whole-operation timings

Each timing is the median per-transform time across 42 pairs, in microseconds.
The ratio is the median of the 42 paired DUCCFFT/RustFFT ratios. Paired gain
is the median of `1 - candidate_ratio / baseline_ratio` for matching process
and pair keys. Positive gain favors the AVX2 candidate. `favored pairs` counts
the paired observations with a positive gain.

| N | Operation | DUCCFFT baseline → candidate | RustFFT baseline → candidate | DUCCFFT/RustFFT baseline → candidate | Paired gain | Favored pairs |
|---:|---|---:|---:|---:|---:|---:|
| 64 | R2C | 0.161 → 0.154 | 0.115 → 0.116 | 1.403× → 1.339× | +5.20% | 41/42 |
| 64 | C2R | 0.160 → 0.143 | 0.075 → 0.075 | 2.134× → 1.906× | +10.85% | 42/42 |
| 128 | R2C | 0.376 → 0.369 | 0.219 → 0.220 | 1.719× → 1.682× | +2.06% | 37/42 |
| 128 | C2R | 0.392 → 0.352 | 0.146 → 0.151 | 2.698× → 2.326× | +14.07% | 42/42 |
| 256 | R2C | 0.729 → 0.622 | 0.436 → 0.442 | 1.674× → 1.406× | +16.20% | 42/42 |
| 256 | C2R | 0.735 → 0.582 | 0.291 → 0.307 | 2.536× → 1.887× | +24.84% | 42/42 |
| 512 | R2C | 1.716 → 1.506 | 0.877 → 0.883 | 1.957× → 1.707× | +12.90% | 42/42 |
| 512 | C2R | 1.754 → 1.436 | 0.602 → 0.627 | 2.913× → 2.293× | +21.22% | 42/42 |
| 1,024 | R2C | 3.373 → 2.521 | 1.800 → 1.816 | 1.878× → 1.390× | +25.74% | 42/42 |
| 1,024 | C2R | 3.393 → 2.367 | 1.236 → 1.289 | 2.754× → 1.842× | +33.12% | 42/42 |
| 2,048 | R2C | 7.859 → 6.265 | 3.743 → 3.796 | 2.096× → 1.655× | +21.31% | 42/42 |
| 2,048 | C2R | 8.046 → 6.092 | 2.630 → 2.731 | 3.055× → 2.221× | +27.37% | 42/42 |
| 4,096 | R2C | 15.605 → 10.747 | 7.663 → 7.728 | 2.037× → 1.392× | +31.74% | 42/42 |
| 4,096 | C2R | 15.846 → 10.411 | 5.472 → 5.640 | 2.912× → 1.837× | +36.69% | 42/42 |

Per-process median gains are positive for each length and operation, ranging
from 1.43% to 39.24%. At N=128 R2C, only 37 of 42 paired gains are positive
and the median gain is 2.06%; treat that small change as weak evidence. All
42 paired gains favor the candidate for both operations at lengths 256
through 4,096. The larger-length results support keeping the targeted kernel;
they do not close the remaining scalar and SIMD gaps to RustFFT.

## Plan and workspace setup

These cells show the median of six process-level medians in microseconds.
Each process median uses 20 plan constructions or workspace allocations.
RustFFT setup includes its planner and the adapter's transform, scratch, and
twiddle arrays.

| N | DUCCFFT plan baseline → candidate | DUCCFFT workspace baseline → candidate | RustFFT adapter setup baseline → candidate |
|---:|---:|---:|---:|
| 64 | 1.327 → 1.208 | 0.130 → 0.121 | 1.062 → 0.942 |
| 128 | 2.129 → 2.134 | 0.120 → 0.121 | 1.683 → 1.603 |
| 256 | 4.153 → 4.148 | 0.131 → 0.131 | 3.146 → 3.115 |
| 512 | 8.150 → 8.140 | 0.180 → 0.180 | 6.006 → 6.082 |
| 1,024 | 16.250 → 16.166 | 0.210 → 0.220 | 12.258 → 12.264 |
| 2,048 | 32.171 → 32.120 | 0.341 → 0.341 | 25.613 → 25.729 |
| 4,096 | 63.861 → 63.846 | 0.556 → 0.571 | 51.277 → 51.383 |

## Correctness and generated code

The f32 real tests compare against independently evaluated f64 DFTs for
lengths 1–64 and lengths 1,024, 2,048, 4,096, 131, 137, 241, 1,025, 2,049,
and 4,097. They also exercise the real complexify planner path, cancellation,
dynamic range, maximum finite values, minimum normal and subnormal values,
and inverse normalization fallback. The reported independent-DFT metric
`scaled_max` is `max_abs_error / max(max_abs_reference, 1)`; the tests also
report maximum absolute error, RMS error, relative L2 error, and nonfinite
output count. The stated `scaled_max` metric is separate from the benchmark's
L2-scaled output-agreement gate above.

| N | Operation | Maximum absolute error | RMS error | Relative L2 error | DFT-scaled maximum error | Nonfinite |
|---:|---|---:|---:|---:|---:|---:|
| 1,024 | R2C | 1.98e−5 | 2.18e−6 | 9.44e−8 | 4.11e−8 | 0 |
| 1,024 | C2R | 2.56e−8 | 2.42e−9 | 1.06e−7 | 2.56e−8 | 0 |
| 2,048 | R2C | 6.34e−5 | 3.75e−6 | 1.15e−7 | 8.02e−8 | 0 |
| 2,048 | C2R | 1.80e−8 | 1.82e−9 | 1.12e−7 | 1.80e−8 | 0 |
| 4,096 | R2C | 6.80e−5 | 4.73e−6 | 1.03e−7 | 3.60e−8 | 0 |
| 4,096 | C2R | 1.99e−8 | 1.28e−9 | 1.11e−7 | 1.99e−8 | 0 |
| 131 | R2C | 4.55e−6 | 1.23e−6 | 1.50e−7 | 9.16e−8 | 0 |
| 131 | C2R | 7.19e−8 | 1.02e−8 | 1.65e−7 | 7.19e−8 | 0 |
| 137 | R2C | 4.40e−6 | 1.92e−6 | 2.28e−7 | 6.91e−8 | 0 |
| 137 | C2R | 3.13e−8 | 9.03e−9 | 1.46e−7 | 3.13e−8 | 0 |
| 241 | R2C | 1.10e−5 | 2.39e−6 | 2.13e−7 | 1.15e−7 | 0 |
| 241 | C2R | 2.89e−8 | 6.97e−9 | 1.52e−7 | 2.89e−8 | 0 |
| 1,025 | R2C | 2.59e−5 | 3.35e−6 | 1.45e−7 | 5.47e−8 | 0 |
| 1,025 | C2R | 4.98e−8 | 3.23e−9 | 1.41e−7 | 4.98e−8 | 0 |
| 2,049 | R2C | 4.40e−5 | 7.36e−6 | 2.25e−7 | 5.80e−8 | 0 |
| 2,049 | C2R | 7.74e−8 | 3.32e−9 | 2.05e−7 | 7.74e−8 | 0 |
| 4,097 | R2C | 1.07e−4 | 1.04e−5 | 2.25e−7 | 5.56e−8 | 0 |
| 4,097 | C2R | 3.45e−8 | 2.10e−9 | 1.83e−7 | 3.45e−8 | 0 |

Release disassembly of both interiors contains packed YMM `vmulps`, `vaddps`,
`vsubps`, `vblendps`, and shuffle instructions. The vector loops contain no
function calls or scalar bounds checks. The compiler keeps vector
intermediates in registers; no stack spills were present. Each vector loop
has one loop branch, with remaining elements handled by the scalar pass.
The reviewer independently checked vector bounds, mirror indices, twiddles,
tail coverage, dispatch, and the generated loops; it found no implementation
defect.

The f64 kernels remain unchanged. Other real radix families, across-row
SIMD, and the remaining RustFFT and DUCC gaps still require work. This
measurement supports one targeted f32 AVX2 pass family; it does not establish
performance completion.
