# CC spin-1 coefficient-reuse A/B data

This directory contains sanitized backend summaries for an exact-grid T720
CC comparison of baseline RustSHT, the CC coefficient-reuse candidate and
native pinned DUCC. `summary.json` includes per-backend latency distributions,
paired process medians, first-call/setup timings, numerical and reset checks,
source/binary/fixture hashes, flags, CPU affinity and summarized host wait.
`summary.csv` contains the same backend rows. Raw per-process output and
detailed host telemetry remain local and untracked.

## Source/build contract

- Baseline Rust source: `9b93cf32d7399fbbd4ca8f8231ec605238c49076`.
- Candidate Rust source: `be3c48eb2e430fa3ca7e91fe33e6cc503f2a1b9d`.
- DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- Rust 1.99.1 / LLVM 23.1.1, release, `-C target-cpu=x86-64-v3`.
- DUCC SHT built with GCC 15.2 and
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the validation
  driver uses `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X. q1 used CPU 0; q8 used physical CPUs 0–7.

Build the Rust worker at each source commit using the same flags. Build the
DUCC worker from the pinned checkout using the CMake project in
[`plan15-spin1-native-baseline-2026-10-21`](../plan15-spin1-native-baseline-2026-10-21/README.md).
Then reproduce this matrix with:

```sh
python3 scripts/benchmark_spin1_f1_coefficient_cache.py \
  --grid CC --baseline "$BASELINE_BIN" --candidate "$CANDIDATE_BIN" \
  --ducc "$DUCC_BIN" --baseline-source 9b93cf32d7399fbbd4ca8f8231ec605238c49076 \
  --candidate-source be3c48eb2e430fa3ca7e91fe33e6cc503f2a1b9d \
  --output "$RAW_CAPTURE"

python3 scripts/summarize_spin1_f1_coefficient_cache.py \
  "$RAW_CAPTURE" reports/data/plan15-spin1-cc-coefficient-reuse-2026-10-21
```

Defaults run three interleaved process pairs, 25 ms calibrated batches, q1/q8
and both analysis/synthesis at exact T720 CC with full (`mmax=720`) and
quarter-order (`mmax=180`), even-smooth longitude sampling and `phi0=0.23`.
The full transform includes setup boundaries and validates outputs outside
timed regions; setup and first-call values are reported separately from warm
latency. Raw captures contain host/process telemetry and are not committed.
