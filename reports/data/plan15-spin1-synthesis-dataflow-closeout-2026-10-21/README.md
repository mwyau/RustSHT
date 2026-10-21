# Plan 15 synthesis dataflow A/B data

These sanitized summaries condense the 72 backend runs into 24 timing rows
and eight pooled workload comparisons for each exact-CC T720 experiment. The
native driver performed 24 per-process three-way validation comparisons.
Summaries retain batch counts, p10/median/p90, per-process medians,
first-call/setup timings, fixture hashes, maximum numerical errors and
executable hashes; raw host/process telemetry remains untracked.

| Files | Experiment | Rust candidate SHA-256 |
|---|---|---|
| `active-lane-summary.csv`, `active-lane-summary.json` | Compact active rings before paired recurrence | `2aac562627f6230058cf4286197a3c3c75cd7bcfb1da37a30cc2bb2e074e6831` |
| `degree-major-summary.csv`, `degree-major-summary.json` | Pair-outer traversal with scratch state between pairs | `566cb227d399bb69e36a5a0f14e09bbf857756aca3299a300f9b80e9557a21f6` |

Both compare baseline binary `fb3cab5bd08a4792ea1663c78173c874e99b313a0660bc0b0668a06df6d36f60`
from Rust source `ae78e18044fea2d65316d230dd434e4b4f8222be` and DUCC binary
`89125091d06ddd4c24589c6f53e6785682d79f40cf0b991f64ad75a034ae2423` from
DUCC commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. See the linked
closeout report for toolchains, flags, CPU affinity and measured conclusions.

`degree-major-experiment.patch` is the rejected loop-order implementation,
for source review only. Apply it to a clean checkout of the baseline source
to rebuild the candidate; it is not an active kernel or dispatch option.
SHA-256: `bebab1607351045cc3e602509aa8e452c51071956818e4550082b30000d2be10`.
