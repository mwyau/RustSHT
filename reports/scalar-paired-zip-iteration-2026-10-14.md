# Scalar paired synthesis iterator experiment

## Hypothesis and change

The scalar f64 fused synthesis loop indexes four coefficient slices for each
degree pair. AVX2 disassembly of the baseline showed four bounds-check branches
inside the pair loop. A temporary candidate sliced the arrays once and
iterated the recurrence factors and transformed even/odd coefficient rows
together, cloning the bounded iterator for each latitude vector and scalar
tail. It preserved the recurrence and accumulation order and added no memory. The x86-v3 AVX2 `FusedSynthesisBlock` function had 68 static conditional
branch instructions in the baseline and 56 in the candidate. This is a static
disassembly count, not an executed instruction or cycle count. It confirmed
the intended code-generation change; whole-transform measurements determined
whether to keep it.

## Native measurements

Baseline source was `79b9eff`; the candidate was a temporary working-tree
change, now removed. Measurements used `examples/m1b_bench`, f64 GL scalar
STANDARD synthesis, full `mmax`, `phi0=0.37`, exact GL grids, reused plan and
workspace, one warm-up and five timed transforms per process. CPU affinity was
one CPU core for q1 and 8 pinned CPU cores for q8. Five alternating fresh-process pairs were
run for each band/thread case, except T720 q8, which was extended to ten
pairs because initial results were noisy. The table uses the median paired
percentage reduction `(baseline-candidate)/baseline`; positive values favor
the candidate.

| Grid | Threads | Baseline median | Candidate median | Paired change | Candidate faster |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 (`512x1025`) | 1 | 43.481 ms | 41.605 ms | +4.57% | 5/5 |
| T511 (`512x1025`) | 8 | 12.274 ms | 12.249 ms | -0.23% | 2/5 |
| T720 (`721x1443`) | 1 | 105.003 ms | 101.243 ms | +3.67% | 4/5 |
| T720 (`721x1443`) | 8 | 36.414 ms | 36.182 ms | -3.39% | 2/10 |

The q1 changes were consistent and useful. q8 T511 was flat. The extended
T720 q8 paired comparison favored the candidate in only two of ten pairs;
the paired median was 3.39% slower. Individual q8 process medians varied
substantially under changing shared-host contention. Because the shared kernel did not improve q8
work and showed a repeated T720 regression, the candidate was removed rather
than introducing a separate q1-only kernel and selector. Structured per-process benchmark results are in
[`data/scalar-paired-zip-iteration-2026-10-14`](data/scalar-paired-zip-iteration-2026-10-14/). The `baseline-*` files contain five-process baseline screens;
`paired-*` files retain the alternating comparisons and within-process
samples. The extended T720 q8 repeat is in
[`paired-t720-q8-repeat.txt`](data/scalar-paired-zip-iteration-2026-10-14/paired-t720-q8-repeat.txt).

## Numerical validation

The candidate passed `scalar_synthesis_matches_ducc_0410_reference`,
`scalar_adjoint_operators_match_ducc_and_inner_products`, and the opt-in
T511/T1023/T2047 scalar DUCC and closed-form test on GL, CC and F1. T2047
DUCC middle-row maximum absolute errors were `2.083e-13` (GL), `2.078e-13`
(CC), and `2.072e-13` (F1); maximum analytic map errors were at most
`9.958e-12`. The tests exercised the same recurrence values and summation
order; no tolerance or numerical convention changed.

## Environment and reproduction

CPU: x86-64 CPU, Linux. Rust `1.99.0`
(`b940084d7`, 2026-09-28), LLVM `23.1.1`; release `opt-level=3`, 16 codegen
units, LTO off, portable Pulp runtime dispatch (AVX2/FMA), no
`target-cpu=native`. Baseline and candidate used the same executable
configuration and were run in alternating fresh processes. Build with
`cargo build --release --example m1b_bench`. For example:

```text
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 511 1 synthesis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 511 8 synthesis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 720 1 synthesis GL STANDARD 721 1443 720 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 720 8 synthesis GL STANDARD 721 1443 720 requested-only
```

The native timing experiment did not remeasure DUCC wall time. Correctness
against pinned DUCC 0.41.0 is covered by the test suite. No production source
change or extra workspace memory remains from this experiment.
