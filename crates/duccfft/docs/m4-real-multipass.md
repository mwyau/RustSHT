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
