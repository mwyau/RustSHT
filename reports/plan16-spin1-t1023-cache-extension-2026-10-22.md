# Plan 16 Milestone E — T1023 cache and analysis-resampling closeout

## Result

The spin-1 synthesis coefficient-cache gate now admits T1023 and allows a 34 MiB bounded coefficient buffer. This unlocks the existing q8 order-major synthesis queue for exact GL and CC T1023 cases. The measured q8 synthesis gain is large and repeatable; q1, analysis, and F1 controls stay on their prior routes. Allocation still uses `try_reserve_exact`, so an allocation failure keeps the checked recurrence fallback.

The CC/F1 analysis resampling stage was also measured directly against DUCC's `resample_to_prepared_CC`. RustSHT's current order-tiled resample/scatter stage is comparable in time to DUCC's stage. Rust's remaining T1023 q8 analysis time is dominated by Legendre recurrence/accumulation, so Milestone E does not justify another resampling-layout prototype.

## T1023 q8 synthesis comparison

Each case uses exact latitude sampling, `phi0=0.23`, full `mmax=1023` or reduced `mmax=255`, and even-smooth longitude counts. Times are warmed medians in milliseconds across three interleaved process pairs, with calibrated batches targeting 25 ms. Cases below 100 ms used 30 batches per process; calls above 100 ms were individually timed (15 samples across three processes, including baseline CC full-order synthesis). Parenthesized values are candidate p10–p90. The `candidate/base` column gives the three paired process ratios; values below 1 favor the cache-enabled candidate.

| Grid | mmax | nphi | Baseline Rust | Candidate Rust [p10, p90] | DUCC | Candidate/base pairs | Candidate/DUCC | Max relative L2 |
|---|---:|---:|---:|---:|---:|---|---:|---:|
| GL | 255 | 512 | 37.410 | 16.085 [15.895, 17.128] | 6.334 | 0.433, 0.431, 0.422 | 2.54 | 5.64e-13 |
| GL | 1023 | 2048 | 62.868 | 39.490 [39.319, 39.781] | 19.745 | 0.630, 0.630, 0.622 | 2.00 | 3.90e-13 |
| CC | 255 | 512 | 52.032 | 16.985 [16.724, 18.566] | 6.537 | 0.327, 0.321, 0.337 | 2.60 | 1.35e-12 |
| CC | 1023 | 2048 | 115.165 | 40.715 [40.486, 40.970] | 20.423 | 0.353, 0.354, 0.353 | 1.99 | 8.02e-13 |
| F1 | 255 | 512 | 37.890 | 37.707 [37.291, 38.086] | 6.291 | 1.005, 0.980, 0.993 | 5.98 | 1.30e-12 |
| F1 | 1023 | 2048 | 62.637 | 63.060 [62.425, 63.840] | 19.732 | 1.017, 0.999, 1.005 | 3.20 | 1.23e-12 |

At T1023, raising the maximum table size from 17 MiB to 34 MiB did not change the reduced GL q8 route: the 17 MiB candidate measured 16.197 ms and the 34 MiB candidate 16.085 ms, with overlapping distributions. The extra budget is used only by full order, where it enables a 33.6 MB cache. The full-order q8 synthesis improved by 37% on GL and 65% on CC. The reduced-order cases improved by 57% on GL and 67% on CC. F1 remains ineligible for the cached route at these geometries and acts as a control.

The q1 and q8 analysis controls did not move materially: paired candidate/baseline ratios ranged 0.971–1.021 for q1 and 0.961–1.033 for q8 analysis, with medians 0.999 for both. For q8 synthesis, the candidate/base ratio range was 0.321–0.337 on CC reduced, 0.353–0.354 on CC full, 0.422–0.433 on GL reduced, and 0.622–0.630 on GL full; F1 remained within 2% of baseline.

Cold behavior was not hidden in warm timings. First-call/setup medians (baseline → candidate, ms) were GL reduced `37.73/3.52 → 19.68/3.54`, GL full `63.55/12.08 → 52.45/12.11`, CC reduced `52.83/8.03 → 20.86/8.40`, and CC full `116.00/24.58 → 54.90/25.01`. The cache adds about 0.6 ms of measured preparation at reduced order and 2.5 ms at full order; the order-major queue avoids repeated order-factor setup across ring groups.

## Executed route, allocation and correctness

The production baseline gate was `lmax <= 720` and 17 MiB. The accepted gate is `lmax <= 1023` and 34 MiB, still only for eligible spin-1 STANDARD synthesis with at least eight execution threads, a reusable GL/CC/F1 geometry, enough ring groups, and successful checked cache preparation. The measured route is specifically active for T1023 q8 GL and CC, at both tested orders. F1's group geometry does not activate the cache at T1023. q1 and analysis do not allocate it.

The exact f64 cache sizes are 524,800 complex values / 8,396,800 bytes at `mmax=255`, and 2,099,200 values / 33,587,200 bytes at `mmax=1023`. The latter remains below the 34 MiB cap. Eight order-major scratch workers are present only when the cached route is selected. The workspace owns the memory and reuses it; no global coefficient cache was added.

