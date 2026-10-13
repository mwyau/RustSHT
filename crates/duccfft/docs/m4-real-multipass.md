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
`eb8085bf45c94d85c3ac083d436ca4586dd17b59`. It uses packed passes for plans
containing only radix 2 and radix 4, while preserving DUCC's conditional
complexify selection. It uses stage twiddles derived from the plan's root
table and two reusable caller-owned real buffers. Other factorisations
continue through the recursive executor. The subsequent scalar radix-3/radix-5 implementation, direct DUCC comparison,
and paired performance measurements are in
[`m4-real-radix35.md`](m4-real-radix35.md).

### Radix-3 and radix-5 passes

The production pass plan now also handles factor trees made from radices 2, 3,
4, and 5. The scalar radix-3 and radix-5 equations follow pinned DUCC's
`rfftp3` and `rfftp5`, including their odd-`ido` indexing and pass order. Odd-length packed spectra keep both components of the final positive-frequency
bin; only even lengths treat the final bin as real Nyquist. The AVX2 radix-2
and radix-4 interiors run only when `ido` is even, as required by their vector
loops; odd `ido` cases use the scalar loops. Direct f64 DFT tests cover lengths 75, 125, and 225 in both directions. f32
tests compare the same lengths with f64 DFTs after f32 input rounding. The
largest relative L2 error was `1.30e-7`, and no output was nonfinite. A
focused output comparison against DUCC at
`c1ed01b8fda69d2a378190f0915f1a19762dddbf` gave these maximum componentwise
absolute errors:

| N | R2C | C2R |
|---:|---:|---:|
| 75 | 1.421e-14 | 2.776e-17 |
| 125 | 4.260e-15 | 2.776e-17 |
| 225 | 7.105e-15 | 1.583e-17 |

These are correctness results only. Paired execution timings against the prior
recursive plan, RustFFT, and portable/native DUCC are still pending. Forward multipass eligibility requires finite inputs with maximum absolute
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
extensive SIMD optimization. An optimized benchmark binary with debug information was built from source
`cc849b5` using Rust 1.99.0, `CARGO_PROFILE_BENCH_DEBUG=true`, and Cargo's
with `bench-profile-real-inverse 1024 normal 100` and
`bench-profile-real-inverse 1024 high 100`: three warm-ups and 100 timed
inverses at length 1,024. The process-wide counts include plan construction,
workspace allocation, and input setup. The normal finite spectrum passes the inverse bound and uses the packed-real
multipass path. Its process records 14,484,805 instruction references. The
inclusive cost under `RealMultipassPlan::inverse` is 11,774,857 references
(81.3% of the process total). `callgrind_annotate` attributes 7,662,582
references (52.9%) as the self-cost of `execute_inverse_pass`; inline
arithmetic attributed to core-library source files is separate. The inverse
pass loop has the largest source-attributed self-cost. The high-range input fills every bin with `MAX/8`, takes the recursive
residual-corrected fallback, and records 57,980,103 references, 4.00 times the
normal profile. The normal/high difference combines the change from
multipass to recursion with high-range scaling and residual recovery. It does
not isolate the correction arithmetic. The M3 profile before production real
multipass measured 44.0 million references for normal input and 57.9 million
for high-range input; the new normal path reduces its process-wide reference
count while retaining the high-range fallback. The two current raw profiles

An earlier uninstrumented production comparison at source `de5f87f` showed
C2R 7.1–7.3% slower than R2C at length 1,024. The later paired `ido == 1`
comparison above, with candidate source `a9486fb`, shows C2R 0.5% slower for a
profile itself was collected from source `cc849b5`; its counts are not timing
measurements. The extreme-value fallback retains the correction path and has
not been optimized by this change. The same optimized f64 binary was inspected with `objdump`. In the general
`ido > 1` radix-4 inverse loop, the default x86-64 build uses scalar
`movsd`/`addsd`/`subsd` operations and emits multiple slice-index bounds
branches in the loop. The inverse normalization copy uses paired SSE2
`movupd`/`mulpd` operations. This identifies the general pass loop and its
indexing as scalar optimization targets; the disassembly does not measure
their separate runtime costs.

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

## Real multipass buffer experiment

The current R2C path at source revision `9b10ef8` executes the first
reverse-order pass from the caller's input, removing the initial workspace
copy. The workspace size and butterfly operations are unchanged. The C2R path
uses its final workspace buffer and applies inverse normalization while
writing the caller's output. A temporary revision, `90821180d8889c60ca736a2b30fa0628c8ea02ed`, also wrote
the final unnormalized C2R pass directly to the caller's output before scaling
that output in place. The initial 12-process comparison used the combined
prototype harness for the old results and the native-only harness for the new
results. Its raw records are retained in
[`m4-real-copy-elimination-process-samples.csv`](../benches/results/m4-real-copy-elimination-process-samples.csv),
but the different harness modes do not support attribution of small timing
changes. The controlled comparison below uses `bench-native-real` for both source
revisions: baseline `6105973ef8573a9035ef8b9500e81f6f80df6ff6` and candidate
`90821180d8889c60ca736a2b30fa0628c8ea02ed`. It contains six process samples
per revision and length, with the run order alternated. Each process is pinned
to one CPU core, warms the plan once, and reports the median of seven timed samples. Plan construction and workspace allocation use 20 repetitions. Both builds
use Rust 1.99.0, `opt-level=3`, no custom `RUSTFLAGS`, no LTO, and no target
CPU setting. Load-average telemetry was retained only in local diagnostics. Treat small differences as provisional. Raw samples
are in
[`m4-real-buffer-paired-process-samples.csv`](../benches/results/m4-real-buffer-paired-process-samples.csv). Each cell gives baseline / candidate medians in microseconds per transform. Single calls and sequential batches of 32 are reported separately.

| N | Operation | Single | Batch of 32 |
|---:|---|---:|---:|
| 64 | R2C | 0.236 / 0.261 | 0.209 / 0.209 |
| 64 | C2R | 0.261 / 0.286 | 0.220 / 0.240 |
| 128 | R2C | 0.506 / 0.490 | 0.472 / 0.473 |
| 128 | C2R | 0.526 / 0.541 | 0.495 / 0.505 |
| 256 | R2C | 1.002 / 1.027 | 0.984 / 1.006 |
| 256 | C2R | 1.082 / 1.122 | 1.053 / 1.071 |
| 512 | R2C | 2.320 / 2.264 | 2.231 / 2.256 |
| 512 | C2R | 2.385 / 2.440 | 2.364 / 2.373 |
| 1,024 | R2C | 4.749 / 4.800 | 4.707 / 4.699 |
| 1,024 | C2R | 5.074 / 5.149 | 5.060 / 5.083 |
| 2,048 | R2C | 10.720 / 10.830 | 10.658 / 10.630 |
| 2,048 | C2R | 11.267 / 11.442 | 11.325 / 11.344 |
| 4,096 | R2C | 22.973 / 22.547 | 23.137 / 22.763 |
| 4,096 | C2R | 24.055 / 24.552 | 24.300 / 24.265 |

