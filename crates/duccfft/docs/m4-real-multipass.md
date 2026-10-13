# Real multipass architecture experiment

## Scope and measurement method

This experiment compares the recursive real executor with a scalar packed-real
radix-2/radix-4 multipass prototype adapted from DUCC. The prototype uses
DUCC's real pass order, packed spectrum layout, and pass equations. It covers
even powers of two from 64 through 4,096 in f64. It is a benchmark prototype;
the production planner and executor were unchanged at source revision
`3c97509b` (`Measure reusable FFT workspace allocation`). The prototype checks every measured length against both the production
DUCCFFT real plan and the RustFFT real adapter in forward and inverse
directions. It checks finite outputs and a maximum-error bound scaled by
machine epsilon, transform length, pass count, and reference magnitude. The
sample input distribution matches the existing RustSHT-style real adapter. Each execution sample includes input copying, packed-spectrum conversion,
inverse normalization, and output handling. Plans and workspaces are reused. The sequential batch contains 32 independent transforms and reports latency
per transform. Outputs are observed after each timer. Each process uses one
warm-up and seven timed samples; the tables report medians across six
processes. Plan construction and workspace allocation use 20 repetitions. The machine was an x86-64 CPU. Processes were pinned to logical one CPU core. Rust used `rustc 1.99.0 (b940084d7, LLVM 23.1.1)`, Cargo's `bench` profile at
`opt-level=3`, no custom `RUSTFLAGS`, no LTO, and no `target-cpu` setting. RustFFT was 6.4.1 with runtime dispatch. Direct DUCC used revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, g++ 15.2.0, and scalar planner
selection. Its portable build used `-std=c++20 -O3 -DNDEBUG -pthread`; its
native build added `-march=native`. The DUCC vectorization planner option was
off in both builds. Host-load variation was reviewed locally; exact transient load measurements remain untracked. CPU affinity limits core
contention, but shared host activity remains possible; treat absolute times as
provisional. Raw Rust timings and plan/workspace measurements are in
[`m4-real-multipass-process-samples.csv`](../benches/results/m4-real-multipass-process-samples.csv). Direct DUCC control measurements are in
[`m4-real-multipass-ducc-control.csv`](../benches/results/m4-real-multipass-ducc-control.csv).

## Whole-operation timings

Each timing cell gives `packed-real prototype / recursive DUCCFFT / RustFFT
adapter / DUCC portable / DUCC native`, in microseconds per transform. Rust
and DUCC use the same deterministic real input and spectrum formulas. The
prototype and DUCCFFT/RustFFT numbers come from the same six Rust process runs;
DUCC control timings come from six separately pinned C++ process runs.

### One transform

| n | R2C | C2R |
|---:|---|---|
| 64 | 0.180 / 1.448 / 0.150 / 0.131 / 0.131 | 0.201 / 1.342 / 0.116 / 0.150 / 0.150 |
| 128 | 0.401 / 3.677 / 0.266 / 0.251 / 0.260 | 0.421 / 3.186 / 0.211 / 0.281 / 0.251 |
| 256 | 0.832 / 7.029 / 0.501 / 0.471 / 0.501 | 0.902 / 6.006 / 0.391 / 0.511 / 0.481 |
| 512 | 1.909 / 17.227 / 1.012 / 1.042 / 1.117 | 2.043 / 14.052 / 0.786 / 1.113 / 0.982 |
| 1,024 | 4.068 / 33.077 / 2.124 / 2.154 / 2.414 | 4.279 / 26.630 / 1.663 / 2.249 / 2.059 |
| 2,048 | 9.368 / 80.246 / 4.488 / 4.824 / 5.290 | 9.829 / 61.716 / 3.631 / 5.230 / 4.563 |
| 4,096 | 20.284 / 156.510 / 9.648 / 11.322 / 12.083 | 21.125 / 118.653 / 7.850 / 11.777 / 10.636 |

### Sequential batch of 32

