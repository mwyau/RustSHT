# Exponent-epoch scalar synthesis q1 screen, results

The isolated 128-ring f64 GL scalar candidate completed the frozen T1023 and
T2047 q1 matrices. Both high-resolution comparisons passed the timing,
numerical and host-validity gates. The candidate remains experimental because
the T511 control sequence was compromised by reused output labels.

The exact source, binary, compiler, dependency, workload and acceptance
configuration are in the [v1 protocol](exponent-epoch-q1-screen-v1-protocol.md).
Each process used a fresh Python process, one warm-up, five timed
`synthesis_into` calls, `phi0=0.37`, full `mmax`, and the same baseline,
candidate, DUCC and SHTns configurations.

## High-resolution q1 results

| L | Baseline median (ms) | Candidate median (ms) | Paired median ratio | 95% bootstrap interval | Speedup | DUCC median (ms) | Candidate / DUCC | SHTns median (ms) | Candidate / SHTns |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1,023 | 303.399 | 179.827 | 0.594264 | 0.591499–0.596868 | 1.6828× | 51.300 | 3.505× | 35.790 | 5.025× |
| 2,047 | 3,397.065 | 1,408.927 | 0.414702 | 0.410220–0.416014 | 2.4114× | 375.620 | 3.751× | 273.022 | 5.160× |

The T1023 baseline/candidate process medians in pair order were
`304.972, 301.205, 301.365, 303.224, 304.330, 303.575` ms and
`181.666, 180.137, 179.472, 179.404, 180.467, 179.517` ms. The paired ratios
were `0.595681, 0.598054, 0.595530, 0.591655, 0.592998, 0.591343`.

The T2047 baseline/candidate process medians were
`3396.399, 3410.194, 3391.333, 3392.752, 3436.649, 3397.732` ms and
`1411.504, 1414.259, 1406.349, 1412.877, 1401.621, 1401.887` ms. The paired
ratios were `0.415588, 0.414715, 0.414689, 0.416440, 0.407845, 0.412595`.

Every RustSHT, DUCC and SHTns five-call series passed the fixed CV and drift
limits. Every candidate process passed the RustSHT-to-DUCC numerical limits.
At T1023, maximum absolute error was `1.556e-13` and relative-L2 error was
`1.182e-13`. At T2047, maximum absolute error was `1.110e-12` and relative-L2
error was `3.015e-13`.

Peak RSS was 267,296–277,100 KiB for baseline and 266,224–273,244 KiB for the
candidate at T1023. At T2047 it was 944,896–953,992 KiB for baseline and
945,480–954,208 KiB for the candidate. These measurements include Python,
NumPy, DUCC and SHTns loaded by the shared harness.

## CPU-pressure readings and review

The table records the exact preflight and postflight Linux CPU PSI `some
avg10` values for every high-resolution process. Values from 0.50% through
2.00% were warnings; no process exceeded 2.00%.

| L | Pair | Baseline pre → post (%) | Candidate pre → post (%) |
|---:|---:|---:|---:|
| 1,023 | 1 | 0.11 → 0.60 | 0.09 → 0.60 |
| 1,023 | 2 | 0.22 → 0.61 | 0.08 → 0.60 |
| 1,023 | 3 | 0.22 → 0.61 | 0.22 → 0.62 |
| 1,023 | 4 | 0.27 → 0.62 | 0.22 → 0.75 |
| 1,023 | 5 | 0.27 → 0.48 | 0.14 → 0.74 |
| 1,023 | 6 | 0.23 → 0.47 | 0.33 → 0.62 |
| 2,047 | 1 | 0.01 → 0.03 | 0.00 → 0.17 |
| 2,047 | 2 | 0.06 → 0.04 | 0.05 → 0.14 |
| 2,047 | 3 | 0.01 → 0.04 | 0.01 → 0.14 |
| 2,047 | 4 | 0.05 → 0.03 | 0.05 → 0.14 |
| 2,047 | 5 | 0.01 → 0.04 | 0.01 → 0.14 |
| 2,047 | 6 | 0.06 → 0.03 | 0.01 → 0.14 |

The process ran with one worker restricted to one logical CPU. Manual review
found no concurrent CPU-intensive build or benchmark, cgroup throttling, or
placement issue. CPU-frequency and thread-placement captures were reviewed
locally. No timing sample was removed or repeated.

## T511 control status

The T511 q1 control is unresolved. The available files show pair 4's candidate
process ran before pair 3's candidate process. The pair 3 candidate label also
had an older warning file; the runner overwrote the corresponding timing and
host captures without clearing that file. The current pair 3 candidate
preflight and postflight PSI values are both `0.00%`, but the older run cannot
be reconstructed. The candidate timing under that label is therefore not
counted as a first or paired sample. No further T511 cell was launched, and no
T511 confidence interval is reported. The surviving T511 captures remain
local diagnostics; any later T511 matrix must use unique labels and preserve
all prior files.

This screen establishes large q1 improvements at T1023 and T2047. It does not
accept the candidate or enable production routing. The scaled-prefix
pre-suffix transition remains the next measured kernel target; its route
profile is in
[`prefix-exponent-epoch-accumulator.md`](prefix-exponent-epoch-accumulator.md).