The R2C input-copy removal improves the 4,096 median by 1.9% for one call
and 1.6% for batches of 32. At smaller lengths, batch results range from 2.2%
slower to 0.3% faster. The direct C2R output path is 9.8% slower at length 64
for a single call and 9.1% slower for batches; at length 4,096 it is 2.1%
slower for a single call and 0.1% faster for batches. It showed no consistent
gain, so revision `9b10ef8` restored the previous C2R destination. For R2C,
the clearest repeatable gain is at length 4,096; smaller-size results vary by
call count and direction. The input path is retained as a measured buffer
traffic reduction, but this experiment does not establish a general speedup. Neither change closes the remaining smooth-radix gap to RustFFT or DUCC.

## `ido == 1` pass loops

The radix-2 and radix-4 pass functions now use a dedicated loop when `ido == 1`. Forward passes read separated branch planes and write contiguous butterfly
groups. Inverse passes read contiguous groups and write separated output
planes. The specialization uses safe slice chunking; other `ido` values keep
the general pass loops. The paired native-only comparison used baseline
`06813e9c7f758098751f8556fac0faad684a1a0d` and candidate source `a9486fb`. Six processes per revision and length ran in alternating order, pinned to one CPU core. Each process warmed the plan once and reported the median of seven
execution samples. The machine was an x86-64 CPU with Rust 1.99.0, `opt-level=3`, no custom `RUSTFLAGS`, no LTO,
and no target CPU setting. The shared Legendre workload was active, so these
results remain provisional. The harness includes the public operation's
packing and output conversion. Batch timings below are per transform for a
sequential batch of 32. Plan and workspace times remain in the raw data but
are not affected by the pass-loop change. Each cell gives baseline / candidate time in microseconds and candidate speed
change relative to baseline, calculated as `(baseline time / candidate time) -
1`. Negative percentages are regressions.

| N | Operation | Single | Batch of 32 |
|---:|---|---:|---:|
| 64 | R2C | 0.231 / 0.240 (-3.7%) | 0.212 / 0.199 (+6.5%) |
| 64 | C2R | 0.261 / 0.271 (-3.7%) | 0.237 / 0.236 (+0.2%) |
| 128 | R2C | 0.501 / 0.486 (+3.1%) | 0.486 / 0.455 (+6.8%) |
| 128 | C2R | 0.541 / 0.521 (+3.8%) | 0.531 / 0.498 (+6.6%) |
| 256 | R2C | 1.042 / 0.982 (+6.1%) | 1.041 / 0.967 (+7.7%) |
| 256 | C2R | 1.102 / 1.042 (+5.8%) | 1.078 / 1.026 (+5.0%) |
| 512 | R2C | 2.400 / 2.204 (+8.9%) | 2.253 / 2.128 (+5.9%) |
| 512 | C2R | 2.400 / 2.274 (+5.5%) | 2.366 / 2.251 (+5.1%) |
| 1,024 | R2C | 4.989 / 4.819 (+3.5%) | 4.798 / 4.541 (+5.7%) |
| 1,024 | C2R | 5.074 / 4.844 (+4.8%) | 5.072 / 4.846 (+4.7%) |
| 2,048 | R2C | 11.085 / 10.704 (+3.6%) | 10.942 / 10.322 (+6.0%) |
| 2,048 | C2R | 11.336 / 10.886 (+4.1%) | 11.388 / 10.980 (+3.7%) |
| 4,096 | R2C | 23.319 / 21.901 (+6.5%) | 23.522 / 22.070 (+6.6%) |
| 4,096 | C2R | 24.391 / 23.308 (+4.6%) | 24.485 / 23.436 (+4.5%) |

The candidate reduces median time at lengths 128–4,096 for both transform
directions and call counts. At length 64, single-call speed declines about
3.7%; the batch results are flat or faster. This pattern supports retaining the
specialization for now while treating the smallest-size effects as noise or
branch overhead. It does not resolve the overall gap to RustFFT. Release disassembly of `execute_forward_pass::<f64>` in the candidate shows
the `ido == 1` radix-4 loop using two-lane SSE2 `addpd` and `subpd` operations;
the corresponding baseline loop uses scalar `addsd` and `subsd`. This confirms
vectorization for that forward pass on the tested x86-64 compiler target. It
does not establish the cause of the full-call timing changes or claim that
inverse passes use packed operations. The full per-process records are in
[`m4-real-ido1-paired-process-samples.csv`](../benches/results/m4-real-ido1-paired-process-samples.csv).

## C2R packed-spectrum input

`pack_spectrum` previously zero-filled its length-`N` destination before
copying the real spectrum into it. For the even lengths accepted by
`RealMultipassPlan`, the DC value, every interior real/imaginary pair, and the
Nyquist value assign all `N` destination elements. The redundant fill is
removed, and a debug assertion checks that the spectrum contains `N/2 + 1`
bins. This changes only the ordinary packed-real inverse path; high-range
spectra still use the residual-corrected fallback. The paired comparison used baseline source
`cc849b59f64429fa717b2244d8393892ced91b52` and candidate source
`74f61eee647deed1a37f6507bcb5d5285f26caae`. The baseline executable was copied
before rebuilding the candidate. Six processes per revision and length ran in
alternating order, pinned to one CPU core of an x86-64 CPU. Both binaries used
Rust 1.99.0, Cargo's `bench` profile with `opt-level=3` and debug information,
no custom `RUSTFLAGS`, no LTO, and no target CPU setting. Plans were constructed
before timing; each process discarded one warm-up execution and reported the
median of seven timed samples. `bench-native-real N 1 32` used deterministic
spectrum inputs. Public-operation timings include packing and normalization;
batch figures are per transform for 32 sequential transforms. Plan and
workspace measurements are retained in the raw CSV. The Legendre workload
was active on the shared host, so the results are provisional. Each timing cell gives baseline / candidate microseconds per transform and
candidate speed change, calculated as `(baseline / candidate) - 1`.

| N | Single C2R | Batch of 32 C2R |
|---:|---:|---:|
| 64 | 0.246 / 0.221 (+11.1%) | 0.213 / 0.197 (+8.2%) |
| 128 | 0.506 / 0.476 (+6.4%) | 0.480 / 0.451 (+6.2%) |
| 256 | 1.017 / 0.997 (+2.1%) | 0.985 / 0.964 (+2.2%) |
| 512 | 2.244 / 2.204 (+1.8%) | 2.238 / 2.180 (+2.6%) |
| 1,024 | 4.794 / 4.674 (+2.6%) | 4.765 / 4.662 (+2.2%) |
| 2,048 | 10.786 / 10.495 (+2.8%) | 10.802 / 10.527 (+2.6%) |
| 4,096 | 22.948 / 22.593 (+1.6%) | 23.211 / 22.820 (+1.7%) |

The six-process medians improve at every tested length. The relative gain is
largest at lengths 64 and 128, where the absolute change is tens of
nanoseconds. R2C timings serve as a paired control, and all baseline/candidate
checksums match. Keep the small-size gains provisional while the shared host is
active. Raw process samples are in
[`m4-real-pack-paired-process-samples.csv`](../benches/results/m4-real-pack-paired-process-samples.csv).

