# Cumulative scalar-analysis kernel experiments

This report tracks the ordered cumulative DUCC-parity experiments. The
starting comparison uses RustSHT `b1ebc31c`, Rust 1.99.1 (LLVM 23.1.1), Pulp
0.22.3 and DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
Rust uses strict floating-point semantics with x86-64-v3 runtime dispatch.
Standalone DUCC SHT code uses `-O3 -ffast-math -ffp-contract=fast
-march=x86-64-v3`; its driver uses `-fno-fast-math`. The Ryzen 9 5950X
selected Pulp V3 and fixed DUCC x86-64-v3 for the measured transforms.

## Experiment matrix

| Step | Change | Result and disposition |
|---|---|---|
| 1 | Scalar f64 analysis pair tile: 64/128/256 | Already implemented at `c19e5382`; production default is 256, with 64/128 controls. Active-pair counts fell from 3,384/1,851 to 1,139 at T720 and from 453/302 to 151 at T300. The prior three-process screen measured T720 full q1 at 27.581/26.462/26.467 ms and q8 at 4.487/4.342/4.290 ms; T300 reduced-order q1 at 2.859/2.752/2.704 ms. Keep 256 pending the combined ablation. See the [tile screen](archive/experiments/scalar-analysis-pair-grouping-2026-10-19.md). |
| 2 | Fixed-capacity contiguous state/spectrum scratch | Candidate is behind `rustsht_scalar_analysis_fixed_simd_arrays`; it packs previous/current/next and the four prepared spectrum planes as contiguous 256-element, 64-byte-aligned arrays. The full-transform screen suggests a small q1 benefit, but q8 is inconclusive under high scheduler wait. Retain for cumulative evaluation; the Vec layout remains the default during the experiment. |
| 3 | DUCC-style packed complex horizontal reduction | Retired after the current-source T720 A/B: isolated kernel-plus-commit medians improved 3.8%/1.2%/0.6% at 64/128/256 lanes, but full-transform changes were only −2.0%..+0.4% alone and −4.9%..+0.6% in combination with fixed scratch, with mixed signs. Numerical checks passed. The cfg branch and packed production path have been removed; measurements remain in the current [T720 A/B](#current-t720-productioncandidate-ab-2026-10-20). |
| 4 | Zipped versus safe indexed SIMD loop | Candidate is gated by `rustsht_indexed_analysis_pair_loop`; the production default remains zipped. At 64/128/256 active lanes the matched kernel-plus-commit Rust median changed by −0.25%/−0.34%/−0.17%, within overlapping distributions. Static code size fell 1.7%, but vector arithmetic, memory operands, and spill behavior were unchanged. No speedup was demonstrated. |
| 5 | Direct output accumulation | Restored as the opt-in `rustsht_scalar_analysis_direct_accumulation` cumulative candidate. It requires a cached recurrence proof and a whole-latitude product bound; otherwise the checked staged route remains active. The default is unchanged. The combined candidate passes release tests; its direct-versus-staged T511 coefficient comparison has relative L2 error `2.5974e-16` and maximum absolute error `2.5324e-17`. The earlier direct-only T720 screen regressed, so this candidate still needs measurement in the final combined 64/128/256-lane evaluation. No new timing was collected while other host jobs saturated CPU capacity. |
| 6–10 | Sectoral initialization; insignificant-prefix recurrence; scaled-to-normal transition; input packing; final coefficient conversion | Continue with the source-guided experiments after the staging candidate. |
| 11 | Scaled-preflight frontier fusion | Correction preparation returns the below-one and all-negative epoch predicates, removing two follow-up epoch-array scans. The range-bound check and correction validation remain. Focused SIMD-tail/invalid-value and full release/no-default library tests pass; no performance measurement is available under host contention. Keep in the cumulative candidate pending a stable comparison. |

## Scaled-preflight frontier fusion

The scaled block now derives `below_limit` and the insignificant-prefix
`all_negative` state while it writes its correction factors. This replaces the
separate SIMD below-one reduction and scalar all-negative scan. The recurrence
epoch-bound preflight remains a separate full pass, and any failure still
returns before staged coefficients are touched. Invalid or partial correction
scratch is discarded with the declined fast path. Tests compare the fused
predicates to the prior SIMD/scalar results for SIMD-width, tail, and mixed
epoch cases, and reject NaN, infinity, and out-of-range correction products.
The 230-test release and 192-test no-default-feature library suites pass. The
host was carrying a load average near 39 on 32 logical CPUs, so no timing was
collected and no speedup is claimed.

## Direct-output candidate validation

The direct-output candidate is compiled only when
`rustsht_scalar_analysis_direct_accumulation` is set. It can write output only
after the workspace cache proves that the same plan/order/tile geometry passed
the checked recurrence and the input preflight bounds the absolute sum across
all reflected latitude lanes. Scaled seeds, subnormal products, unsafe values,
and unproven blocks keep the staged checked path. The candidate preserves the
existing transactional fallback for any decline before output writes.

The combined 256-lane candidate with fixed-capacity scratch, packed complex
reduction, the safe indexed loop, and direct output passed 229 release library
tests (23 ignored) and 191 `--no-default-features` release library tests (14
ignored). All-target, all-feature Clippy passed with warnings denied. The
focused cached-versus-checked output test reports 37,253 bitwise-different
coefficients from the changed accumulation order, relative L2 error
`2.5974e-16`, and maximum absolute error `2.5324e-17`; its error bound is
`4e-14`, matching the existing tight low-degree DUCC reference tolerance.
Subnormal, extreme-finite, zero, SIMD-tail, DUCC, analytic, adjoint, and
thread-count tests also passed in the combined build. These checks establish
correctness only. Re-run calibrated performance comparisons after host CPU
contention clears and after the remaining cumulative candidates are available.

The initial fixed-scratch comparison is full native synthesis and analysis,
including packing, Legendre work, FFT and output production. Ten GL geometries
cover T300 with `mmax=75,300` and T720 with `mmax=1,180,720`, each with
odd-minimum and even-smooth longitude sampling. It has 40 operation/thread
rows: both directions at q1/q8. Every operation used three interleaved
Rust/DUCC process pairs and ten calibrated batches per process/backend; the
short-operation target was 25 ms. The same deterministic fixture inputs were
used in both runs.

## Fixed-scratch results

| Threads | Rows | Median candidate/baseline Rust latency | Median change in Rust/DUCC ratio | Candidate faster rows |
|---:|---:|---:|---:|---:|
| q1 | 20 | 0.9861× | 0.9838× | 18/20 |
| q8 | 20 | 0.9691× | 0.9990× | 11/20 |

The q1 result is a modest directional improvement: candidate Rust latency was
about 1.4% lower at the median, and its median Rust/DUCC ratio was about 1.6%
lower. Individual Rust/DUCC ratio changes ranged from 0.930× to 1.014×. At q8,
the candidate/baseline latency ratios ranged from 0.658× to 1.360×, and
Rust/DUCC ratio changes ranged from 0.567× to 1.345×; this run does not
establish a q8 effect.

Numerical comparison completed 120 outputs. Maximum relative L2 error was
`1.841e-13`; maximum absolute error was `3.742e-10`. The candidate did not
change the recurrence or numerical guards. The production route at T720 is
the ordinary fused scalar analysis recurrence, since the compact table is
limited to T159.

Host contention was high in both runs, so q8 timing attribution is uncertain;
all timing samples were retained. Raw process and host samples remain local.
Per-case medians, p10/p90, DUCC comparisons and numerical errors are in the sanitized
[machine-readable table](data/scalar-analysis-cumulative-2026-10-20/fixed-scratch/summary.csv).
The benchmark setup and commands are in the adjacent [reproduction notes](data/scalar-analysis-cumulative-2026-10-20/fixed-scratch/README.md).

## Code-generation evidence

The Pulp V3 fused ordinary `FusedScalarAnalysisPair<true, false>` symbol in
matched optimized release test binaries remained 6,113 bytes with 1,238
instructions, 112 packed FMA instructions and 59 YMM memory operands. Neither
build used YMM stack spills. This layout change did not alter the recurrence
loop's generated instruction counts.

In the full-transform benchmark binary, the enclosing
`ShtPlan::analyze_symmetric_order_group` symbol decreased from 7,467 to 6,456
bytes. In a matching unit-test build its size decreased from 8,428 to 7,622
bytes, with instruction count 1,636→1,457, call count 71→49 and memory
operands 855→742. This is caller and workspace-addressing simplification;
there is no evidence that it improved the fused arithmetic loop itself. The
seven scratch planes use the same 14,336-byte f64 payload per worker as seven
256-element vectors; the candidate removes those seven data allocations and
places the arrays inline. Peak RSS was not measured in this screen.

## Packed complex reduction ablation

The source baseline was `1548b5c5`; candidate `0db88bd5` adds a cfg-gated
Pulp reduction and an exact lane-sum test. With four f64 lanes, it interleaves
the real/imaginary accumulator vectors and reduces two complex vectors per
coefficient pair. Other lane widths and the default build keep separate f64
reductions. The recurrence, product preparation, range checks, safety cache,
and scaled fallback are unchanged.

The direct comparison used T720 GL, `m=1`, checked-proven warmed recurrence,
and 64/128/256 active-ring lanes. Rust 1.99.1 used Pulp V3 and strict FP under
`-C target-cpu=x86-64-v3`; DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`
used GCC 15.2, an AVX2/FMA v3 kernel compiled with `-O3 -ffast-math
-ffp-contract=fast -march=x86-64-v3`, and a strict driver. Ten alternating
process pairs produced 60 calibrated 20–100 ms batches per backend, lane
count, and timing boundary. The unchanged fixture and output checks were used
for both binaries.

| Active lanes | Rust kernel only, base→candidate (µs) | Change | Rust kernel+commit, base→candidate (µs) | DUCC kernel+commit, base/candidate (µs) | Paired Rust/DUCC, base→candidate | Candidate relative L2 |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 7.821→7.601 | −2.8% | 8.024→7.789 | 8.117 / 8.122 | 0.990→0.959 | `6.670e-14` |
| 128 | 14.250→13.894 | −2.5% | 14.400→14.130 | 15.768 / 15.861 | 0.914→0.889 | `1.628e-13` |
| 256 | 26.574→26.270 | −1.1% | 26.708→26.464 | 30.994 / 30.806 | 0.862→0.861 | `5.928e-14` |

The candidate’s Rust process-median spread was 1.3–3.3%; the baseline spread
was 1.7–6.3%. DUCC spread was 3.1–7.9%. The total kernel-only improvement is
small at 256 lanes, where recurrence and accumulation remain the larger body
of work. Numerical maximum
absolute errors were `5.009e-12`, `1.896e-11`, and `2.559e-11` at 64/128/256
lanes; the scaled coefficient difference was `1.970e-16` for each case.

In the optimized V3 pair symbol, the packed candidate reduced code size from
6,113 to 5,648 bytes and the `objdump` instruction count from 1,257 to 1,170.
Packed `vfmadd*pd` increased from 46 to 58, scalar-double FMA decreased from
66 to 42, and all counted scalar-double instructions decreased from 159 to
96. YMM memory operands increased from 108 to 116; neither version spilled
YMM values to the stack. These are whole-symbol static counts, not dynamic
loop counts. The candidate exchanges some scalar reduction work for vector
interleave/reduction work; timing, rather than the instruction counts, is the
evidence for the modest kernel improvement.

A full-transform check used T720 GL `mmax=1`, `ntheta=721`, odd-minimum
`nphi=3`, analysis and synthesis, q1/q8, and three process pairs with ten
25 ms-target batches per backend. Baseline versus the cumulative fixed-scratch
plus packed-reduction candidate measured q1 analysis at 175.383 versus
172.493 µs and q1 synthesis at 240.252 versus 240.224 µs. Their paired Rust /
DUCC ratios were 0.733→0.709 for analysis and 1.073→1.069 for synthesis.
These were separate runs, and the candidate also enables fixed scratch, so
the q1 difference is directional and does not isolate the reduction. q8
ranges overlapped broadly, and run-queue contention was high in both runs; do
not attribute a q8 change. A separate reduction-only full-transform
invocation entered a different timing regime: both Rust and DUCC latencies
shifted substantially, so it is retained in the CSV as a host-regime
diagnostic and excluded from effect estimation.

The kernel candidate remains compile-time gated. The T720 q1 full-transform
screen supports no production policy change, and the q8 result is
inconclusive. Next, compare the recurrence and four-spectrum accumulation
costs separately, then combine only candidates whose individual and
cumulative timings remain stable. Machine-readable results and reproduction
commands are in [the packed-reduction data directory](data/scalar-analysis-cumulative-2026-10-20/packed-reduction/README.md).

## Zipped versus indexed SIMD loop

The comparison used baseline `68381d3d` and candidate `2d4fd6b6`. The
candidate keeps the same per-vector recurrence and eight accumulators in a
shared macro, then visits validated equal-length SIMD slices using safe
indices under `rustsht_indexed_analysis_pair_loop`. The default build still
uses the existing zipped iterators. No recurrence, accumulation, safety check,
or reduction arithmetic changed.

Both builds used Rust 1.99.1, strict FP, Pulp V3, and the same native kernel
harness as the packed-reduction comparison. DUCC was compiled with GCC 15.2
using `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its driver was
strict. Ten alternating Rust/DUCC process pairs collected 60 calibrated
20–100 ms batches at each lane count, with matched T720 GL `m=1` inputs.

| Active lanes | Rust kernel only, zipped→indexed (µs) | Rust kernel+commit, zipped→indexed (µs) | DUCC kernel+commit (µs) | Paired ratio, zipped→indexed | Relative L2 |
|---:|---:|---:|---:|---:|---:|
| 64 | 4.728→4.735 | 4.861→4.849 | 4.108 / 4.104 | 1.182→1.179 | `6.670e-14` |
| 128 | 8.747→8.745 | 8.892→8.862 | 8.043 / 8.046 | 1.105→1.102 | `1.628e-13` |
| 256 | 17.057→17.072 | 17.237→17.207 | 15.948 / 15.819 | 1.083→1.087 | `5.928e-14` |

The p10–p90 ranges overlap at all lane counts. The indexed 64-lane run had
one process median outside the other samples; the benchmark retained it and
flagged that lane's process spread as unstable. Measured run-queue wait was
negligible in both executions, so no timing samples were filtered.

The executed Pulp V3 pair symbol decreased from 6,113 to 6,008 bytes and from
1,257 to 1,228 disassembled instructions. It retained the same 46 packed
`vfmadd*pd`, 66 scalar-double FMA, 159 total scalar-double, and 108 YMM memory
instructions; both binaries had zero YMM stack operands. The static change is
consistent with less iterator control, but it did not improve measured time.
Keep the indexed form only as a compile-time candidate for cumulative
evaluation; do not change production loop selection. See the
[machine-readable comparison and reproduction notes](data/scalar-analysis-cumulative-2026-10-20/indexed-loop/README.md).

## Cumulative packed-reduction plus indexed-loop screen

At source `5f0158a8`, a matched direct-kernel screen compared the default
zipped/separate-reduction path with both existing cfg candidates enabled:
`rustsht_indexed_analysis_pair_loop` and
`rustsht_packed_complex_pair_reduction`. Rust 1.99.1 used strict FP and Pulp
V3; pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2 with
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for its SHT kernel and
strict FP for the driver. Ten interleaved process pairs collected 60 calibrated
batches per timing boundary and lane count.

| Active lanes | Rust kernel+commit, default→combined (µs) | DUCC, default/combined (µs) | Rust/DUCC, default→combined | Numerical relative L2 |
|---:|---:|---:|---:|---:|
| 64 | 4.887→4.676 | 4.151 / 4.145 | 1.179→1.130 | `6.670e-14` |
| 128 | 8.953→8.804 | 8.098 / 8.102 | 1.108→1.087 | `1.628e-13` |
| 256 | 17.567→18.300 | 16.345 / 17.001 | 1.070→1.079 | `5.928e-14` |

The 64- and 128-lane direct-kernel candidate medians were 4.3% and 1.7%
lower, respectively, but the 256-lane candidate was unstable: its process
spread was 30.6%, and maximum measured run-queue wait was 46.5%. These small
results do not establish a production benefit.

A second screen compared the full T720 transform at six geometries spanning
`mmax=1,180,720`, odd-minimum/even-smooth longitude counts, A/S and q1/q8
(24 operation rows, 72 numerical comparisons per build). The maximum relative
L2 error was `1.841e-13`. It is not a usable timing ablation: baseline and
candidate ran in different host regimes. Candidate median run-queue wait was
15.4% (p90 79.0%), versus 0.58% (p90 16.8%) for baseline. Both runs and all
samples are retained outside the repository; no timings were removed. The
two cfgs remain experimental and production still uses zipped iteration and
separate reductions. The subsequent recurrence/accumulation attribution is
recorded below.

## Recurrence and accumulation attribution

At source `4c6945fc` plus a benchmark-only alignment correction, the probe now
uses the same state and coefficient buffers and the same unchecked wrapper as
the ordinary kernel timing. An earlier run used separate scratch allocations;
its full-component times did not agree with the main timing and are excluded.
In the corrected run, the component full-path median matched the ordinary
kernel-only median within 0.4% at every lane count.

The benchmark used T720 GL, `m=1`, q1, and 64/128/256 active latitude lanes.
Rust 1.99.1 used Pulp V3 with strict FP and
`-C target-cpu=x86-64-v3`. DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`
used GCC 15.2; its SHT object used
`-O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3`, while its
driver used `-fno-fast-math`. Ten interleaved process pairs collected six
20–100 ms batches per process and lane count. All 360 component batches were
21.75–52.01 ms. The Ryzen 9 5950X frequency median was 3.923 GHz; host runqueue
wait median was 2.2%, with a 99.7% maximum outlier retained.

| Active lanes | Recurrence-only (µs) | Accumulation + reduction (µs) | Combined-path total (µs) | Component full / ordinary kernel-only | Rust/DUCC kernel + commit | Paired Rust/DUCC |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 3.706 | 3.781 | 7.498 | 7.498 / 7.484 | 7.681 / 8.154 | 0.940× |
| 128 | 6.901 | 6.820 | 13.717 | 13.717 / 13.762 | 13.869 / 15.837 | 0.874× |
| 256 | 13.438 | 12.096 | 25.551 | 25.551 / 25.529 | 25.785 / 30.859 | 0.838× |

The accumulation-plus-reduction branch contributes 47–50% of the ordinary
kernel time in this matched `m=1` fixture. It includes four-spectrum loads,
eight SIMD accumulation chains, horizontal reductions, and coefficient
construction; this probe does not separate those costs. Earlier reduction
ablations found less than 1% benefit from the private AVX2 reduction and
regressions from grouped reductions, so the evidence points to the spectrum
loads and accumulator chains as the larger part of this combined cost. No
individual share is claimed. The scalar V3 loop processes the two-pair group
with four recurrence FMAs and eight accumulation FMAs per SIMD vector.

The current direct warmed kernel-plus-commit timing is faster than DUCC at all
three widths, with relative L2 errors of `6.670e-14`, `1.628e-13`, and
`5.928e-14`. This result does not establish full-transform or q8 parity. The
next step is to separately measure horizontal reduction and coefficient
construction, then profile the largest remaining full-transform stage before
changing the ordinary production loop. The first retained-vector attempt was
invalid and is documented below.

### Failed retained-vector split

A follow-up benchmark-only variant kept the eight SIMD accumulation chains,
passed their vector results through `std::hint::black_box`, and omitted the
horizontal reductions. It used the same Rust/DUCC build and q1 T720 GL `m=1`
work, with ten process pairs and six 20–100 ms target batches per case. Three
of 540 batches fell below 20 ms (minimum 19.12 ms); they were retained. The
host-frequency median was 3.968 GHz, and host runqueue wait was 0.92% median
with an 81.0% maximum outlier.

| Active lanes | Full path (µs) | Retained vectors, no reduction (µs) | Recurrence-only (µs) | Paired full-minus-retained delta (µs) |
|---:|---:|---:|---:|---:|
| 64 | 7.805 | 8.809 | 3.702 | −0.991 |
| 128 | 14.110 | 13.473 | 7.102 | +0.694 |
| 256 | 26.366 | 27.643 | 13.837 | −1.266 |

This split is invalid for reduction-cost attribution. The optimized Rust
dispatch path shows the eight vector accumulators being written to YMM stack
slots at the end of each two-pair group so the black-box sink can observe them.
That artificial memory traffic made the retained-vector variant slower than
the full path at 64 and 256 lanes, while the 128-lane delta had the opposite
sign. Recurrence-state equality and the standard Rust/DUCC numerical checks
passed (`6.670e-14..1.628e-13` relative L2), but those checks do not validate
the timing decomposition. The source patch is retained for review in the
[probe data directory](data/scalar-analysis-cumulative-2026-10-20/retained-vector-probe.md);
the invalid specialization was removed from `src/`. Do not interpret these
deltas as horizontal-reduction cost. A replacement probe needs an explicit
scratch sink with its write cost controlled, or a separately timed reduction
microbenchmark. The ordinary warmed kernel remains faster than DUCC in the
matched comparison above, so no production recurrence change follows from
this failed split.

### Sectoral seed-prefix reuse candidate

When the ordinary SIMD sectoral initializer encounters an unsafe vector, it
returns the count of preceding vectors that passed its range checks. The scaled
pass now starts at that vector instead of recomputing the earlier integer
powers. It still prepares later vectors and the scalar tail with the existing
scaled exponent logic; invalid inputs still decline to the robust fallback.
Ordinary prefix scale epochs remain zero.

The focused test verifies a two-vector ordinary prefix followed by a scaled
vector and tail at the runtime SIMD lane width. With the cumulative
tile-256/fixed-scratch/packed-reduction/indexed-loop/direct-output cfgs, release
library tests passed (230 passed, 23 ignored), no-default-feature release
library tests passed (192 passed, 14 ignored), formatting passed, and
all-target/all-feature Clippy passed. No timing was collected because unrelated
host CPU work saturated the machine. This change remains an unmeasured
cumulative candidate; do not infer a latency improvement before the matched
64/128/256-lane and representative full-transform rerun.

### Scaled-prefix epoch-state reuse candidate

The scaled two-pair prefix previously scanned the `scale_epochs` slice after a
rescale pass to decide whether every epoch remained negative. Epochs only
increase in this prefix. The SIMD recurrence now carries that condition while
it already updates vector epochs and scalar tail lanes, and returns it with the
existing transition state. If the input state was not all-negative, it stays
false; if it was, changed vectors and tail epochs determine whether it remains
true. This preserves the validation gate while removing the follow-up scalar
read of the epoch slice.

The focused vector/tail test compares the carried condition with a direct
post-update scan, including the case where an epoch advances from -1 to 0 while
remaining below the ordinary-scale boundary. The cumulative configuration
passes 230 release library tests (23 ignored), 192 no-default-feature release
library tests (14 ignored), formatting, and all-target/all-feature Clippy. No
timing was collected because the host remained contended. Retain as an
unmeasured candidate pending the matched lane-count and full-transform
ablation.

## T720 production versus cumulative candidate (2026-10-20)

The matched full-transform A/B used RustSHT source `fbba7c39`, Rust 1.99.1
(LLVM 23.1.1), strict x86-64-v3/Pulp V3 and DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (GCC 15.2, DUCC SHT flags
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`). The six T720 cases
covered `mmax=1,180,720`, odd-minimum and even-smooth longitude counts, A/S,
q1/q8, three interleaved process pairs and 10 calibrated batches per backend
and row. The cumulative Rust build enabled fixed SIMD scratch, packed complex
reduction, indexed-loop traversal and direct output accumulation. Its results
are retained as an experimental candidate, not a production recommendation.

