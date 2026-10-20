# Scalar-analysis SIMD code-generation follow-up

## Scope and timing

The comparison used `a707780` as the Rust baseline and pinned DUCC revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. Rust 1.99.0/LLVM 23.1.1 and GCC
15.2.0 generated optimized x86-64-v3 AVX2/FMA code on an AMD Ryzen 9 5950X.
The Rust screen reused plans and workspaces, with one warmup and five
interleaved calls. It timed full f64 GL analysis, including FFT and scheduling;
it is not a Legendre-only comparison. Aggregate RSS and hardware counters were
not collected. The candidate includes the preceding scaled-transition and
coefficient-flow changes, so the table does not isolate the workspace move.

| Workload | q | `a707780` | Candidate | Candidate / baseline | Relative L2 difference |
|---|---:|---:|---:|---:|---:|
| T720, full `mmax`, 721×1442 | 1 | 29.203 ms | 31.215 ms | 1.069× | `1.185e-17` |
| T720, full `mmax`, 721×1442 | 8 | 6.493 ms | 7.150 ms | 1.101× | `1.185e-17` |
| T300, `mmax=150`, 301×603 | 1 | 3.378 ms | 3.029 ms | 0.897× | `1.631e-18` |
| T300, `mmax=150`, 301×603 | 8 | 0.667 ms | 0.681 ms | 1.021× | `1.631e-18` |

This screen does not establish a general speedup. Reduced T300 q1 improved;
T300 q8 was near neutral. T720 regressed in this run. The q8 values are one
short screen and do not establish a stable regression. No DUCC ratio is inferred
from these full-call timings.

An earlier same-session T720 stage probe, before the workspace-only scratch
move, measured Rust recurrence plus accumulation at 25.56/3.73 ms (q1/q8) and
the pinned DUCC `leg2alm` stage at 10.819/1.899 ms. These intervals include
different preparation and output work, so their roughly 2.36×/1.96× ratios are
directional. The pinned DUCC source remains the reference; the 2026-10-18
[stage profile](scalar-analysis-stage-profile-2026-10-18.md) contains the
earlier source state and setup details.

## Assembly findings and source changes

Matched x86-64-v3-only builds of `fused_scalar_analysis_pair_block_impl::<true>`
reserved 1088 bytes at `a707780` and 896 bytes in the candidate, with the same
saved general-purpose registers. YMM stack-memory operands fell from 31 to 16;
fixed YMM stores fell from 18 to 3. The baseline's 512-byte cosine-square
table was prepared in a local array per block. Production now prepares cosine
squares in reusable workspace storage. The scaled route's 512-byte correction
table had become another local array; it is now per-worker workspace scratch.
Several remaining YMM accesses belong to scaled/checked transition branches,
including a stored vector reloaded at multiple branch sites. These are not
ordinary recurrence-loop spills. The frame size alone is not a count of
register spills.

The inspected DUCC `map2alm_kernel` clone at the pinned revision saves four
general-purpose registers, reserves no local stack space, and has no YMM/XMM
stack operands. The earlier 1208-byte Rust versus 416-byte DUCC comparison used
different function boundaries: Rust's block wrapper includes initialization
and safety routing, while that DUCC symbol is the recurrence kernel. It should
not be interpreted as a like-for-like spill comparison.

For T720, the existing active-ring counts are 3,384 blocks: 2,910 ordinary
initializations and 474 scaled initializations. The old ordinary loop reduced
a lane-validity vector after each coefficient pair, about 689,643 horizontal
reductions for this geometry. The candidate folds lane masks across pairs and
reduces once per block, about 3,384 reductions (99.5% fewer). Lane comparisons
remain in the loop; the subnormal/zero guard is retained. A per-pair upper
bound comparison was removed after factor preflight and the normalized-harmonic
bound were checked. The source-level geometry estimate indicates about 193k
repeated cosine-square multiplications avoided by per-latitude preparation;
the recurrence still reads cosine-square values as needed.

An out-of-line scaled-initializer experiment regressed the full-call screen at
both sizes and thread counts (ratios 1.19–2.49×) and was reverted. The accepted
workspace change preserves the existing inline route and numerical checks.
Do not restore multi-pair unrolling unchanged: it has already regressed q1/q8.
Next work should target the ordinary loop's remaining lane checks and data
flow, then revisit spin-1 after a scalar-analysis improvement is established.
