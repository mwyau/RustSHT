# Exponent-epoch scalar synthesis q1 screen, protocol v1

## Purpose and decision boundary

Measure the isolated 128-ring f64 GL scalar fast route with the exponent-epoch
prefix accumulator against the clean production baseline. This is a complete
transform screen at q1. A passing screen may authorize a separate q8 and
regression matrix; it does not accept the candidate or enable production
routing. The candidate remains isolated until numerical, T511, q8, spin-1,
memory and complete regression checks pass.

## Frozen sources and binaries

- Clean baseline source: `opt/sht-performance-parity` commit
  `2148c7b6a7db332bb3678640efd18b4e979fc6e2`; the later `b495e763` commit
  changes reports only. Its release Python extension SHA-256 is
  `33744684470d79ba84ba7560079f1a25c951fe4dab65b7d8a089712316d4871d`.
- Candidate source starts from `2148c7b6` and applies the archived tile-128
  fast-route integration, then
  [`prefix-exponent-epoch-accumulator.patch`](prefix-exponent-epoch-accumulator.patch)
  (SHA-256
  `badc0b317705d27f2d846028baf577fd4fb4d880ab3b7e603a92c5bff89c1fb2`).
  The resulting `src/legendre/synthesis_fast.rs` SHA-256 is
  `0f12d2aa0881d97dff66fb2b0a63ddbbfae0fb295bc67b066fe9fcced624d072`.
  Its release Python extension SHA-256 is
  `ae0a04e70c2c5096ac68d03ea88ca4a5ace8c9a67f9e3b66324283e8c6967783`.
- Both wheels were built separately with `uv build --wheel`, Rust/Cargo
  1.99.0, the same release profile and lockfile
  (`Cargo.lock` SHA-256
  `07e3ac929c8f1f6b708b36810accef15d5a9e71918f35d541246c6df15da26d1`).
  No source or build configuration changed between their build and this
  screen. The build command did not set target-specific Rust flags.
- Shared harness: `examples/python_perf_baseline.py`, SHA-256
  `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`.
  It times `ShtPlan::synthesis_into` and DUCC's output-buffer synthesis and
  compares maps against DUCC. SHTns 3.7.5 is included as a reference.
- DUCC 0.42.0 source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`, extension SHA-256
  `1218502fa03116cd14578fe73f716f5a1d88db74d296359cc9e8c27e04047d1e`.
  SHTns source revision `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`,
  library SHA-256
  `641195ac574ba02078f3238b357caecf0e268b3f4ef7eba1d87898197fcc888f`.
- Python 3.11.15 and NumPy 2.4.6. Rust is
  `1.99.0 (b940084d7 2026-09-28)`, LLVM 23.1.1. Hardware is an AMD Ryzen 9
  5950X, x86-64 with AVX2/FMA, 16 physical and 32 logical cores, and 96 GiB
  RAM. The execution policy pins one worker to one logical CPU; CPU IDs and
  transient host-state captures remain local.

## Workload and fixed order

Each process runs GL f64 scalar `STANDARD` synthesis, full `mmax`,
`phi0=0.37`, one Rayon worker, and a grid of `(L+1) x 2(L+1)`. The harness
performs one warm-up and five timed complete calls for each backend. The timed
Rust path writes into a reusable output map. Each fresh process checks the
Rust map against DUCC and SHTns against DUCC before collecting timings.

First run two independent clean-baseline T511 processes as a stability
diagnostic. If either fails timing or host validity, mark the T511 control
invalid and do not retry it; proceed to the high-resolution q1 cells. Then run
six fresh-process pairs at T1023, followed by six at T2047, alternating order:
pair 1 baseline/candidate, pair 2 candidate/baseline, and so on. Run the six
T511 baseline/candidate control pairs after the high-resolution cells only if
the two baseline diagnostic processes passed. Do not start T2047 if a T1023
process fails a validity gate. Run T511 controls regardless of observed
high-resolution speedup when their diagnostic passed.

Use the same binaries, Python executable, references, CPU affinity policy and
workload arguments throughout. Do not compile or run another benchmark during
acquisition.

## Validity, numerical and performance rules

- Reject a process if preflight or postflight CPU PSI `some avg10` exceeds
  2.00%. Readings from 0.50% through 2.00% are warnings. Retain the exact
  readings in local captures and report the gate outcome.
- Require population CV at most 5% and first-three/final-two median drift at
  most 5% for each RustSHT, DUCC and SHTns five-call series. Do not discard
  samples, retry a failed process or change the matrix after seeing results.
- Review actual process affinity and worker placement, CPU frequency,
  involuntary context switches, concurrent CPU-intensive work, cgroup
  pressure and PSI after every process. Clear evidence of interference makes
  that process inconclusive even below the PSI rejection threshold. Stop the
  affected cell; preserve its samples as diagnostic evidence.
- Require RustSHT versus DUCC relative-L2 error at most `2e-11` and maximum
  absolute error at most `2e-12` for every candidate process. Candidate
  extreme-finite, polar and recurrence-fallback checks are covered by the
  focused and workspace tests recorded in the experiment report.
- For q1 expansion, at least one of T1023/T2047 must have median paired
  candidate/baseline ratio at most 0.90 and the upper endpoint of its 95%
  bootstrap interval below 1.00; the other high-resolution interval's upper
  endpoint must be at most 1.02. Use 50,000 resamples, seed `20261017`, the
  median per resample and linearly interpolated 2.5th/97.5th percentiles. The
  T511 control interval's upper endpoint must be at most 1.02. A control that
  fails validity leaves the candidate unaccepted.
- Keep raw console, GNU-time, process, PSI, frequency, affinity, placement and
  context-switch records local and untracked. Commit sanitized samples,
  validity outcomes, aggregate RSS and conclusions only.
