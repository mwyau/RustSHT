# Tile-128 q1 acquisition: high-resolution criteria pass; T511 gate failed

The fixed v3 q1 screen passed its six-pair timing and numerical gates at T1023 and T2047. The candidate/baseline median ratios were 0.679651 (95% bootstrap interval 0.674353–0.685599) and 0.533182 (0.520310–0.550602), respectively. The first T511 baseline process then failed the unchanged Rust timing-stability limits: CV 8.3308% and first-three/final-two median drift 11.4305%. No T511 candidate mate or retry was run. The candidate remains isolated and unaccepted; the T511 q1 interval and q8/regression controls are incomplete.

## Run and provenance

- Harness: [`python_perf_baseline.py`](../../../../examples/python_perf_baseline.py), SHA-256 `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`.
- Baseline extension: SHA-256 `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`, from clean source HEAD `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`.
- Candidate extension: SHA-256 `5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`, from candidate HEAD `739275d20987fec35506916cb682f961e8a7ffaf` plus the archived tile-128 patch SHA `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`. It ran for all six T1023 and T2047 candidate processes; no T511 candidate process ran.
- The pinned revisions and workload configuration are in [provenance.txt](provenance.txt). The v3 process method is in [protocol-v3.md](protocol-v3.md) and the frozen runner is [run-one-v3.sh](run-one-v3.sh). Neither source nor extension changed during the acquisition.

## Earlier T511 q1 process (v1 host-gate rejection)

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

The structured record retains the benchmark configuration, timing samples and numerical errors in [`timing-samples.jsonl`](timing-samples.jsonl). Rust's five synthesis samples were 39.944, 39.821, 39.975, 39.890, 39.974 ms (median 39.944 ms; CV 0.147%; first-three/final-two drift 0.030%). DUCC samples were 7.995, 7.928, 7.854, 7.845, 7.892 ms (median 7.892 ms; CV 0.693%; drift 0.754%). SHTns adapter samples were 4.688, 4.652, 4.602, 4.686, 4.595 ms (median 4.652 ms; CV 0.857%; drift 0.247%). All three sample-series stability checks passed, and the numerical comparisons passed: Rust versus DUCC relative L2 error `6.632e-14`, SHTns adapter versus DUCC `4.286e-14`. Whole-process elapsed time was 33.60 s and aggregate peak RSS was 99,372 kB; context-switch diagnostics remain local. Despite stable individual series and passing numerical checks, the host-gate failure makes this process ineligible for a paired timing comparison. No paired ratio, bootstrap interval, or acceptance assessment is available. The q1 matrix remains incomplete and was not resumed.

## Prospective protocol v2

Protocol v2 applies only to processes started after its adoption. It raises
the preflight and postflight CPU PSI `some avg10` rejection threshold to
2.00%. Readings from 0.50% through 2.00% are recorded warnings that require
manual interference review before the next process; they do not invalidate a
completed process by themselves. The fixed CV and first-three/final-two
median-drift limits remain 5%. Clear evidence of interference still makes a
run inconclusive below the PSI rejection threshold. The exact rules and
runner are in [`protocol-v2.md`](protocol-v2.md),
[`run-one-v2.sh`](run-one-v2.sh), and
[`check-stability-v2.py`](check-stability-v2.py). The v1 T511 process above remains diagnostic under its original 0.50% host
gate and is not reclassified. When this protocol section was first recorded,
no v2 performance process had started. The planned sequence is six alternating T1023 q1 pairs, followed by T2047 q1 only if all T1023 processes pass the declared validity rules and manual review finds no unresolved interference. Raw captures are local and untracked; the committed summary retains the validity decision.

## First v2 T1023 process: stopped at baseline

