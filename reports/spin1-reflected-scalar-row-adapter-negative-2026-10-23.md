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