| mmax | longitude | op | threads | Rust production ms | Rust candidate ms | change | DUCC production → candidate ms | candidate Rust/DUCC | relative L2 |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|
| 180 | even-smooth | A | q1 | 6.409 | 16.765 | +161.6% | 5.923 → 5.951 | 2.817× | `4.975e-14` |
| 180 | even-smooth | A | q8 | 1.140 | 2.573 | +125.6% | 0.939 → 0.986 | 2.610× | `4.975e-14` |
| 180 | odd-minimum | A | q1 | 13.153 | 18.105 | +37.7% | 11.806 → 6.093 | 2.972× | `4.952e-14` |
| 180 | odd-minimum | A | q8 | 1.350 | 2.746 | +103.3% | 0.955 → 0.967 | 2.839× | `4.952e-14` |
| 720 | even-smooth | A | q1 | 20.834 | 41.115 | +97.3% | 27.095 → 14.603 | 2.816× | `4.657e-14` |
| 720 | even-smooth | A | q8 | 4.190 | 6.874 | +64.0% | 2.859 → 2.133 | 3.223× | `4.657e-14` |
| 720 | odd-minimum | A | q1 | 23.674 | 48.344 | +104.2% | 22.273 → 22.329 | 2.165× | `4.659e-14` |
| 720 | odd-minimum | A | q8 | 3.856 | 7.343 | +90.4% | 3.191 → 3.195 | 2.298× | `4.659e-14` |

