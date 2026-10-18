# Scaled-prefix accumulator T1023 q1 screen protocol v1

## Purpose and stop rule

This is a one-pair screen of the isolated scaled-prefix accumulator against
the clean scalar baseline. It is not an acceptance matrix. The screen proceeds
from baseline to candidate once. If either process fails a validity gate, no
candidate mate, retry, replacement, or additional workload runs. If both
processes are valid, continue to a six-pair q1 matrix only when the candidate
RustSHT median is at least 3% lower than the baseline median. Otherwise stop
and retain the result as a negative screen.

The prior T1023/T2047 ratios belong to earlier tile-128 source and binary
revisions. They are historical context only and are not combined with this
screen.

## Frozen sources and workload

- Clean baseline source HEAD:
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; extension SHA-256
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Candidate source base HEAD:
  `739275d20987fec35506916cb682f961e8a7ffaf`, with the archived tile-128
  integration patch SHA-256
  `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7` and
  `synthesis_fast.rs` SHA-256
  `edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`.
- Candidate release extension SHA-256:
  `48db7ad3a6ba4c7fb0a8c50a303d9fbe948a7cac989c6f8a6e39b2f5e1b40184`.
- Harness: `examples/python_perf_baseline.py`, SHA-256
  `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`.
- References: DUCC 0.42.0 at source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`; SHTns source revision
  `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`.
- Workload: GL f64 scalar `STANDARD` `synthesis_into`, T1023,
  `mmax=1023`, `1024x2048`, `phi0=0.37`, one worker. Each fresh process
  performs one warm-up and five timed full transformations for RustSHT, DUCC,
  and SHTns. Both variants use the same fixed compiler, harness, references,
  one-core affinity policy, and workload configuration.
- Host/software specification: AMD Ryzen 9 5950X, x86-64, AVX2/FMA, 16
  physical and 32 logical cores, 96 GiB RAM, Ubuntu 26.04.1 LTS, Rust 1.99.0,
  and Python 3.11. Raw CPU IDs and host-state captures remain local.

## Validity and numerical gates

- Preflight or postflight CPU PSI `some avg10` above 2.00% rejects the process.
  Readings from 0.50% through 2.00% are warnings and must be reported exactly.
- Each RustSHT, DUCC, and SHTns five-call series must have population CV at
  most 5% and first-three/final-two median drift at most 5%.
- Review CPU affinity, worker-thread placement, CPU frequency, involuntary
  context switches, concurrent CPU-intensive work, cgroup pressure, and PSI.
  Clear evidence of interference makes the process inconclusive below the PSI
  rejection threshold.
- Candidate and baseline maps must each remain within relative-L2 error
  `2e-11` and maximum absolute error `2e-12` against DUCC, matching the
  isolated kernel reference-test limits. The numerical comparisons do not
  substitute for the timing validity gates.
- Do not remove timing samples, retry a failed process, or revise these rules
  after observing results. Keep raw preflight, during, postflight, GNU-time,
  process, frequency, placement, and console captures local and untracked.
  Commit only sanitized timings, numerical results, validity decisions, and
  conclusions.

The one-pair screen cannot accept the candidate. A valid result with at least
3% lower candidate RustSHT median only authorizes the predeclared six-pair q1
matrix against the clean baseline. No q8 or scalar/spin-1 regression work is
authorized by this screen alone.