The first T1023 q1 process used the frozen clean-baseline extension. CPU-pressure warnings were reviewed under the declared protocol; exact transient readings remain local. Local worker-placement and interference review matched the one-worker setup and found no disqualifying contention. Detailed placement and CPU-frequency telemetry remain local and untracked. The
Whole-process elapsed time was 18.78 s and peak RSS was 276,132 kB. Raw host-state and GNU-time captures remain local and untracked. RustSHT synthesis measured 295.985, 293.720, 293.657, 293.395, and
293.677 ms (median 293.677 ms, CV 0.3250%, drift 0.0627%). DUCC measured
49.831, 49.399, 48.647, 46.737, and 47.190 ms (median 48.647 ms, CV
2.5034%, drift 5.0065%). The DUCC drift exceeds the fixed 5% limit by
0.0065 percentage points. SHTns measured 35.002, 34.627, 34.781, 34.673,
and 35.392 ms (median 34.781 ms, CV 0.8030%, drift 0.7231%). The RustSHT
and SHTns timing series passed; the DUCC series failed. The runner stopped
with status 24. Numerical comparisons passed for this baseline process:
RustSHT versus DUCC relative L2 error was `1.182e-13`, and SHTns versus DUCC
was `8.764e-14`. No candidate process or T2047 process was launched. No samples were removed
and no process was retried. Because the baseline DUCC series failed the
predeclared stability rule, this process yields no paired timing ratio and
does not establish the T1023 performance result. The q1 cell stopped as
specified by protocol v2. The numerical series remains diagnostic evidence;
detailed host and process captures are local and untracked.


## Independent v3 acquisition

V2 remains stopped at its first T1023 baseline process because DUCC's first-three/final-two median drift exceeded the fixed 5% limit by 0.0065 percentage points. That sample remains diagnostic and is not reclassified. The v3 protocol predeclared a complete six-pair q1 acquisition from pair 1 for both variants, with unchanged timing and CPU-pressure gates. It uses the frozen baseline and tile-128 candidate binaries whose hashes were rechecked after the history cleanup. Pair 01 is now complete; see [`protocol-v3.md`](protocol-v3.md) and [`run-one-v3.sh`](run-one-v3.sh).

### T1023 q1 pair 01: both processes valid, candidate remains unaccepted

Pair 01 followed the predeclared baseline-then-candidate order. Both processes
passed the five-sample CV and first-three/final-two median-drift limits for
RustSHT, DUCC, and SHTns. Both processes passed numerical comparisons against
DUCC. PSI warnings and placement checks did not reveal
interference; both completed processes remain valid under protocol v3.

| Variant / backend | Samples (ms) | Median (ms) | CV | Drift | Peak RSS (kB) |
| --- | --- | ---: | ---: | ---: | ---: |
| Baseline RustSHT | 293.791, 296.880, 293.939, 294.236, 294.618 | 294.236 | 0.3833% | 0.1659% | 278,356 |
| Baseline DUCC | 47.606, 47.493, 50.612, 49.324, 48.117 | 48.117 | 2.4356% | 2.3162% | — |
| Baseline SHTns | 35.299, 34.948, 36.091, 35.632, 36.222 | 35.632 | 1.3383% | 1.7625% | — |
| Tile 128 RustSHT | 200.670, 200.039, 200.386, 200.691, 201.202 | 200.670 | 0.1913% | 0.2793% | 274,540 |
| Tile 128 DUCC | 50.425, 48.688, 47.776, 47.379, 48.383 | 48.383 | 2.1672% | 1.6679% | — |
| Tile 128 SHTns | 34.734, 34.406, 34.065, 34.369, 34.237 | 34.369 | 0.6431% | 0.2997% | — |

The paired RustSHT median ratio is 0.6820 (1.466x, or 31.8% lower latency)
for this single pair. RustSHT was 6.11x DUCC and 8.26x SHTns for the baseline
process, and 4.15x DUCC and 5.84x SHTns for the candidate process. These are
one-pair observations, not the six-pair acceptance result. The candidate is
still isolated and unaccepted.

Numerical checks gave RustSHT-versus-DUCC relative L2 error `1.182e-13` for
both variants and SHTns-versus-DUCC `8.764e-14`. Context-switch diagnostics remain local. Whole-process elapsed
times were 18.88 s and 18.63 s; peak RSS was 278,356 kB and 274,540 kB,
respectively. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Each
benchmark worker stayed within its one-worker placement; no competing build or
benchmark and local interference review found no disqualifying contention. The
Cgroup review found no disqualifying interference.

CPU-pressure validity decisions are summarized here; exact transient readings remain local.


The process passed the v3 pressure gate; warning-band readings did not cause rejection. Raw host-state, GNU-time, and console captures remain local and untracked. No
process was retried and no samples were removed.

