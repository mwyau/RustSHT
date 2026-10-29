# Native generic scalar synthesis screen — 2026-10-29

**Status: GAP; isolated source only.** The shared order-major/reflected
kernel was instantiated for native f32 and f64 in scratch. Production dispatch,
the f64-child f32 sidecar, and maintained source were left unchanged. The
candidate did not meet DUCC timing, and a near-pole high-degree f32 diagnostic
still shows a numerical discrepancy that needs an asserted independent test.

## Main candidate

- Parent: `39dd8a883d21a269654e1b74de862ee2e97d22ee`.
- Candidate patch: [`native-generic-scalar-f64-batch-skip-init.patch`](patches/native-generic-scalar-f64-batch-skip-init.patch.gz), SHA-256
  `661a116785aa76bc2d8cb160cb1425ce048246b905f405f7b713178f4b21f443`.
- Candidate binary SHA-256: `1a77e5788d3ed6ba010e6f59e4a0bbe9c64f0fce2913a974d9bae81d049e81fe`.
- Parent binary SHA-256: `8e462e4f574f1614309dd437dbb852630739344589581ee50353fe142ae18e20`.
- DUCC: 0.42.0, commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, binary SHA-256
  `be18c89ea434258e4a07c28ccd8fd31d6262ac3503dcaf81d8aa5176415f1484`.
- Rust flags: `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; AMD Ryzen 9 5950X;
  q1 used one pinned physical core and q8 used eight pinned physical cores; exact CPU masks are recorded in the result JSON.
- Fifteen balanced, interleaved parent/candidate/DUCC process pairs per cell.
  The [`result JSON`](results/f32-parent-candidate-ducc-15.json) records every
  pair, fixture hash, binary hash, output check, SIMD route, and sample.

| GL T720 scalar synthesis | Parent ms | Native generic ms | DUCC ms | Candidate/DUCC median (95% paired bootstrap CI) |
|---|---:|---:|---:|---:|
| Full q1 | 15.936 | 26.061 | 19.233 | 1.355 (1.349–1.370) |
| Full q8 | 2.712 | 3.924 | 2.946 | 1.338 (1.223–1.360) |
| Reduced q1 | 6.523 | 8.846 | 8.418 | 1.054 (1.045–1.058) |
| Reduced q8 | 1.242 | 1.773 | 1.339 | 1.134 (1.078–1.344) |

Intervals are percentile bootstrap intervals over the 15 paired candidate/DUCC
ratios, 20,000 resamples with Python `random` seed `5481`. The candidate used
native f32 coefficients, recurrence, Fourier storage, FFT, and map storage;
Pulp selected V3/AVX2 with eight f32 lanes. All candidate outputs were finite,
repeat-consistent, and reset-consistent. Maximum candidate/DUCC relative L2
error was `1.8914e-4`; maximum absolute error was `5.2983e-4`. These checks do
not establish the required numerical acceptance.

## Scaling-leaf screens

The scale-count, exponent-restoration, and vector-rescale variants below all
missed DUCC in their three-pair screens. Each result JSON contains exact source
patch hash, binaries, fixtures, pair timings, and checks. Patch aliases and
unique patch hashes are listed in [`patch-index.tsv`](patch-index.tsv).

| Candidate | Candidate/DUCC medians: full q1 / full q8 / reduced q1 / reduced q8 | Result |
|---|---|---|
| Scale-count helper | 1.451 / 1.478 / 1.131 / 1.269 | [JSON](results/f32-scale-count-screen-3.json) |
| Scale-count multi-epoch | 1.456 / 1.413 / 1.110 / 1.085 | [JSON](results/f32-scale-count-multiepoch-screen-3.json) |
| i32 scale restoration | 1.615 / 1.623 / 1.234 / 1.112 | [JSON](results/f32-scale-i32-screen-3.json) |
| i32 scalar restoration | 1.426 / 1.268 / 1.078 / 1.180 | [JSON](results/f32-scale-i32-scalar-restore-screen-3.json) |
| Coefficient preparation | 1.341 / 1.177 / 1.035 / 1.244 | [JSON](results/f32-coeff-prep-screen-3.json) |
| Direct normal-range scaler | 1.554 / 1.068 / 1.205 / 1.256 | [JSON](results/f32-direct-normal-scale-screen-3.json) |
| Common-scale vector leaf | 1.476 / 1.352 / 1.112 / 1.233 | [JSON](results/f32-vector-scale-common-screen-3.json) |
| Scale epoch 64 | 1.439 / 1.444 / 1.124 / 1.114 | [JSON](results/f32-scale64-epoch-screen-3.json) |
| Epoch 64 plus subnormal vector restoration | 1.406 / 1.366 / 1.077 / 1.095 | [JSON](results/f32-scale64-vector-subnormal-screen-3.json) |
| Normal-only vector scaler | 1.580 / 1.534 / 1.212 / 1.231 | [JSON](results/f32-vector-scale-screen-3.json) |
| Small-stack vector scaler | 1.648 / 1.612 / 1.293 / 1.230 | [JSON](results/f32-vector-scale-small-stack-screen-3.json) |
| Vector subnormal restoration, epoch 80 | 1.442 / 1.320 / 1.119 / 1.138 | [JSON](results/f32-vector-subnormal-scale80-screen-3.json) |
| Vector-mask fallback | 1.423 / 1.403 / 1.119 / 1.081 | [JSON](results/f32-vector-mask-fallback-screen-3.json) |
| Vector rescale leaf | 1.464 / 1.330 / 1.106 / 1.548 | [JSON](results/f32-vector-rescale-screen-3.json) |
| Forced-inline vector rescale | 1.461 / 1.670 / 1.091 / 1.044 | [JSON](results/f32-vector-rescale-inline-screen-3.json) |

These screens are diagnostic and do not replace 15-pair acceptance. The
forced-inline q8 samples were noisy; all four medians remain above 1.0.

## Ordinary-suffix inlining screen

The initial generic candidate's V3 ordinary suffix already emits direct
YMM `vfmadd*` instructions and uses a 40-byte stack frame. A separate
`#[inline(always)]` screen removed the suffix symbol but did not produce a
repeatable whole-transform gain. The matched 3-pair comparison used the
non-inlined generic candidate as parent; all outputs were bitwise identical.

