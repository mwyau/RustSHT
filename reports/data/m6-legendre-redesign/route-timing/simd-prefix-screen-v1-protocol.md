# SIMD prefix accumulation screen protocol v1.1

## Version update

Version 1 required SHTns timing. Its shared library was unavailable at the
preflight, before any benchmark sample was collected. Version 1.1 excludes
SHTns from this focused Rust variant screen; it changes no timing, numerical,
CPU-pressure, or stop threshold. The screen makes no new SHTns performance
claim.

## Purpose

This screen compares the isolated tile-128 accumulator with scalar prefix
accumulation against a version that vectorizes coefficient factors and sums
across ordinary latitude lanes. It is a one-pair full-transform screen, not an
acceptance matrix. A passing result with at least 3% lower candidate RustSHT
median authorizes a six-pair q1 comparison. The candidate remains experimental
until the T511 scalar control and the complete q1 regression gate pass.

## Frozen source and binaries

| Variant | `synthesis_fast.rs` SHA-256 | Release extension SHA-256 |
|---|---|---|
| Scalar prefix | `edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0` | `7e41cdc2804e11a9c49a64394ec305104e758e9d0b24525765e40cf09cd00fcf` |
| SIMD prefix | `b4a849f0fbac0537cf13a203462d75f233cdb5d0a11ef4ce9ac0d956b0521d10` | `2e161316c276140c04ce46928569a483fb3d2f9f8391100c7086f5d9b150fa16` |

Both extensions were rebuilt in the same isolated worktree with the same release
configuration, Rust 1.99.0, Python 3.11.15, NumPy 2.4.6, DUCC 0.41.0, and the
dependency lockfile. The only source change between variants is the prefix
accumulation and its SIMD range checks. The benchmark harness is
`examples/python_perf_baseline.py`, SHA-256
`e7ffbe889922cc4da1b1ece63bc24bf264603c7373adb195abde5d705bb51c07`.

The numerical reference is DUCC 0.41.0, source revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. The test machine is an AMD Ryzen
9 5950X with x86-64 AVX2/FMA, 16 physical and 32 logical cores, 96 GiB RAM,
and Ubuntu 26.04.1 LTS.

## Workload and process order

The single cell is GL f64 scalar `STANDARD` `synthesis_into`, T2047, full
`mmax=2047`, `2048x4096`, `phi0=0.37`, and one worker. Each process performs
one warm-up and five timed complete transformations for RustSHT and DUCC.
Run the scalar-prefix process first and the SIMD-prefix process second.
Do not run builds or other benchmarks during either process.

```sh
RAYON_NUM_THREADS=1 RUSTSHT_PERF_RAW=1 \
  taskset -c "${CPUSET:?set one allowed logical CPU}" \
  .venv/bin/python examples/python_perf_baseline.py \
  2047 0 1 GL 1 into synthesis both STANDARD 4096 2048
```

The command is run once per frozen extension. The harness compares each
RustSHT map with DUCC. Record all five timings for each backend, numerical
errors, aggregate peak RSS, involuntary context switches, actual worker
placement, CPU frequency, concurrent CPU-intensive processes, cgroup
throttling, and preflight/postflight CPU PSI `some avg10`.

## Validity and stop rules

- Reject a process if preflight or postflight CPU PSI `some avg10` exceeds
  2.00%. keep exact transient readings local and report the pressure-gate outcome. Readings from 0.50% to 2.00% are warnings.
- Each backend's five timing samples must have population CV at most 5% and
  first-three/final-two median drift at most 5%.
- Mark a process inconclusive when placement, contention, frequency, or other
  system evidence shows interference, even when PSI is below 2.00%.
- Keep all samples. Do not retry, replace a process, or revise the criteria
  after seeing timings. If either process is invalid, stop the screen.
- Both RustSHT outputs must meet relative-L2 error `2e-11` and maximum absolute
  error `2e-12` against DUCC.
- A valid SIMD-prefix RustSHT median at least 3% below the scalar-prefix median
  authorizes only a six-pair q1 comparison. A smaller gain or a regression ends
  this variant. This screen does not authorize q8, spin-1, production routing,
  or a candidate acceptance decision.

Keep raw preflight, during-run, postflight, process, frequency, GNU-time, and
console captures local and untracked. Commit sanitized measurements and the
validity decision.