### T1023 q1 pair 02: both processes valid

The second pair followed the fixed candidate-then-baseline order. Both
processes passed the stability limits for RustSHT, DUCC, and SHTns and passed
their numerical comparisons against DUCC.

| Variant / backend | Samples (ms) | Median (ms) | CV | Drift | Peak RSS (kB) |
| --- | --- | ---: | ---: | ---: | ---: |
| Tile 128 RustSHT | 199.511, 199.124, 200.275, 198.509, 199.191 | 199.191 | 0.2892% | 0.3318% | 278,572 |
| Tile 128 DUCC | 50.713, 49.188, 49.260, 49.766, 49.777 | 49.766 | 1.0952% | 1.0278% | — |
| Tile 128 SHTns | 34.518, 35.483, 34.964, 35.522, 35.197 | 35.197 | 1.0531% | 1.1237% | — |
| Baseline RustSHT | 292.824, 300.213, 303.707, 294.313, 294.244 | 294.313 | 1.4086% | 2.0164% | 274,592 |
| Baseline DUCC | 47.185, 46.988, 49.131, 47.674, 47.366 | 47.366 | 1.6051% | 0.7073% | — |
| Baseline SHTns | 35.059, 35.252, 35.401, 36.714, 35.407 | 35.401 | 1.6520% | 2.2838% | — |

The paired RustSHT median ratio is 0.6768 (1.4775x, or 32.32% lower latency).
RustSHT was 6.21x DUCC and 8.31x SHTns for the baseline process, and 4.00x
DUCC and 5.66x SHTns for the candidate process. Together with pair 01, both
valid T1023 q1 observations favor tile 128, but the six-pair matrix and its
bootstrap acceptance gate are still incomplete.

RustSHT-versus-DUCC relative L2 error was `1.182e-13` for both variants;
SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed times were 18.52 s
for candidate and 19.13 s for baseline. Context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Each
worker stayed within its one-worker placement. The local interference review found no disqualifying contention.

CPU-pressure validity decisions are summarized here; exact transient readings remain local.


The candidate postflight and baseline during-run readings in the warning
range are warnings, not rejections. Raw diagnostic captures remain local and
untracked. No process was retried and no samples were removed.

### T1023 q1 pair 03: both processes valid

The third pair followed the predeclared baseline-then-candidate order. All
three baseline timing series and its numerical comparison against DUCC passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 294.935, 294.478, 294.609, 295.034, 295.201 | 294.935 | 0.0911% | 0.1724% |
| DUCC | 49.663, 47.753, 49.565, 47.624, 49.628 | 49.565 | 1.9387% | 1.8945% |
| SHTns | 36.480, 36.180, 36.221, 35.775, 36.170 | 36.180 | 0.6241% | 0.6868% |

RustSHT-versus-DUCC relative L2 error was `1.182e-13`; SHTns-versus-DUCC
was `8.764e-14`. Whole-process elapsed time was 18.98 s, peak RSS was
274,560 kB; context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its
one-worker placement. Local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The process passed the v3 pressure gate; warning-band readings did not cause rejection. Raw diagnostic captures remain local and untracked.
The candidate mate also passed all timing and numerical gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift | Peak RSS (kB) |
| --- | --- | ---: | ---: | ---: | ---: |
| RustSHT | 201.267, 200.352, 201.178, 199.609, 199.910 | 200.352 | 0.3313% | 0.7080% | 274,596 |
| DUCC | 46.980, 49.625, 48.889, 47.499, 47.699 | 47.699 | 2.0177% | 2.7045% | — |
| SHTns | 35.720, 36.303, 35.597, 36.404, 36.213 | 36.213 | 0.9032% | 1.6251% | — |

