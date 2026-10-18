# Polar-split T511 q1 release validation screen

## Scope and stop condition

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

## T511 synthesis results

| Version | Rust samples (ms) | Rust median | CV | first3/final2 drift | DUCC median (CV) | SHTns adapter median (CV) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 39.942, 40.099, 40.229, 40.021, 39.888 | 40.021 ms | 0.30% | 0.36% | 7.603 ms (1.16%) | 4.636 ms (0.45%) |
| Candidate | 46.231, 46.307, 46.229, 49.353, 48.706 | 46.307 ms | 2.90% | 6.04% | 7.538 ms (0.29%) | 4.635 ms (1.62%) |

Candidate/baseline Rust median ratio was `1.157068` (+15.71%). Candidate numerical relative L2 error against DUCC was `6.632e-14` (`max_abs=3.482e-14`, `rms=7.845e-16`); baseline had the same reported error. SHTns adapter versus DUCC relative L2 error was `4.286e-14` for both. GNU time covers the entire Python process, including setup not timed by the harness. Baseline: 33.93 s elapsed, 33.84 s user, 99,344 KiB max RSS, involuntary context-switch detail omitted. Candidate: 33.97 s elapsed, 33.86 s user, 99,032 KiB max RSS, involuntary context-switch detail omitted. Both preflights showed no competing compiler or benchmark process.

## Evidence

`provenance.txt`, `build.log`, and `candidate-source.patch` preserve source, compiler, CPU/ISA, reference versions, import, and artifact hashes. `run-one.sh`, `run-exactly.sh`, and `check-stability.py` retain the procedure and gate calculation. Each process has `.command`, `.stdout`, `.time`, `.preflight`, `.postflight`, and `.stability` files. This is a one-cell validation screen and does not accept the candidate.
