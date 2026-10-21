# Plan 15 GL spin-1 dynamic ring-group scheduling A/B

**Date:** 2026-10-21. **Production change:** `4e62e2003a90b0b69fe9c3e0e02779dd554630a5`.

## Finding

The q8 f64 GL STANDARD spin-1 synthesis route had statically grouped several
latitude-ring jobs per recurrence workspace. Work per group varies with the
polar order limits, so equal group counts did not mean equal work. The accepted
change makes each of the bounded active recurrence workspaces pull one group
at a time from a shared queue. It is restricted to the existing q8 GL paired
scalar-coefficient-cache path; other operations and routes keep the prior
scheduler. The queue allocates no per-transform storage and uses at most
`min(group_count, execution_threads)` recurrence workspaces.

Three interleaved process pairs, with ten calibrated batches per process and
backend, show lower Rust latency in all four matched T720 workloads. Candidate
median latency fell 19–39% versus the immediately preceding production source.
Outputs were bitwise identical to that source. The remaining Rust/DUCC gap is
still 3.1–6.5x, so this is a measured scheduling improvement rather than
performance parity. These are exact-source results; the more favorable ratios
in the initial `4e62e200` commit message came from an earlier, noisier screen
and are superseded by this recheck.

## Matched whole-transform results

All times are median milliseconds per two-channel synthesis call. Parentheses
show the p10–p90 range across the 30 batch observations. Candidate/parent is
also listed as the three individual paired-process median ratios.

| `mmax` | `nphi` | longitude | Parent Rust | Candidate Rust | Cand./parent paired ratios | DUCC | Cand./DUCC | Candidate relative L2 |
|---:|---:|---|---:|---:|---|---:|---:|---:|
| 180 | 378 | even smooth | 35.246 (34.581–36.010) | 28.165 (22.947–31.777) | 0.823, 0.788, 0.803 | 4.328 (3.913–5.095) | 6.508x | `5.22e-13` |
| 180 | 361 | odd minimum | 36.428 (35.693–37.325) | 29.571 (28.435–33.456) | 0.824, 0.874, 0.785 | 4.531 (3.899–5.147) | 6.526x | `4.57e-13` |
| 720 | 1458 | even smooth | 59.283 (53.129–61.293) | 36.353 (34.104–40.267) | 0.614, 0.635, 0.578 | 10.102 (9.470–11.312) | 3.599x | `3.93e-13` |
| 720 | 1441 | odd minimum | 64.808 (57.842–68.077) | 42.550 (37.328–47.304) | 0.638, 0.748, 0.588 | 13.631 (12.490–15.695) | 3.122x | `3.84e-13` |

The absolute candidate reductions versus parent were 7.081, 6.857, 22.930
and 22.257 ms, respectively. Candidate-minus-DUCC remained +23.837, +25.040,
+26.251 and +28.919 ms. Odd longitude is a control only; no FFT algorithm or
FFT dispatch was changed.

## Stage attribution and limits

A separate test-only phase probe ran 30 warmed calls at T720, full `mmax`,
`nphi=1458`, q8 on the candidate source. Median total was 35.545 ms. The
separately measured phase medians were 1.074 ms for latitude/spectrum setup,
2.907 ms for spin-1 scalar coefficient-cache preparation, 29.819 ms for order
scheduling plus recurrence and accumulation, and 1.980 ms for the two inverse
FFTs plus map writeback. Independent phase medians need not sum to the total.
The median per-worker group duration ranged from 21.526 to 28.642 ms (1.33x
max/min). Dynamic assignment reduced the observed imbalance while leaving the
recurrence/scheduling phase as the largest Rust stage. The stage boundaries do
not match DUCC's full-call timing and cannot apportion the entire remaining
Rust/DUCC excess to recurrence alone.

The final matched process-median runqueue-wait medians were 11.6% for parent,
12.5% for candidate and 18.8% for DUCC; maxima were 21.5%, 20.5% and 25.8%.
The candidate p10–p90 spread, especially reduced-even, reflects residual host
variation. All three paired candidate/parent process ratios nevertheless
favored the candidate in every case. Treat the size of the gain as directional,
not as a universal hardware result.

An earlier experiment setting Rayon `with_min_len(1)` was removed: it could
schedule more jobs than the four recurrence workspaces prepared for that
workspace. The bounded queue avoids that capacity mismatch. The previous
static `with_min_len(3)` route remains for all non-target cases.

## Reproducibility and validation

- Parent Rust source: `ac8e754791322c33d305be55631dbaa3403214aa`; candidate:
  `4e62e2003a90b0b69fe9c3e0e02779dd554630a5`.
- Rust 1.99.1, LLVM 23.1.1, release `-C target-cpu=x86-64-v3
  -Aunexpected_cfgs`, Pulp V3.
- DUCC 0.42.0 commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; GCC 15.2,
  SHT `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, strict driver.
- Host: AMD Ryzen 9 5950X; q8 affinity CPUs 0–7 on one L3; exact GL
  `ntheta=721`, `phi0=0.23`; full/reduced order and both longitude controls.
- The native full-transform driver is
  [`benchmark_ducc_spin1_native.py`](../scripts/benchmark_ducc_spin1_native.py).
  The retained JSON has binary SHA-256 values, 30 batch samples for each of
  12 backend/case summaries, per-process medians, all 24 DUCC-output comparisons,
  affinity and host-wait summaries:
  [`dynamic-scheduling-ab.json`](data/plan15-spin1-gl-dynamic-scheduling-2026-10-21/dynamic-scheduling-ab.json).
- Candidate outputs matched the parent binary bitwise. Relative L2 versus DUCC
  was at most `5.23e-13`, with maximum absolute error `3.97e-9`.
- Format, strict Clippy, all-feature release tests and release
  no-default-feature tests passed. The no-default suite had one transient
  existing worker-probe assertion failure; the test passed in isolation and
  the complete rerun passed.

## Next Plan 15 work

Continue GL. The q8 queue change is complete; next attribute the larger q1
T1023 synthesis absolute excess by matched stages, then decide whether a GL
candidate is justified. After GL attribution/closeout, proceed to spin-1 CC
and F1 as planned. No Plan 14 or scalar code was reopened.