The paired RustSHT median ratio is 0.6793 (1.4721x, or 32.07% lower
latency). RustSHT was 5.95x DUCC and 8.15x SHTns for baseline, and 4.20x
DUCC and 5.53x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `1.182e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 18.27 s, peak RSS was 274,596 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its one-worker placement, and the local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The postflight reading is a warning, not
a rejection. At this point pairs 01, 02, and 03 all favored tile 128. The
six-pair matrix and bootstrap gate remain incomplete, so
the candidate remains unaccepted. No process was retried and no samples
were removed. The next process was pair 04 candidate.

### T1023 q1 pair 04: both processes valid

The fourth pair followed the fixed candidate-then-baseline order. All three
timing series for both processes and both numerical comparisons against DUCC
passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 201.180, 203.438, 209.279, 199.597, 213.683 | 203.438 | 2.5664% | 1.5739% |
| DUCC | 50.969, 50.747, 50.767, 51.644, 51.457 | 50.969 | 0.7186% | 1.5372% |
| SHTns | 34.543, 35.275, 34.481, 35.274, 35.297 | 35.274 | 1.0803% | 2.1049% |

RustSHT-versus-DUCC relative L2 error was `1.182e-13`; SHTns-versus-DUCC
was `8.764e-14`. The candidate was 3.99x DUCC and 5.77x SHTns in this
process; the pair ratio awaits its baseline mate. Whole-process elapsed time
was 18.44 s, peak RSS was 274,504 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked.
Worker placement matched the one-worker policy. Local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The process passed the v3 pressure gate; warning-band status did not cause rejection. Raw diagnostic captures remain local and
untracked.

The baseline mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 297.031, 293.925, 294.685, 295.769, 295.182 | 295.182 | 0.3551% | 0.2678% |
| DUCC | 46.849, 49.075, 47.640, 46.766, 47.635 | 47.635 | 1.7422% | 0.9226% |
| SHTns | 35.825, 35.858, 35.305, 35.274, 35.906 | 35.825 | 0.7922% | 0.6560% |

The paired RustSHT median ratio is 0.6892 (1.4510x, or 31.08% lower
latency). RustSHT was 6.20x DUCC and 8.24x SHTns for baseline, and 3.99x
DUCC and 5.77x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `1.182e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 19.13 s, peak RSS was 274,596 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Worker placement matched the one-worker policy. The local interference review
found no disqualifying contention.

CPU-pressure warnings were reviewed under the declared protocol; exact transient readings remain local. The four valid T1023 q1 pairs all favor tile 128,
but the six-pair matrix and bootstrap gate remain incomplete, so the candidate
remains unaccepted. No process was retried and no samples were removed. The
next process was pair 05 baseline.

### T1023 q1 pair 05: both processes valid

The fifth pair followed the fixed baseline-then-candidate order. All three
timing series for both processes and both numerical comparisons against DUCC
passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 319.585, 295.916, 297.065, 298.956, 297.581 | 297.581 | 2.9606% | 0.4044% |
| DUCC | 49.203, 47.206, 49.516, 50.720, 51.194 | 49.516 | 2.8079% | 3.5423% |
| SHTns | 37.251, 36.074, 35.117, 35.268, 35.438 | 35.438 | 2.1822% | 2.0345% |

RustSHT-versus-DUCC relative L2 error was `1.182e-13`; SHTns-versus-DUCC
was `8.764e-14`. Whole-process elapsed time was 19.01 s, peak RSS was
274,616 kB; context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its
one-worker placement. Local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The
during-run readings above 0.50% are warnings, not rejections. Raw diagnostic
captures remain local and untracked.

The candidate mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift | Peak RSS (kB) |
| --- | --- | ---: | ---: | ---: | ---: |
| RustSHT | 204.429, 199.218, 201.309, 212.672, 202.353 | 202.353 | 2.2805% | 3.0657% | 274,608 |
| DUCC | 49.201, 48.971, 47.312, 47.441, 47.275 | 47.441 | 1.7879% | 3.4000% | — |
| SHTns | 40.584, 36.443, 36.944, 36.591, 36.229 | 36.591 | 4.3622% | 1.4594% | — |

