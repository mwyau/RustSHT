# Final scalar-analysis cumulative ablation

This directory contains sanitized, per-case summaries from the final T720
GL scalar-analysis A/B. It compares the current default, each required
candidate, the safe cumulative tile candidates, the range-cache control, and
the all-candidate build.

## Matrix and protocol

- Workload: deterministic T720 GL analysis, even-smooth `nphi`, `phi0=0.23`,
  `mmax=1` low-work control, `mmax=180`, and full `mmax=720`; q1/q8.
- Backends: RustSHT `2b8d31893d84ffcdc3dc8f76f69581323ae65aa6` and DUCC
  `80388904b8cfc32400ba84af8a6a45aabfe60f86` (0.42.0).
- Rust: 1.99.1 / LLVM 23.1.1, Pulp 0.22.3, runtime V3 AVX2/FMA,
  `-C target-cpu=x86-64-v3`, strict Rust floating-point semantics.
- DUCC: GCC 15.2, numerical sources `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; strict benchmark driver.
- Affinity: q1 pinned to one physical core; q8 pinned to eight physical cores in one L3 domain.
- Repetitions: three independent process pairs, ten calibrated ~25 ms batches
  per process/backend. Rust and DUCC were interleaved within each build; source
  variants ran sequentially, so small between-variant differences are
  provisional. First-call, setup, warm p10/median/p90, paired ratios, numerical
  errors and reset checks are preserved in `summary.csv`.

The machine was a Ryzen 9 5950X. In the current-default run, measured
runqueue-wait fraction was 0.24% median and 15.5% p90; CPU frequency ranged
from 1.75 to 4.74 GHz (3.97 GHz median). The q8 tail and frequency variation
make sub-percent or low-single-digit differences inconclusive.

## Files

- `summary.csv`: 54 rows, nine variants × six matched workloads, with Rust and
  DUCC latency distributions, paired ratios, cold/setup time, numerical error,
  reset-consistency counts, RSS and host summary fields.
- `variants.csv`: build flags, source/toolchain revisions, executable hashes,
  affinity and repetition settings for each variant.

The native `results.json` files contain raw process and host observations and
remain in the local diagnostic worktree; they are intentionally not committed.
The CSV records the compact statistics needed to review the conclusions.