## Inverse radix-four branch slices

The inverse radix-4 pass reads each input `k` group from four contiguous
segments, while its output is stored in four branch-major segments. The pass
now obtains those segments once with safe slices and advances through
`chunks_exact`, instead of recomputing multidimensional input and output
indices for each butterfly element. The radix constant `2` is also constructed
once per pass. The radix equations, pass order, and scratch ownership are
unchanged. The paired comparison used baseline source
`74f61eee647deed1a37f6507bcb5d5285f26caae` and candidate source
`4cbe4e940f1912228e2f1376b200dd304460601c`. Six processes per source and length
ran in alternating order, pinned to one CPU core on the x86-64 CPU. Both binaries
used Rust 1.99.0 and Cargo's `bench` profile with `opt-level=3`, debug
information, no custom `RUSTFLAGS`, no LTO, and no target CPU setting. Plans
were built before timing. Each process discarded one warm-up and reported the
median of seven samples from `bench-native-real N 1 32`. Public-operation
timings include packing and normalization; batch figures are per transform
for 32 sequential transforms. The Legendre workload was active, so these
timings are provisional. Plan construction, scratch allocation, R2C control
timings, and all per-process samples are in the raw CSV. Each timing cell gives baseline / candidate microseconds per transform and
candidate speed change, calculated as `(baseline / candidate) - 1`.

| N | Single C2R | Batch of 32 C2R |
|---:|---:|---:|
| 64 | 0.230 / 0.190 (+21.1%) | 0.206 / 0.175 (+17.5%) |
| 128 | 0.491 / 0.451 (+8.9%) | 0.477 / 0.394 (+21.1%) |
| 256 | 1.012 / 0.782 (+29.5%) | 1.003 / 0.725 (+38.3%) |
| 512 | 2.204 / 1.749 (+26.1%) | 2.187 / 1.727 (+26.6%) |
| 1,024 | 4.694 / 3.461 (+35.6%) | 4.700 / 3.428 (+37.1%) |
| 2,048 | 10.605 / 8.000 (+32.6%) | 10.597 / 8.023 (+32.1%) |
| 4,096 | 22.593 / 15.985 (+41.3%) | 22.784 / 16.056 (+41.9%) |

The unchanged R2C operation is a paired timing control. For each process, I
calculated `baseline_norm = baseline_C2R / baseline_R2C` and
`candidate_norm = candidate_C2R / candidate_R2C`, then calculated
`(baseline_norm / candidate_norm) - 1`. The median of those six paired speed
gains at lengths 256–4,096 is 31.9–42.0% for one transform and 27.8–42.5% for
batches. At lengths 64 and 128, results vary more across process pairs; treat those
gains cautiously. Baseline and candidate checksums and workspace sizes match.

ups and 100 timed C2R inverses at N=1,024. The profile includes plan
construction, workspace allocation, and input setup; reported instruction
counts are not timings. For normal finite spectra, counts fell from
13,587,745 at baseline to 9,712,753 at candidate, a 28.52% reduction. The
`execute_inverse_pass` self-cost row fell from 7,662,582 to 3,943,458
references. For the high-range residual-corrected fallback, counts were
57,981,992 and 57,980,016, respectively. The fallback work is unchanged. The
[`baseline normal`](../benches/results/m4-real-radix4-slices-baseline-normal.out.gz),
[`candidate normal`](../benches/results/m4-real-radix4-slices-candidate-normal.out.gz),
[`baseline high-range`](../benches/results/m4-real-radix4-slices-baseline-high.out.gz),
and [`candidate high-range`](../benches/results/m4-real-radix4-slices-candidate-high.out.gz). Release disassembly for the f64 `execute_inverse_pass` symbol contained 59
`jae`/`jbe` branches in the baseline and 30 in the candidate. This count also
includes loop-control branches. The general `ido > 1` butterfly still uses
scalar `movsd`/`addsd`/`subsd`/`mulsd` operations, so slice partitioning has
not closed the remaining vectorization gap. The full process samples are in
[`m4-real-radix4-slices-paired-process-samples.csv`](../benches/results/m4-real-radix4-slices-paired-process-samples.csv).

## Current real production comparison

The comparison below uses production source revision
`4cbe4e940f1912228e2f1376b200dd304460601c`. Rust measurements were collected
in six CPU-pinned processes with `bench-real-multipass`, which verifies the
production outputs against the packed-real prototype and RustFFT adapter
before timing. Each operation reports the median of seven timed samples after
one warm-up. Plan construction and workspace allocation report 20-repetition
means. The direct C++ DUCC controls were measured in six more CPU-pinned
processes, alternating portable and native build order. The C++ harness uses
the same deterministic real inputs and public half-spectrum conversion. The
independent Rust and C++ process sets match checksums to the reported decimal
precision. Both sets used an x86-64 CPU, logical one CPU core, one execution thread,
f64, and Rust 1.99.0 (`b940084d7`, LLVM 23.1.1), Cargo 1.99.0, the optimized
`bench` profile, no custom Rust flags, LTO, or target CPU setting. RustFFT was
6.4.1 with runtime dispatch. DUCC was pinned to
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, built with g++ 15.2.0. The
portable flags were `-std=c++20 -O3 -DNDEBUG -pthread`; the native build added
`-march=native`. The Legendre workload was active during the runs, so absolute
times remain provisional. Each cell gives `DUCCFFT / RustFFT adapter / DUCC portable / DUCC native` in
microseconds per transform, followed by the DUCCFFT slowdown ratios against
RustFFT, portable DUCC, and native DUCC in that order.

### One transform

| N | R2C, µs (ratios) | C2R, µs (ratios) |
|---:|---|---|
| 64 | 0.210 / 0.140 / 0.136 / 0.136 (1.50 / 1.55 / 1.55×) | 0.181 / 0.111 / 0.141 / 0.150 (1.64 / 1.28 / 1.21×) |
| 128 | 0.441 / 0.261 / 0.250 / 0.251 (1.69 / 1.76 / 1.76×) | 0.411 / 0.201 / 0.281 / 0.251 (2.04 / 1.46 / 1.64×) |
| 256 | 0.931 / 0.501 / 0.476 / 0.511 (1.86 / 1.96 / 1.82×) | 0.766 / 0.391 / 0.511 / 0.481 (1.96 / 1.50 / 1.59×) |
| 512 | 2.109 / 1.022 / 1.052 / 1.102 (2.06 / 2.00 / 1.91×) | 1.809 / 0.786 / 1.112 / 0.962 (2.30 / 1.63 / 1.88×) |
| 1,024 | 4.448 / 2.179 / 2.194 / 2.360 (2.04 / 2.03 / 1.88×) | 3.446 / 1.718 / 2.255 / 2.054 (2.01 / 1.53 / 1.68×) |
| 2,048 | 9.954 / 4.473 / 4.824 / 5.240 (2.22 / 2.06 / 1.90×) | 7.990 / 3.592 / 5.215 / 4.583 (2.22 / 1.53 / 1.74×) |
| 4,096 | 21.324 / 9.723 / 11.287 / 12.358 (2.19 / 1.89 / 1.73×) | 15.770 / 7.804 / 11.752 / 10.555 (2.02 / 1.34 / 1.49×) |

