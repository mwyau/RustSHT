# Polar-split T511 q1 release validation screen

## Scope and stop condition

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

## T511 synthesis results

| Version | Rust samples (ms) | Rust median | CV | first3/final2 drift | DUCC median (CV) | SHTns adapter median (CV) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 39.942, 40.099, 40.229, 40.021, 39.888 | 40.021 ms | 0.30% | 0.36% | 7.603 ms (1.16%) | 4.636 ms (0.45%) |
| Candidate | 46.231, 46.307, 46.229, 49.353, 48.706 | 46.307 ms | 2.90% | 6.04% | 7.538 ms (0.29%) | 4.635 ms (1.62%) |

Candidate/baseline Rust median ratio was `1.157068` (+15.71%). Candidate numerical relative L2 error against DUCC was `6.632e-14` (`max_abs=3.482e-14`, `rms=7.845e-16`); baseline had the same reported error. SHTns adapter versus DUCC relative L2 error was `4.286e-14` for both. Whole-process elapsed time, including setup, was 33.93 s for baseline and 33.97 s for candidate. Aggregate peak RSS was 99,344 KiB and 99,032 KiB, respectively; context-switch diagnostics remain local. Both preflights showed no competing compiler or benchmark process.

## Evidence

`provenance.txt`, `candidate-source.patch`, the runner scripts, and this summary preserve source, compiler/ISA, reference versions, workload configuration, hashes, timings, and numerical results. Raw build, process, and host-state captures remain local and untracked. This is a one-cell validation screen and does not accept the candidate.
