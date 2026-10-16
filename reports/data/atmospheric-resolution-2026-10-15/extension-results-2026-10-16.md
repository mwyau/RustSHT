# Additional atmospheric-resolution measurements

This note records f32, q16, analysis- and synthesis-phase, and analysis-leaf measurements added after the initial f64 q1/q8 matrix. Raw outputs and runner scripts remain beside this file.

## Environment and scope

The Rust runs used the AMD Ryzen 9 5950X (16 physical cores, 32 logical CPUs), Rust 1.99.0 / LLVM 23.1.1, the default release profile, portable Pulp runtime dispatch, and the reusable native plan/workspace interface. No `target-cpu=native` or custom global CPU flags were used. The host was shared; affinity does not provide CPU isolation. The f32 binary SHA256 is recorded in `native/f32/binary.sha256`; the q16 binary SHA256 and source revision are in `native/q16/`.

## f32 screen

The f32 harness used GL `ntheta=lmax+1`, full `mmax`, `phi0=0.37`, even `nphi`, one initial call and five measured samples per operation. It measures reused plans and workspaces. The table reports medians in milliseconds for q1 / q8 and the process peak RSS.

| Truncation and grid | Spin | Synthesis q1 / q8 | Analysis q1 / q8 | Peak RSS |
| --- | ---: | ---: | ---: | ---: |
| T511, 512×1024 | 0 | 551 / 105 | 286 / 54 | 15.7 MiB |
| T511, 512×1024 | 1 | 1,064 / 148 | 902 / 164 | 15.8 MiB |
| T1023, 1024×2048 | 0 | 4,064 / 675 | 2,058 / 367 | 51.8 MiB |
| T1023, 1024×2048 | 1 | 8,353 / 1,155 | 6,872 / 1,287 | 51.9 MiB |

The first T1023 run overlapped unrelated Cargo/rustc activity. A repeat on CPUs 0–7 produced medians within 4.6% for every direction/spin/thread combination: scalar synthesis 3,932 / 680 ms and analysis 1,964 / 363 ms; spin-1 synthesis 8,059 / 1,175 ms and analysis 6,620 / 1,272 ms. The repeat is an independent-affinity screen, not a paired isolated-host experiment. The f32 results show a substantially higher cost than the corresponding f64 screen at these sizes. The current f32 generic path should be profiled before f32 atmospheric runs are treated as a performance target.

## q16 screen

These native f64 q1/q16 comparisons use full-band GL plans at T1023 (`1024×2048`) and T1279 (`1280×2560`). Values are medians in milliseconds. The q16 T1023 processes overlapped a separate RustFFT build/test pinned to CPU 30, the SMT sibling of CPU 14; treat them as a noisy screen. q16 T1279 is also a shared-host result.

| Truncation | Spin | Synthesis q1 / q16 | Analysis q1 / q16 | Peak RSS, q16 |
| --- | ---: | ---: | ---: | ---: |
| T1023 | 0 | 306 / 52.3 | 113 / 18.2 | 167 MiB |
| T1023 | 1 | 291 / 56.4 | 188 / 32.8 | 168 MiB |
| T1279 | 0 | 643 / 195 | 228 / 32.5 | 234 MiB |
| T1279 | 1 | 574 / 194 | 358 / 44.9 | 286 MiB |

## Analysis phase profile

`profiles/phase-profile.log` contains five samples per case. The diagnostic uses `nphi=2*lmax+3`: grids are `721×1443` at T720, `1024×2049` at T1023, `1280×2561` at T1279, and `2048×4099` at T2047. The T720 phase grid has 15 fewer longitudes than the FFT-friendly `721×1458` grid in the main matrix. The T1023 and T1279 phase grids have one more longitude than their minimum-even matrix grids. q1/q8 wall and phase medians are summarized below in milliseconds. The recurrence timer combines Legendre evaluation and coefficient accumulation. At q>1, the forward-FFT stage schedules latitude row groups across the worker pool; each worker processes its assigned rows serially. The timer does not isolate individual RustFFT calls.

| Truncation | Diagnostic grid | Spin | Threads | Whole call | Row FFT | Legendre recurrence + accumulation |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T720 | 721×1443 | 0 | 1 | 50.6 | 10.8 (21%) | 38.7 (77%) |
| T720 | 721×1443 | 0 | 8 | 8.62 | 1.72 (20%) | 6.02 (70%) |
| T720 | 721×1443 | 1 | 1 | 85.1 | 20.1 (24%) | 64.5 (76%) |
| T720 | 721×1443 | 1 | 8 | 15.1 | 3.00 (20%) | 11.9 (78%) |
| T1023 | 1024×2049 | 0 | 1 | 143 | 37.1 (26%) | 104 (72%) |
| T1023 | 1024×2049 | 0 | 8 | 23.7 | 5.35 (23%) | 16.2 (68%) |
| T1023 | 1024×2049 | 1 | 1 | 252 | 74.1 (29%) | 177 (70%) |
| T1023 | 1024×2049 | 1 | 8 | 41.1 | 9.74 (24%) | 30.6 (74%) |
| T1279 | 1280×2561 | 0 | 1 | 256 | 42.0 (16%) | 210 (82%) |
| T1279 | 1280×2561 | 0 | 8 | 42.9 | 5.02 (12%) | 33.3 (77%) |
| T1279 | 1280×2561 | 1 | 1 | 442 | 73.0 (17%) | 368 (83%) |
| T1279 | 1280×2561 | 1 | 8 | 74.0 | 9.92 (13%) | 62.7 (85%) |
| T2047 | 2048×4099 | 0 | 1 | 1,204 | 123 (10%) | 1,059 (88%) |
| T2047 | 2048×4099 | 0 | 8 | 234 | 16.5 (7%) | 191 (82%) |
| T2047 | 2048×4099 | 1 | 1 | 1,981 | 225 (11%) | 1,753 (89%) |
| T2047 | 2048×4099 | 1 | 8 | 390 | 34.0 (9%) | 352 (90%) |

