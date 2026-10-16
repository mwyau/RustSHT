# f64 spin-1 T511 DUCC comparison remains inconclusive

Five alternating process pairs measured f64 GL T511 STANDARD analysis at q1
and q8. RustSHT's q1 median was `33.793 ms`; DUCC's was `26.361 ms`, for a
paired median ratio of `1.28x`. At q8, the medians were `15.376 ms` and
`4.446 ms`, for `3.07x`. The q1 pairs were consistent within this screen.
RustSHT's q8 process medians ranged from `8.394` to `18.434 ms`, so the q8
comparison is too variable to update the measured gap.

The DUCC medians are about twice the earlier q1/q8 values of `13.056/1.943
ms` in the [thread-scaling report](spin1-analysis-thread-scaling-2026-10-14.md).
The earlier RustSHT values came from a different source revision. The current
screen used the same DUCC package version and benchmark script, but its host
load average rose from `3.44` before the run to `5.55` afterward. CPU affinity
was fixed; other processes could use those CPUs. The screen therefore does
not establish an updated RustSHT/DUCC gap.

## Workload and method

The operation was f64 GL T511 STANDARD analysis on a `512x1025` grid with
`mmax=511`, `phi0=0.37`, and deterministic full-band map input. RustSHT used a
reused native plan and workspace from commit `7ac443a40b034d5c52526d23ef30f6cef4b13d19`.
DUCC used `ducc0==0.41.0`, source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, through the Python
`analysis_2d` call. Each process warmed once and recorded five inner samples;
the DUCC process used two five-sample batches. Five process pairs alternated
RustSHT and DUCC at each thread count. q1 was pinned to CPU 0; q8 was pinned to
CPUs 0–7. The native reusable-plan and Python operation call boundaries differ.

| Threads | RustSHT median (range) | DUCC median (range) | Paired ratio median (range) |
| ---: | ---: | ---: | ---: |
| 1 | `33.793 ms` (`33.641–34.833`) | `26.361 ms` (`25.824–27.597`) | `1.28x` (`1.26–1.31x`) |
| 8 | `15.376 ms` (`8.394–18.434`) | `4.446 ms` (`4.135–5.251`) | `3.07x` (`2.02–4.15x`) |

The host was an AMD Ryzen 9 5950X running Linux `7.0.14-20-pve`. Rust was
`1.99.0` with LLVM `23.1.1`; the release build used portable Pulp runtime
dispatch and no `target-cpu=native`. Python was `3.11.15` with NumPy `2.4.6`.

Raw inner samples, process timings, load snapshots, command lines and binary
hashes are in [`the benchmark data`](data/spin1-analysis-ducc-refresh-2026-10-15/).
The `preliminary-*` files record an earlier run whose native q1 arguments were
incorrect; the table above uses the corrected q1 and q8 runs. This screen is a
timing comparison, not a numerical validation. Existing spin-1 analytic,
DUCC, adjoint and high-bandlimit tests provide the numerical checks. No source
code changed during this comparison.

Repeat the paired comparison when the CPUs are less shared before updating
the T511 DUCC gap. Preserve the native and Python timing boundaries in the
report.