| n | R2C | C2R |
|---:|---|---|
| 64 | 0.164 / 1.420 / 0.124 / 0.122 / 0.120 | 0.182 / 1.321 / 0.095 / 0.120 / 0.123 |
| 128 | 0.382 / 3.663 / 0.250 / 0.241 / 0.250 | 0.405 / 3.197 / 0.193 / 0.254 / 0.231 |
| 256 | 0.814 / 7.016 / 0.490 / 0.469 / 0.495 | 0.876 / 5.987 / 0.380 / 0.498 / 0.473 |
| 512 | 1.886 / 17.573 / 1.020 / 1.069 / 1.149 | 2.004 / 14.105 / 0.791 / 1.123 / 0.982 |
| 1,024 | 4.097 / 34.086 / 2.215 / 2.273 / 2.473 | 4.315 / 26.922 / 1.720 / 2.307 / 2.076 |
| 2,048 | 9.495 / 80.558 / 4.673 / 5.089 / 5.595 | 9.874 / 62.381 / 3.675 / 5.307 / 4.635 |
| 4,096 | 20.688 / 155.964 / 10.225 / 11.652 / 12.625 | 21.413 / 119.426 / 8.063 / 12.169 / 10.985 |

Across both call counts and all lengths, the prototype reduces R2C time by
7.54–9.59× and C2R time by 5.58–7.90× relative to the recursive executor. The prototype remains 1.20–2.10× slower than RustFFT for R2C and 1.74–2.71×
slower for C2R. It is 1.34–1.94× slower than portable DUCC for R2C and
1.34–1.90× slower for C2R. Against native-target DUCC, the gaps are 1.36–1.77×
for R2C and 1.34–2.15× for C2R. These measurements support a multipass
production path; they do not establish scalar performance completion.

## Plan and workspace costs

Values are medians for one-transform cases. Prototype plan-table bytes count
only copied scalar twiddle values; they exclude vector and allocator
metadata. DUCCFFT and RustFFT do not expose plan-table sizes. Scratch bytes
count reusable initialized workspace and exclude input, spectrum, output,
and allocator metadata.

| n | Implementation | Plan construction, µs | Workspace allocation, µs | Plan-table bytes | Scratch bytes |
|---:|---|---:|---:|---:|---:|
| 64 | Packed-real prototype | 0.551 | 0.084 | 432 | 1,024 |
| 64 | Recursive DUCCFFT | 1.018 | 0.099 | not exposed | 1,024 |
| 64 | RustFFT adapter | 1.085 | 0.019 | not exposed | 512 |
| 64 | DUCC portable | 0.358 | 0.013 | not exposed | 512 |
| 64 | DUCC native | 0.367 | 0.014 | not exposed | 512 |
| 1,024 | Packed-real prototype | 7.446 | 0.158 | 8,064 | 16,384 |
| 1,024 | Recursive DUCCFFT | 15.150 | 0.184 | not exposed | 11,392 |
| 1,024 | RustFFT adapter | 17.363 | 0.136 | not exposed | 16,384 |
| 1,024 | DUCC portable | 1.292 | 0.072 | not exposed | 8,192 |
| 1,024 | DUCC native | 1.175 | 0.071 | not exposed | 8,192 |
| 4,096 | Packed-real prototype | 29.823 | 0.546 | 32,616 | 65,536 |
| 4,096 | Recursive DUCCFFT | 60.365 | 0.465 | not exposed | 44,224 |
| 4,096 | RustFFT adapter | 61.051 | 0.505 | not exposed | 65,536 |
| 4,096 | DUCC portable | 3.347 | 0.311 | not exposed | 32,768 |
| 4,096 | DUCC native | 2.885 | 0.314 | not exposed | 32,768 |

The prototype halves plan construction time relative to the recursive DUCCFFT
plan at these examples, but its workspace needs about 1.5× as many bytes at
4,096. Plan construction remains substantially slower than direct DUCC.

## Architecture decision and limits