The native comparison covered GL, CC and F1; full/reduced order; analysis/synthesis; q1/q8: 72 backend process records and 72 numerical comparisons across the three grids. Candidate/base output relative L2 was at most `3.24e-18`; candidate versus pinned DUCC was at most `1.35e-12`. Every case remained finite where expected, and all repeat-consistency and output-reset checks passed. The new T1023 test confirms the 8/32 MiB allocations and eight-worker route, reuses the workspace after changing inputs to zero, and checks the unsafe `1.44e308` converted-order fallback against q1 including finite/Inf/NaN classifications. Existing T720 cache tests continue to cover subnormals, extreme finite values and poisoned workspace reuse.

## CC/F1 analysis resampling attribution

At T1023 full order q8, current-source Rust phase profiles are medians of eight measured calls. The `resample_to_prepared_CC` timing comes from an isolated DUCC 0.42.0 build with a temporary `steady_clock` timer around that function; the DUCC kernel flags were unchanged. These are stage timings, not a synthetic stage-by-stage full-transform subtraction.

| Grid | Rust total | Rust forward FFT | Rust resample/scatter | Rust recurrence/accumulation | DUCC `resample_to_prepared_CC` |
|---|---:|---:|---:|---:|---:|
| CC | 71.429 ms | 2.659 ms | 16.194 ms [15.933, 16.405] | 51.684 ms [51.511, 51.728] | 17.087 ms [16.478, 17.528] |
| F1 | 72.056 ms | 2.622 ms | 16.443 ms [16.192, 16.970] | 52.064 ms [51.765, 52.246] | 17.982 ms [17.304, 18.461] |

The matched uninstrumented q8 analysis medians were 72.572 ms Rust versus 33.526 ms DUCC for CC, and 73.147 ms versus 33.968 ms for F1. Rust resampling/scatter is about 23% of its profiled wall time, but its direct stage time is similar to DUCC's. The larger measured residual is in Rust's recurrence/accumulation. Keep the validated bounded order-tile resampler; no new layout prototype is warranted by this profile. DUCC's implementation is `sht.cc::resample_to_prepared_CC`: dynamic chunks of 64 orders, per-worker temporary full-ring arrays and FFT plans, north/south reflection, weighted complex FFT resampling and truncation.

## Conditions and reproducibility

- Rust baseline: `79e28abb`; production candidate: `afbdcc57` (gate constants only differ in the measured source candidate; the T1023 route/reset test is test-only).
- Rust 1.99.1, LLVM 23.1.1, release `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, Pulp V3.
- DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2.0. SHT sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver restores `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X. q1 CPU 0; q8 CPUs 0–7 in one L3 domain. CPU frequency varied during capture; raw host/process samples remain local. Median q8 runqueue wait was about 0.4–0.5% for Rust and 0.04% for DUCC; the largest sampled q8 wait was 6.9%. The three paired synthesis improvements were consistent despite occasional host variation.
- Binary SHA-256: baseline Rust `efb7a5e6be9897aed7eef1ba4432b6628d174d89c07bca8e79d8bb6f3015a1db`; candidate Rust `6a8e41ba2637cee9a1c68ded25e19c5be02f7e6ba9ae64a0abd464c3ffb85656`; DUCC `e3908ae1ce2502b1a7b9c9ede1442d85d4f977efbb62adc39542482d2352f61d`.
- Sanitized machine results: [`results.csv`](data/plan16-spin1-t1023-cache-extension-2026-10-22/results.csv), [`analysis-phase-profile.csv`](data/plan16-spin1-t1023-cache-extension-2026-10-22/analysis-phase-profile.csv), [`route-evidence.csv`](data/plan16-spin1-t1023-cache-extension-2026-10-22/route-evidence.csv), [`reduced-cache-budget-ablation.csv`](data/plan16-spin1-t1023-cache-extension-2026-10-22/reduced-cache-budget-ablation.csv), and [`metadata.json`](data/plan16-spin1-t1023-cache-extension-2026-10-22/metadata.json). Local raw host diagnostics are not included.

The summary files are reproducible from the benchmark JSONs and phase logs with `data/plan16-spin1-t1023-cache-extension-2026-10-22/summarize.py`. The native benchmark command is:

```sh
python3 scripts/benchmark_spin1_f1_coefficient_cache.py \
  --baseline /path/to/baseline-rust-worker \
  --candidate /path/to/candidate-rust-worker \
  --ducc /path/to/pinned-ducc-worker \
  --output t1023-gl.json --grid GL --lmax 1023 \
  --baseline-source 79e28abb --candidate-source afbdcc57 \
  --process-pairs 3 --target-ms 25
```

Repeat with `--grid CC` and `--grid F1`. The isolated DUCC resampler timer patch is archived at [`ducc-resample-timer.patch`](data/plan16-spin1-t1023-cache-extension-2026-10-22/ducc-resample-timer.patch); apply it only to a local copy of the pinned source and compile with `-DDUCC_PROFILE_CC_RESAMPLE=1`.

## Milestone decision

Plan 16 Milestone E is complete. Promote the 1023/34 MiB eligibility gate: it delivers large, repeatable q8 T1023 GL/CC synthesis gains, leaves q1 and analysis unchanged, and preserves the checked fallback. Keep F1 on its existing route. CC/F1 analysis resampling is already comparable to DUCC's corresponding stage, so the next Plan 17 work should continue after the owner-directed scalar dispatch and paired-FFT validation checkpoint.
