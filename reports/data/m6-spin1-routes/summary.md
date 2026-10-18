# M1 spin-1 synthesis route profile

This diagnostic profiles GL STANDARD f64 spin-1 synthesis at `T=511`, `1023`,
and `2047`, with one and eight Rayon workers. Each cell is one fresh process. The test performs a warm-up, one uninstrumented synthesis, then one synthesis
with thread-local counters and timers. It asserts that both Q and U outputs
match bitwise and records an FNV-1a hash over their IEEE-754 bit patterns. The counter timers are sums of worker CPU time; they are not wall-time shares. The base is `c0e11ad99f033073fb5ac56b170a72e26ede8d4f` plus the committed
working-tree patch recorded in `source-hashes.txt`. The isolated profile adds
test-only hooks in four Rust files. It excludes P01 and P02. The test binary,
source hashes, build command, settings, and per-worker records are retained in
this directory. The DUCC comparison uses the matched uninstrumented extension
and DUCC 0.42.0 at revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`.

| T | workers | map hash | DUCC relative L2 | disabled / instrumented (ms) | process elapsed (s) | peak RSS (MiB) |
|---:|---:|---|---:|---:|---:|---:|
| 511 | 1 | `44a057f2fadd1c72` | `6.699e-14` | 42.531 / 43.854 | 0.15 | 32.5 |
| 511 | 8 | `2ccbfa8b744a78af` | `4.718e-14` | 13.132 / 13.603 | 0.07 | 41.8 |
| 1023 | 1 | `7fee58e92f884bfe` | `1.216e-13` | 280.283 / 281.224 | 0.93 | 117.0 |
| 1023 | 8 | `7fee58e92f884bfe` | `1.216e-13` | 72.998 / 72.881 | 0.31 | 117.4 |
| 2047 | 1 | `9000b8471ca9eb7c` | `3.707e-13` | 4080.171 / 4173.754 | 12.67 | 453.5 |
| 2047 | 8 | `9000b8471ca9eb7c` | `3.707e-13` | 982.844 / 987.582 | 3.31 | 454.4 |

The DUCC errors are below `4e-13` in all six cells. Since the instrumented
output equals the uninstrumented output bitwise in the test, the same-source
DUCC errors apply to the profiled output. The run is diagnostic and makes no
performance acceptance claim.

## Route counts and worker CPU timer sums

`orders` counts instrumented order invocations across workers and may repeat
an order in the eight-worker path. `scalar visitor lanes` and `rescale lanes`
count lane-degree work; `transition prefix lanes` and `transition fused
pairs` count committed transition work. The q8 T511 hemisphere counts identify
the cached hemisphere path. Generic fused, transition, coefficient, and
factor timer columns are the existing shared scalar-kernel instrumentation. Every timer below is a sum across workers in milliseconds, not wall time.

| T | q | converted / cached order invocations | scalar blocks | scalar visitor degree-lanes | rescale lanes | hemisphere pair-lanes / attempts | transition prefix lanes | transition fused pairs | declined transitions | fused / transition / coefficient / factor timers | conversion / cache-copy / spin-1 visitor / rescale / hemisphere timers |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 511 | 1 | 512 / 0 | 4,096 | 199,138 | 204,462 | 0 / 0 | 812,278 | 3,729,010 | 6 | 0.897 / 19.938 / 1.772 / 0.420 | 1.995 / 0 / 2.411 / 0.599 / 0 |
| 511 | 8 | 0 / 8,192 | 4,370 | 81,922 | 12,526 | 14,025,232 / 6,102 | 199,606 | 1,273,522 | 6 | 0.311 / 6.906 / 5.052 / 6.879 | 0 / 2.540 / 1.325 / 0.105 / 26.016 |
| 1023 | 1 | 1,024 / 0 | 16,384 | 350,982 | 751,552 | 0 / 0 | 5,879,808 | 32,824,870 | 0 | 1.608 / 150.084 / 13.605 / 1.657 | 7.884 / 0 / 4.784 / 2.720 / 0 |
| 1023 | 8 | 6,730 / 0 | 13,460 | 350,982 | 751,552 | 0 / 0 | 5,879,808 | 32,824,870 | 0 | 1.921 / 155.371 / 14.108 / 12.614 | 59.910 / 0 / 4.837 / 2.081 / 0 |
| 2047 | 1 | 2,048 / 0 | 65,536 | 168,888,650 | 2,716,504 | 0 / 0 | 45,788,452 | 346,986,362 | 3,009 | 3.005 / 1,391.849 / 106.793 / 6.596 | 32.331 / 0 / 1,990.204 / 13.257 / 0 |
| 2047 | 8 | 12,944 / 0 | 51,776 | 168,888,650 | 2,716,504 | 0 / 0 | 45,788,452 | 346,986,362 | 3,009 | 6.166 / 1,422.661 / 117.271 / 49.999 | 243.964 / 0 / 2,050.335 / 13.733 / 0 |

The T2047 measurements place substantial work in both the scalar visitor and
the paired transition path. These timer sums locate work in the profiled
implementation; they do not establish a wall-time bottleneck breakdown. The
per-worker timing and route counts are retained in `timing-samples.jsonl` for examining
worker balance and route variation. At T511 q8, 8,192 cached order invocations, 14.0 million hemisphere pair-lanes,
and 6,102 hemisphere attempts accompany the special hemisphere path. This
path does not appear in the T1023 or T2047 q8 profiles. At T2047 q8 the route
unit totals match q1, while work remains uneven among workers. Further kernel
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
fixed `0.50` threshold. That refusal is retained as
`t2047-q8-refused-initial.preflight`. Two later recovery snapshots and the
runner's immediate preflight were clean; the subsequent profile passed. Raw
preflight, stdout, and `/usr/bin/time -v` files are kept for every cell.