### Sequential batch of 32

| N | R2C, µs (ratios) | C2R, µs (ratios) |
|---:|---|---|
| 64 | 0.182 / 0.121 / 0.121 / 0.118 (1.51 / 1.51 / 1.54×) | 0.163 / 0.093 / 0.120 / 0.123 (1.76 / 1.36 / 1.33×) |
| 128 | 0.428 / 0.247 / 0.242 / 0.251 (1.73 / 1.77 / 1.70×) | 0.401 / 0.185 / 0.254 / 0.229 (2.16 / 1.57 / 1.75×) |
| 256 | 0.900 / 0.488 / 0.467 / 0.496 (1.84 / 1.93 / 1.81×) | 0.728 / 0.374 / 0.497 / 0.477 (1.95 / 1.46 / 1.53×) |
| 512 | 2.049 / 1.010 / 1.071 / 1.150 (2.03 / 1.91 / 1.78×) | 1.720 / 0.792 / 1.126 / 0.981 (2.17 / 1.53 / 1.75×) |
| 1,024 | 4.373 / 2.201 / 2.261 / 2.457 (1.99 / 1.93 / 1.78×) | 3.425 / 1.702 / 2.286 / 2.066 (2.01 / 1.50 / 1.66×) |
| 2,048 | 10.058 / 4.672 / 5.070 / 5.588 (2.15 / 1.98 / 1.80×) | 8.038 / 3.653 / 5.297 / 4.608 (2.20 / 1.52 / 1.74×) |
| 4,096 | 21.517 / 10.187 / 11.579 / 12.498 (2.11 / 1.86 / 1.72×) | 15.924 / 8.027 / 12.176 / 11.030 (1.98 / 1.31 / 1.44×) |

Plan-construction and workspace-allocation medians are in microseconds. Each
setup value is the median of records from six processes, pooling the R2C and
C2R records with `batch_count=1` only. Scratch is reusable workspace in bytes. The DUCC values report kernel scratch and its allocation cost; they exclude the
adapter's packed working buffer, while the Rust workspace counts include the
corresponding working storage. Plan-table sizes are not exposed by the
production plan, RustFFT adapter, or direct DUCC plan.

| N | DUCCFFT production: plan / workspace / scratch | RustFFT: plan / workspace / scratch | DUCC portable: plan / workspace / scratch | DUCC native: plan / workspace / scratch |
|---:|---:|---:|---:|---:|
| 64 | 1.108 / 0.131 / 2,048 | 0.975 / 0.018 / 512 | 0.351 / 0.014 / 512 | 0.356 / 0.013 / 512 |
| 1,024 | 16.785 / 0.357 / 27,776 | 17.458 / 0.139 / 16,384 | 1.297 / 0.071 / 8,192 | 1.179 / 0.071 / 8,192 |
| 4,096 | 66.016 / 1.018 / 109,760 | 62.415 / 0.510 / 65,536 | 3.442 / 0.310 / 32,768 | 3.063 / 0.312 / 32,768 |

The real inverse radix-four slice change lowered normal N=1,024 process-wide
instruction references by 28.52%; its inverse pass self-cost fell from
7,662,582 to 3,943,458 references. The updated public-operation comparison
places smooth R2C and C2R at about 1.5–2.3× RustFFT, down from the recursive
real executor's roughly 10–18× gap. The ordinary finite-input path uses the
M3 range check to skip residual construction; high-range values keep the
correction fallback. The historical M2 measurements show large C2R/R2C
asymmetry at lengths 1,025 and 2,049. Current measurements reduce the ratios
to about 1.5× and 1.0×, respectively. M3 introduced Bluestein handling and the
inverse range fast path in the same period, so these whole-operation timings
do not isolate the contribution of either change to that reduction. The
smooth C2R timings here are close to R2C timings, while the remaining smooth
gap and the still-scalar general `ido > 1` inverse loop warrant further M4
optimization. Raw process samples are in
[`m4-real-slices-production-vs-rustfft-process-samples.csv`](../benches/results/m4-real-slices-production-vs-rustfft-process-samples.csv)
and
[`m4-real-slices-ducc-controls.csv`](../benches/results/m4-real-slices-ducc-controls.csv).

## Terminal-pass AVX2 experiment

I tested a f64 AVX2 kernel for the inverse radix-four `ido == 1` pass. It
transposed four packed butterflies, evaluated four independent scalar
butterflies in vector lanes, and stored the four output branches. The
candidate used runtime AVX2 detection and retained the scalar fallback. It
matched the scalar pass in direct kernel tests and passed the complete
`duccfft` test suite, including the existing real inverse extreme-value and
reference tests. The baseline source was `6a58762976a5b6cdfd264fd5edbaf16c291dd6f0`; the
candidate source was `d5c8c63c2317f5cc2e230bda42d8f1d21ae3dec3`. Twelve
process pairs were pinned to logical one CPU core. Pair order alternated between
baseline-first and candidate-first. Each process measured public R2C/C2R calls
with one transform and batch count 32, using the harness's deterministic
normal inputs, one warm-up, and seven timed samples. Rust used version 1.99.0,
Cargo's `bench` profile at `opt-level=3`, no custom `RUSTFLAGS`, no LTO, and no
`target-cpu` setting. The CPU was an x86-64 CPU with AVX2. The Legendre
workload was active, so these wall-time results remain provisional. The table gives the median paired speed gain for the candidate in percent;
positive values mean lower candidate time. R2C is a control because it does
not use the inverse kernel.

| N | R2C, one / batch 32 | C2R, one / batch 32 |
|---:|---:|---:|
| 64 | -0.23 / +3.09 | +5.26 / +0.58 |
| 128 | +2.24 / +1.63 | -2.14 / +0.90 |
| 256 | +0.00 / -0.41 | +1.47 / +2.63 |
| 512 | +0.71 / +1.64 | -0.30 / -0.25 |
| 1,024 | +1.21 / +1.88 | +0.15 / -0.02 |
| 2,048 | +2.43 / +2.21 | -0.65 / -1.12 |
| 4,096 | +1.17 / +1.57 | -1.86 / -1.61 |

All paired checksums match. The small R2C changes show the size of variation
that cannot be attributed to this inverse-only kernel. C2R gains vary by
length and turn into small regressions at 2,048 and 4,096. The counterbalanced
public-call data therefore does not support retaining the extra kernel and
dispatch path.

instruction references fell from 42,132,211 to 41,014,476 (2.65%). The
plan creation and setup, and are not execution-time measurements. Release
disassembly contains `vmovupd`, `vunpcklpd`/`vunpckhpd`, `vperm2f128`,
`vaddpd`, and `vsubpd` in the candidate's vector loop. The lower instruction
count did not result in a consistent whole-call C2R gain. The kernel was
removed in `a6a2560` pending a more substantial gain from the scalar
`ido > 1` inverse loop. Raw paired samples are in
[`m4-real-radix4-avx2-counterbalanced-process-samples.csv`](../benches/results/m4-real-radix4-avx2-counterbalanced-process-samples.csv). Compressed process profiles are in
and

