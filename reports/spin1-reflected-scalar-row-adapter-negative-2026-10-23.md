# Spin-1 reflected scalar row adapter: rejected

**Date:** 2026-10-23
**Baseline source:** `44674c5c`
**Reference:** DUCC 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`

## Candidate

The q8 order-major spin-1 queue staged each converted E/B scalar row through
the reflected scalar recurrence on a symmetric central latitude band
(`sin(theta) >= 0.01`), retaining the existing spin path for polar rings. The
first whole-sphere version exceeded existing q1/q8 numerical tolerances; a
reduced-order version also failed the existing T720 test. The final checked
candidate was limited to full order and passed the T720 q1/q8 comparison on
GL, CC and F1. The scalar row adapter itself matched the full reflected
recurrence on all three exact grids.

## Focused q8 comparison

Exact T720, full order, even-smooth longitude, synthesis only. Each backend
ran 30 timed batches across three independent process pairs, with approximately
25 ms targeted work per batch, matched q8 affinity and the same fixture per
grid. The candidate was compared with the parent source and pinned DUCC.

| Grid | Rust parent median (ms) | Candidate median (ms) | DUCC median (ms) | Candidate / parent by pair | Candidate / DUCC median |
|---|---:|---:|---:|---|---:|
| GL | 15.043 | 15.089 | 5.870 | 1.000, 1.005, 1.004 | 2.574 |
| CC | 15.966 | 15.928 | 6.124 | 1.027, 0.992, 0.998 | 2.613 |
| F1 | 15.038 | 15.152 | 5.848 | 1.008, 0.999, 1.017 | 2.579 |

The paired changes vary by grid and process; distributions overlap and there is
no repeatable whole-transform gain. Candidate-to-DUCC relative L2 error was at
most `5.24e-13`; candidate-to-parent error was at most `2.47e-14`. Every
backend passed repeat and reset checks. Raw samples and host telemetry remain
local and untracked.

**Decision:** do not retain or promote this adapter. Its extra coefficient
conversion and two separate reflected recurrences did not reduce end-to-end
latency. Continue with a fused two-channel reflected recurrence as a separate
source change before the next benchmark.

## Follow-up separate-channel implementation

A follow-up implementation used the existing checked scalar ordered engine
for the two converted channels separately. It passed the T1023 GL
order-major q1/q8 cache and exceptional-value regression. A focused exact
T720 CC full-order q8 synthesis comparison used one deterministic even-smooth
fixture, 30 timed batches per backend across three interleaved process pairs,
and pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`:

| Backend/source | Median (ms) | p10–p90 (ms) | Relative L2 vs DUCC |
|---|---:|---:|---:|
| Pre-change Rust `3b1d546a` | 15.056 | 14.877–15.227 | `2.29e-13` |
| Separate-channel candidate | 24.029 | 23.826–24.379 | `2.68e-13` |
| DUCC | 6.136 | 6.097–6.194 | — |

The candidate was `1.596x` slower than pre-change Rust and `3.915x` DUCC.
Host wait fractions were below `0.023` for all three backends. The numerical
comparison passed, but the performance regression is decisive; the candidate
was removed. This confirms that the next attempt must share recurrence work
across both converted channels instead of dispatching two independent scalar
recurrences. The candidate source and raw per-batch samples were temporary and
were not retained in the worktree.
