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

V2 remains stopped at its first T1023 baseline process because DUCC's first-three/final-two median drift exceeded the fixed 5% limit by 0.0065 percentage points. That sample remains diagnostic and is not reclassified. The v3 protocol predeclares a complete six-pair q1 acquisition from pair 1 for both variants, with unchanged timing and CPU-pressure gates. It uses the frozen baseline and tile-128 candidate binaries whose hashes were rechecked after the history cleanup. No v3 process has started yet. See [`protocol-v3.md`](protocol-v3.md) and [`run-one-v3.sh`](run-one-v3.sh).