The measurement timer also records order-spectrum preparation and setup phases; their raw samples are retained. The three named phase totals explain the large majority of the call. At q8, recurrence and accumulation remain the largest phase for these grids. The row-parallel FFT stage is a secondary cost, so these profiles do not justify FFT replacement without a grid-length-specific end-to-end comparison.

## Synthesis phase profile

`profiles/synthesis-phase-profile.log` contains five timed samples after one warmup for each q1/q8 case. It uses the same FFT-friendly GL grids as the high-resolution native matrix: T720 `721×1458`, T1023 `1024×2048`, T1279 `1280×2560`, and T2047 `2048×4096`. The test-only diagnostic reuses the plan and workspace. Setup includes latitude preparation and spectrum clearing. The recurrence phase also includes spin coefficient conversion or cached coefficient preparation when selected, plus spectral accumulation. The inverse phase includes serial inverse FFT execution and row copy-out. It does not isolate individual RustFFT calls. Absolute call times are specific to the test diagnostic and its deterministic coefficient input; phase shares describe the execution path and should not be substituted for the native benchmark medians.

| Truncation | Grid | Spin | Whole q1 / q8 (ms) | Setup q1 / q8 (ms) | Legendre and accumulation q1 / q8 (ms) | Inverse FFT and writeback q1 / q8 (ms) | q8 Legendre share | q8 FFT share |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T720 | 721×1458 | 0 | 88.0 / 33.7 | 0.143 / 0.142 | 85.5 / 31.2 | 2.323 / 2.260 | 92.6% | 6.7% |
| T720 | 721×1458 | 1 | 98.4 / 40.3 | 0.468 / 0.751 | 92.2 / 33.7 | 5.683 / 5.839 | 83.6% | 14.5% |
| T1023 | 1024×2048 | 0 | 271 / 67.7 | 0.427 / 0.653 | 266 / 62.0 | 4.693 / 5.009 | 91.5% | 7.4% |
| T1023 | 1024×2048 | 1 | 293 / 73.9 | 1.399 / 1.490 | 281 / 62.9 | 9.514 / 9.471 | 85.1% | 12.8% |
| T1279 | 1280×2560 | 0 | 589 / 275 | 1.229 / 1.322 | 580 / 266 | 7.657 / 7.898 | 96.6% | 2.9% |
| T1279 | 1280×2560 | 1 | 590 / 286 | 2.500 / 2.891 | 572 / 267 | 15.502 / 15.854 | 93.4% | 5.6% |
| T2047 | 2048×4096 | 0 | 3,128 / 752 | 3.813 / 3.783 | 3,104 / 728 | 20.141 / 20.218 | 96.8% | 2.7% |
| T2047 | 2048×4096 | 1 | 4,184 / 989 | 7.904 / 8.234 | 4,135 / 936 | 40.680 / 41.106 | 94.7% | 4.2% |

The q8 Legendre phase remains dominant on these matrix-matched grids. The largest FFT share is spin 1 at T720, where the two serial inverse transforms account for 14.5% of diagnostic wall time; at T1279 and T2047 they account for 5.6% and 4.2%. The earlier successful odd-longitude diagnostic is preserved as `profiles/synthesis-phase-profile-odd-longitudes.log`. Its odd counts use the full-complex FFT path, so it is excluded from these conclusions. Run metadata includes the exact diagnostic binary hash, source diff hash, compiler, CPU, affinity, and host load in `profiles/synthesis-phase-profile-metadata.txt`.

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

The f32 commands are in `run-f32-atmospheric.sh` and `run-f32-T1023-repeat.sh`; q16 commands are in `run-high-q16.sh`. The synthesis phase profile uses `run-synthesis-phase-profile.sh`. The analysis profiling commands are:

```sh
TMPDIR=target/rustsht-test-tmp taskset -c 8-15 cargo test --release --lib m5_analysis_phase_profile -- --ignored --test-threads=1 --nocapture
TMPDIR=target/rustsht-test-tmp taskset -c 8-15 cargo test --release --lib m5_analysis_leaf_imbalance_profile -- --ignored --test-threads=1 --nocapture
```

The exact raw profile logs are in `profiles/`. These are diagnostic timings from a shared host, not alternating candidate/baseline pairs. The initial synthesis attempt with invalid nonreal `m=0` B coefficients is also retained with an explicit filename and is not used in the report.
