# Fused synthesis loop invariant and register-pressure experiments

## Decision

Neither prototype was retained. The compiler already hoists the repeated `cos(theta)^2` multiply out of the degree-pair loop in the AVX2 kernel. Interleaving the four spin-1 channel accumulations did not remove the observed vector spill and regressed the single-thread whole transform.

## Source and benchmark setup

- Baseline: `cd65415` (`Unroll paired analysis vector accumulation`).
- CPU: AMD Ryzen 9 5950X, Linux; q1 runs pinned to CPU 0 and q8 runs pinned to CPUs 0–15.
- Compiler: rustc 1.99.0, Cargo release build with default release settings and runtime Pulp dispatch. The executed kernel is AVX2/FMA; this CPU has no AVX-512.
- Benchmark: `examples/m1b_bench.rs`, f64, GL, `phi0=0.37`, `ntheta=512`, `nphi=1025`, T511, full `mmax`, deterministic coefficients, reused plan/workspace, one warm-up and five timed calls per process.
- Command form:

```sh
cargo build --release --example m1b_bench
taskset -c 0 target/release/examples/m1b_bench 1 511 1 synthesis GL STANDARD 512 1025 511 requested-only
taskset -c 0-15 target/release/examples/m1b_bench 1 511 8 synthesis GL STANDARD 512 1025 511 requested-only
```

The benchmark host was shared. Paired process medians and ranges are reported; overlapping ranges do not establish a performance change.

## Explicit `cos(theta)^2` hoist

The source computed `cosine * cosine` inside each degree-pair loop in `FusedSynthesisBlock`, `FusedSynthesisPairBlock` and `FusedPreparedSynthesisPairBlock`. The candidate moved the operation outside that loop for both SIMD lanes and scalar tails. The recurrence and accumulation order did not change.

| Operation | Threads | Baseline median (ms) | Hoist median (ms) | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 43.087 | 42.946 | -0.3% |
| Scalar synthesis | 8 | 12.211 | 12.054 | -1.3% |
| Spin-1 synthesis | 1 | 56.530 | 56.434 | -0.2% |
| Spin-1 synthesis | 8 | 20.543 | 20.824 | +1.4% |

Process ranges overlapped. AVX2 disassembly showed one static `vmulpd` in the prepared kernel in both binaries, before its degree-pair loop. The optimizer had already hoisted the expression. Static instruction counts for the AVX-512 variant also matched. The source edit only changed scheduling around the same instruction and produced no repeatable whole-transform gain.

## Interleaved spin-1 channel accumulators

The q8 Callgrind profile from the preceding coefficient-cache experiment attributed 31.1% of its self instruction references to `FusedPreparedSynthesisPairBlock`; that profile had a reduced denominator because the cache removed repeated coefficient conversion. Its AVX2 assembly stores and reloads one YMM register through a stack slot inside the degree-pair loop. The prototype interleaved coefficient formation and accumulation for each real/imaginary component to shorten coefficient-vector live ranges. It did not introduce a new recurrence or change the per-component FMA order.

Across five baseline/candidate process pairs, T511 spin-1 q1 process medians were `56.417, 57.062, 57.013, 56.737, 56.575 ms` for baseline and `57.905, 58.582, 58.045, 57.790, 57.933 ms` for the candidate. The median across process medians increased from `56.737` to `57.933 ms` (`+2.1%`). At q8, baseline medians were `21.095, 21.779, 23.644, 23.997, 20.455 ms`; candidate medians were `23.326, 20.160, 21.212, 21.026, 21.087 ms`. Their ranges overlap (`20.455–23.997` and `20.160–23.326 ms`), and the candidate was slower in two of five paired runs.

The candidate AVX2 function had 665 static instruction lines versus 661 in the baseline disassembly. Both had the same three vector stack moves in the function, including the save and reload inside the pair loop. The rewrite did not remove the spill and serialized otherwise independent channel FMAs. `spin1_scalar_fast_synthesis_matches_general_channel_cases` passed in release mode; the candidate was removed because it did not meet whole-transform performance acceptance.

## Follow-up

Keep the current fused kernels unchanged. The source-level cosine hoist is redundant under rustc 1.99.0, while the interleaved accumulation increases the q1 cost without removing the measured AVX2 spill. A useful follow-up needs a distinct machine-level change that reduces the prepared kernel's live register set or spill traffic while preserving parallel FMA scheduling, then must pass paired q1/q8 whole-transform tests.