The bounded comparison supports DUCC's packed-real multipass structure over
recursive child transforms for smooth radix-2/radix-4 lengths. Production
migration retains the existing public API and numerical fallback. The
ordinary inverse path uses the multipass executor only when
`inverse_normalization_is_safe` accepts the spectrum. High-range or nonfinite
inputs continue through the residual-corrected recursive executor. The
reviewer confirmed that the inverse bound conservatively covers radix-2/4
intermediate growth and specifically checked the `N=64`, DC=Nyquist=`MAX`
case that overflows in the unscaled prototype. The prototype uses f64 and supports only powers of two. It does not implement
radix-3/5, general-prime, Bluestein, or DUCC's conditional real complexify
passes. It therefore does not replace the production fallback by itself. The
production path preserves the conditional complexify planner thresholds and
keeps unsupported factorizations on their validated path until their pass
implementations are available. The first production step now specializes
radix-2/radix-4 lengths; the remaining pass families still need to be ported
before removing recursion. The initial length-16 inverse check caught a negative radix-4 inverse twiddle
sign in the prototype. Correcting it to match pinned `rfftp4` made the
independent DUCCFFT and RustFFT comparisons pass for lengths 1 through 4,096. No timing sample was retained from the failing version.

## Production radix-2/radix-4 path

The scalar production path was added at source revision
`eb8085bf45c94d85c3ac083d436ca4586dd17b59`. It uses the
same packed passes as the reviewed prototype for plans containing only radix
2 and radix 4, while preserving DUCC's conditional complexify selection. It
uses stage twiddles derived from the plan's root table and two reusable
caller-owned real buffers. Other factorisations continue through the recursive
executor. Forward multipass eligibility requires finite inputs with maximum absolute
sample at most `MAX/(4*N)`. Larger or nonfinite inputs use the existing
recursive path. Inverse multipass eligibility uses the existing
`inverse_normalization_is_safe` check, which requires finite relevant spectrum
components and a maximum at most `MAX/(4*N)`; imaginary DC and even Nyquist
values remain ignored. Ineligible inverse inputs continue through the
residual-corrected executor. The guard bounds are conservative headroom checks;
they do not claim bitwise equivalence between pass orders. Tests cover f32 and
f64 through length 4,096, direct f64 references, cancellation, subnormals,
maximum-finite impulses, and the reviewed N=64 maximum-finite DC/Nyquist case. The integrated measurements use the Rust benchmark binary built from source
revision `de5f87f796722bfff1e83c7a620f91b5ab28c35d`. Six processes were pinned
to logical one CPU core. Each process reports a median of seven executions after one
warm-up; plan and workspace allocation medians use 20 repetitions. Host-load variation was reviewed locally; exact transient load measurements remain untracked. Rust used rustc 1.99.0, Cargo
1.99.0, `opt-level=3`, no custom `RUSTFLAGS`, no LTO, and no target CPU flag. RustFFT 6.4.1 used its default runtime dispatch. Direct DUCC controls are from
the six-process pinned runs described above, using the pinned reference and
portable and `-march=native` builds. Raw Rust samples are in
[`m4-real-production-process-samples.csv`](../benches/results/m4-real-production-process-samples.csv). Each cell gives `DUCCFFT production / RustFFT adapter / DUCC portable / DUCC
native`, in microseconds per transform.

### One transform

| N | R2C | C2R |
|---:|---|---|
| 64 | 0.245 / 0.146 / 0.131 / 0.131 | 0.260 / 0.111 / 0.150 / 0.150 |
| 128 | 0.491 / 0.270 / 0.251 / 0.260 | 0.516 / 0.206 / 0.281 / 0.251 |
| 256 | 1.012 / 0.511 / 0.471 / 0.501 | 1.082 / 0.396 / 0.511 / 0.481 |
| 512 | 2.234 / 1.012 / 1.042 / 1.117 | 2.385 / 0.787 / 1.113 / 0.982 |
| 1,024 | 4.729 / 2.124 / 2.154 / 2.414 | 5.065 / 1.708 / 2.250 / 2.059 |
| 2,048 | 10.680 / 4.479 / 4.824 / 5.290 | 11.291 / 3.617 / 5.230 / 4.563 |
| 4,096 | 23.099 / 9.638 / 11.322 / 12.083 | 24.767 / 7.850 / 11.777 / 10.636 |

