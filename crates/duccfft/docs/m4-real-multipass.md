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
migration will retain the existing public API and numerical fallback. The
ordinary inverse path may use the multipass executor only when
`inverse_normalization_is_safe` accepts the spectrum. High-range or nonfinite
inputs will continue through the residual-corrected recursive executor. The
reviewer confirmed that the current bound conservatively covers radix-2/4
intermediate growth and specifically checked the `N=64`, DC=Nyquist=`MAX`
case that overflows in the unscaled prototype. The prototype uses f64 and supports only powers of two. It does not implement
radix-3/5, general-prime, Bluestein, or DUCC's conditional real complexify
passes. It therefore does not replace the production fallback by itself. The
production migration must add f32 and extreme-value coverage, preserve the
current conditional complexify planner thresholds, and keep unsupported
factorizations on their validated path until their pass implementations are
available. A first production step will specialize radix-2/radix-4 lengths;
later work will port the remaining pass families before removing recursion. The initial length-16 inverse check caught a negative radix-4 inverse twiddle
sign in the prototype. Correcting it to match pinned `rfftp4` made the
independent DUCCFFT and RustFFT comparisons pass for lengths 1 through 4,096. No timing sample was retained from the failing version.

## R5 review

The read-only reviewer found no pass-equation, twiddle-sign, pass-order,
packed-spectrum, or buffer-ownership defect in the corrected prototype. It
accepted the measurements as evidence for production migration, subject to
the inverse overflow guard and broader f32/factorization coverage above. The
review also corrected the speedup ranges to R2C 7.54–9.59× and C2R
5.58–7.90×. See [`review-log.md`](review-log.md).
