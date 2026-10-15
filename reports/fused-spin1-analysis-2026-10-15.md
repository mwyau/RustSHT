# Fused spin-1 analysis reduction (2026-10-15)

The f64 GL spin-1 analysis path now advances the scalar-conversion recurrence and accumulates Q and U in one SIMD dispatch. The kernel stages both coefficient rows in existing recurrence-worker scratch. It adds both rows to the output only after both channels complete in the normal range. If either channel is unsafe or the recurrence enters the scaled range, the paired scaled recurrence handles the block. No workspace allocation or public API changed.

The fused path is enabled for `lmax >= 255` only when execution uses one thread. Five paired whole-analysis processes at T511 and T1023 showed repeatable q1 gains. The q8 and q16 ungated screen did not establish a parallel gain, so the path remains disabled for those worker counts.

## Correctness checks

The staging regression forces a late recurrence decline after both rows have partial values and verifies that neither output row changes before fallback. A second regression makes only Q products unsafe, then compares the paired fallback with the general spin recurrence. `spin1_gl_paired_analysis_matches_full_latitude_recurrence` compares the paired GL latitude recurrence with its full-latitude reference. `spin1_standard_analysis_scalar_conversion_matches_general_and_adjoint_on_all_grids` compares spin-1 analysis with the general recurrence and checks the analysis adjoint identity on GL, CC and F1 through T255.

The opt-in release test `m3_spin_t511_all_grids_matches_ducc_and_roundtrips` checks spin-1 analysis and synthesis on GL, CC and F1 at T511. `m3_spin_t1023_exact_analysis_matches_ducc` checks exact spin-1 analysis on all three grids at T1023. Their static numerical references use `ducc0==0.41.0`, from DUCC source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Both high-bandlimit tests passed. A read-only Sol High review found no correctness findings.

The focused checks passed with `RAYON_NUM_THREADS=1` where needed to exercise the q1 fused path. The workspace-wide test and Clippy suites remain part of the final E milestone validation. Python tests were not run because this change does not affect bindings.

The focused commands were:

```text
RAYON_NUM_THREADS=1 cargo test -p rustsht --all-features spin1_gl_paired_analysis_matches_full_latitude_recurrence -- --nocapture
RAYON_NUM_THREADS=1 cargo test -p rustsht --all-features spin1_fused_analysis_stages_both_channels_before_a_late_scaled_fallback -- --nocapture
RAYON_NUM_THREADS=1 cargo test -p rustsht --all-features spin1_fused_analysis_falls_back_when_only_q_products_are_unsafe -- --nocapture
RAYON_NUM_THREADS=1 cargo test -p rustsht --all-features spin1_standard_analysis_scalar_conversion_matches_general_and_adjoint_on_all_grids -- --nocapture
RAYON_NUM_THREADS=1 cargo test --release -p rustsht --all-features m3_spin_t511_all_grids_matches_ducc_and_roundtrips -- --ignored --test-threads=1
RAYON_NUM_THREADS=1 cargo test --release -p rustsht --all-features m3_spin_t1023_exact_analysis_matches_ducc -- --ignored --test-threads=1
```

## Performance screen

The benchmark used the `m1b_bench` release harness for f64 GL STANDARD analysis with `nphi=2*lmax+3`, `ntheta=lmax+1`, full `mmax`, and the automatic plan strategy. The T511 and T1023 cases use the recurrence path. Each process performed one warm-up and recorded five samples; five alternating baseline/candidate process pairs were run per case. The table reports the median of the five per-process medians and the median paired change. Negative changes indicate lower candidate time.

| Degree | Threads | Baseline median (ms) | Candidate median (ms) | Median paired change | Paired range | Faster pairs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T63 | 1 | 0.256 | 0.255 | -0.39% | -1.56% to +2.36% | 3/5 |
| T127 | 1 | 1.084 | 1.076 | -0.74% | -3.49% to -0.28% | 5/5 |
| T511 | 1 | 35.615 | 32.197 | -9.65% | -10.12% to -9.25% | 5/5 |
| T1023 | 1 | 275.650 | 249.305 | -9.61% | -10.13% to -8.38% | 5/5 |

T63 and T127 are below the fusion threshold; the T127 plan selects the existing compact table. Their small changes do not measure the fused kernel. The q1 T511 and T1023 cases each improved in all five pairs.

An initial ungated screen measured T511 and T1023 at q1, q8 and q16. q8/q16 paired ranges crossed zero in all four cells: T511 q8 `-3.59%` (range `-9.59%` to `+8.13%`), T511 q16 `-1.38%` (`-6.92%` to `+3.62%`), T1023 q8 `-1.75%` (`-3.76%` to `+4.93%`), and T1023 q16 `-5.42%` (`-8.63%` to `+3.57%`). After adding the q1 gate, a second five-pair screen compared the current candidate with baseline at q8 and q16. Those ranges also crossed zero:

| Degree | Threads | Baseline median (ms) | Candidate median (ms) | Median paired change | Paired range |
| ---: | ---: | ---: | ---: | ---: | ---: |
| T511 | 8 | 11.819 | 11.770 | -0.98% | -6.07% to +2.72% |
| T511 | 16 | 12.028 | 11.868 | -1.17% | -2.79% to +6.69% |
| T1023 | 8 | 105.778 | 106.230 | +0.13% | -0.31% to +3.59% |
| T1023 | 16 | 102.735 | 102.938 | -1.89% | -5.79% to +2.22% |

The q8/q16 controls do not show a conclusive regression or gain with fusion disabled. The code gate therefore retains only the measured q1 improvement.

The host was an AMD Ryzen 9 5950X with 16 cores and 32 logical CPUs, running Linux `7.0.14-20-pve`. The compiler was rustc `1.99.0 (b940084d7 2026-09-28)`, LLVM 23.1.1, release `opt-level=3`, with portable Pulp runtime dispatch using AVX2/FMA. q1 used CPU 0; q8 used CPUs 0–7; q16 used CPUs 0–15. Median process peak RSS at q1 was 27,448/27,268 KiB for baseline/candidate at T511 and 101,168/101,212 KiB at T1023. The kernel reuses scratch and the measurements show no consistent memory increase.

Rebuild the current harness with `cargo build --release --example m1b_bench`. Then run `reports/data/spin1-fused-analysis-2026-10-15/run-q1-paired-controls.sh` and `reports/data/spin1-fused-analysis-2026-10-15/run-gated-parallel-controls.sh`. Raw inner samples, elapsed time, maximum RSS, host load and both reproduction scripts are in [`the benchmark data`](data/spin1-fused-analysis-2026-10-15/). The ungated results are retained separately from the q1 and gated-parallel controls.
