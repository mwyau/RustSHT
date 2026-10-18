# Frozen q1 comparison: stopped at first process

The comparison stopped after the first T511 baseline process because the fixed postflight host gate failed. No candidate process, retry, or later-bandlimit process was started. This is a host-validity stop, not a candidate performance result.

## Run and provenance

- Harness: [`python_perf_baseline.py`](../../../../examples/python_perf_baseline.py), SHA-256 `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`.
- Baseline extension: SHA-256 `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`, from clean source HEAD `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`.
- Candidate extension: SHA-256 `5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`, from candidate HEAD `739275d20987fec35506916cb682f961e8a7ffaf` plus the archived tile-128 patch SHA `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`. Candidate was not run.
- The complete pinned revisions, environment and test configuration are in [provenance.txt](provenance.txt). The frozen run helper is [run-one.sh](run-one.sh); no source or extension was changed.

## T511 baseline process

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

Raw outputs remain unchanged in [`T511-p01-baseline.stdout`](T511-p01-baseline.stdout). Rust's five synthesis samples were 39.944, 39.821, 39.975, 39.890, 39.974 ms (median 39.944 ms; CV 0.147%; first-three/final-two drift 0.030%). DUCC samples were 7.995, 7.928, 7.854, 7.845, 7.892 ms (median 7.892 ms; CV 0.693%; drift 0.754%). SHTns adapter samples were 4.688, 4.652, 4.602, 4.686, 4.595 ms (median 4.652 ms; CV 0.857%; drift 0.247%). All three sample-series stability checks passed, and the numerical comparisons passed: Rust versus DUCC relative L2 error `6.632e-14`, SHTns adapter versus DUCC `4.286e-14`. GNU time reported 99,372 kB peak RSS, 33.60 s elapsed, and involuntary context-switch detail omitted. Despite stable individual series and passing numerical checks, the host-gate failure makes this process ineligible for a paired timing comparison. No paired ratio, bootstrap interval, or acceptance assessment is available. The q1 matrix remains incomplete and was not resumed.

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

The first T1023 q1 process used the frozen clean-baseline extension. CPU-pressure warnings were reviewed under the declared protocol; exact transient readings remain local. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Snapshots showed the benchmark
thread pinned to one CPU core, no competing build or benchmark, and no sustained
external CPU load on one CPU core. Concurrent process placement and CPU-frequency telemetry remain in local diagnostics only. The
process recorded involuntary context-switch detail omitted, 276,132 kB peak RSS, and
18.78 s elapsed time. Raw snapshots and GNU time output remain local and untracked. RustSHT synthesis measured 295.985, 293.720, 293.657, 293.395, and
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
both variants and SHTns-versus-DUCC `8.764e-14`. Context-switch diagnostic counts remain local. Whole-process elapsed
times were 18.88 s and 18.63 s; peak RSS was 278,356 kB and 274,540 kB,
respectively. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. Each
benchmark worker stayed within its one-core affinity; no competing build or
benchmark and no sustained external CPU load on that core were observed. The
cgroup recorded no throttling.

CPU-pressure validity decisions were:



The postflight readings are warnings under the fixed v3 rule and remain below
the 2.00% rejection threshold. Raw preflight, periodic process, thread,
frequency, GNU-time, and console captures remain local and untracked. No
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
worker stayed within its one-core affinity. Neither process showed a competing
build or benchmark or sustained external CPU load on that core. The cgroup
had no CPU quota and recorded no throttling.

CPU-pressure validity decisions were:



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
274,560 kB, and Context-switch diagnostics were reviewed locally; exact counts remain untracked. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its
one-core affinity. No competing build or benchmark or sustained external CPU
load was observed on that core. The cgroup had no CPU quota and recorded no
throttling.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The two during-run values above 0.50% are warnings under v3; this
process remains valid. Raw diagnostic captures remain local and untracked.
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
time was 18.27 s, peak RSS was 274,596 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its one-core affinity, with no competing build
or benchmark or sustained external CPU load on that core; the cgroup recorded
no throttling.

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
The worker stayed within its one-core affinity. Process and thread snapshots
showed no competing build or benchmark and no sustained external CPU load on
that core; the cgroup recorded no throttling.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The postflight value is
a warning, not a rejection. Raw diagnostic captures remain local and
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
time was 19.13 s, peak RSS was 274,596 kB, and context-switch diagnostic counts remain local. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its one-core affinity. Process and thread
snapshots showed no competing build or benchmark or sustained external CPU
load on that core; the cgroup recorded no throttling.

CPU-pressure warnings were reviewed under the declared protocol; exact transient readings remain local. The four valid T1023 q1 pairs all favor tile 128,
but the six-pair matrix and bootstrap gate remain incomplete, so the candidate
remains unaccepted. No process was retried and no samples were removed. The
next process was pair 05 baseline.

### T1023 q1 pair 05: baseline valid, candidate mate pending

The fifth pair followed the fixed baseline-then-candidate order. All three
baseline timing series and its numerical comparison against DUCC passed.

| Backend | Samples (ms) | Median (ms) | CV | Drift |
| --- | --- | ---: | ---: | ---: |
| RustSHT | 319.585, 295.916, 297.065, 298.956, 297.581 | 297.581 | 2.9606% | 0.4044% |
| DUCC | 49.203, 47.206, 49.516, 50.720, 51.194 | 49.516 | 2.8079% | 3.5423% |
| SHTns | 37.251, 36.074, 35.117, 35.268, 35.438 | 35.438 | 2.1822% | 2.0345% |

RustSHT-versus-DUCC relative L2 error was `1.182e-13`; SHTns-versus-DUCC
was `8.764e-14`. Whole-process elapsed time was 19.01 s, peak RSS was
274,616 kB, and Context-switch diagnostics were reviewed locally; exact counts remain untracked. CPU-frequency behavior was reviewed locally; exact transient samples remain untracked. The worker stayed within its
one-core affinity. No competing build or benchmark or sustained external CPU
load was observed on that core; the cgroup recorded no throttling.

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The
during-run readings above 0.50% are warnings, not rejections. Raw diagnostic
captures remain local and untracked. The next process is the pair 05 candidate
mate.