| GL T720 scalar synthesis | Non-inline ms | Inline ms | DUCC ms | Inline/parent paired median |
|---|---:|---:|---:|---:|
| Full q1 | 26.358 | 26.303 | 19.532 | 0.997 |
| Full q8 | 3.930 | 3.847 | 2.968 | 0.976 |
| Reduced q1 | 8.925 | 9.000 | 8.412 | 0.998 |
| Reduced q8 | 1.697 | 1.821 | 2.497 | 1.062 (noisy) |

The q8 reduced samples varied enough that the DUCC ratio is inconclusive. The
other three cells remain slower than DUCC. Do not repeat this suffix-inlining
change unchanged. The exact patch, pair data, runner and baseline AVX2 excerpt
are in [`ordinary-suffix-inline/`](ordinary-suffix-inline/).

## Work and code generation

The initial candidate's full-M720 work-count probe recorded 3.15M scaled-pair
updates, 37.83M ordinary SIMD lane updates, and a maximum 341-pair scaled
prefix. Callgrind on the vector-rescale leaf recorded 940,371,839 Ir for one
warmed q1 transform; Ir are instruction references, not cycles. The
`TRACK_ANY=true` rescale entry was called 394,106 times and the
`TRACK_PREFIX=true` entry 24,604 times. The former call site attributed
106,092,851 Ir to the rescale leaf. The archived [V3 disassembly](codegen/vector-rescale-v3-symbol.asm)
shows a 640-byte stack frame, stack-backed lane flags, and repeated out-of-line
AVX setup. This matches the screen regression: the vector comparison and
multiply did not offset per-lane checked exponent work and setup overhead.

The Callgrind profile used `mode-alm0.f32` (SHA-256
`7207bfb3a8d7f57d4d44a53584f25bd79fa35aa3d520748ae57a57e08d1ae1a6`) and
produced a finite map (SHA-256
`bebd2b497df8be178af9af29df8eb1937efa6d68c2164efa31e24cfb3179d536`). The
[`compressed profile`](profiles/vector-rescale-f32-full-q1.callgrind.gz),
[harness](profiles/m6_f32_scalar_synthesis_callgrind.rs), and [client shim](profiles/callgrind-shim.c)
are retained for inspection. The profile fixture is separate from the timed
fixture; its instruction count is not a timing comparison.

## Numerical and f64 controls

The high-degree zonal probe at `sin(theta)=0.003333085` produced f32
`0.018705474` versus the independent f64 recurrence value `0.018541995` near
the pole. This diagnostic is not tolerance-asserting and remains an open
correctness caveat for the candidate.

A separate f64 GL T1023 spin-1 synthesis control (patch SHA
`1310f21c3335eb121253cab0182f912a66e40ee9e2cce4864a33da18dfc32fef`) stayed
about 3% slower than its parent at q1/q8, while remaining below DUCC at
0.767x/0.746x. It is retained as a protection warning, not a PASS.

The initial isolated source passed `cargo fmt --all -- --check`,
`git diff --check`, and `cargo test --release --features parallel --lib`
(317 passed, 37 ignored). The separate ordinary-suffix inlining candidate
passed the same format/diff checks and release lib tests (315 passed, 37
ignored). No production source or dispatch was promoted. Further work should redesign
the native-f32 scaled-prefix execution from its work and code-generation
evidence; the rejected scale and rescale variants above should not be repeated
unchanged.

`SHA256SUMS` covers all archived patches, result JSON, runners, profile files,
and the disassembly excerpt. No benchmark binaries or duplicate fixture files
are included.
