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
gate and is not reclassified. No v2 performance process had started when this
summary was updated. The next measurements are the six alternating T1023 q1
pairs, followed by T2047 q1 only if all T1023 processes pass the declared
validity rules and their snapshots show no unresolved interference.
