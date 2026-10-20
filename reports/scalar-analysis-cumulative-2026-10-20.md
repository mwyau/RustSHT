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
| 3 | DUCC-style packed complex horizontal reduction | Next. The AVX2 reduction experiments recorded earlier are not reintroduced unchanged. |
| 4–10 | Range-check placement; output staging; sectoral initialization; prefix; scaled transition; input packing; final conversion | Pending in the owner-specified sequence after the reduction experiment. |

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

The implementation remains experimental pending the cumulative 64/128/256
comparison and ablations. Next, implement a distinct packed reduction
candidate, measure its isolated and full-transform effects, and continue with
the remaining steps before selecting the final production layout.
