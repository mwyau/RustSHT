# Plan 15 CC synthesis scheduling data

This directory contains sanitized, reviewable aggregates for the q8 CC
synthesis scheduling change and its T511 dispatch-gate control.

- `summary.json` / `summary.csv`: T720 exact-CC, full/reduced `mmax`, q1/q8
  analysis and synthesis; includes paired process ratios, 30 short-call batch
  observations per backend/workload, first-call/setup values, numerical checks,
  binary hashes, and summarized host wait/frequency data.
- `t511/summary.json` / `t511/summary.csv`: the same controls at T511, including
  full and reduced order. The new queue route is selected only for full order.
- `f1-current-control.json` / `.csv`: a current-source F1 q8 synthesis timing
  recheck, retained because the older high-wait run differed substantially.

Source/toolchain metadata is embedded in each JSON. The full reproduction
commands and phase-probe settings are in
[`../../plan15-spin1-cc-group-scheduling-2026-10-21.md`](../../plan15-spin1-cc-group-scheduling-2026-10-21.md).
Raw per-process captures and detailed CPU telemetry remain local and
untracked.