The extreme reduced-order `mmax=1` rows are also included below. They are
short, low-work controls; their raw timing distribution is in the CSV. This
path did not show the warmed-analysis regression seen at larger `mmax`.

| longitude | op | threads | Rust production → candidate ms | change | DUCC production → candidate ms | candidate Rust/DUCC | relative L2 |
|---|---|---:|---:|---:|---:|---:|---:|
| even-smooth | A | q1 | 0.107827 → 0.107827 | +0.0% | 0.135974 → 0.135458 | 0.796× | `6.121e-14` |
| even-smooth | A | q8 | 0.089372 → 0.088678 | -0.8% | 0.138593 → 0.132577 | 0.669× | `6.121e-14` |
| odd-minimum | A | q1 | 0.100400 → 0.100561 | +0.2% | 0.133680 → 0.133283 | 0.754× | `5.277e-14` |
| odd-minimum | A | q8 | 0.081847 → 0.080131 | -2.1% | 0.132453 → 0.138873 | 0.577× | `5.277e-14` |
| even-smooth | S | q1 | 0.142745 → 0.143066 | +0.2% | 0.151587 → 0.152774 | 0.936× | `1.841e-13` |
| even-smooth | S | q8 | 0.157346 → 0.112220 | -28.7% | 0.143192 → 0.131518 | 0.853× | `1.841e-13` |
| odd-minimum | S | q1 | 0.144713 → 0.144470 | -0.2% | 0.151259 → 0.155575 | 0.929× | `1.077e-13` |
| odd-minimum | S | q8 | 0.145044 → 0.110371 | -23.9% | 0.136345 → 0.133240 | 0.828× | `1.077e-13` |