## Inverse radix-four interior AVX2 experiment

The terminal-pass experiment did not vectorize enough work to produce a
repeatable whole-call gain. I then tested the heavier inverse radix-four
interior loop. The f64 AVX2 kernel processes two adjacent packed complex
positions per vector, handles all four output branches, and returns the first
position for the scalar remainder. Plan construction installs the kernel only
when runtime AVX2 detection succeeds. The scalar equations remain the fallback
for short `ido` values and the unvectorized tail. The forward real transform
does not call this kernel. The baseline source was
`6a58762976a5b6cdfd264fd5edbaf16c291dd6f0`; the candidate source was
`8e285c4d8e6e456dd9ac785ae6d39140a7d44fcc`. Each source was built with Rust
1.99.0 (`b940084d7`, LLVM 23.1.1) and Cargo 1.99.0 on an x86-64 CPU. The bench profile used Cargo defaults: `-C opt-level=3`,
`-C embed-bitcode=no`, and `-C strip=debuginfo`. No `RUSTFLAGS`, LTO, or
`target-cpu` setting was present. Both binaries used the same `x86_64-unknown-
linux-gnu` target. Each process ran `bench-native-real N 1 32`, pinned to
logical one CPU core. The deterministic inputs come from the benchmark's bounded
`real_value` and `spectrum_value` functions. Each process performed a warm-up
and seven timed samples; twelve baseline/candidate process pairs per length
alternated execution order. The table reports median per-transform times
across the process samples. The ratio is baseline divided by candidate; the
paired gain is the median of the twelve per-pair values of
`baseline/candidate - 1`.

| N | R2C, one call, µs (ratio; paired gain) | R2C, batch 32, µs (ratio; paired gain) | C2R, one call, µs (ratio; paired gain) | C2R, batch 32, µs (ratio; paired gain) |
|---:|---:|---:|---:|---:|
| 64 | 0.220 / 0.221 (0.995×; −0.45%) | 0.189 / 0.184 (1.023×; +2.29%) | 0.180 / 0.170 (1.059×; +5.88%) | 0.156 / 0.146 (1.065×; +6.09%) |
| 128 | 0.451 / 0.446 (1.012×; +2.27%) | 0.434 / 0.429 (1.012×; +1.28%) | 0.401 / 0.381 (1.051×; +2.62%) | 0.375 / 0.361 (1.038×; +3.32%) |
| 256 | 0.947 / 0.942 (1.005×; +0.49%) | 0.914 / 0.908 (1.007×; +0.76%) | 0.717 / 0.632 (1.134×; +14.17%) | 0.687 / 0.609 (1.127×; +13.20%) |
| 512 | 2.104 / 2.094 (1.005×; +0.24%) | 2.092 / 2.052 (1.019×; +2.02%) | 1.664 / 1.508 (1.103×; +10.32%) | 1.642 / 1.488 (1.103×; +9.86%) |
| 1,024 | 4.468 / 4.478 (0.998×; +0.11%) | 4.456 / 4.344 (1.026×; +2.62%) | 3.221 / 2.675 (1.204×; +20.68%) | 3.234 / 2.676 (1.208×; +20.43%) |
| 2,048 | 10.124 / 10.174 (0.995×; −0.54%) | 10.164 / 10.061 (1.010×; +1.31%) | 7.549 / 6.527 (1.157×; +15.88%) | 7.578 / 6.554 (1.156×; +15.81%) |
| 4,096 | 21.675 / 21.370 (1.014×; +1.43%) | 21.896 / 21.629 (1.012×; +1.45%) | 15.168 / 11.988 (1.265×; +26.23%) | 15.287 / 12.114 (1.262×; +25.91%) |

All reported paired checksums match. R2C is a control and does not execute the
inverse kernel. Its batch changes range from +0.76% to +2.62%; its single-call
changes range from −0.54% to +2.27%. C2R batch gains are positive in all
twelve process pairs at lengths 256 through 4,096, with median paired gains of
9.86% to 25.91%. At 64 and 128, the smaller C2R gains are not clearly
separated from shared-host variation. The Legendre workload was active during
the runs, so these measurements remain provisional. To account for process-to-process speed shifts, I also divided each pair's
C2R speed ratio by that pair's R2C control ratio, then took the median across
twelve pairs. Positive values indicate a C2R gain beyond the matched R2C
change.

| N | Single C2R gain after R2C control | Batch 32 C2R gain after R2C control |
|---:|---:|---:|
| 256 | +10.6% | +11.3% |
| 512 | +12.4% | +7.8% |
| 1,024 | +20.3% | +17.5% |
| 2,048 | +16.0% | +14.8% |
| 4,096 | +24.3% | +24.2% |

The public-call setup measurements show no new workspace storage. For each
length, baseline and candidate report the same scratch byte count: 2,048;
4,128; 7,232; 15,456; 27,776; 60,576; and 109,760 bytes, respectively. Median
plan construction times were 1.475 / 1.499, 2.667 / 2.726, 4.824 / 4.704,
9.078 / 8.940, 17.149 / 17.445, 34.293 / 34.516, and 67.685 / 67.955 µs
(baseline / candidate). Median workspace allocation times were 0.315 / 0.312,
0.377 / 0.385, 0.516 / 0.426, 0.682 / 0.598, 0.743 / 0.811, 1.326 / 1.314,
and 2.144 / 2.146 µs. These setup measurements show no consistent candidate
penalty; the candidate stores only a function pointer in its immutable plan. Release disassembly of
`duccfft::simd::x86::f64_real_kernel::inverse_radix4_interior` contains packed
YMM operations including `vmovupd`, `vpermpd`, `vaddpd`, `vsubpd`, `vmulpd`,
and `vunpcklpd`. The vector loop has a loop branch and no helper calls or
stack-local spill stores. The function saves callee-preserved registers in its
prologue. This confirms that the measured path executes packed arithmetic;
the whole-call timing remains the performance evidence. The batch controls and all-pair C2R gains at lengths 256 and above support
retaining this targeted kernel while the wider M4 review proceeds. They also
show that scalar radix-four interior work accounted for a measurable share of
smooth C2R time: the matching R2C calls did not use the new kernel and remained
near their baseline. This result concerns smooth radix-four passes and does
not explain the larger residual C2R/R2C gap at generic-prime SHT lengths. The
terminal-pass-only kernel remains rejected. Raw process samples, including plan and workspace timings, are in
[`m4-real-radix4-ido-interior-paired-process-samples.csv`](../benches/results/m4-real-radix4-ido-interior-paired-process-samples.csv). The revised scalar unit test asserts the kernel's expected first scalar-tail
index and verifies that the terminal endpoint and unprocessed tail remain
untouched. The f64/f32 direct inverse-DFT test now covers random spectra at
lengths 1,024, 2,048, and 4,096. The pinned DUCC comparison at
`c1ed01b8fda69d2a378190f0915f1a19762dddbf` used the portable g++ 15.2.0 build
(`-std=c++20 -O3 -DNDEBUG -pthread`, `vectorize=false`). Across C2C, R2C, and
C2R it compared 202,347 output records (404,694 scalar components). DUCCFFT's
maximum absolute difference was `1.819e-12`; maximum scaled difference was
`6.821e-13`. The comparison also checked RustFFT against both references. The comparison summary and build metadata are in
[`m4-real-radix4-ido-interior-ducc-comparison.txt`](../benches/results/m4-real-radix4-ido-interior-ducc-comparison.txt).