The paired RustSHT median ratio is 0.6800 (1.4706x, or 32.00% lower
latency). RustSHT was 6.01x DUCC and 8.40x SHTns for baseline, and 4.27x
DUCC and 5.53x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `1.182e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 18.41 s, peak RSS was 274,608 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Worker placement matched the one-worker policy. The local interference review
found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The postflight value is a warning, not a rejection. The
five valid T1023 q1 pairs all favor tile 128, but the six-pair matrix and
bootstrap gate remain incomplete, so the candidate remains unaccepted. No
process was retried and no samples were removed. The next process was pair 06
candidate.

### T1023 q1 pair 06: both processes valid

The sixth pair followed the fixed candidate-then-baseline order. All three
timing series for both processes and both numerical comparisons against DUCC
passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 212.259, 203.282, 200.550, 200.658, 198.883 | 200.658 | 2.3524% | 1.7500% |
| DUCC | 48.480, 46.992, 46.701, 48.642, 46.730 | 46.992 | 1.8237% | 1.4768% |
| SHTns | 34.924, 35.339, 34.329, 34.689, 35.140 | 34.924 | 1.0096% | 0.0272% |

RustSHT-versus-DUCC relative L2 error was `1.182e-13`; SHTns-versus-DUCC
was `8.764e-14`. The candidate was 4.27x DUCC and 5.74x SHTns in this
process. Whole-process elapsed time was 18.35 s, peak RSS was 278,416 kB, and
Context-switch diagnostics were reviewed locally; exact counts remain untracked. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its one-core
affinity. The local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Both latter values
are warnings under v3, not rejections. Raw diagnostic captures remain local
and untracked.

The baseline mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 298.784, 294.033, 308.337, 298.640, 294.710 | 298.640 | 1.7084% | 0.7062% |
| DUCC | 47.175, 47.059, 49.732, 47.310, 47.078 | 47.175 | 2.1699% | 0.0403% |
| SHTns | 36.517, 35.977, 36.882, 35.388, 35.019 | 35.977 | 1.9160% | 3.6509% |

The paired RustSHT median ratio is 0.6719 (1.4883x, or 32.81% lower
latency). RustSHT was 6.33x DUCC and 8.30x SHTns for baseline, and 4.27x
DUCC and 5.75x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `1.182e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 18.96 s, peak RSS was 278,560 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within the configured placement. The local interference
review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The process passed the v3 pressure gate; warning-band readings did not cause rejection.

### T1023 q1 six-pair result

All six baseline/candidate pairs passed timing stability, interference review,
and numerical checks. The candidate/baseline RustSHT median ratios were
`0.682004`, `0.676800`, `0.679309`, `0.689195`, `0.679993`, and `0.671906`.
Their median is `0.679651`; Python `random.Random(20261017)` with 50,000
resamples, replacement sampling, the median statistic, and linear percentile
interpolation gives `0.674353–0.685599`. T1023 meets the high-resolution
component of the q1 criterion. The later T2047 result also passes that
component; the T511 control failure is recorded below.

### T2047 q1 pair 01: baseline valid, candidate mate pending

The first T2047 pair followed the fixed baseline-then-candidate order. All
three baseline timing series and its numerical comparison against DUCC passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 3,343.972, 3,333.515, 3,411.343, 3,344.738, 3,357.054 | 3,344.738 | 0.8229% | 0.2070% |
| DUCC | 370.783, 372.902, 380.217, 370.120, 373.924 | 372.902 | 0.9608% | 0.2360% |
| SHTns | 273.191, 270.804, 266.204, 275.300, 266.790 | 270.804 | 1.3081% | 0.0890% |

RustSHT-versus-DUCC relative L2 error was `3.015e-13`; SHTns-versus-DUCC
was `8.764e-14`. The baseline was 8.97x DUCC and 12.35x SHTns in this
process. Whole-process elapsed time was 1:26.66, peak RSS was 955,568 kB,
; context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its one-core
affinity. Local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The two readings above 0.50% are warnings, not rejections.
Raw diagnostics remain local and untracked.

The candidate mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 1,741.603, 1,745.272, 1,748.765, 1,745.539, 1,745.836 | 1,745.539 | 0.1306% | 0.0238% |
| DUCC | 366.926, 365.775, 363.630, 366.670, 365.243 | 365.775 | 0.3220% | 0.0496% |
| SHTns | 263.192, 263.869, 264.774, 266.116, 264.698 | 264.698 | 0.3716% | 0.5810% |