Synthesis was the unchanged-operation control. The shifts below are not
candidate effects; they show the run-to-run timing regime changed, especially
for q8.

| mmax | longitude | threads | Rust production → candidate ms | change | DUCC production → candidate ms | candidate Rust/DUCC | relative L2 |
|---:|---|---:|---:|---:|---:|---:|---:|
| 180 | even-smooth | q1 | 9.163 → 9.110 | -0.6% | 8.512 → 8.320 | 1.095× | `4.707e-14` |
| 180 | even-smooth | q8 | 2.122 → 1.743 | -17.9% | 1.228 → 1.224 | 1.424× | `4.707e-14` |
| 180 | odd-minimum | q1 | 12.434 → 10.858 | -12.7% | 11.476 → 8.627 | 1.259× | `5.195e-14` |
| 180 | odd-minimum | q8 | 2.387 → 2.000 | -16.2% | 1.361 → 1.270 | 1.575× | `5.195e-14` |
| 720 | even-smooth | q1 | 21.919 → 20.518 | -6.4% | 20.302 → 19.037 | 1.078× | `4.405e-14` |
| 720 | even-smooth | q8 | 5.980 → 3.927 | -34.3% | 3.608 → 2.709 | 1.450× | `4.405e-14` |
| 720 | odd-minimum | q1 | 28.339 → 28.257 | -0.3% | 27.517 → 27.906 | 1.013× | `4.448e-14` |
| 720 | odd-minimum | q8 | 7.789 → 5.143 | -34.0% | 4.419 → 3.894 | 1.321× | `4.448e-14` |

