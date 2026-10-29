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
| Per-lane i32 exponent AVX2 restoration | 1.615 / 1.623 / 1.234 / 1.112 | [JSON](results/f32-scale-i32-screen-3.json) |
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

The per-lane i32 candidate adjusted normal f32 exponent fields in eight-lane
vectors and used the exact scalar scaler for zero, subnormal, overflow, and
underflow cases. Its full-q1 Callgrind profile recorded 548,274,765 Ir; the
scaler accounted for 202,461,043 Ir (36.93%) and prefix preparation for
114,781,841 Ir (20.94%). The screen missed DUCC in all four cells, so the
AVX2 lane operation did not offset its mask, exception, and prefix overhead.
Ir are instruction references, not cycles. The candidate used parent
`39dd8a883d21a269654e1b74de862ee2e97d22ee`, patch SHA
`8dfb9227e674fcd5f6a505de60503f9d016350fbe9d46276f9cade2541a66465`, binary
SHA `80351b1cc53c0f29465f4551d9c34849c364843d1631dad1f36bc86230b01fd5`,
and the pinned DUCC binary. The compressed profile is in
[`profiles/scale-i32-vector-full-q1.callgrind.gz`](profiles/scale-i32-vector-full-q1.callgrind.gz);
its original scratch profile SHA-256 was
`332530fa2d813a567d4f9960c9030e624c3f215a0cd8a7ca3e13e0e12a16fdb2`.

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

## Exact-zero scaled-basis restore screen

The native-f32 scaled prefix can avoid calling the exact scalar scaler when
every lane is proven to restore to signed zero. With f32 exponent step 80,
scale at most -2 and mantissa magnitude at most 2^-60 bound the restored
magnitude by 2^-220, below half the least subnormal. The shortcut checks every
lane, finite input and checked exponent multiplication, preserves signed zero,
and leaves the recurrence update intact. It is guarded by the f32 precision
policy; the f64 instantiation does not select shared-basis prefix execution.
No production dispatch was changed.

The precision-gated candidate was measured against the generic parent and
pinned native-f32 DUCC with 15 balanced interleaved pairs per cell. Ratios are
paired medians; intervals are 50,000-resample percentile bootstrap intervals
over paired ratios (seed 1729).

| GL T720 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent | Candidate/DUCC | Status |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 26.653 | 24.852 | 19.484 | 0.926 [0.922, 0.940] | 1.272 [1.254, 1.286] | GAP |
| Full q8 | 3.927 | 3.658 | 2.932 | 0.931 [0.920, 0.948] | 1.244 [1.207, 1.255] | GAP |
| Reduced q1 | 9.013 | 8.491 | 8.681 | 0.940 [0.935, 0.956] | 0.980 [0.970, 1.009] | INCONCLUSIVE |
| Reduced q8 | 1.745 | 1.597 | 2.050 | 0.932 [0.796, 0.979] | 1.015 [0.718, 1.161] | INCONCLUSIVE |

Pulp used the V3 AVX2 route with eight native f32 lanes. Candidate output was
bitwise equal to the generic parent in all 15 pairs per cell; all outputs were
finite and repeat/reset checks passed. This parent equivalence does not settle
independent accuracy. The existing near-pole zonal diagnostic remains
`0.018705474` in f32 versus `0.018541995` from the independent f64 recurrence.
The candidate is not numerically accepted and remains isolated.

The first, ungated shortcut had a repeatable protected f64 T1023 spin-1
synthesis q1 slowdown (candidate/parent 1.013, 95% interval 1.007–1.021).
Adding a compile-time precision gate changed the f32 code generation, so the
full f32 matrix was rerun. On the gated source, f64 output stayed bitwise equal
to parent: q1 candidate/parent was 0.996 [0.991, 1.002], while q8 was
1.016 [0.999, 1.055] and remains INCONCLUSIVE under host noise. Candidate/DUCC
medians were 0.733 q1 and 0.764 q8. The full paired data contains fixture and
binary hashes.

A subsequent variant skipped the four accumulation FMAs for an all-zero
restored vector while retaining recurrence updates. It remained bitwise equal
to the parent and passed the release library tests (316 passed, 37 ignored),
but the 15-pair f32 matrix still missed full-grid DUCC at 1.282 q1 and 1.276
q8; reduced q1 was 1.012 and reduced q8 was too noisy to decide. This does not
demonstrate an added whole-transform gain over the precision-gated shortcut,
so do not repeat the FMA-skip variant unchanged. Its three-pair protected f64
control was interrupted before completion and produced no result; there is no
f64 conclusion for that last variant.

## Uniform-exponent f32 SIMD scaler follow-up