### Sequential batch of 32

| N | R2C | C2R |
|---:|---|---|
| 64 | 0.209 / 0.123 / 0.122 / 0.120 | 0.220 / 0.094 / 0.120 / 0.123 |
| 128 | 0.471 / 0.251 / 0.241 / 0.250 | 0.495 / 0.189 / 0.254 / 0.231 |
| 256 | 0.987 / 0.491 / 0.469 / 0.495 | 1.050 / 0.378 / 0.498 / 0.473 |
| 512 | 2.224 / 1.015 / 1.069 / 1.149 | 2.376 / 0.784 / 1.123 / 0.982 |
| 1,024 | 4.714 / 2.202 / 2.273 / 2.473 | 5.057 / 1.738 / 2.307 / 2.076 |
| 2,048 | 10.661 / 4.682 / 5.089 / 5.595 | 11.332 / 3.676 / 5.307 / 4.635 |
| 4,096 | 23.555 / 10.266 / 11.652 / 12.625 | 24.987 / 8.119 / 12.169 / 10.985 |

Against the earlier recursive measurements at source revision `3c97509b`,
the production path is 5.90–7.90× faster for R2C and 4.78–6.46× faster for
C2R across the two call counts and tested lengths. In a paired run, the
production implementation is 1.13–1.36× slower than the no-guard prototype
for R2C and 1.15–1.30× slower for C2R. That difference includes the
production eligibility scan and dispatch. Production remains 1.69–2.40×
slower than RustFFT for R2C and 2.33–3.16× slower for C2R. Across single and
batch calls, it is 1.72–2.21× slower than portable DUCC for R2C and
1.73–2.25× slower for C2R. The inverse
timing is now close to forward timing for the same length, unlike the earlier
SHT prime-size results. These comparisons show that multipass removed most
of the recursive slowdown; they leave scalar kernel and dispatch overhead to
address before SIMD work is complete. Plan and workspace medians for selected lengths follow. Scratch counts
include the recursive fallback buffers and the two real multipass buffers in
production; all counts exclude allocator metadata and transform arguments.

| N | Plan construction, µs (prototype / production / RustFFT) | Workspace allocation, µs (prototype / production / RustFFT) | Scratch bytes (prototype / production / RustFFT) |
|---:|---:|---:|---:|
| 64 | 0.469 / 1.121 / 0.997 | 0.085 / 0.129 / 0.018 | 1,024 / 2,048 / 512 |
| 128 | 0.906 / 2.243 / 1.925 | 0.074 / 0.165 / 0.026 | 2,048 / 4,128 / 2,048 |
| 256 | 1.797 / 4.241 / 3.742 | 0.065 / 0.180 / 0.050 | 4,096 / 7,232 / 4,096 |
| 512 | 3.769 / 8.380 / 7.212 | 0.097 / 0.250 / 0.079 | 8,192 / 15,456 / 8,192 |
| 1,024 | 7.420 / 16.571 / 17.214 | 0.157 / 0.343 / 0.137 | 16,384 / 27,776 / 16,384 |
| 2,048 | 14.859 / 33.226 / 31.274 | 0.286 / 0.621 / 0.287 | 32,768 / 60,576 / 32,768 |
| 4,096 | 29.795 / 65.419 / 61.328 | 0.578 / 0.991 / 0.508 | 65,536 / 109,760 / 65,536 |

The production guard preserves the reviewed extreme-value behavior, but the
current staged migration duplicates scratch for the recursive fallback. At
4,096, production workspace scratch is 109,760 bytes compared with 65,536
bytes for the prototype and RustFFT adapter. Production plan construction is
also slower than the prototype because it retains the full root table and
builds pass-local twiddles. Porting the other pass families is necessary
before the recursive fallback and its storage can be removed.