Across all 72 output comparisons the candidate maximum relative L2 error was
`1.841e-13` and maximum absolute error was `3.742e-10`. Its first analysis
call was close to production, then warmed analysis was much slower: T720
`mmax=180` even-smooth q1 changed from 7.997/6.409 ms (first/warm) to
7.879/16.765 ms, and full-order odd-minimum q1 changed from 27.283/23.674 ms
to 27.067/48.344 ms. This points at the direct-output route selected after
the cached checked proof. Source inspection also found that this route
performs a separate full-latitude product-bound scan and retries ordinary
sectoral initialization before scaled fallback can run. The direct-output
path was removed after the combined candidate reproduced the large warmed
regression. Exact first-versus-warm output comparison was false for 24 of 144
candidate worker samples; numerical comparisons still passed.

Do not attribute every row change to the candidate: synthesis was unchanged
by these flags but several synthesis rows shifted 6–34%, and the production
run-queue wait median/p90 was `0.97% / 15.4%` versus `0.48% / 10.1%` for the
candidate run. The no-direct combined rerun (fixed scratch, packed reduction,
indexed loop) was unusable: wait median/p90 rose to `13.7% / 67.5%`. Its
timings are excluded. Existing isolated kernel screens found fixed scratch
neutral at 128/256 lanes, packed reduction faster by 5.0/1.5/0.6% at
64/128/256 lanes (256-lane run unstable), and indexed traversal within 0.4%
of neutral. The indexed-loop candidate was removed; fixed scratch and packed
reduction remain gated for a later uncontended end-to-end comparison.

