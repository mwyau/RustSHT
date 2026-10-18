
## Scope and stop reason

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

## T511 GL scalar synthesis q1

Configuration: f64, STANDARD, `ntheta=512`, `nphi=1024`, full `mmax=511`, `phi0=0.37`, `scope=into`, `backend=all`, `runs=1`, `RAYON_NUM_THREADS=1`, one CPU core. Each process performs the harness correctness comparison, one untimed warm-up, then five timed transform calls per backend.

| Version | Rust samples (ms) | Rust median | CV | first3/final2 drift | DUCC median (CV) | SHTns adapter median (CV) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 40.372, 40.089, 40.026, 40.228, 40.394 | 40.228 ms | 0.37% | 0.55% | 7.565 ms (0.54%) | 4.669 ms (0.39%) |
| Candidate | 118.861, 118.984, 119.013, 118.950, 119.088 | 118.984 ms | 0.06% | 0.03% | 7.544 ms (0.48%) | 4.657 ms (0.58%) |

Both processes passed all three timed-series stability checks (CV and first-three/final-two median drift each <=5%). Baseline/candidate Rust synthesis relative L2 errors against DUCC were both `6.632e-14` (`max_abs=3.482e-14`, `rms=7.845e-16`). SHTns adapter versus DUCC relative L2 error was `4.286e-14` for both. The observed T511 candidate/baseline Rust median ratio is `2.957741` (+195.77%). This is one stable, unfavorable cell; it is not a full-matrix performance decision. Whole-process elapsed time, including setup, was 33.55 s for baseline and 34.18 s for candidate. Aggregate peak RSS was 99,304 KiB and 98,996 KiB, respectively; context-switch diagnostics remain local. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local.

## Incomplete cells

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

The runner, stability checker, exact process commands, source/build provenance, structured benchmark timings are retained; raw process and host captures remain local. No acceptance decision is made from this incomplete screen.