## Forward radix-four interior AVX2 experiment

The forward radix-four interior pass now gathers its four branch slices once
per `l1` group and uses an f64 AVX2 kernel when runtime detection succeeds and
`ido >= 16`. The kernel evaluates two adjacent complex positions per vector;
the scalar equations handle the remaining tail and all non-AVX2 cases. The
plan stores the kernel pointer and allocates no additional execution scratch. The candidate also replaces the former multidimensional `index` calls in the
scalar interior loop with branch-local slices. The whole-call comparison
measures both changes together, so it does not attribute the full gain to
vector arithmetic alone. f32 and non-x86 targets continue to use scalar
execution. The paired comparison used baseline source
`3f5731c70702293449e50bf35419b9293fdf59bc` and candidate source
`ffe4524b345eb9f2192f534ecd63152b5f5053d3`. Twelve baseline/candidate process
pairs per length alternated execution order and were pinned to logical one CPU core
of an x86-64 CPU. Each process ran `bench-native-real N 1 32`, with one
warm-up and seven timed samples. Inputs use the harness's deterministic
`real_value` and `spectrum_value` functions. Rust used rustc 1.99.0
(`b940084d7`, LLVM 23.1.1), Cargo 1.99.0, the default optimized `bench`
profile (`-C opt-level=3`, `-C embed-bitcode=no`, `-C strip=debuginfo`), no
custom `RUSTFLAGS`, no LTO, and no target CPU flag. The Legendre workload was
active during the runs, so wall times remain provisional. The table gives the median paired speed gain `(baseline / candidate) - 1`. R2C calls execute the candidate pass; C2R is a control. Gains are positive
when the candidate is faster.

| N | R2C single | R2C batch 32 | C2R single control | C2R batch 32 control |
|---:|---:|---:|---:|---:|
| 64 | +16.32% | +14.91% | −0.29% | +0.79% |
| 128 | +12.17% | +12.11% | +1.18% | +1.95% |
| 256 | +31.95% | +31.12% | +0.08% | +1.83% |
| 512 | +24.65% | +25.07% | +0.30% | +0.13% |
| 1,024 | +45.58% | +44.67% | −0.21% | +0.79% |
| 2,048 | +39.54% | +37.24% | +0.23% | +0.54% |
| 4,096 | +55.08% | +55.04% | −0.00% | −0.20% |

All twelve R2C pairs favored the candidate at every length. C2R control
changes range from −0.29% to +1.95%, consistent with process variation. Raw
paired timings, setup costs, scratch sizes, and checksums are in
[`m4-real-radix4-forward-avx2-paired-process-samples.csv`](../benches/results/m4-real-radix4-forward-avx2-paired-process-samples.csv). For cross-library timings, twelve CPU-pinned processes per library and length
used the same public-operation harness and input formulas. Task order rotated
across the four implementations. Ratios use the ratio of the displayed
medians. Each timing cell gives `DUCCFFT / RustFFT / DUCC portable / DUCC
native` in microseconds per transform, followed by the DUCCFFT ratios against
RustFFT, portable DUCC, and native-target DUCC.

### Single transform

| N | R2C, µs (timings; ratios) | C2R, µs (timings; ratios) |
|---:|---|---|
| 64 | 0.181 / 0.140 / 0.140 / 0.140 (1.29 / 1.29 / 1.29×) | 0.171 / 0.111 / 0.141 / 0.141 (1.55 / 1.21 / 1.21×) |
| 128 | 0.401 / 0.261 / 0.266 / 0.266 (1.54 / 1.51 / 1.51×) | 0.381 / 0.210 / 0.281 / 0.250 (1.81 / 1.36 / 1.52×) |
| 256 | 0.717 / 0.501 / 0.491 / 0.501 (1.43 / 1.46 / 1.43×) | 0.642 / 0.391 / 0.531 / 0.471 (1.64 / 1.21 / 1.36×) |
| 512 | 1.663 / 1.032 / 1.187 / 1.152 (1.61 / 1.40 / 1.44×) | 1.508 / 0.802 / 1.213 / 1.012 (1.88 / 1.24 / 1.49×) |
| 1,024 | 3.201 / 2.244 / 2.620 / 2.550 (1.43 / 1.22 / 1.26×) | 2.685 / 1.829 / 2.655 / 2.200 (1.47 / 1.01 / 1.22×) |
| 2,048 | 7.384 / 4.559 / 5.972 / 5.701 (1.62 / 1.24 / 1.30×) | 6.537 / 3.777 / 6.056 / 5.025 (1.73 / 1.08 / 1.30×) |
| 4,096 | 13.806 / 9.654 / 12.439 / 12.649 (1.43 / 1.11 / 1.09×) | 12.033 / 7.845 / 12.224 / 10.520 (1.53 / 0.98 / 1.14×) |

### Sequential batch of 32

| N | R2C, µs (timings; ratios) | C2R, µs (timings; ratios) |
|---:|---|---|
| 64 | 0.161 / 0.121 / 0.134 / 0.127 (1.33 / 1.20 / 1.27×) | 0.147 / 0.092 / 0.135 / 0.126 (1.59 / 1.08 / 1.16×) |
| 128 | 0.385 / 0.246 / 0.267 / 0.261 (1.57 / 1.44 / 1.48×) | 0.356 / 0.185 / 0.269 / 0.246 (1.92 / 1.32 / 1.45×) |
| 256 | 0.696 / 0.492 / 0.517 / 0.500 (1.41 / 1.35 / 1.39×) | 0.625 / 0.377 / 0.489 / 0.455 (1.66 / 1.28 / 1.37×) |
| 512 | 1.640 / 1.037 / 1.083 / 1.152 (1.58 / 1.51 / 1.42×) | 1.487 / 0.786 / 1.134 / 0.988 (1.89 / 1.31 / 1.51×) |
| 1,024 | 3.066 / 2.176 / 2.249 / 2.448 (1.41 / 1.36 / 1.25×) | 2.662 / 1.698 / 2.301 / 2.115 (1.57 / 1.16 / 1.26×) |
| 2,048 | 7.330 / 4.685 / 5.059 / 5.593 (1.56 / 1.45 / 1.31×) | 6.516 / 3.660 / 5.347 / 4.696 (1.78 / 1.22 / 1.39×) |
| 4,096 | 13.876 / 10.248 / 11.210 / 12.390 (1.35 / 1.24 / 1.12×) | 12.130 / 8.169 / 11.612 / 10.546 (1.48 / 1.04 / 1.15×) |

