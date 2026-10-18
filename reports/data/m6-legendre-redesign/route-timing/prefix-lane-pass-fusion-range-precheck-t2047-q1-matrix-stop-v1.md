# Range-precheck T2047 q1 matrix stopped at baseline process 1

## Scope and decision

The frozen six-pair matrix stopped after its first clean-baseline process.
The baseline process passed the CPU PSI, per-backend timing stability, and
DUCC numerical gates. Postflight diagnostics showed system-wide memory
pressure, corroborated by much slower untimed NumPy copy measurements. This
meets the existing protocol's interference stop condition. No candidate
matrix process ran; no paired ratio, bootstrap interval, or matrix acceptance
result exists. Do not retry or replace this process in the matrix.

The earlier one-pair screen remains useful as diagnostic evidence, but its
large NumPy-copy slowdown is now corroborated on a clean-baseline process.
Under the unchanged clear-interference clause, the one-pair comparison is
therefore treated as inconclusive. Its favorable timing ratio is not evidence
for a reproducible speedup and does not authorize continuation on this host.

## Fixed workload and references

The process used the frozen T2047 q1 protocol: GL f64 scalar `STANDARD`
synthesis into an existing output buffer, `2048x4096`, full `mmax=2047`,
`phi0=0.37`, one worker, and the v2 fixed warm-up sequence. References were
DUCC 0.42.0 and SHTns 3.7.5. Source, binary, harness, compiler, CPU, and OS
identities are in the
[matrix protocol](prefix-lane-pass-fusion-range-precheck-t2047-q1-matrix-v1-protocol.md).

## Baseline timing samples

All values are milliseconds. They are retained as diagnostic measurements,
not as a valid baseline/candidate pair.

| Backend | Five samples | Median | CV | First-three/final-two drift |
|---|---|---:|---:|---:|
| RustSHT | 3280.237, 3284.805, 3275.301, 3278.719, 3382.365 | 3280.237 | 1.2470% | 1.5336% |
| DUCC | 367.412, 374.308, 374.139, 374.216, 370.963 | 374.139 | 0.7281% | 0.4142% |
| SHTns adapter | 267.722, 266.705, 265.973, 266.741, 267.874 | 266.741 | 0.2645% | 0.2259% |

RustSHT agreed with DUCC at relative-L2 `3.015e-13` and maximum absolute
error `1.110e-12`. SHTns agreed with DUCC at relative-L2 `2.425e-13` and
maximum absolute error `6.487e-13`. Peak RSS was 955,796 KiB; runner elapsed
time was 97.08 s.

## Interference evidence

The CPU PSI preflight and postflight gates passed. The worker remained within
its one-CPU affinity, and cgroup CPU throttling was not observed. However,
postflight memory-pressure telemetry showed sustained system-wide stalls.
The harness's untimed NumPy copy medians were 56.817 ms for the coefficient
copy and 101.246 ms for the map copy. The earlier one-pair screen reported
3.693 ms and 10.690 ms for its baseline process using the same harness. Those
copy intervals are outside the transform timer, but their repeated slowdown
and the memory-pressure signal show that the host was not comparable across
processes. Exact PSI, frequency, placement, process, and GNU-time captures
remain local and untracked.

No acceptance threshold changed. The existing rule that clear interference
invalidates a process applies even when CPU PSI remains below its rejection
limit. The matrix stopped before its candidate process, as required.

## Status

The range-precheck candidate remains isolated and unaccepted. The favorable
single-pair timing result is reclassified as inconclusive for comparative
performance because of the later host-pressure evidence. The measured
scaled-prefix route remains the next kernel bottleneck, but no further
expensive timing matrix should run until the host can provide interpretable
paired measurements.