The paired RustSHT median ratio is 0.5219 (1.9162x, or 47.81% lower
latency). RustSHT was 8.97x DUCC and 12.35x SHTns for baseline, and 4.77x
DUCC and 6.59x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `3.015e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 1:12.54, peak RSS was 955,232 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Worker placement matched the one-worker policy. The local interference review
found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The first two nonzero readings are
warnings, not rejections. Raw diagnostic captures remain local and
untracked. This single pair favors tile 128 but does not accept the candidate;
the T2047 six-pair matrix remains incomplete. The next process was pair 02
candidate.

### T2047 q1 pair 02: both processes valid

The second pair followed the fixed candidate-then-baseline order. All three
timing series for both processes and both numerical comparisons against DUCC
passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 1,741.729, 1,742.245, 1,741.535, 1,746.631, 1,744.072 | 1,742.245 | 0.1100% | 0.2079% |
| DUCC | 371.279, 371.620, 374.232, 375.060, 375.052 | 374.232 | 0.4454% | 0.9181% |
| SHTns | 264.835, 265.811, 265.342, 264.951, 265.407 | 265.342 | 0.1314% | 0.0614% |

RustSHT-versus-DUCC relative L2 error was `3.015e-13`; SHTns-versus-DUCC
was `8.764e-14`. The candidate was 4.66x DUCC and 6.57x SHTns in this
process. Whole-process elapsed time was 1:11.87, peak RSS was 955,136 kB, and
Context-switch diagnostics were reviewed locally; exact counts remain untracked. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its one-core
affinity. The local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The first two nonzero readings are
warnings, not rejections. Raw diagnostic captures remain local and
untracked.

The baseline mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 3,259.119, 3,258.040, 3,265.884, 3,259.915, 3,257.353 | 3,259.119 | 0.0933% | 0.0149% |
| DUCC | 370.816, 371.014, 370.100, 372.734, 373.209 | 371.014 | 0.3202% | 0.5810% |
| SHTns | 268.273, 266.874, 266.841, 266.764, 266.663 | 266.841 | 0.2244% | 0.0601% |

The paired RustSHT median ratio is 0.5346 (1.8706x, or 46.54% lower
latency). RustSHT was 8.78x DUCC and 12.21x SHTns for baseline, and 4.66x
DUCC and 6.57x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `3.015e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 1:22.76, peak RSS was 955,304 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within the configured placement. The local interference
review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Warning-band readings passed v3; no pressure rejection occurred. The next process was pair 03 baseline.

### T2047 q1 pair 03: both processes valid

The third pair followed the fixed baseline-then-candidate order. All three
timing series for both processes and both numerical comparisons against DUCC
passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 3,259.577, 3,260.226, 3,266.956, 3,261.457, 3,260.247 | 3,260.247 | 0.0828% | 0.0192% |
| DUCC | 372.056, 375.288, 352.541, 372.978, 372.755 | 372.755 | 2.2654% | 0.2174% |
| SHTns | 265.847, 265.880, 264.555, 264.588, 264.635 | 264.635 | 0.2351% | 0.4669% |

RustSHT-versus-DUCC relative L2 error was `3.015e-13`; SHTns-versus-DUCC
was `8.764e-14`. Whole-process elapsed time was 1:22.60, peak RSS was
955,420 kB; context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its
one-worker placement. The local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Warning-band readings passed v3; no pressure rejection occurred. Raw diagnostics remain local and untracked.

The candidate mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 1,848.176, 1,846.184, 1,992.752, 1,824.478, 1,824.243 | 1,846.184 | 3.4073% | 1.2900% |
| DUCC | 376.295, 373.658, 367.503, 370.616, 374.278 | 373.658 | 0.8268% | 0.3241% |
| SHTns | 264.192, 265.753, 264.964, 265.385, 264.767 | 264.964 | 0.2013% | 0.0423% |

