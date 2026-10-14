# Spin-1 shared coefficient conversion experiment

## Decision

Rejected the workspace cache prototype. It substantially reduced Callgrind instruction references attributed to `spin1_to_scalar`, but did not establish a repeatable whole-transform improvement beyond timing variation. The one-time serial conversion also made cold execution slower. The production implementation remains unchanged.

## Hypothesis

At T511 with eight Rayon threads, each of 16 latitude groups recomputed the same spin-1-to-scalar coefficient conversion. A compact triangular coefficient buffer built once per transform might replace repeated arithmetic with one serial conversion and per-group copies while retaining the existing 32-ring group schedule.

The temporary prototype added a workspace-owned, per-transform buffer, capped at 34 MiB. It was populated from recurrence worker zero and copied by order into each group’s existing coefficient scratch. It did not cache across transform calls because input coefficients can change between calls.

## Workload and method

- Source baseline: `cd65415` (`Unroll paired analysis vector accumulation`); candidate contained only the temporary cache prototype and a focused test.
- CPU: AMD Ryzen 9 5950X, Linux, affinity restricted with `taskset -c 0-15`.
- Compiler: rustc 1.99.0, release profile, default release flags, runtime Pulp AVX2/FMA dispatch.
- Workload: f64 spin-1 STANDARD synthesis, GL, T511, `ntheta=512`, `nphi=1025`, `lmax=mmax=511`, `phi0=0.37`, deterministic `m1b_bench` coefficients.
- Each process warmed the operation and collected five steady-state samples. Seven baseline/candidate process pairs ran sequentially on the same affinity, with the baseline first in each pair.
- Commands:

```sh
cargo build --release --example m1b_bench
taskset -c 0-15 target/release/examples/m1b_bench 1 511 8 synthesis GL STANDARD 512 1025 511 requested-only
taskset -c 0-15 /tmp/rustsht-baseline-cd65415/target/release/examples/m1b_bench 1 511 8 synthesis GL STANDARD 512 1025 511 requested-only
```

`requested-only` measures the requested eight-thread operation and its first call. The paired baseline executable was built from `cd65415`. The host was not idle throughout; seven other logged-in users were present and load average was 3.29 at the start of the repeated run.

## Timing results

The reported process values below are medians of each process’s five steady-state samples. Their median across seven processes was 21.384 ms for baseline and 20.532 ms for the candidate, a 4.0% difference. The baseline process medians ranged from 20.438 to 23.366 ms; candidate medians ranged from 19.629 to 22.180 ms. The paired candidate was faster in five of seven pairs, but the distributions overlap substantially.

| Run | Baseline q8 warm median (ms) | Candidate q8 warm median (ms) | Candidate first call (ms) |
|---:|---:|---:|---:|
| 1 | 23.366 | 20.253 | 25.040 |
| 2 | 23.286 | 21.851 | 25.181 |
| 3 | 20.707 | 20.532 | 24.528 |
| 4 | 20.438 | 21.948 | 25.165 |
| 5 | 20.622 | 20.205 | 25.791 |
| 6 | 21.384 | 22.180 | 25.245 |
| 7 | 23.226 | 19.629 | 27.601 |

First-call baseline median was 24.071 ms; candidate median was 25.181 ms, 4.6% slower. The first-call candidate paid for buffer allocation and serial coefficient preparation. A separate full harness run measured one-thread synthesis at 57.514 ms for the candidate; the recent baseline was 57.374 ms. No one-thread benefit was observed.

The observed q8 warm difference is not accepted as a performance improvement: it is smaller than the run-to-run spread, and two paired candidate runs were slower. The cold regression and retained workspace memory weigh against selecting the prototype.

## Instruction and memory evidence

Callgrind 3.26.0, collecting Ir for the T511 eight-thread benchmark process, counted 10,262,541,312 references for the baseline run recorded before this experiment and 7,384,159,470 for the candidate run. In the corresponding profiles, `spin1_to_scalar` fell from 2,365,646,892 Ir (23.05% of the baseline total) to 160,258,014 Ir (2.17% of the candidate total). Candidate `memmove` counted 107,440,545 Ir (1.46%). The candidate reduced total references by 28.0% in these profiles.

These are simulated instruction references summed across threads, not CPU cycles or elapsed time. They show that the arithmetic was removed from worker groups, but do not show a corresponding execution-time gain. The prototype moves the conversion to one serial preparation step; the original work ran concurrently with the independently scheduled latitude groups. The profile and wall-clock result therefore support retaining the parallel per-group conversion.

At full mmax, the cache payload would be 4,218,880 bytes (4.02 MiB) at T511, 8,352,064 bytes (7.97 MiB) at T720 and 16,826,368 bytes (16.05 MiB) at T1023. The f64 T2047 payload would be 67,207,168 bytes (64.09 MiB), above the 34 MiB cap and therefore ineligible. These are payload estimates; process peak RSS was not measured.

## Correctness check

A temporary focused release test compared cached four-thread T127 GL spin-1 synthesis against one-thread synthesis for a rectangular `mmax=93` plan. Both output components matched within `2e-12`, and the workspace contained the expected compact cache length. The test passed. The prototype was then removed, so this test is not retained in the production suite. Existing DUCC numerical references remain unchanged; the prototype changed only how the same per-order converted coefficients reached the existing synthesis kernel.

## Follow-up

Do not retain the cache or enlarge its memory budget. A future attempt would need a design that keeps conversion parallel without duplicating it, and must demonstrate a repeatable whole-transform gain under paired process measurements. Continue profiling the remaining synthesis recurrence and accumulation work; the cache profile left the fused prepared recurrence and staged transition as the largest instruction-attribution entries.