The T720 CSV summaries and reproduction notes are in
[`data/scalar-analysis-cumulative-2026-10-20/t720-ab/`](data/scalar-analysis-cumulative-2026-10-20/t720-ab/).

## T2047/T2048 seed-route profile

An ignored release test with test-only counters ran one full-triangle scalar
analysis for each geometry, using a zero map so recurrence routing depended on
geometry and factors rather than input spectral magnitude. This is route
counting, not timing. The source was based on `7cb427e8`; Rust was 1.99.1 with
the `parallel` feature. The tile was selected by the same compile-time cfg as
the production analysis workspace.

| Tile | lmax | ntheta | Normal seeds | Prefix-enabled scaled route | Checked-scaled route |
|---:|---:|---:|---:|---:|---:|
| 256 | 2047 | 2048 | 4039 | 2432 | 0 |
| 256 | 2047 | 2049 | 4144 | 0 | 2434 |
| 256 | 2048 | 2049 | 4146 | 0 | 2434 |
| 128 | 2047 | 2048 | 8259 | 3798 | 0 |
| 128 | 2047 | 2049 | 8369 | 0 | 3802 |
| 128 | 2048 | 2049 | 8373 | 0 | 3802 |
| 64 | 2047 | 2048 | 16735 | 6466 | 0 |
| 64 | 2047 | 2049 | 16856 | 0 | 6471 |
| 64 | 2048 | 2049 | 16861 | 0 | 6473 |

