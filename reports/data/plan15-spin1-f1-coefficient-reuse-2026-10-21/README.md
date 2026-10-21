# F1 spin-1 coefficient-reuse A/B data

This directory contains the sanitized aggregate for the three-way comparison
of baseline RustSHT, coefficient-reuse candidate RustSHT and native DUCC at
T720 F1. `summary.json` retains per-backend latency distributions, paired
process medians, setup/first-call values, numerical checks, binary hashes,
fixture hashes and summarized host wait/frequency observations. `summary.csv`
is the same backend-level latency table in flat form. Raw process captures
and detailed host telemetry remain local, untracked benchmark evidence.

## Source and build contract

- Baseline Rust source: `0b2e1336409ba0bbe9fd6fa4cff70db0e327cd52`.
- Candidate Rust source: `7f47f06f875991a982caf0f4c396323a32c0d72b`.
- DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- Rust: 1.99.1 / LLVM 23.1.1, release, `-C target-cpu=x86-64-v3`.
- DUCC SHT: GCC 15.2, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`;
  driver compiled with `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X. q1 affinity used the first allowed physical core;
  q8 used the first eight allowed physical cores (CPUs 0–7 in this run).

The full-transform worker, deterministic fixture generator, DUCC build setup
and measurement protocol are shared with
[`plan15-spin1-native-baseline-2026-10-21`](../plan15-spin1-native-baseline-2026-10-21/README.md).
For an exact repeat, build the worker from each source revision in a detached
worktree with the same Rust flags, then build the native DUCC worker from the
pinned checkout using the CMake project in that baseline data directory. The
runner verifies numerical outputs outside timed regions and stores the raw
process diagnostics at the requested output path:

```sh
export BASELINE_BIN="<baseline worker executable>"
export CANDIDATE_BIN="<candidate worker executable>"
export DUCC_BIN="<pinned DUCC worker executable>"
export RAW_CAPTURE="f1-coefficient-cache-raw.json"
python3 scripts/benchmark_spin1_f1_coefficient_cache.py \
  --baseline "$BASELINE_BIN" --candidate "$CANDIDATE_BIN" \
  --ducc "$DUCC_BIN" --output "$RAW_CAPTURE"

python3 scripts/summarize_spin1_f1_coefficient_cache.py \
  "$RAW_CAPTURE" \
  reports/data/plan15-spin1-f1-coefficient-reuse-2026-10-21
```

Defaults reproduce three interleaved process pairs, 25 ms calibrated
batches, q1/q8 and both analysis/synthesis for exact-grid, even-smooth F1
T720 full (`mmax=720`) and quarter-order (`mmax=180`) cases. Raw JSON is
deliberately not committed because it contains machine/process telemetry.