RustFFT uses its default runtime planner. Portable DUCC used g++ 15.2.0 with
`-std=c++20 -O3 -DNDEBUG -pthread`; native-target DUCC added `-march=native`. Both DUCC builds used the scalar planner (`vectorize=false`). The Rust build
used no native target flag, so the native DUCC comparison is labeled as an ISA
difference. Per-process cross-library data are in
[`m4-real-radix4-forward-avx2-cross-library-process-samples.csv`](../benches/results/m4-real-radix4-forward-avx2-cross-library-process-samples.csv). Plan construction, workspace allocation, and reusable scratch storage are
separate from execution. Each setup cell below is the median of the 24
single-call records for R2C and C2R pooled across twelve processes per
operation. DUCC scratch counts its kernel buffer and excludes the real
adapter's packed working array; the Rust workspace counts include its working
buffers.

| N | DUCCFFT: plan / workspace / scratch | RustFFT: plan / workspace / scratch | DUCC portable: plan / workspace / kernel scratch | DUCC native: plan / workspace / kernel scratch |
|---:|---:|---:|---:|---:|
| 64 | 1.473 / 0.368 / 2,048 | 2.729 / 0.018 / 512 | 0.380 / 0.013 / 512 | 0.377 / 0.013 / 512 |
| 128 | 2.692 / 0.439 / 4,128 | 3.787 / 0.028 / 2,048 | 0.512 / 0.015 / 1,024 | 0.510 / 0.015 / 1,024 |
| 256 | 4.876 / 0.484 / 7,232 | 5.863 / 0.053 / 4,096 | 0.701 / 0.030 / 2,048 | 0.680 / 0.030 / 2,048 |
| 512 | 8.843 / 0.617 / 15,456 | 9.557 / 0.144 / 8,192 | 1.045 / 0.116 / 4,096 | 1.028 / 0.115 / 4,096 |
| 1,024 | 17.766 / 0.826 / 27,776 | 20.395 / 0.266 / 16,384 | 1.465 / 0.211 / 8,192 | 1.402 / 0.210 / 8,192 |
| 2,048 | 34.172 / 1.459 / 60,576 | 34.749 / 0.513 / 32,768 | 2.464 / 0.685 / 16,384 | 2.430 / 0.679 / 16,384 |
| 4,096 | 67.012 / 2.227 / 109,760 | 66.345 / 0.982 / 65,536 | 3.662 / 0.734 / 32,768 | 3.353 / 0.747 / 32,768 |

Plan and workspace times are in microseconds. The candidate uses the same
workspace byte counts as the baseline; the additional kernel metadata is one
function pointer in the immutable plan. Release disassembly of
`f64_real_kernel::forward_radix4_interior` contains `vmovupd`, `vmulpd`,
`vaddpd`, `vsubpd`, `vpermpd`, and `vunpcklpd` in the vector loop. It has a
single loop branch, no helper calls, and no stack-local spills; the prologue
saves callee-preserved registers. This confirms packed instructions execute
on the measured x86-64 path. The pinned DUCC numerical comparison checked
202,347 f64 output records across C2C, R2C, and C2R; DUCCFFT's maximum absolute
error was `1.819e-12` and maximum scaled error was `6.821e-13`. The record is
[`m4-real-radix4-forward-avx2-ducc-comparison.txt`](../benches/results/m4-real-radix4-forward-avx2-ducc-comparison.txt). The kernel cuts most of the previous R2C gap. DUCCFFT is 1.29–1.62× slower
than RustFFT for single transforms and 1.33–1.58× slower for batches of 32. C2R is 1.47–1.88× and 1.48–1.92× slower, respectively. The AVX2 path covers
the f64 radix-four interior pass. A separate f64 AVX2 kernel now covers the
inverse radix-two interior pass; its paired timings and no-radix-two controls
are in [`m4-real-radix2-avx2.md`](m4-real-radix2-avx2.md). Small-`ido`, f32,
and generic-prime execution need separate optimization. These measurements
support keeping the radix-four kernel during M4 and do not establish
performance completion.

## Compiler-flag controls

Compiler settings were compared separately from the algorithm change at
lengths 1,024 and 4,096. All builds used source commit
`23cfbd399c76308dadd4831e3cc7bf3dae35b8e3`, the same optimized `bench`
profile, and separate Cargo target directories. The portable build used no
custom `RUSTFLAGS`, LTO, or target CPU setting. The native-target build added
`RUSTFLAGS=-C target-cpu=native`; the LTO build set
`CARGO_PROFILE_BENCH_LTO=true` and left the target CPU unset. These are
benchmark-only builds. Twelve process sets per length rotated the three build variants and were
pinned to logical one CPU core. Each process ran `bench-native-real N 1 32`, with
one warm-up and seven timed samples. The table gives the median process time
in nanoseconds per transform. Parentheses give the median paired speed gain
of the native-target and LTO builds relative to the portable build. The
Legendre workload was active, so these timings remain provisional.

| N | Operation | Single: portable / native / LTO (gain native / LTO) | Batch 32: portable / native / LTO (gain native / LTO) |
|---:|---|---:|---:|
| 1,024 | R2C | 3,085 / 2,846 / 3,081 (+8.45% / −0.02%) | 3,044 / 2,823 / 3,047 (+8.06% / −0.15%) |
| 1,024 | C2R | 2,675 / 2,380 / 2,705 (+11.97% / −1.13%) | 2,661 / 2,375 / 2,694 (+12.28% / −1.29%) |
| 4,096 | R2C | 13,916 / 13,230 / 13,887 (+4.89% / −0.14%) | 14,049 / 13,341 / 13,980 (+5.09% / +0.58%) |
| 4,096 | C2R | 12,233 / 11,196 / 12,163 (+8.69% / −0.00%) | 12,328 / 11,271 / 12,344 (+9.65% / −0.05%) |

The native-target gains were positive in all twelve process pairs for every
case except single R2C at 1,024, where ten pairs favored it. LTO gains were
mixed across pairs and lengths and ranged from −1.29% to +0.58%. Native CPU
targeting affects generated scalar code and narrows the remaining gap, but
the portable target remains the reported default. LTO showed no repeatable
execution gain in these cases. Raw timings, workspace costs, and checksums are
in
[`m4-real-radix4-forward-avx2-build-flag-samples.csv`](../benches/results/m4-real-radix4-forward-avx2-build-flag-samples.csv).

## General odd-prime terminal pass at N=1025