The next isolated patch adds an exact eight-lane f32 scaler for the common
binary exponent `-80`. It adjusts normal exponent fields directly and uses
integer significand rounding for subnormal outputs; signed zeros and input
subnormals are preserved, while vectors with non-finite lanes decline to the
existing checked scalar path. Other exponents and the f64 instantiation also
use the exact scalar path. The source diff is based on
`b6a96aa7161a64f678b614d73478496f50f0551e`, SHA-256
`9d5c7b578aa2298ca774d263c7f1c15932473d4ba640cf5fad12f80353e9fd4f`;
the candidate f32 binary is
`d9158760d31937718f43e2ad09542a60ca2ca90e0018164238b2445345d3d932`.

The full matrix used 15 balanced interleaved candidate/parent/DUCC pairs,
50,000 paired-bootstrap resamples (seed 1729), native f32 coefficients,
recurrence, FFT, and output. The candidate was bitwise equal to its parent in
all 60 outputs, finite, reset-consistent and used V3/AVX2 with eight f32
lanes. Those checks do not establish independent accuracy.

| GL T720 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/DUCC paired median (95% CI) | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 26.647 | 24.474 | 19.618 | 1.249 (1.233–1.258) | GAP |
| Full q8 | 4.217 | 3.739 | 2.998 | 1.256 (1.135–1.320) | GAP |
| Reduced q1 | 8.993 | 8.519 | 8.530 | 0.995 (0.992–1.009) | INCONCLUSIVE |
| Reduced q8 | 1.603 | 1.671 | 1.504 | 1.082 (1.032–1.148) | GAP |

The candidate improved the generic parent by 5–10% in three cells, but the
reduced q8 median regressed and three cells remain above DUCC. A matched
three-pair comparison against the prior precision-gated shortcut did not
show a stable broad gain. Work counters recorded 107,235 full-grid and
21,158 reduced-grid scale-`-80` SIMD groups. The archived AVX2 excerpt
[`uniform-scale-v3-prepare-prefix.asm`](codegen/uniform-scale-v3-prepare-prefix.asm)
shows YMM integer shifts, adds/subtracts, comparisons and blends in the
scaled-prefix helper. It does not remove the larger recurrence/loop gap.

A 15-pair protected f64 GL T1023 spin-1 synthesis control remained bitwise
equal to parent in all 30 outputs. Candidate/parent was `0.999 [0.987, 1.006]`
q1 and `0.982 [0.944, 1.000]` q8; candidate/DUCC was `0.754` and `0.750`.
Treat this as a preserved f64 control, not evidence of a meaningful f64 gain.

The high-degree near-pole zonal result remains `0.018705474` in f32 against
`0.018541995` from the independent f64 recurrence at the same f32 sine
(0.88% relative difference). The probe is currently diagnostic-only and does
not assert a tolerance. The candidate is therefore numerically unaccepted,
remains isolated, and has not changed production dispatch. Library tests
passed (`317 passed, 37 ignored`), including exact bitwise scaler coverage;
the probe passing as a test is not an accuracy pass.

## Prepared m=0 basis dot-product diagnostic

A later isolated variant stores a degree-major high/low m=0 basis for GL T720
and replaces the output m=0 Fourier column with a compensated native-f32 dot
product. Against an independent recurrence, it reduces the isolated T720
`l=720,m=0` maximum error from `2.759e-3` to `1.483e-8`. It does not improve
random whole-transform error against DUCC (`1.864e-4` full and `1.885e-4`
reduced relative L2) and adds `1–45%` over its uncorrected parent in the
three-pair screen. Its plan construction also computes the full m=0 basis
with an f64 recurrence, beyond the allowed small plan-time constant
preparation. Reject it for implementation; do not retain a second
post-transform correction pass. Exact source, hashes, tests, work counts,
assembly and timings are in [`zonal-basis-dotproduct/`](zonal-basis-dotproduct/).

The full 15-pair results, the 3-pair gated A/B, protected f64 data, runner and
candidate diff are retained under `results/`, `runners/` and `patches/`.
Their hashes are listed in `SHA256SUMS`.

## Near-pole accuracy check and double-single feasibility

A small driver calls pinned DUCC's public f32 `alm2leg` stage at
`theta=asin(sine_f32)` for the same zonal input used by the Rust probe:
`lmax=720`, `m=0`, and `alm[720,0]=1`. It returns `0.01854199543595314`
(`0x3c97e562`); the independent f64 recurrence gives
`0.01854199474347936`. The isolated Rust candidate returns
`0.018705474212765694` (`0x3c993c39`), an absolute error of
`1.6347947e-4` and relative error `0.8817%`. The C++ driver, structured
values, and test-only arithmetic patch/snippet are in `probes/` and
`results/near-pole-ducc-double-single-probe.json`.

