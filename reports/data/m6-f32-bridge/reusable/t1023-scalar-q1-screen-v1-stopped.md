# Reusable f32 bridge T1023 scalar q1 screen (stopped)

## Decision

The Rust process completed, but the screen is invalid under the frozen
interference rule. The telemetry supervisor itself crossed its CPU-activity
trigger while the benchmark child was running, and its affinity was not
excluded from the pinned worker CPU. It may have competed with the measured
process. Preflight and postflight Linux CPU PSI checks were below the warning
band; the cgroup throttling counters did not increase. The screen protocol
therefore stops here. No DUCC comparison process ran, no
reusable-versus-allocating speedup is accepted, and the T1023 cell will not be
retried under protocol v1.

The Rust sample series passed their within-process CV and drift checks, but the
timings below are retained only as diagnostic observations. No timing
comparison or acceptance decision is based on them. The local raw process,
frequency, thread, PSI, cgroup, and console captures remain untracked.

## Frozen inputs and workload

- Build revision: `0102d240f439eb275b198e260b3039baf8503b2c`; Rust numerical
  sources match `83bd755edfa44e6b60a19548025d316c20cc49fe`.
- Prototype source SHA-256 values: `m6_f32_reusable_bridge.rs`
  `d612b5a0afe54926e27d43c2edbc0672e532dafc9383742bd36ce1010222cbd9`;
  `reusable_bridge.rs`
  `91931b1f820197fec1f2d6b409e680566bab4eb7169d324c0a7c2015fb36d309`.
- Release binary SHA-256:
  `ba5c44c8226e758ffd9c1a43f2d5970538c87e21044c3625e602f3f488c102f8`.
- Workload: GL f32 scalar STANDARD synthesis, T1023 (`1024x2048`), full
  `mmax`, `phi0=0.37`, one worker. Rust 1.99.0. The process was pinned to one
  allowed CPU; observed worker placement remained within that mask.
- Four variants ran in one process: native f32, allocating f64 bridge,
  reusable-buffer f64 bridge, and native f64 with the same rounded inputs.
  The reusable output was bitwise equal to the f64 output after f32 narrowing;
  reusable buffer capacities were unchanged.

## Rust timing observations

The protocol requires retaining every sample. Values are milliseconds; these
rows do not establish comparative performance because the process was invalid.

| Variant | Samples | Median | CV | First-three/final-two drift |
|---|---|---:|---:|---:|
| Native f32 | 3764.962, 3787.029, 3794.692, 3811.485, 3813.654 | 3794.692 | 0.47% | 0.70% |
| Allocating bridge | 302.564, 299.084, 300.611, 300.629, 299.922 | 300.611 | 0.38% | 0.01% |
| Reusable bridge | 295.902, 300.790, 301.645, 299.272, 301.331 | 300.790 | 0.70% | 0.18% |
| Native f64 rounded inputs | 295.683, 295.091, 299.137, 299.455, 300.552 | 299.137 | 0.73% | 1.65% |

Plan construction took `0.217 ms`; workspace and reusable-buffer setup took
`33.348 ms`. The retained reusable f64 buffers contain `25,174,016` bytes
(`24.0 MiB`), equal to the allocating bridge's temporary payload for this
scalar case. Sampled process high-water RSS was `214,068 KiB`.

## Next action

The timing result is inconclusive. The self-interference in the telemetry
supervisor invalidated the cell, so these samples cannot authorize a paired
screen. The frozen protocol forbids a retry. Continue M6 through the separate
f64 synthesis workstream; do not route the reusable f32 bridge into production.