No prefix-initialization, prefix-recurrence, or checked-scaled recurrence
declines occurred. Crossing 2048 to 2049 latitudes moves thousands of
initially scaled blocks from the prefix-enabled scaled route to checked-scaled
recurrence. The counters identify route selection, not how many insignificant
pairs each block advances. At fixed 2049 latitudes, adding degree 2048 changes
the checked-scaled block count by zero at tiles 128 and 256 and by two at tile
64. This supports the cutoff as a likely contributor but does not measure its
latency cost. Keep the safety bound unchanged. Timings remain pending because
host scheduling was oversubscribed during this diagnostic.

Reproduce the route counts with the three tile builds:

```sh
cargo test --release --features parallel --lib report_t2048_scalar_analysis_seed_routes -- --ignored --nocapture --test-threads=1
RUSTFLAGS='--cfg rustsht_scalar_analysis_pair_tile_64 -Aunexpected_cfgs' cargo test --release --features parallel --lib report_t2048_scalar_analysis_seed_routes -- --ignored --nocapture --test-threads=1
RUSTFLAGS='--cfg rustsht_scalar_analysis_pair_tile_128 -Aunexpected_cfgs' cargo test --release --features parallel --lib report_t2048_scalar_analysis_seed_routes -- --ignored --nocapture --test-threads=1
```

The timing comparison remains T2047/2048 latitudes, T2047/2049 latitudes, and
T2048/2049 latitudes with `mmax=2047`, `nphi=4096`, q1/q8, and tiles 64/128/256.
This separates the safety-bound crossing from the extra degree while keeping
the longitude FFT fixed.


## Current T720 production/candidate A/B (2026-10-20)

This rerun uses current source b25c011d4bd6a4f1cae832a16f20cd5a12611897,
Rust 1.99.1 / LLVM 23.1.1, strict -C target-cpu=x86-64-v3 with runtime Pulp
V3, and DUCC 0.42.0 at
42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b (GCC 15.2). DUCC numerical SHT
sources used -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; the
validation driver remained strict. The comparison differs only by compile-time
candidate flags: default production, fixed aligned SIMD scratch,
packed-complex reduction, and both. The retired direct-output and indexed-loop
routes were not present.

The full-transform matrix used T720 GL with mmax=180 and 720, odd-minimum
and even-smooth longitude counts (nphi=361/378 and 1441/1458), analysis and
synthesis, q1/q8. Rust and DUCC trials were interleaved within each of three
process pairs; each row has 30 timed batches per backend, with 25 ms target
batches. The Rust plan/workspace setup is outside the warmed call; first-call
latency and reset consistency are recorded separately. Pulp dispatch was V3
for all Rust rows. All outputs were checked against DUCC and the harness ran
its basis, roundtrip, adjoint, zero, subnormal and extreme-finite checks.
The largest relative L2 error was 5.20e-14; machine-readable rows retain
absolute and maximum errors, first-call times, p10/p90, paired intervals,
frequency samples and host run-queue summaries.

| mmax | longitude | threads | production ms | fixed scratch ms | packed reduction ms | combined ms | DUCC ms | combined/base | combined Rust/DUCC | relative L2 |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 180 | odd-minimum | q1 | 7.893 | 7.676 | 7.832 | 7.509 | 6.114 | -4.9% | 1.240x | 4.89e-14 |
| 180 | odd-minimum | q8 | 1.324 | 1.305 | 1.315 | 1.332 | 0.946 | +0.6% | 1.402x | 4.89e-14 |
| 180 | even-smooth | q1 | 6.420 | 6.385 | 6.293 | 6.316 | 5.930 | -1.6% | 1.057x | 4.91e-14 |
| 180 | even-smooth | q8 | 1.125 | 1.127 | 1.120 | 1.103 | 0.924 | -1.9% | 1.194x | 4.91e-14 |
| 720 | odd-minimum | q1 | 23.599 | 23.562 | 23.689 | 23.392 | 22.159 | -0.9% | 1.015x | 4.63e-14 |
| 720 | odd-minimum | q8 | 3.903 | 3.805 | 3.872 | 3.785 | 3.186 | -3.0% | 1.187x | 4.63e-14 |
| 720 | even-smooth | q1 | 16.382 | 16.154 | 16.245 | 16.004 | 14.596 | -2.3% | 1.101x | 4.63e-14 |
| 720 | even-smooth | q8 | 3.040 | 2.996 | 3.015 | 2.967 | 2.128 | -2.4% | 1.404x | 4.63e-14 |

