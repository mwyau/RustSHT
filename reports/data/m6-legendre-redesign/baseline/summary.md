# Clean-source f64 scalar synthesis baseline screen

## Scope and stop condition

The isolated RustSHT extension was built from clean source commit `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`. The established Python harness was `examples/python_perf_baseline.py`, SHA256 `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`. The planned order was three independent processes per size in round-robin order: T511, T1023, T2047, repeated three times. Each process used GL f64 scalar STANDARD synthesis, `ntheta=L+1`, `nphi=2(L+1)`, full `mmax`, `phi0=0.37`, `scope=into`, `backend=all`, `runs=1`, `RAYON_NUM_THREADS=1`, and one CPU core affinity. The harness performed its normal correctness comparisons and warm-up, then retained five timed calls for each backend. Only T511 run 1 launched. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. CPU-pressure warnings were reviewed under the declared protocol; exact transient readings remain local. The next scheduled T1023 preflight failed the historical v1 pressure gate; no Python timing process launched. The refusal decision is summarized here and its raw capture remains local. The remaining seven processes were not attempted. No sample was removed or rerun. This is an incomplete baseline screen, not a performance conclusion.

## T511 run 1

| Backend/operator | Five timed calls (ms) | Median (ms) | Population CV | First 3 vs final 2 median drift |
|---|---:|---:|---:|---:|
| RustSHT `synthesis_into` | 40.266, 40.139, 41.471, 40.322, 40.082 | 40.266 | 1.27% | 0.16% |
| DUCC synthesis | 7.620, 7.531, 7.517, 7.517, 7.498 | 7.517 | 0.57% | 0.31% |
| SHTns adapter synthesis | 4.608, 4.675, 4.648, 4.651, 4.653 | 4.651 | 0.47% | 0.09% |

CV is population standard deviation divided by the five-sample mean. Drift is the absolute difference between the first-three and final-two medians divided by the all-five median. All three timed series meet the prospective within-process CV and drift limits of 5%; however, the process's postflight host PSI fails the host gate. RustSHT versus DUCC synthesis relative L2 error was `6.632e-14` (`max_abs=3.482e-14`, `rms=7.845e-16`). SHTns versus DUCC relative L2 error was `4.286e-14` (`max_abs=2.136e-14`, `rms=5.070e-16`). These are numerical checks for this cell only. Whole-process elapsed time was 34.18 s and aggregate peak RSS was 99,108 KiB; these include setup outside the timed transform calls. Thus the 34 s process duration is not a transform-call duration. The CPU-pressure validity decisions, numerical samples, and RSS are retained in this summary. Detailed process and host-state captures are local and untracked; they did not identify the cause of the postflight pressure increase.

## Provenance and reproducibility

`provenance.txt` records compiler, Python/reference revisions, workload configuration, and binary hashes. `build.txt`, `run-one.sh`, and `run-matrix.sh` record the isolated build and planned order. Numerical measurements and CPU-pressure validity decisions are summarized above. Raw stdout, GNU-time, command, and host snapshots remain local and untracked. The generated Rust extension remains only in the isolated baseline worktree.
