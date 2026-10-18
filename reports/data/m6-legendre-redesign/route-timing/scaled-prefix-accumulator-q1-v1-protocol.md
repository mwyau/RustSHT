# Scaled-prefix accumulator q1 matrix protocol v1

## Acquisition plan

This is the complete q1 acquisition authorized by the valid one-pair screen
in [`screen protocol v1`](scaled-prefix-accumulator-screen-v1-protocol.md).
The screen's samples are historical and are not reused in this matrix. Run six
fresh-process pairs for T1023 first. Run six T2047 pairs only if every T1023
process passes its validity gates. After both high-resolution cells, run the
six-pair T511 scalar q1 control regardless of the high-resolution speedup.
Stop a cell at its first invalid process; do not retry, replace a pair, or
remove samples.

For each bandlimit, pair 01 is clean baseline then candidate, pair 02 is
candidate then clean baseline, alternating through pair 06. Review each
completed process's local placement and interference records before launching
the next process. All binaries and the harness remain frozen through collection.

## Frozen variants and workload

- Clean baseline source HEAD:
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; extension SHA-256
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Candidate source base HEAD:
  `739275d20987fec35506916cb682f961e8a7ffaf`, with tile-128 integration
  patch SHA-256
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
- Workloads: GL f64 scalar `STANDARD` `synthesis_into`, full `mmax`,
  `phi0=0.37`, one worker. T511 is `512x1024`, T1023 is `1024x2048`, and
  T2047 is `2048x4096`. Each fresh process performs one warm-up and five
  timed full transformations for RustSHT, DUCC, and SHTns.
- Host/software specification: AMD Ryzen 9 5950X, x86-64, AVX2/FMA, 16
  physical and 32 logical cores, 96 GiB RAM, Ubuntu 26.04.1 LTS, Rust 1.99.0,
  and Python 3.11. Raw CPU IDs and host-state captures remain local.

## Validity and numerical gates

- Preflight or postflight CPU PSI `some avg10` above 2.00% rejects a process.
  Values from 0.50% through 2.00% are warnings. Report the gate outcome and keep exact transient readings local.
- Every RustSHT, DUCC, and SHTns five-call series must have population CV at
  most 5% and first-three/final-two median drift at most 5%.
- Review affinity, worker-thread placement, frequency, involuntary context
  switches, concurrent CPU-intensive work, cgroup pressure, and PSI. Clear
  evidence of interference makes a process inconclusive even below 2.00% PSI.
- Candidate and baseline maps must each remain within relative-L2 error
  `2e-11` and maximum absolute error `2e-12` against DUCC. Do not alter the
  existing numerical tolerances.
- Preserve complete timing samples and paired medians. After six valid pairs,
  compute 50,000 bootstrap resamples with seed `20261017`, the median ratio per
  resample, and linearly interpolated 2.5th/97.5th percentiles.
- The q1 gate passes only when one of T1023 or T2047 has median candidate /
  baseline ratio at most 0.90 and interval upper bound below 1.00, while the
  other high-resolution interval upper bound is at most 1.02. The T511 control
  interval upper bound must also be at most 1.02 before q8 or broader scalar/
  spin-1 regression measurements.
- Do not discard samples, retry a process, or change thresholds after seeing
  results. Keep raw preflight, during, postflight, GNU-time, process,
  frequency, placement, and console captures local and untracked. Commit only
  sanitized samples, aggregate memory, validity decisions, and conclusions.

Passing this q1 matrix does not by itself accept the candidate. The candidate
must also pass the remaining correctness and regression controls before
production routing changes.
