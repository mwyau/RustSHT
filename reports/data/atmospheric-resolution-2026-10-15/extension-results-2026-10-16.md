# Additional atmospheric-resolution measurements

This note records f32, q16, analysis-phase, and analysis-leaf measurements added after the initial f64 q1/q8 matrix. Raw outputs and runner scripts remain beside this file.

## Environment and scope

The Rust runs used the x86-64 CPU (host topology omitted), Rust 1.99.0 / LLVM 23.1.1, the default release profile, portable Pulp runtime dispatch, and the reusable native plan/workspace interface. No `target-cpu=native` or custom global CPU flags were used. The host was shared; affinity does not provide CPU isolation. The f32 binary SHA256 is recorded in `native/f32/binary.sha256`; the q16 binary SHA256 and source revision are in `native/q16/`.

## f32 screen

The f32 harness used GL `ntheta=lmax+1`, full `mmax`, `phi0=0.37`, even `nphi`, one initial call and five measured samples per operation. It measures reused plans and workspaces. The table reports medians in milliseconds for q1 / q8 and the process peak RSS.

| Truncation and grid | Spin | Synthesis q1 / q8 | Analysis q1 / q8 | Peak RSS |
| --- | ---: | ---: | ---: | ---: |
| T511, 512×1024 | 0 | 551 / 105 | 286 / 54 | 15.7 MiB |
| T511, 512×1024 | 1 | 1,064 / 148 | 902 / 164 | 15.8 MiB |
| T1023, 1024×2048 | 0 | 4,064 / 675 | 2,058 / 367 | 51.8 MiB |
| T1023, 1024×2048 | 1 | 8,353 / 1,155 | 6,872 / 1,287 | 51.9 MiB |

The first T1023 run overlapped unrelated Cargo/rustc activity. A repeat on 8 pinned CPU cores produced medians within 4.6% for every direction/spin/thread combination: scalar synthesis 3,932 / 680 ms and analysis 1,964 / 363 ms; spin-1 synthesis 8,059 / 1,175 ms and analysis 6,620 / 1,272 ms. The repeat is an independent-affinity screen, not a paired isolated-host experiment. The f32 results show a substantially higher cost than the corresponding f64 screen at these sizes. The current f32 generic path should be profiled before f32 atmospheric runs are treated as a performance target.

## q16 screen

These native f64 q1/q16 comparisons use full-band GL plans at T1023 (`1024×2048`) and T1279 (`1280×2560`). Values are medians in milliseconds. Concurrent process-placement details were retained only in local diagnostics. The diagnostic grid uses `nphi=2*lmax+3`, which is one longitude larger than the paired even grids at T1023/T1279 and the FFT-friendly T720 grid in the main matrix. q1/q8 wall and phase medians are summarized below in milliseconds. The recurrence timer combines Legendre evaluation and coefficient accumulation. At q>1, the forward-FFT stage schedules latitude row groups across the worker pool; each worker processes its assigned rows serially. The timer does not isolate individual RustFFT calls.

| Truncation | Spin | Threads | Whole call | Row FFT | Legendre recurrence + accumulation |
| --- | ---: | ---: | ---: | ---: | ---: |
| T720 | 0 | 1 | 50.6 | 10.8 (21%) | 38.7 (77%) |
| T720 | 0 | 8 | 8.62 | 1.72 (20%) | 6.02 (70%) |
| T720 | 1 | 1 | 85.1 | 20.1 (24%) | 64.5 (76%) |
| T720 | 1 | 8 | 15.1 | 3.00 (20%) | 11.9 (78%) |
| T1023 | 0 | 1 | 143 | 37.1 (26%) | 104 (72%) |
| T1023 | 0 | 8 | 23.7 | 5.35 (23%) | 16.2 (68%) |
| T1023 | 1 | 1 | 252 | 74.1 (29%) | 177 (70%) |
| T1023 | 1 | 8 | 41.1 | 9.74 (24%) | 30.6 (74%) |
| T1279 | 0 | 1 | 256 | 42.0 (16%) | 210 (82%) |
| T1279 | 0 | 8 | 42.9 | 5.02 (12%) | 33.3 (77%) |
| T1279 | 1 | 1 | 442 | 73.0 (17%) | 368 (83%) |
| T1279 | 1 | 8 | 74.0 | 9.92 (13%) | 62.7 (85%) |
| T2047 | 0 | 1 | 1,204 | 123 (10%) | 1,059 (88%) |
| T2047 | 0 | 8 | 234 | 16.5 (7%) | 191 (82%) |
| T2047 | 1 | 1 | 1,981 | 225 (11%) | 1,753 (89%) |
| T2047 | 1 | 8 | 390 | 34.0 (9%) | 352 (90%) |

The measurement timer also records order-spectrum preparation and setup phases; their raw samples are retained. The three named phase totals explain the large majority of the call. At q8, recurrence and accumulation remain the largest phase for these grids. The row-parallel FFT stage is a secondary cost, so these profiles do not justify FFT replacement without a grid-length-specific end-to-end comparison.

## Analysis-leaf profile

`profiles/leaf-profile.log` preserves per-leaf ranges, work estimates, worker IDs, phase offsets, and recurrence-block counts. The table gives medians across five samples at q8.

| Truncation | Spin | Whole call | Leaf capacity fraction | Slowest / fastest leaf time | Max / min estimated work |
| --- | ---: | ---: | ---: | ---: | ---: |
| T720 | 0 | 8.56 ms | 0.571 | 1.82× | 1.020× |
| T720 | 1 | 15.7 ms | 0.638 | 1.59× | 1.020× |
| T1279 | 0 | 43.1 ms | 0.629 | 1.91× | 1.007× |
| T1279 | 1 | 74.4 ms | 0.722 | 1.61× | 1.007× |
| T2047 | 0 | 232 ms | 0.587 | 2.51× | 1.007× |
| T2047 | 1 | 387 ms | 0.663 | 2.19× | 1.007× |

The current work estimate equalizes estimated leaf work closely at T1279/T2047 but does not equalize elapsed time. This is evidence that the estimate omits costs that vary by order range. It does not by itself justify increasing the leaf count: the existing overpartition experiment regressed complete transforms at T511 and T720.

## Reproduction

The f32 commands are in `run-f32-atmospheric.sh` and `run-f32-T1023-repeat.sh`; q16 commands are in `run-high-q16.sh`. The profiling command for each diagnostic is:

```sh
TMPDIR=target/rustsht-test-tmp taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo test --release --lib m5_analysis_phase_profile -- --ignored --test-threads=1 --nocapture
TMPDIR=target/rustsht-test-tmp taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo test --release --lib m5_analysis_leaf_imbalance_profile -- --ignored --test-threads=1 --nocapture
```

The exact raw profile logs are in `profiles/`. These are diagnostic timings from a shared host, not alternating candidate/baseline pairs.