The paired RustSHT median ratio is 0.5663 (1.7659x, or 43.37% lower
latency). RustSHT was 8.75x DUCC and 12.32x SHTns for baseline, and 4.94x
DUCC and 6.97x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `3.015e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 1:12.98, peak RSS was 955,268 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Worker placement matched the one-worker policy. The local interference review
found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The first two nonzero
readings are warnings, not rejections. The T2047 six-pair matrix remains
incomplete and the candidate remains unaccepted. The next process was pair 04
candidate.

### T2047 q1 pair 04: both processes valid

The fourth pair followed the fixed candidate-then-baseline order. All three
timing series for both processes and both numerical comparisons against DUCC
passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 1,741.183, 1,748.762, 1,739.926, 1,736.287, 1,739.317 | 1,739.926 | 0.2388% | 0.1943% |
| DUCC | 371.858, 370.286, 371.345, 360.304, 372.871 | 371.345 | 1.2430% | 1.2812% |
| SHTns | 267.260, 267.112, 266.416, 265.798, 264.276 | 266.416 | 0.4068% | 0.7789% |

RustSHT-versus-DUCC relative L2 error was `3.015e-13`; SHTns-versus-DUCC
was `8.764e-14`. The candidate was 4.68x DUCC and 6.53x SHTns in this
process; the paired ratio awaits its baseline mate. Whole-process elapsed
time was 1:12.04, peak RSS was 955,260 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Worker placement matched the one-worker policy. The local interference review
found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The first two nonzero readings are
warnings, not rejections. Raw diagnostics remain local and untracked.

The baseline mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 3,272.423, 3,271.838, 3,264.329, 3,277.523, 3,259.257 | 3,271.838 | 0.1978% | 0.1054% |
| DUCC | 368.979, 362.039, 368.066, 367.951, 366.840 | 367.951 | 0.6716% | 0.1822% |
| SHTns | 265.652, 265.256, 265.983, 264.958, 264.770 | 265.256 | 0.1675% | 0.2971% |

The paired RustSHT median ratio is 0.5318 (1.8804x, or 46.82% lower
latency). RustSHT was 8.89x DUCC and 12.33x SHTns for baseline, and 4.69x
DUCC and 6.53x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `3.015e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 1:22.65, peak RSS was 955,368 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Worker placement matched the one-worker policy. The local interference review
found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Warning-band readings passed v3; no pressure rejection occurred. The
candidate remains unaccepted; the next process was pair 05 baseline.

### T2047 q1 pair 05: both processes valid

The fifth pair followed the fixed baseline-then-candidate order. All three
timing series for both processes and both numerical comparisons against DUCC
passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 3,246.342, 3,256.661, 3,253.832, 3,251.629, 3,255.053 | 3,253.832 | 0.1101% | 0.0151% |
| DUCC | 370.283, 370.447, 370.034, 362.341, 370.349 | 370.283 | 0.8619% | 1.0635% |
| SHTns | 265.878, 265.597, 264.507, 264.363, 266.192 | 265.597 | 0.2782% | 0.1203% |

RustSHT-versus-DUCC relative L2 error was `3.015e-13`; SHTns-versus-DUCC
was `8.764e-14`. Whole-process elapsed time was 1:22.49, peak RSS was
955,288 kB; context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its
one-worker placement. The local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Warning-band readings passed v3; no pressure rejection occurred. Raw diagnostics remain local and untracked.

The candidate mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 1,742.325, 1,734.431, 1,739.117, 1,740.582, 1,741.233 | 1,740.582 | 0.1585% | 0.1029% |
| DUCC | 370.516, 371.085, 368.635, 365.635, 369.266 | 369.266 | 0.5166% | 0.8302% |
| SHTns | 264.473, 264.269, 263.441, 262.805, 263.158 | 263.441 | 0.2434% | 0.4887% |

