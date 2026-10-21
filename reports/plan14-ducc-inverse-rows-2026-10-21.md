# Plan 14: matched Rust/DUCC inverse-row profile

**Rust source:** `41dc652030ad6fa3b328e656579d2a00511bd9ab` (production
`src/fft.rs` unchanged during this experiment). **Reference:** pinned DUCC
0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Host: Ryzen 9 5950X.
Rust 1.99.1 / LLVM 23.1.1 used release optimization and
`-C target-cpu=x86-64-v3`; runtime AVX2 and FMA were true. The source-traced
RustFFT planner selects its AVX/FMA planner on this host. DUCC used GCC 15.2,
upstream C++17, and the production numerical flags
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the benchmark driver
used `-fno-fast-math -ffp-contract=off`. The exact build script and source
checks are in [`benches/plan14_q8_odd_inverse`](../benches/plan14_q8_odd_inverse/).

## Matched boundary and method

This compares T720 synthesis inverse rows after the spectra are available:
721 rows, `mmax=180`, `nphi=361` (odd minimum) and `nphi=378` (even smooth),
at q1 and q8. Deterministic inputs fill every retained mode from 0 through
180; higher even-length bins are zero. Rust compiles the exact private
`src/fft.rs` and `src/error.rs` modules into the harness and calls
`RealFftPlan::inverse_series`, with the same serial loop at q1 and
`par_chunks_mut` row groups at q8. DUCC calls `pocketfft_r<double>` through
`execDynamic`, with the same packed half spectrum, worker scratch, and output
shape. Plans, output buffers, workspaces, and thread pools are created before
timing. Every call overwrites every output sample. Checksums and cross-library
error are evaluated after the timed batches. The first execution is recorded
separately from warmed samples.

Three independent Rust/DUCC process pairs were run for each case, alternating
which backend ran first. Each process calibrated 30 batches to approximately
30 ms and reported the per-batch p10, median, and p90. q1 was pinned to CPU 0;
q8 used CPUs 0–7, which map to eight distinct physical cores on this host.
Raw process logs, per-pair medians and validation are in
[`reports/data/plan14-ducc-fft-rows-2026-10-21`](data/plan14-ducc-fft-rows-2026-10-21/).
The runner also captures PSI and frequency snapshots locally; raw host
captures are kept untracked.

## Results

The q1 timings were stable across three process pairs. Values below are
medians across process medians; ratios and excesses are medians of paired
process results.

| Rows / nphi / threads | Rust ms (process range) | DUCC ms (process range) | Paired Rust/DUCC | Paired excess ms | Relative L2 |
|---|---:|---:|---:|---:|---:|
| 721 / 361 / q1 | 2.189 (2.176–2.195) | 0.864 (0.861–0.890) | 2.540× | +1.328 | 5.40e-16 |
| 721 / 378 / q1 | 0.391 (0.388–0.395) | 0.570 (0.569–0.571) | 0.687× | −0.178 | 4.90e-16 |

q8 needed a focused repeat because CPU pressure increased during the first
run. The two episodes are reported separately; each row shows the median of
three process medians and the median paired ratio/excess.

| Episode / nphi | Rust ms (process range) | DUCC ms (process range) | Paired Rust/DUCC | Paired excess ms |
|---|---:|---:|---:|---:|
| Initial q8 / 361 | 0.458 (0.453–0.508) | 0.163 (0.163–0.166) | 2.811× | +0.295 |
| Initial q8 / 378 | 0.103 (0.100–0.108) | 0.113 (0.112–0.114) | 0.912× | −0.010 |
| q8 repeat / 361 | 0.453 (0.446–0.458) | 0.165 (0.163–0.169) | 2.771× | +0.290 |
| q8 repeat / 378 | 0.102 (0.100–0.111) | 0.115 (0.112–0.118) | 0.869× | −0.016 |

For odd q8 the within-process Rust p90 range was `0.469–0.570 ms` in the
initial episode and `0.461–0.482 ms` in the repeat; DUCC p90s were
`0.164–0.170 ms` and `0.165–0.169 ms`. Host pressure rose during both q8
episodes, with scheduling stalls visible in the repeat. Thus the q8 absolute
latencies and exact ratios are host-sensitive, but both independent episodes
retain the odd-length gap and an even-length Rust lead. Treat q8 as a
directional attribution, not a clean absolute baseline. The q1 results remain
the stable control.

Plan/workspace creation is excluded. The raw logs and `summary.csv` record
first execution times separately from warm batches; those calls may include
first-use worker startup and are not used in warmed ratios. The governor was
`performance`, but CPU frequency varied and was not locked or sampled per
batch. The runner captured raw transient host and frequency snapshots
locally; they are intentionally kept untracked.

## Attribution and next work

The difference is specific to the odd-length transform path. Rust's
`src/fft.rs` expands the positive spectrum to a length-361 Hermitian complex
array, executes a complex inverse, then extracts real samples. DUCC packs the
real half spectrum and runs its dedicated real inverse. For the even control,
Rust's production real-half algorithm is faster than DUCC at both thread
counts, which argues against a generic Rust row scheduler or output-write
penalty as the main cause of the odd gap.

The independently measured whole T720 reduced odd synthesis call was
1.695 ms Rust versus 1.288 ms DUCC (+0.407 ms). The matched inverse-row excess
was +0.29–0.30 ms in the contention-affected q8 episodes, the same order as
the whole-call excess. This is a ranking comparison, not an exact subtraction
of timings collected under different host conditions. The previously tested
`rustsht-fft` real C2R candidate was
8.8–10.0% slower than the current Rust odd-complex path, so it remains
rejected and production FFT code is unchanged. No new replacement is
justified from that candidate.

The next Plan 14 task is the remaining small-q1 T127 analysis residual. Keep
the odd real-FFT gap as a documented lead; revisit it only with a different,
measured implementation mechanism that beats the existing complex route.