## R5 review

The read-only reviewer found no pass-equation, twiddle-sign, pass-order,
packed-spectrum, or buffer-ownership defect in the corrected prototype. It
accepted the measurements as evidence for production migration, subject to
the inverse overflow guard and broader f32/factorization coverage above. The
review also corrected the speedup ranges to R2C 7.54–9.59× and C2R
5.58–7.90×. See [`review-log.md`](review-log.md).

## Extended f32 reference results

The test rounds deterministic inputs to f32 and compares R2C and C2R outputs
with direct f64 DFT results. The inverse reference starts from an independently
generated spectrum rounded to f32. Relative L2 error is measured against the
reference norm; scaled maximum error is `max_abs / max(max_abs(reference), 1)`. The regression threshold is `2e-5` for both relative L2 and scaled maximum
error. All cases produced finite outputs.

| N | Operation | Maximum absolute error | RMS error | Relative L2 error | Scaled maximum error | Nonfinite outputs |
|---:|---|---:|---:|---:|---:|---:|
| 1,024 | R2C | 1.979333e-5 | 2.177696e-6 | 9.435559e-8 | 4.108365e-8 | 0 |
| 1,024 | C2R | 2.557050e-8 | 2.420661e-9 | 1.055837e-7 | 2.557050e-8 | 0 |
| 2,048 | R2C | 6.339549e-5 | 3.750638e-6 | 1.148544e-7 | 8.019161e-8 | 0 |
| 2,048 | C2R | 1.802713e-8 | 1.818367e-9 | 1.121747e-7 | 1.802713e-8 | 0 |
| 4,096 | R2C | 6.797638e-5 | 4.729309e-6 | 1.025061e-7 | 3.601881e-8 | 0 |
| 4,096 | C2R | 1.989863e-8 | 1.279362e-9 | 1.113922e-7 | 1.989863e-8 | 0 |
| 131 | R2C | 4.552524e-6 | 1.230563e-6 | 1.495768e-7 | 9.161282e-8 | 0 |
| 131 | C2R | 7.193387e-8 | 1.016111e-8 | 1.646036e-7 | 7.193387e-8 | 0 |
| 137 | R2C | 4.397613e-6 | 1.922694e-6 | 2.283866e-7 | 6.906177e-8 | 0 |
| 137 | C2R | 3.126593e-8 | 9.028811e-9 | 1.461969e-7 | 3.126593e-8 | 0 |
| 241 | R2C | 1.098623e-5 | 2.386483e-6 | 2.128470e-7 | 1.146154e-7 | 0 |
| 241 | C2R | 2.890058e-8 | 6.972531e-9 | 1.520110e-7 | 2.890058e-8 | 0 |
| 1,025 | R2C | 2.590250e-5 | 3.348933e-6 | 1.448441e-7 | 5.470471e-8 | 0 |
| 1,025 | C2R | 4.983872e-8 | 3.231033e-9 | 1.410660e-7 | 4.983872e-8 | 0 |
| 2,049 | R2C | 4.402593e-5 | 7.356031e-6 | 2.252050e-7 | 5.799603e-8 | 0 |
| 2,049 | C2R | 7.740066e-8 | 3.320199e-9 | 2.049111e-7 | 7.740066e-8 | 0 |
| 4,097 | R2C | 1.069371e-4 | 1.036629e-5 | 2.246139e-7 | 5.556124e-8 | 0 |
| 4,097 | C2R | 3.450700e-8 | 2.100125e-9 | 1.828404e-7 | 3.450700e-8 | 0 |

These cases cover smooth lengths, general-prime factors, Bluestein plans, and
the SHT lengths 1,025, 2,049, and 4,097. Direct DFT agreement is established
for these inputs and lengths; it does not test every input distribution.

## Release-code inspection

