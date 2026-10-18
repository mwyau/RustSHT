# T511 baseline warm-up and stability diagnostic v1

## Result

The frozen clean-baseline binary was run in two fresh processes. Each process
used four blocks per backend, with one unmeasured warm-up and five timed calls
per block. The existing per-block limits were unchanged. Timing failures
recurred: RustSHT failed two blocks in process A, and DUCC failed two blocks
in process B. The T511 control remains inconclusive. No candidate process,
retry, q8 run, or broader regression run followed.

All timing samples and per-block calculations are in
[`t511-baseline-convergence-v1-samples.jsonl`](t511-baseline-convergence-v1-samples.jsonl).
The fixed workload and validity method are in the
[`diagnostic protocol`](t511-baseline-convergence-v1-protocol.md).

## Frozen configuration

- Baseline source HEAD:
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; extension SHA-256
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Harness SHA-256:
  `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`.
- DUCC 0.42.0, source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`; SHTns source revision
  `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`.
- GL f64 scalar `STANDARD` `synthesis_into`, T511 (`512x1024`), full
  `mmax`, `phi0=0.37`, one worker and one pinned logical CPU.
- Process A then B used the same extension, references and configuration.
  Each backend had four warm-up/timing blocks; no candidate was loaded.

## Timing summary

Each entry lists the four block medians. CV and drift are the maximum among
those blocks. The individual samples and all block-level checks are retained
in the JSONL record.

| Process | Backend | Block medians (ms) | Max CV | Max drift | Process median (ms) | Result |
|---|---|---|---:|---:|---:|---|
| A | RustSHT | 40.136, 40.601, 40.749, 40.583 | 16.148% | 6.587% | 40.592 | Fail |
| A | DUCC | 8.026, 7.710, 8.014, 7.679 | 2.660% | 3.353% | 7.862 | Pass |
| A | SHTns adapter | 4.574, 4.545, 4.518, 4.533 | 0.600% | 1.251% | 4.539 | Pass |
| B | RustSHT | 40.312, 40.511, 40.272, 40.409 | 1.839% | 2.533% | 40.361 | Pass |
| B | DUCC | 7.651, 7.650, 9.276, 7.648 | 6.730% | 8.608% | 7.651 | Fail |
| B | SHTns adapter | 4.502, 4.475, 4.482, 4.481 | 0.631% | 0.402% | 4.482 | Pass |

Process medians are descriptive summaries of four block medians, not
acceptance statistics. They differed by 0.57% for RustSHT, 2.68% for DUCC and
1.26% for SHTns. Process A's fourth Rust block contained a `58.344 ms` sample.
Process B's DUCC median moved from `7.650 ms` in block 2 to `9.276 ms` in block
3, then returned to `7.648 ms` in block 4. Repeated one-call warm-ups did not
produce a stationary sequence across backends.

## Correctness, resources and interference review

Both processes passed numerical comparisons: RustSHT versus DUCC relative-L2
error was `6.632e-14` (maximum absolute error `3.482e-14`); the SHTns adapter
versus DUCC relative-L2 error was `4.286e-14` (maximum absolute error
`2.136e-14`).

| Process | Elapsed (s) | Peak RSS (KiB) | Voluntary context switches | Involuntary context switches |
|---|---:|---:|---:|---:|
| A | 34.95 | 98,336 | 167 | 596 |
| B | 35.01 | 98,428 | 65 | 602 |

The configured one-core affinity and sampled worker placement matched. No
competing compiler or benchmark process was observed, and cgroup throttling did
not increase during either run. Frequency telemetry was sampled less often
than individual calls, so it cannot establish a causal match; it showed no
large sustained frequency collapse. PSI gates passed; one postflight reading
fell in the warning band. Exact pressure and frequency captures remain local
and untracked.

## Decision

This diagnostic does not show a simple cold-start pattern: outliers appeared
after repeated calls and performance returned to the earlier range. It also
does not identify a competing workload, PSI rejection or cgroup throttle as
the cause. The source of the T511 timing instability remains unresolved.
Stop further T511 acceptance attempts under this method and do not launch the
candidate's q8 or scalar/spin-1 regression matrix until a valid T511 control is
available. The frozen 128-ring and scaled-prefix accumulator candidates remain
isolated and unaccepted.

The accumulator candidate's separate six-pair q1 matrices remain favorable:
the `synthesis_fast.rs` source SHA-256 is
`edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`, and its
extension SHA-256 is
`48db7ad3a6ba4c7fb0a8c50a303d9fbe948a7cac989c6f8a6e39b2f5e1b40184`.
Candidate/baseline median ratios were `0.568982` at T1023 and `0.407892` at
T2047. Its T511 gate remains unresolved; these high-resolution results do not
accept it. The next measured kernel target is still the scaled pre-suffix
transition. A T2047 route profile measured `715.931 ms` there versus
`543.517 ms` in the ordinary SIMD suffix; the sampled prefix profile points to
factor checks, exponent rebasing and accumulation for further source-level
diagnosis. These are instrumented route times, not an additive transform
breakdown.
