# Plan 14: current-source small-q1 attribution

**Rust source:** `f3f222b7ae42a874c53fba2ca5e88f8c3451c24d`.
**DUCC:** 0.42.0, pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
**Scope:** f64 scalar GL STANDARD, q1, T1–T64. This closes the small-q1
fixed-cost question; it does not change a numerical kernel or dispatch.

## Matched whole-transform screen

Rust 1.99.1 / LLVM 23.1.1 used release mode, strict FP,
`-C target-cpu=x86-64-v3`, and runtime Pulp V3. DUCC used GCC 15.2 with SHT
objects built using `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`;
the driver remained strict-FP. The native harness compared reusable-plan,
full-`mmax` calls at exact GL geometry with even-smooth longitude sampling.
It used three interleaved process pairs and 30 calibrated 25 ms batches per
backend/case. Outputs, reset behavior, selected basis modes, roundtrip,
adjoints, zero, subnormal and extreme-finite cases were checked outside the
timed region.

| `lmax` | Analysis Rust / DUCC / excess (µs) | Synthesis Rust / DUCC / excess (µs) |
| ---: | ---: | ---: |
| 1 | 0.378 / 1.206 / −0.828 | 0.427 / 1.113 / −0.686 |
| 2 | 0.600 / 1.641 / −1.041 | 0.627 / 1.493 / −0.867 |
| 4 | 1.186 / 2.116 / −0.930 | 1.099 / 1.953 / −0.854 |
| 16 | 7.878 / 7.004 / +0.874 | 6.702 / 7.161 / −0.458 |
| 31 | 21.714 / 16.506 / +5.208 | 18.285 / 16.846 / +1.440 |
| 32 | 25.508 / 21.906 / +3.602 | 22.762 / 22.456 / +0.305 |
| 63 | 61.223 / 62.311 / −1.088 | 72.493 / 62.510 / +9.982 |
| 64 | 71.600 / 73.277 / −1.677 | 84.332 / 76.629 / +7.703 |

The largest sampled positive excess was T63 synthesis, followed by T64
synthesis and T31 analysis. T1–T4 Rust calls were faster in absolute time,
so the data do not support a general Rust per-call fixed-cost penalty. The
largest absolute excesses rise with useful transform work. The T63 synthesis
Rust p90 reached 99.9 µs, and the T64 synthesis third process also had a
long tail. Runqueue wait was 10.5% median / 31.6% p90. CPU frequency was
sampled but exact transient readings are omitted from the committed artifact.
Treat close small shifts as directional; retain the distributions rather than
claiming parity from individual cases.

Maximum Rust-versus-DUCC relative L2 error was `3.61e-15` for analysis and
`1.49e-14` for synthesis; maximum absolute errors were `1.65e-15` and
`3.51e-12`. Detailed per-case timing, cold/setup measurements, all 96
process records, fixtures, numerical comparisons, and host observations are
in [`results.csv`](data/plan14-small-q1-current-2026-10-21/results.csv) and
[`results.json`](data/plan14-small-q1-current-2026-10-21/results.json).

## Stage evidence and disposition

Three separate, 30-batch Rust phase captures pinned to physical CPU 0 covered
T31 full-order analysis, T63 synthesis and T64 synthesis. The phase probe is
test-only; phase times are Rust component measurements, not directly
subtractable against the uninstrumented DUCC whole-call time.

| Case | Rust whole call (µs, process medians) | Legendre + accumulation (µs) | FFT (µs) | geometry/setup (µs) |
| --- | ---: | ---: | ---: | ---: |
| T31 analysis | 21.273 / 21.430 / 21.282 | 15.346 / 15.499 / 15.391 | forward 4.823 / 4.848 / 4.817 | 0.646 / 0.651 / 0.645 |
| T63 synthesis | 70.662 / 70.976 / 73.724 | 60.041 / 60.311 / 62.869 | inverse 8.769 / 8.789 / 8.796 | 1.660 / 1.665 / 1.809 |
| T64 synthesis | 84.212 / 84.372 / 86.691 | 67.182 / 67.330 / 68.948 | inverse 15.075 / 15.043 / 15.675 | 1.745 / 1.766 / 1.812 |

T31's strategy, table retrieval, coefficient initialization/cleanup and order
preparation together were under 0.2 µs; adding latitude setup brings these
non-FFT/non-Legendre phases to about 0.8 µs. T63/T64 synthesis geometry and
spectrum clearing stayed below 1.9 µs. The measured Rust excess therefore
does not point to fixed setup or buffer clearing. Recurrence and, at T64,
the inverse stage dominate Rust's time. This profile does not establish how
much of each component is slower than DUCC: no DUCC small-q1 internal stage
timer was enabled for these cases. It therefore does not justify an
unmeasured recurrence or FFT rewrite.

Phase A is closed without a production change. Preserve the current q1
routes and safety checks. The next comparison re-ranks the current large-q8
whole-transform excess, first revalidating the previously largest absolute
case (T3072 full-order analysis) on the current source before selecting a
stage-level target.

## Reproduction

Use the pinned source worktree and the script's recorded host protocol:

```sh
RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "${DUCC_SRC:?set pinned DUCC checkout}" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 1 --lmax 2 --lmax 4 --lmax 16 --lmax 31 --lmax 32 --lmax 63 --lmax 64 \
  --full-mmax-only --longitude-mode even_smooth --threads 1 \
  --results-dir reports/data/plan14-small-q1-current-2026-10-21 --no-report
```

The Rust phase runner is the ignored test `plan14_analysis_phase_batch_profile`
or `plan14_synthesis_phase_batch_profile`. Set `RAYON_NUM_THREADS=1`, pin the
test process to one physical CPU, and set `RUSTSHT_PROFILE_Q=1`, the case's
`LMAX`, `MMAX`, `NTHETA`, `NPHI`, `BATCHES=30`, and `BATCH_MS=25`. The compact
per-process phase distributions are in
[`phase-summary.csv`](data/plan14-small-q1-current-2026-10-21/phase-summary.csv).
