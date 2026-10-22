# Plan 16 paired-coefficient preparation experiment

This directory records a rejected, no-extra-workspace optimization that reused
prepared spin-1 scalar coefficient streams across 64-latitude blocks within a
ring group. The whole-transform candidate did not produce a repeatable gain.
See the [experiment report](../../../plan16-spin1-paired-coefficient-preparation-2026-10-22.md).

- `case-summary.csv`: all 16 T720 exact-CC case/operation/thread combinations,
  with separate baseline and candidate pass distributions, matched DUCC
  latencies, numerical error, repeat and output-reset checks.
- `metadata.json`: source and binary hashes, toolchains, flags, affinity,
  benchmark boundaries and reproduction command.
- `candidate.patch`: exact temporary candidate source diff; it is retained for
  review but is not part of production `src/`.
- `run_exact_cc.py`: thin wrapper around the existing native benchmark harness
  that limits the T720 CC selection to the exact `ntheta=722` cases.

The 2,655 raw per-batch timing rows and their local process/host observations
are kept outside the tracked repository. The summary retains per-run p10,
median and p90 distributions, and the wrapper reproduces the comparison.

The reference was commit `dd39f82714e7b27b9521671048aab2a84c80f14e`; DUCC
was pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
