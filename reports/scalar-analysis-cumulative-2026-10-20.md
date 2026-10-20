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
| 3 | DUCC-style packed complex horizontal reduction | Candidate is gated by `rustsht_packed_complex_pair_reduction`; default remains the eight scalar f64 reductions. In matched T720 `m=1` kernel batches, kernel-only medians improved 2.8%, 2.5% and 1.1% at 64/128/256 active lanes. Kernel-plus-commit Rust medians improved 2.9%, 1.9% and 0.9%; numerical checks passed. A separate full-transform q1 screen was neutral-to-small but did not isolate the reduction; q8 was host-contended. Keep for cumulative evaluation; do not change the default yet. |
| 4 | Zipped versus safe indexed SIMD loop | Candidate is gated by `rustsht_indexed_analysis_pair_loop`; the production default remains zipped. At 64/128/256 active lanes the matched kernel-plus-commit Rust median changed by −0.25%/−0.34%/−0.17%, within overlapping distributions. Static code size fell 1.7%, but vector arithmetic, memory operands, and spill behavior were unchanged. No speedup was demonstrated. |
| 5–10 | Recurrence/accumulation attribution; output staging; sectoral initialization; prefix; scaled transition; input packing; final conversion | Continue in the owner-specified sequence after the reduction and loop-form ablations. |

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
