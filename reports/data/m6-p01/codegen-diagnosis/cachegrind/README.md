# P01 T511 spin-1 cache model

This diagnostic compares the retained baseline, default-link P01 and
ordered-link P01 benchmark executables. The paired native measurements show a
layout-linked timing shift for spin-1 synthesis; normalized instructions in
the identified spin-1 functions are unchanged. Linux denies hardware PMU
a model and its counters are not hardware timings or an acceptance test. All three processes use one CPU core, RustSHT's deterministic T511 spin-1 GL
STANDARD f64 synthesis case (`512×1024`, full `mmax`, `phi0=0.37`), one warmup
and five measured invocations inside `m1b_bench`. The order is baseline,
default P01, ordered P01. Runs are serialized; the runner records PSI and
process snapshots before each run and refuses when CPU `some avg10` reaches
8%. No compilation is performed.

function summaries, process stdout, and `/usr/bin/time -v` data. Simulated
instruction and cache counters include process setup and all six synthesis
calls, so compare full process totals only. Any function-level comparison is
indicative because plan setup, allocator and timer code also run under

## Observed process totals

| Variant | Instructions | I1 misses | Last-level instruction misses | D1 misses | Last-level data misses | Conditional branch mispredicts |
|---|---:|---:|---:|---:|---:|---:|
| Baseline | 5,368,206,418 | 72,920 | 10,378 | 26,070,497 | 1,304,408 | 5,904,911 |
| Default P01 | 5,368,200,085 | 79,368 | 10,250 | 25,802,908 | 1,298,678 | 6,013,750 |
| Ordered P01 | 5,368,213,641 | 86,531 | 10,331 | 25,779,645 | 1,298,493 | 6,561,796 |

The native ordered/default median ratio was 0.96668 [0.96024, 0.97375], so
more L1 instruction misses and conditional-branch mispredicts for the faster
ordered executable, while last-level instruction misses were nearly equal. Instruction references also differed by less than 0.001%. These model
counters do not explain the native timing shift; they rule out a simple
`brk segment overflow` warning in each run, although all processes exited
successfully and reported every sample. This further limits interpretation. Hardware counters could provide a closer measurement, but `perf stat` cannot
access cycles or instructions in this host because
beyond the measured correlation with final executable text placement.