DUCC 0.42.0's scalar Legendre inner loop uses a double SIMD state
(`Tv=native_simd<double>` in its pinned `sht_inner_loop.h`) even with f32 I/O.
This explains the f64-level result; it is reference behavior, not a Rust
implementation direction.

A test-only f32 double-single prototype then represented each recurrence
value as `hi + lo`, used f32 `mul_add` residuals, and kept the wide-prepared
factors and cosine-squared constant as high/low f32 pairs. With the candidate's
f32 sectoral seed it returned `0.01854199372991772`, `1.014e-9` absolute from
the f64 reference; the test asserts both DS cases remain within `2e-7`.
Negative controls show both pieces matter: retaining only rounded f32
`cos(theta)^2` returns `0.0323911674`, while storing the prepared recurrence
and output factors only as their f32 high parts returns `0.0070728625` even
with exact high/low geometry. These are arithmetic feasibility probes, not a
production fix or a performance result. Any follow-up must retain low parts
for the plan-invariant recurrence constants and polar `cos(theta)^2`, combine
runtime f32 coefficients using f32 compensated operations, and keep the
ordinary recurrence path unchanged unless measurements justify broader use.

The follow-up source remains isolated. Its direct DUCC driver, Rust test
snippet, incremental test patch and numeric output are checksummed. No
production dispatch changed.

The three recoverable source diffs are based on parent
`b6a96aa7161a64f678b614d73478496f50f0551e`: ungated shortcut
`be8737d3d1e44fdea5431648e7bb8408082e74cefc120eb3850785e059c4edf8`,
precision-gated shortcut
`57e58a06c46b3af90209c239c2fda419ee8edf7c19b0d7b2a8d3c9742a70bbd3`, and
FMA-skip follow-up
`f3c6da5709baf8f0d16bfde5735089b73cd98c13481f7e3507036c127484ef13`.
See `patch-index.tsv`, the compressed result files and the environment-driven
runners for exact binary hashes, fixtures and pair records. No build artifacts
or duplicate fixtures are retained.

A separate test-only work-count probe measured ordinary-suffix fusion in the
same generic candidate. The two-vector fused path handled 74.5541% of full
M720 and 83.7808% of reduced M180 ordinary lane-pair updates. It is already
used for most ordinary recurrence work, so suffix pairing alone is not the
next target. The probe also counted all-lane scale `<= -3` groups; this does
not establish that their accumulated contributions are safe to omit. See the
[full work-count record](results/f32-suffix-fusion-work-count.txt) and
[test-only instrumentation patch](patches/native-generic-scalar-suffix-fusion-counters.patch.gz).

## DUCC geometry and recurrence-setup comparison

A direct GL T720 comparison used the pinned DUCC `GL_Integrator` and
`Ylmgen::prepare` alongside the isolated candidate. All 720 f64 angles,
sine, cosine and cosine-squared values match bitwise. Candidate f32 sine and
cosine are the nearest f32 values; its f32 cosine-squared differs from DUCC's
f64 value by at most two ULP. For 968 `m=1–16,l=600–720` pairs, the actual
candidate f32 alpha, recurrence A/B and epsilon tables match nearest-f32
rounding of DUCC's double tables. Prepared even/odd coefficient pairs differ
by `3.97e-8`/`3.61e-8` relative L2. Source inspection confirms that DUCC's f32
API keeps scalar Legendre work arrays and recurrence state in double before
narrowing the result to f32. This explains the reference precision but does
not justify a Rust f64 backend. The source, test-only dump patch, raw values,
reproducer and caveats are in [`setup-comparison/`](setup-comparison/).

## Compensated recurrence replay and vector cost screen

A test-only twofold f32 replay lowers the selected `m=1–16,l=600–720`
recurrence error from `1.64e-4` to `1.20e-12` relative RMS when both the
`A*cos²+B` contraction and state update are compensated. Compensating only
the state while retaining the rounded f32 contraction leaves `1.63e-4`
relative RMS error against the full widened recurrence. The AVX2 full twofold
recurrence plus real even/odd accumulation took `17.24x` the ordinary f32
microkernel median across 15 interleaved pairs. Its emitted vector loop has
116 instructions per pair in this test callback, versus 19 in the ordinary
callback. This is not a whole-transform result and does not justify production
dispatch. Keep the implementation isolated while testing a cheaper residual
recovery method. Full evidence, exact test patch, logs and assembly are in the
[`compensated-recurrence-probe/`](compensated-recurrence-probe/).

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
ignored). No production source or dispatch was promoted. Further work should
redesign native-f32 scaled-prefix execution from its work and code-generation
evidence. In particular, per-lane exponent SIMD restoration is a measured
rejection and should not be repeated unchanged.

`SHA256SUMS` covers all archived patches, result JSON, runners, profile files,
and the disassembly excerpt. No benchmark binaries or duplicate fixture files
are included.
