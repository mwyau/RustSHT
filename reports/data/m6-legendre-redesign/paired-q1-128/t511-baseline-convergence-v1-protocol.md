# T511 baseline warm-up and stability diagnostic v1

## Purpose

The tile-128 q1 acquisition stopped at a T511 baseline process because its
RustSHT series exceeded the fixed CV and drift limits. A later accumulator
candidate acquisition stopped at T511 because reference timings were unstable
and the CPU-pressure gate failed. This short diagnostic checks whether the
existing baseline binary shows call-to-call convergence across repeated
warm-up blocks. It is diagnostic evidence only; it does not accept a candidate
or change any acceptance threshold.

## Frozen workload

- Baseline RustSHT extension SHA-256:
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Harness SHA-256:
  `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`.
- References: DUCC 0.42.0, source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`; SHTns source revision
  `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`.
- GL f64 scalar `STANDARD` `synthesis_into`, T511 (`512x1024`), full
  `mmax`, `phi0=0.37`, one Rayon worker, one allowed logical CPU.
- Two fresh processes use the same baseline extension and references, in fixed
  order A then B. There is no candidate process and no retry.
- In each process, run four fixed measurement blocks per backend. Each block
  begins with one unmeasured warm-up call and then records five timed calls.
  Retain every sample and every block median. Backend order is the frozen
  harness order. Timing blocks are diagnostic and are not paired acceptance
  samples.

## Validity checks

- Preflight or postflight CPU PSI `some avg10` above 2.00% invalidates the
  process. Values from 0.50% through 2.00% are warnings and require review.
- Keep the existing five-call limits unchanged: population CV at most 5% and
  first-three/final-two median drift at most 5% for every backend block.
- Record the actual worker placement, affinity, frequency behavior,
  involuntary context switches, competing CPU-intensive work and cgroup
  pressure locally. Clear interference makes the process inconclusive even
  below 2.00% PSI.
- Run process B as part of this predeclared two-process diagnostic unless
  pre/postflight pressure exceeds the gate or clear interference is present.
  A failed timing series in A alone does not trigger a selective retry; all
  captured samples remain in the diagnostic record.
- Compare block medians across successive blocks and process medians A/B to
  assess convergence. Apply the same 5% scale when describing deviations, but
  treat this convergence view as diagnostic, not as a replacement acceptance
  gate.

## Decision boundary

Do not use this diagnostic to declare a performance result accepted. Do not
change the frozen baseline, candidate, q1 matrix, CPU-pressure threshold or
timing limits. If the repeated baseline series remains nonstationary or the
host fails validity checks, stop the q1 control acquisition and record the
limitation. Raw process, frequency, placement, PSI, GNU-time and console
captures stay local and untracked; commit only aggregate samples, resources,
validity decisions and conclusions.
