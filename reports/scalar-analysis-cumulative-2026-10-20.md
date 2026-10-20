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
separate reductions. Next, measure recurrence and accumulation components
with a same-dispatch benchmark before making another ordinary-loop change.
