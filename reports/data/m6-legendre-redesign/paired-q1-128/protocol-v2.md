# q1 paired screen protocol v2

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

## Frozen workload

Use the same frozen extensions and harness recorded in [provenance.txt](provenance.txt), without rebuilding or editing sources. For each requested bandlimit, run six fresh-process pairs, alternating baseline-first and candidate-first by pair. Each process uses GL scalar f64 STANDARD `synthesis_into`, full mmax, phi0=0.37, `RAYON_NUM_THREADS=1`, CPU core affinity, backend `all`, and `runs=1`. The harness performs one untimed warm-up and five timed calls per backend. Run T1023 before T2047. Do not run q8. Keep every launched process and sample; do not retry or remove any sample.

## Predeclared q1 expansion rule

For each completed pair, divide the candidate Rust synthesis process median by
the baseline Rust synthesis process median. Summarize the six pair ratios with
the established M6 procedure: 50,000 bootstrap resamples of six pair ratios
with replacement, seed `20261017`, median per resample, and linearly
interpolated 2.5th/97.5th percentiles. Expand to the planned T511 and q8
controls only if at least one of T1023 or T2047 has a median paired ratio
`<=0.90` and a 95% interval upper bound below `1.0`, while the other high-L
cell's interval upper bound is `<=1.02`. The T511 scalar q1 control must also
have an interval upper bound `<=1.02` before any q8 result is considered. This is only an experiment-expansion screen; it does not accept the kernel.

## Host and process gates

- Before launch, reject on CPU PSI `some avg10` only when it is above 2.00; a competing compiler or benchmark still refuses the process. Values from 0.50 through 2.00 inclusive are warnings, not automatic invalidations. Preserve the exact preflight reading in the snapshot and warning log.
- Apply the same PSI threshold after the process. A postflight value above 2.00 invalidates that process and stops the cell. Values from 0.50 through 2.00 inclusive leave the completed process available for review, but require manual host/interference review before another process starts. The exact postflight reading remains in the snapshot and warning log.
- A runner exit status of 25 means `REVIEW`: retain the completed process, inspect its host evidence, and do not launch the next process until that review is complete. It does not mark the run invalid by itself.
- Require all three per-process raw series (`synthesis_rust_plan`, `synthesis_ducc`, `synthesis_shtns_adapter`) to contain five finite positive samples, population CV <=5%, and absolute first-three/final-two median drift <=5% of that series' median.
- Preserve periodic PSI readings above 2.00 for review alongside the process
  snapshots; the automatic 2.00 rejection threshold applies to preflight and
  postflight readings.
- Review the periodic snapshots for interference as well as PSI. Mark a process inconclusive, preserve it, and stop the cell when the logs show a nonbenchmark process using >=50% CPU in at least three consecutive two-second samples, or a nonbenchmark thread placed on CPU core using >=10% CPU in at least three consecutive samples. Also stop if another build or benchmark appears at any time. These are interference-review triggers; PSI below 2.00 alone does not establish a clean run.
- Any refusal, nonzero process exit, postflight gate failure, per-backend stability failure, or clear interference stops the current cell. Do not retry. Proceed from T1023 to T2047 only if all 12 processes in T1023 pass the gates and the snapshots have no unresolved interference warning. The checks establish measurement validity only. They do not constitute a performance acceptance decision.
