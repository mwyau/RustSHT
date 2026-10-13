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

## Batch of 32 transforms

The batch measurement calls the public `forward_batch` or `inverse_batch`
method for 32 distinct rows and reports latency per transform. DUCCFFT
validates the flat row-major buffers once, then executes the rows serially
with one reusable workspace. The RustFFT control reuses one adapter plan and
loops over the same rows. Input, spectrum, output, and plan allocations are
outside timing. A timed sample repeats
`max(floor(262144 / n / 32), 1)` batch calls. The comparison checks all 32
outputs after timing with the same relative-L2 and L2-scaled maximum-error
limits as the single-transform benchmark.

The batch extension is source commit `0cbf36d` (SHA-256
`76ea65520ac18ab604087c9011c81f61b5f54b597dbfb3b45ad7cf9320c2b665`). The
baseline was built from `fc3c876` with that exact harness source copied into
its detached worktree; the candidate executable was built from `0cbf36d`,
whose FFT implementation is unchanged from `2ef4906`. Both builds used the
same `Cargo.lock` hash and compiler configuration recorded above. The paired
run's one-minute load average was 2.08, with 5- and 15-minute averages of 2.33
and 2.87. The host was still shared.

Raw batch data contain 588 records per executable:
[`baseline batch samples`](../benches/results/m4-f32-real-avx2-batch-baseline-process-samples.csv)
and
[`candidate batch samples`](../benches/results/m4-f32-real-avx2-batch-candidate-process-samples.csv).
The keys match across versions. These rows measure the AVX2 production change
inside the public batch methods; the separate comparison of the serial batch
API against a loop of single DUCCFFT calls is in
[`m4-batch-api.md`](m4-batch-api.md).

Each table time below is the median latency in microseconds per transform
across 42 paired samples. The ratio and paired-gain definitions match the
single-transform table above.

| N | Operation | DUCCFFT baseline → candidate | RustFFT baseline → candidate | DUCCFFT/RustFFT baseline → candidate | Paired gain | Favored pairs |
|---:|---|---:|---:|---:|---:|---:|
| 64 | R2C batch 32 | 0.167 → 0.152 | 0.115 → 0.115 | 1.453× → 1.315× | +9.08% | 42/42 |
| 64 | C2R batch 32 | 0.154 → 0.143 | 0.075 → 0.075 | 2.062× → 1.906× | +7.25% | 42/42 |
| 128 | R2C batch 32 | 0.390 → 0.362 | 0.219 → 0.220 | 1.780× → 1.647× | +7.38% | 41/42 |
| 128 | C2R batch 32 | 0.380 → 0.355 | 0.146 → 0.152 | 2.612× → 2.344× | +10.36% | 38/42 |
| 256 | R2C batch 32 | 0.752 → 0.605 | 0.438 → 0.438 | 1.716× → 1.384× | +19.55% | 42/42 |
| 256 | C2R batch 32 | 0.716 → 0.579 | 0.293 → 0.304 | 2.447× → 1.905× | +21.26% | 42/42 |
| 512 | R2C batch 32 | 1.767 → 1.507 | 0.897 → 0.912 | 1.968× → 1.655× | +15.99% | 42/42 |
| 512 | C2R batch 32 | 1.712 → 1.465 | 0.601 → 0.635 | 2.844× → 2.310× | +18.64% | 42/42 |
| 1,024 | R2C batch 32 | 3.487 → 2.529 | 1.803 → 1.841 | 1.941× → 1.367× | +29.06% | 42/42 |
| 1,024 | C2R batch 32 | 3.324 → 2.396 | 1.246 → 1.317 | 2.663× → 1.817× | +31.92% | 42/42 |
| 2,048 | R2C batch 32 | 8.111 → 6.263 | 3.761 → 3.796 | 2.152× → 1.650× | +23.09% | 42/42 |
| 2,048 | C2R batch 32 | 7.890 → 6.032 | 2.604 → 2.732 | 3.023× → 2.204× | +27.50% | 42/42 |
| 4,096 | R2C batch 32 | 16.465 → 10.909 | 7.695 → 7.788 | 2.138× → 1.401× | +34.57% | 42/42 |
| 4,096 | C2R batch 32 | 15.981 → 10.612 | 5.468 → 5.653 | 2.926× → 1.886× | +35.59% | 42/42 |

At N=128, 41 of 42 R2C pairs and 38 of 42 C2R pairs favor the candidate; the
per-process median gain ranges include a −0.02% C2R result, so treat the
smallest sizes cautiously. All pairs favor the candidate for both operations
at lengths 256–4,096. These results show that the single-transform AVX2 gains
carry into a 32-row call. They do not measure across-row SIMD, which remains
unimplemented.

## Plan and workspace setup

The table below uses the single-transform sweep's six processes. Each cell
shows the median of the six process-level medians in microseconds; each process
median uses 20 plan constructions or workspace allocations. RustFFT setup
includes its planner and the adapter's transform, scratch, and twiddle arrays.
The batch sweep also records setup values in its raw CSVs, but reuses the plan
and workspace for each timed batch call.

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

## Callgrind profile at N=4,096

Two Callgrind process profiles per source used the same benchmark source,
binary settings, length, and CPU affinity as the timing run. Valgrind 3.26.0
ran the executables pinned to CPU 30. Each process measured all seven paired
R2C and C2R samples, performed the output comparisons, and included setup;
these are whole-process instruction counts, not isolated operation counts or
wall times. The two whole-process totals were 529,986,594 and 529,989,033
instruction references for the baseline, and 387,447,434 and 387,446,758 for
the candidate. Their medians differ by 26.90% in favor of the candidate.

The following function counts are inclusive and therefore overlap. Each
listed function had the same count in both profiles of its source.

| Function | Baseline instruction references | Candidate instruction references | Reduction |
|---|---:|---:|---:|
| `RealFftPlan<f32>::forward` | 189,200,967 | 117,754,740 | 37.76% |
| `RealFftPlan<f32>::inverse` | 177,679,627 | 106,580,477 | 40.02% |
| `execute_forward_pass::<f32>` | 144,053,119 | 72,606,892 | 49.60% |
| `execute_inverse_pass::<f32>` | 128,614,254 | 57,515,104 | 55.28% |

The candidate's `forward_radix4_interior` and `inverse_radix4_interior`
functions account for 14,143,949 and 12,495,670 references, respectively.
Together they are 6.88% of the candidate whole-process count. The pass
executors still account for substantial inclusive work after this kernel;
this profile does not separate scalar endpoints and tails from vector-loop
cost inside those executors. It supports a pass-level follow-up and does not
assign the full whole-call gain to SIMD instructions.

Compressed profiles are archived for both runs:
[`baseline`](../benches/results/m4-f32-real-avx2-baseline-callgrind.out.gz),
[`baseline repeat`](../benches/results/m4-f32-real-avx2-baseline-repeat-callgrind.out.gz),
[`candidate`](../benches/results/m4-f32-real-avx2-candidate-callgrind.out.gz),
and
[`candidate repeat`](../benches/results/m4-f32-real-avx2-candidate-repeat-callgrind.out.gz).