The release symbol `duccfft::real_multipass::execute_forward_pass::<f64>` in
the benchmark binary built from `de5f87f` contains scalar `addsd`, `subsd`,
and `mulsd` operations. The inspected forward and inverse pass symbols contain
no packed floating-point arithmetic or V-prefixed instructions. The forward
symbol has a 184-byte stack frame, about 140 stack-reference instructions, 74
branches, and 64 comparisons. The disassembly includes runtime index and loop
bounds checks. These observations show that the pass is scalar and retains
indexing overhead; they do not attribute a measured fraction of execution time
to any instruction class. The production path is 1.7–3.2× slower than the
compared RustFFT adapter, so scalar loop and memory access work remains before
extensive SIMD optimization.

## R6 production review

The read-only reviewer accepted the production radix-2/radix-4 path with no
actionable correctness or measurement findings. The review checked pass
equations, twiddle signs and indexing, pass order, packed layout, disjoint
workspace use, the forward and inverse eligibility guards, checked allocation
and index arithmetic, and conditional complexify precedence. It confirmed
that the six-process raw samples support the reported production gains over
the earlier recursive implementation and the reported RustFFT and portable
DUCC comparisons. The reviewer also noted that fallback scratch is duplicated
in the current workspace. The review did not run builds or tests. See
[`review-log.md`](review-log.md).

## First and final pass buffer experiment

At source revision `90821180d8889c60ca736a2b30fa0628c8ea02ed`, R2C executes
the first reverse-order pass from the caller's input, removing the initial
workspace copy. C2R writes its final unnormalized pass to the caller's output,
then scales that output in place. This changes the final pass destination but
does not remove the normalization loop. The workspace size is unchanged. The comparison uses six process samples from the earlier production build at
`de5f87f796722bfff1e83c7a620f91b5ab28c35d` and 12 process samples from the
buffer experiment. Each process is pinned to one CPU core and reports the median of
one warm-up and seven timed samples. Plan and workspace construction use 20
repetitions. Both builds use Rust 1.99.0, `opt-level=3`, no custom `RUSTFLAGS`,
no LTO, and no target CPU setting. Load-average telemetry was retained only in local diagnostics. Raw measurements are in
[`m4-real-copy-elimination-process-samples.csv`](../benches/results/m4-real-copy-elimination-process-samples.csv). Each cell gives the earlier production median followed by the buffer
experiment median in microseconds per transform. Single calls and batches of
32 are separate measurements.

| N | Operation | Single | Batch of 32 |
|---:|---|---:|---:|
| 64 | R2C | 0.245 / 0.235 | 0.209 / 0.205 |
| 64 | C2R | 0.260 / 0.276 | 0.220 / 0.236 |
| 128 | R2C | 0.491 / 0.591 | 0.471 / 0.473 |
| 128 | C2R | 0.516 / 0.531 | 0.495 / 0.507 |
| 256 | R2C | 1.012 / 1.252 | 0.987 / 1.010 |
| 256 | C2R | 1.082 / 1.122 | 1.050 / 1.071 |
| 512 | R2C | 2.234 / 2.269 | 2.224 / 2.234 |
| 512 | C2R | 2.385 / 2.450 | 2.376 / 2.373 |
| 1,024 | R2C | 4.729 / 4.834 | 4.714 / 4.704 |
| 1,024 | C2R | 5.064 / 5.145 | 5.057 / 5.053 |
| 2,048 | R2C | 10.680 / 10.796 | 10.661 / 10.684 |
| 2,048 | C2R | 11.291 / 11.482 | 11.332 / 11.335 |
| 4,096 | R2C | 23.099 / 22.537 | 23.555 / 22.676 |
| 4,096 | C2R | 24.767 / 24.462 | 24.987 / 24.251 |

At 4,096, the buffer experiment is 2.4–3.7% faster for R2C and 1.2–3.0%
faster for C2R. Batch timings are within 2.3% at every smaller length; the
single-call results below 512 show two timing clusters and do not support a
stable difference. The measured gain is limited to the largest tested length. The code keeps the direct pass boundaries, while the report treats smaller
lengths as ties or regressions and does not count this result as broad scalar
performance improvement.