The real multipass planner accepts an odd prime radix below 135 when its pass
has odd `ido`. This covers the terminal radix-41 pass in the
`1025 = 5^2 × 41` factorization and interior odd-prime passes such as radix 17
in `4097 = 17 × 241`. The forward schedule runs the terminal 41 pass first;
the inverse schedule runs it last and applies normalization while writing the
output. Terminal passes use a symmetric direct real DFT. Interior passes use
direct complex sums with the pass twiddles and DUCC's packed real layout. These scalar kernels follow the pass structure but do not yet use DUCC's
optimized `rfftpg` coefficient-sum kernel. Even-`ido` general-prime passes
remain on the recursive executor. The pass is used only for finite values whose maximum magnitude is at most
`MAX/(4*N)`. Larger or nonfinite inputs retain the recursive residual-corrected
path. Tests compare N=1025 forward and inverse outputs with independent f64
DFTs, compare f32 outputs with f64 references, and check that maximum-finite
f32 and f64 inputs take the fallback without producing nonfinite results. The native Rust outputs also agree with pinned DUCC revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`: maximum scaled component errors
were `6.01e-14` for R2C and `2.78e-17` for C2R.

### Paired public-operation timings

The benchmark compares the recursive path at source commit `a05b2b4` with the
multipass path at `b49b962`. Both binaries used the existing
`bench-native-real` harness, the same deterministic inputs, warm plans, and
one warm-up followed by seven timed samples per process. Five interleaved
process pairs measured one transform; three pairs measured 512 transforms
sequentially and report nanoseconds per transform. Processes were pinned to
logical one CPU core on an x86-64 CPU. Rust was 1.99.0, Cargo's `bench`
profile used `opt-level=3`, and no custom `RUSTFLAGS`, LTO, or target CPU flag
was set. The Legendre workload was active, so absolute times are provisional. The paired direction of the change was consistent: the multipass binary was
faster in every process pair. Raw process medians are in
[`m4-real-prime-process-samples.csv`](../benches/results/m4-real-prime-process-samples.csv).

| Rows | Operation | Recursive, median | Multipass, median | Ratio |
|---:|---|---:|---:|---:|
| 1 | R2C | 32.621 µs | 13.836 µs | 2.36× |
| 1 | C2R | 46.918 µs | 20.979 µs | 2.24× |
| 512 | R2C | 32.686 µs | 12.314 µs | 2.65× |
| 512 | C2R | 48.066 µs | 21.530 µs | 2.23× |

Single-row process medians show plan construction increasing from 16.703 to
18.163 µs for R2C and 15.411 to 16.976 µs for C2R. Reusable workspace grew
from 12,544 to 28,944 bytes because the multipass schedule adds two N-element
f64 buffers while retaining recursive fallback scratch. Workspace allocation
increased from 0.677 to 1.156 µs for R2C and from 0.191 to 0.372 µs for C2R. The operation-time reduction is large relative to these setup costs for
repeated plan use. Corresponding baseline/candidate benchmark checksums differ
by at most `8.89e-16`; this only checks benchmark-output consistency and is
not a numerical accuracy result. The raw CSV records the source commits,
binary SHA-256 hashes, per-process plan and workspace measurements, workspace
bytes, operation timings, and checksums. This result supports the DUCC-style pass schedule for the 1025 workload; it
does not establish performance for the interior-prime kernel. The N=98 case
(`2 × 7 × 7`) is checked against independent forward and inverse DFTs and the
pinned DUCC reference. It exercises an interior radix-7 pass with `l1=2` and
`ido=7`, which guards the group-major packed layout. N=4097 now routes its
radix-17 (`ido=241`) stage through the multipass general-prime kernel and
retains radix-241 Bluestein at `ido=1`. Screening measurements show no
reliable whole-call speedup at N=4097, and workspace is about twice the
recursive-only baseline. Lengths 257 and 2049 already use terminal Bluestein
passes in the real multipass executor. Earlier cross-library measurements
reported 10× and 2.8× RustFFT gaps. A later scalar nested-FFT optimization
improved both operations at both lengths in all six paired process samples;
see [`m3-scalar-performance.md`](m3-scalar-performance.md). That screen did not
refresh the cross-library controls, so current gap ratios remain unmeasured. The general-prime implementation is a correctness and execution-structure
step; further scalar profiling is needed before SIMD work.

## Symmetric terminal real-prime inverse

The terminal inverse pass for an odd real radix computes the outputs for
sample indices `n` and `radix-n` together. Their cosine terms are equal and
their sine terms have opposite signs. The loop shares each `Re(X_k) cos(...)`
product and applies the two sine signs to produce both samples. This follows
the paired real-coefficient sums in DUCC's pinned
[`rfftpg`](https://github.com/mreineck/ducc/blob/c1ed01b8fda69d2a378190f0915f1a19762dddbf/src/ducc0/fft/fft1d_impl.h#L2229-L2523)
pass. It changes only terminal real-prime C2R passes with radix below 135. The comparison used the `bench-native-real` harness, deterministic
`spectrum_value` inputs, warm reusable plans, one warm-up and seven timed
samples per process. Six alternating baseline/candidate process pairs ran on
logical one CPU core of an x86-64 CPU. Rust and Cargo were 1.99.0; the
optimized `bench` profile used `opt-level=3`, no custom `RUSTFLAGS`, no LTO,
and no target CPU flag. Each operation was single-threaded. The Legendre
workload was active, so timings are provisional. The baseline binary was
built from `1f0b14b2124eb421b5628e2d6d0d92255131a1ce`; the candidate was built
from `0c0ee9aab8481fcfd6f09e153563a7f1f45dc4e1`. The raw CSV records both
binary hashes and all process measurements.

| N | Operation | Rows | Baseline median | Candidate median | Paired speedup | Pairs favoring candidate |
|---:|---|---:|---:|---:|---:|---:|
| 1,025 | C2R | 1 | 25.367 µs | 14.142 µs | 1.793× | 6/6 |
| 1,025 | C2R | 32 | 25.512 µs | 14.152 µs | 1.801× | 6/6 |
| 1,025 | R2C | 1 | 14.166 µs | 14.176 µs | 0.999× | 1/6 |
| 1,025 | R2C | 32 | 14.227 µs | 14.139 µs | 1.002× | 3/6 |
| 4,097 | C2R | 1 | 241.931 µs | 245.929 µs | 0.984× | 0/6 |
| 4,097 | C2R | 32 | 245.290 µs | 249.620 µs | 0.984× | 0/6 |
| 4,097 | R2C | 1 | 228.972 µs | 234.863 µs | 0.974× | 0/6 |
| 4,097 | R2C | 32 | 231.830 µs | 238.509 µs | 0.974× | 0/6 |

Paired speedups are the median of six baseline-to-candidate latency ratios. Batch rows run sequentially with a shared plan and workspace; table values are
per-transform times. At N=1,025, C2R plan construction medians were 16.98 and
16.99 µs, workspace allocation medians were 0.384 and 0.381 µs, and scratch
remained 28,944 bytes. R2C timings at the same length show no execution gain.

references fell from 14,696,836 to 9,642,013 (34.4%); simulated conditional
branches fell from 2,142,428 to 1,186,934 (44.6%), and simulated conditional
simulation results, not hardware performance counters. Linux denied
uses scalar `mulsd`/`addsd` operations; it has no packed floating-point
instructions. The archived profiles are
and

The candidate passed all crate tests, including f32 comparisons with f64
references and direct DFT checks. Its f64 C2R outputs agreed with pinned DUCC
for prime lengths 7, 13, 17, 31, 41, 53, 107, 109, 113, 127, and 131, and
composite lengths 49, 98, and 1,025; the maximum absolute error was
`6.94e-17`. The N=4,097 control was slower in all six pairs even though its
radix-17 pass uses the interior loop. R2C at N=4,097 also slowed, where this
inverse kernel does not execute. The cause may be shared-host variation or
binary code layout; these measurements do not distinguish them. Raw timing rows are in
[`m4-real-prime-pairsym-process-samples.csv`](../benches/results/m4-real-prime-pairsym-process-samples.csv).