These whole-call analysis changes are modest and not uniform. Fixed scratch
alone changes analysis by -2.8%..+0.2%; packed reduction alone by
-2.0%..+0.4%. The separate native kernel comparison used 10 independent
process pairs and six calibrated batches per pair (60 batches per backend at
each 64/128/256 lane count). Fixed scratch was neutral within the kernel
spread. Packed reduction changed the unchecked kernel-plus-commit medians
from 4.872/8.955/17.341 us to 4.689/8.849/17.229 us, respectively. Its
128-lane DUCC pairing was unstable; reset-inclusive ratios and all lane
results are in data/scalar-analysis-cumulative-2026-10-20/t720-ab-current/kernel/summary.csv.
Numerical output relative L2 stayed within 6.67e-14..1.63e-13. This kernel
result alone does not justify promoting packed reduction.

The separate full-matrix runs showed fixed-scratch q8 synthesis 12-18% faster
although synthesis does not use the analysis recurrence. A focused confirmation
then interleaved production and fixed-scratch Rust runs with DUCC at q8, across
four T720 mmax/longitude cases, five process pairs and 50 batches per
backend/case. Paired fixed/production Rust median ratios were
0.820/0.857/0.875/0.886 (18.0%, 14.3%, 12.5%, and 11.4% faster); q1 controls
were near-neutral. Maximum relative L2 errors matched between layouts
(4.41e-14..5.20e-14). Run-queue wait was low in this confirmation
(median/p90 0.30%/1.87% production and 0.31%/1.09% fixed). This is a real
end-to-end signal but the mechanism is not yet isolated; fixed scratch remains
opt-in pending a broader workspace/memory check.

Clean T720 full-transform bundles and the long-form comparison are in
reports/data/scalar-analysis-cumulative-2026-10-20/t720-ab-current/. The
interleaved q8 confirmation's compact data is interleaved-q8-synthesis.csv;
per-process host logs remain local. The 2026-10-20 fbba7c39 direct-output run
above is historical and is not used for current candidate conclusions.

## T2047/T2048 cutoff and pair-tile comparison (2026-10-20)

This full-transform screen used Rust source
`86f43486b75774bd30046cc78dc712aa28d58077`, Rust 1.99.1/LLVM 23.1.1 with
strict `-C target-cpu=x86-64-v3`, and DUCC
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. DUCC SHT sources used GCC 15.2
with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver used
strict floating-point semantics. Runtime dispatch was Pulp V3 versus fixed
DUCC V3. Every case used GL, `mmax=2047`, `nphi=4096`, and identical seeded
maps per geometry. The three geometries separate the latitude cutoff from
adding degree 2048. Three interleaved rounds rotated tile order around DUCC.
Each process performed a cold call and warmup, then either five whole-call
samples for operations at least 100 ms or ten calibrated batches. This yields
15 long-call observations per backend/tile and 30 shorter q8 batches where
applicable. All outputs were compared after timing; reset checks passed.

| Geometry | Threads | DUCC ms | Rust tile 256/128/64 ms | Paired Rust/DUCC ratio 256/128/64 | Relative L2 |
|---|---:|---:|---:|---:|---:|
| T2047, 2048 latitudes | q1 | 248.645 | 251.498 / 261.872 / 282.107 | 1.013 / 1.050 / 1.125 | 1.18e-13 |
| T2047, 2048 latitudes | q8 | 45.793 | 62.201 / 63.074 / 67.679 | 1.371 / 1.388 / 1.468 | 1.18e-13 |
| T2047, 2049 latitudes | q1 | 249.445 | 828.435 / 718.965 / 696.002 | 3.326 / 2.888 / 2.781 | 1.18e-13 |
| T2047, 2049 latitudes | q8 | 45.928 | 143.732 / 129.008 / 122.973 | 2.926 / 2.741 / 2.647 | 1.18e-13 |
| T2048, 2049 latitudes | q1 | 250.061 | 832.444 / 716.284 / 693.156 | 3.328 / 2.857 / 2.774 | 1.18e-13 |
| T2048, 2049 latitudes | q8 | 42.401 | 133.959 / 121.908 / 115.758 | 3.198 / 2.899 / 2.752 | 1.18e-13 |

The one-latitude increase at fixed T2047 and longitude sampling raises tile-256
Rust q1 latency from 251.5 to 828.4 ms while DUCC remains 248.6–249.4 ms.
This matches the route profile: 2,432 blocks use prefix-enabled scaled
recurrence at 2048 latitudes, while 2,434 use checked-scaled recurrence at
2049. This is the dominant measured cause. At the same 2049-latitude geometry,
adding degree 2048 has no material cost (T2047/T2048 Rust medians are within
the observed spread). Tile selection is a second, separate contributor:
tile 64 is 12.2% slower than tile 256 below the cutoff, but 14.4–16.7% faster
at 2049 latitudes. It only recovers part of the regression; the best 2049-row
Rust/DUCC ratios remain 2.65–2.79x. Do not replace the general tile policy
based on this narrow geometry.

The p10/p90, per-process wait/frequency summaries and errors are in
[`t2048-cutoff-current/summary.csv`](data/scalar-analysis-cumulative-2026-10-20/t2048-cutoff-current/summary.csv).
CPU PSI `full` remained zero; background activity raised `some` from a low
starting level, and an `ncdu` process briefly shared one q8-eligible physical
core. All samples are retained. The q1 cutoff effect is much larger than the
observed run-queue wait; q8 ratios are correspondingly more variable.
Reproduce with `scripts/benchmark_scalar_analysis_tile_cutoff.py`; build
instructions and exact commands are in the adjacent data README. The raw
per-process/host capture stays local and untracked.