The paired RustSHT median ratio is 0.5349 (1.8694x, or 46.51% lower
latency). RustSHT was 8.79x DUCC and 12.25x SHTns for baseline, and 4.71x
DUCC and 6.61x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `3.015e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 1:11.98, peak RSS was 955,472 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Worker placement matched the one-worker policy. The local interference review
found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. A warning-band CPU-pressure reading was recorded; its exact value remains local. The next process was pair 06 candidate.

### T2047 q1 pair 06: both processes valid

The sixth pair followed the fixed candidate-then-baseline order. All three
timing series for both processes and both numerical comparisons against DUCC
passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 1,739.564, 1,738.812, 1,742.063, 1,741.857, 1,732.645 | 1,739.564 | 0.1963% | 0.1330% |
| DUCC | 366.280, 367.873, 366.534, 368.167, 366.765 | 366.765 | 0.2052% | 0.2541% |
| SHTns | 264.798, 264.826, 265.064, 263.916, 265.342 | 264.826 | 0.1807% | 0.0744% |

RustSHT-versus-DUCC relative L2 error was `3.015e-13`; SHTns-versus-DUCC
was `8.764e-14`. The candidate was 4.74x DUCC and 6.57x SHTns in this
process. Whole-process elapsed time was 1:11.75, peak RSS was 955,312 kB, and
Context-switch diagnostics were reviewed locally; exact counts remain untracked. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its one-core
affinity. The local interference review found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The first two nonzero readings are
warnings, not rejections. Raw diagnostics remain local and untracked.

The baseline mate also passed all gates:

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 3,339.506, 3,322.307, 3,355.191, 3,357.599, 3,353.419 | 3,353.419 | 0.3958% | 0.4772% |
| DUCC | 374.182, 376.811, 374.361, 360.150, 379.493 | 374.361 | 1.7987% | 1.2126% |
| SHTns | 277.245, 265.504, 268.163, 265.648, 266.179 | 266.179 | 1.6576% | 0.8451% |

The paired RustSHT median ratio is 0.5187 (1.9287x, or 48.13% lower
latency). RustSHT was 8.96x DUCC and 12.60x SHTns for baseline, and 4.74x
DUCC and 6.57x SHTns for tile 128. RustSHT-versus-DUCC relative L2 error
was `3.015e-13`; SHTns-versus-DUCC was `8.764e-14`. Whole-process elapsed
time was 1:24.86, peak RSS was 955,528 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Worker placement matched the one-worker policy. The local interference review
found no disqualifying contention.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Warning-band readings passed v3; no pressure rejection occurred.

### T2047 q1 six-pair result

All six baseline/candidate pairs passed timing stability, interference review,
and numerical checks. The candidate/baseline RustSHT median ratios were
`0.521876`, `0.534575`, `0.566271`, `0.531789`, `0.534933`, and `0.518743`.
Their median is `0.533182`; Python `random.Random(20261017)` with 50,000
resamples, replacement sampling, the median statistic, and linear percentile
interpolation gives `0.520310–0.550602`. Both T1023 and T2047 meet the
high-resolution q1 criteria. Pair-level RustSHT/DUCC gaps were 3.99–4.27x at
T1023 and 4.66–4.94x at T2047. The T511 q1 baseline failed its timing-stability
gate before a candidate mate could run, so no T511 paired interval exists and
the protocol does not permit q8 or the broader regression matrix to start.

## Current T511 q1 pair 01 baseline: stopped on Rust timing stability

The v3 process used GL f64 scalar `synthesis_into`, full `mmax`, `phi0=0.37`,
and one worker, with one warm-up and five timed calls per backend. DUCC and
SHTns passed the unchanged CV and drift limits. RustSHT did not: its CV was
8.3308% and its first-three/final-two median drift was 11.4305%, both above
5%. The process was not retried, and the candidate mate was not run.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 40.028, 40.076, 42.433, 49.193, 40.134 | 40.134 | 8.3308% | 11.4305% |
| DUCC | 7.889, 7.827, 7.716, 7.510, 7.526 | 7.716 | 1.9991% | 4.0047% |
| SHTns | 4.663, 4.643, 4.628, 4.606, 4.616 | 4.628 | 0.4347% | 0.6914% |

The Rust map's relative L2 error versus DUCC was `6.632e-14`; the SHTns
adapter's error versus DUCC was `4.286e-14`. Thus the numerical comparisons
passed for this baseline process, but it yields no candidate/baseline timing
ratio. Peak RSS was 98,204 kB, process elapsed time was 33.89 s, and the
Context-switch diagnostics were reviewed locally; exact counts remain untracked.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The process passed the v3 pressure gate without a warning-band result. Worker placement matched the one-worker policy; the local interference review found no disqualifying contention. Cgroup review found no disqualifying interference. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The available host
measurements do not explain the Rust timing outlier.

The high-resolution q1 result is favorable but incomplete as an acceptance
matrix. The next measured kernel bottleneck is the scaled prefix: the separate
T2047 route-timing diagnostic measured 1,012.1 ms in prefix work, 529.5 ms in
the ordinary suffix, and 12.2 ms in fallback work for the 128-ring tile. Those
timers are nested and instrumented; see the [route-timing record](../route-timing/summary.md)
for route counts, occupancy and the 128-versus-256 comparison. The T511
stability stop blocks further performance acceptance measurements in this
acquisition.
