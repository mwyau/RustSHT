# Scalar-analysis experimental results — current evidence index

**Disposition (2026-10-21):** The preceding 40k-character cumulative
experiment log is now archived
[verbatim as a historical snapshot](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md).
This concise report keeps the **measured current conclusions** and links
to the original machine-readable files; previous candidate proposals and
“next” directions in the archive are retired. **Current instructions are
[Plan 11](../plans/11-ducc-equivalent-scalar-analysis.md), not this report.**

## T2047/T2048 cutoff and pair-tile comparison (2026-10-20)

Fixed native f64 GL STANDARD **analysis** geometry:
`mmax=2047`, `nphi=4096`; Ryzen 5950X; Rust 1.99.1
strict x86-64-v3/Pulp V3; DUCC `42ce8ab5` with GCC 15.2
native V3 `-O3 -ffast-math -ffp-contract=fast`.
Data: [cutoff study README and summary CSV](data/scalar-analysis-cumulative-2026-10-20/t2048-cutoff-current/).

| Geometry | Threads | DUCC | Rust tile 256 / 128 / 64 | Paired ratio tile 256 / 128 / 64 |
| --- | ---: | ---: | ---: | ---: |
| T2047, 2048 latitudes | q1 | 248.645 ms | 251.498 / 261.872 / 282.107 ms | 1.013 / 1.050 / 1.125 |
| T2047, 2048 latitudes | q8 | 45.793 ms | 62.201 / 63.074 / 67.679 ms | 1.371 / 1.388 / 1.468 |
| T2047, 2049 latitudes | q1 | 249.445 ms | 828.435 / 718.965 / 696.002 ms | 3.326 / 2.888 / 2.781 |
| T2047, 2049 latitudes | q8 | 45.928 ms | 143.732 / 129.008 / 122.973 ms | 2.926 / 2.741 / 2.647 |
| T2048, 2049 latitudes | q1 | 250.061 ms | 832.444 / 716.284 / 693.156 ms | 3.328 / 2.857 / 2.774 |
| T2048, 2049 latitudes | q8 | 42.401 ms | 133.959 / 121.908 / 115.758 ms | 3.198 / 2.899 / 2.752 |

Paired ratios come from the interleaved per-process measurements;
they need not equal a ratio of individually rounded medians.
Three interleaved rounds gave 15 whole-call measurements per long
backend/tile case and 30 calibrated short-case q8 batches.
Maximum relative L2 error `1.18e-13`; all worker reset checks passed.
The timing data include host contention diagnostics, especially at q8.

A single additional latitude changes Rust's prefix eligibility
and raises whole-call latency dramatically; fixing `lmax` while
changing only `ntheta` establishes that adding harmonic degree
2048 is **not** the cause. Route counts at tile 256 change from
2,432 prefix-enabled blocks (`ntheta=2048`) to 2,434
checked-scaled blocks (`ntheta=2049`). RustSHT's *extra*
`ntheta * 2^-60 <= 8*EPSILON` global gate is absent from
DUCC's `iter_to_ieee`. At the time of this baseline, removing that gate had
**not yet been validated** as a safe production change. The follow-up below
records the controlled validation and supersedes that status; independent
input-range, scale, polar/subnormal and fallback checks remain in place.

**Tile conclusion:** 256 is the **retained production default**;
smaller tiles only recover a fraction of the disabled-prefix
slowdown and remain much slower than DUCC. The existing 64/128
comparisons are historical diagnostics, **not a request to run them
again**. The original cutoff note deferred a 512 trial until after the
prefix issue; it remains optional only if a separate cache/setup hypothesis
justifies expanding and validating fixed scratch capacity.

## T2048 fixed-`sht_ftol` follow-up (2026-10-20)

Commit `0bc5f153` removes only RustSHT's global condition
`latitude_count * ANALYSIS_FTOL <= 8 * f64::EPSILON`. The threshold remains
exactly `2^-60`; finite-value, exponent, recurrence-range, coefficient
product and transactional fallback checks remain unchanged. Production tile
size remains 256 reflected latitude pairs.

Pinned DUCC 0.42 uses `sht_ftol=2^-60` in `init_lambda` and `iter_to_ieee`.
`rescale` advances lanes according to their scaled recurrence state, and
`calc_map2alm` handles mixed scaled/ordinary lanes before ordinary SIMD
accumulation. It has no latitude-count multiplier. Rust's removed gate was a
stricter error budget for the worst-case coherent sum of omitted prefix
contributions, not a finite-value or exponent-safety invariant. The
validated Rust route retains its independent safety checks and decline
fallbacks. The cases below found no need for the extra bound; this is
empirical coverage, not a claim of bitwise identity for every possible input.

The matched before/after full-transform matrix uses Rust 1.99.1, Pulp V3,
DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2 with
DUCC SHT flags `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and a
strict validation driver. Baseline source was `86f43486`; candidate source
was `0bc5f153`. Geometry is f64 GL STANDARD analysis with `mmax=2047`,
`nphi=4096`, three interleaved process rounds, 15 whole-call observations
per q1 backend/tile and 30 calibrated q8 batches. Ratios below pair each
Rust result with that phase's DUCC samples; small DUCC shifts between phases
are measurement variation.

| Geometry | q | Rust before → after (ms) | Rust/DUCC before → after | Rust latency change |
|---|---:|---:|---:|---:|
| T2047, 2048 latitudes | 1 | 251.50 → 254.59 | 1.011× → 1.033× | +1.2% |
| T2047, 2048 latitudes | 8 | 62.20 → 58.60 | 1.358× → 1.384× | −5.8% |
| T2047, 2049 latitudes | 1 | 828.43 → 254.64 | 3.321× → 1.036× | −69.3% |
| T2047, 2049 latitudes | 8 | 143.73 → 57.31 | 3.129× → 1.361× | −60.1% |
| T2048, 2049 latitudes | 1 | 832.44 → 255.40 | 3.329× → 1.029× | −69.3% |
| T2048, 2049 latitudes | 8 | 133.96 → 58.32 | 3.159× → 1.383× | −56.5% |

The one-latitude performance discontinuity is removed. On the already-fast
T2047/2048-latitude control, q1 is within its observed spread; the q8 median
was lower in this run. The remaining q8 whole-transform excess at 2049
latitudes is about 1.36–1.38×, down from 3.13–3.16×.

After fixing the prefix route, the one permitted tile diagnostic still
favors 256 at the 2049-latitude shapes:

| Geometry | q | Tile 256 | Tile 128 | Tile 64 |
|---|---:|---:|---:|---:|
| T2047, 2049 latitudes | 1 | **254.64 ms** | 263.54 ms | 280.66 ms |
| T2047, 2049 latitudes | 8 | **57.31 ms** | 59.18 ms | 61.86 ms |
| T2048, 2049 latitudes | 1 | **255.40 ms** | 263.57 ms | 282.08 ms |
| T2048, 2049 latitudes | 8 | **58.32 ms** | 58.56 ms | 60.89 ms |

Tiles 64/128 are diagnostic only; no dispatch change is justified, and no
further tile sweep is planned. Route counters at tile 256 now report
2,432/2,434/2,434/5,780 prefix-scaled blocks and zero checked-scaled blocks
for T2047/2048-latitude, T2047/2049-latitude, T2048/2049-latitude and
T3072/3073-latitude cases, respectively. Initialization and recurrence
decline counters are zero in all four.

The T3072 full-order GL check used `mmax=3072`, `ntheta=3073`, `nphi=6174`.
Its one-process, five-call q1 screen measured analysis at 775.8 ms Rust vs
735.1 ms DUCC (1.056×), and synthesis at 1121.4 vs 1152.6 ms (0.973×); these
timings are directional rather than a full benchmark campaign. Random
analysis/synthesis relative L2 errors were `1.96e-13`/`2.11e-13`, with
absolute maxima `1.44e-15`/`1.07e-8`. Rust adjoint residuals were
`1.19e-15` for synthesis and `1.10e-14` for analysis. Zero outputs matched
exactly; extreme and subnormal cases remained finite, with subnormal outputs
matching DUCC exactly. The `(3072,0)` and `(3072,3072)` basis maps differed
from DUCC by relative L2 `1.26e-11` and `2.24e-15`. Antisymmetric north/south
maps at T2047, T2048 and T3072 produced exact zero for every even-degree
`m=0` coefficient in both libraries; maximum Rust/DUCC relative L2 errors
were `1.80e-13`, `1.16e-13` and `1.89e-13`.

Compact before/after timings and errors are in
[`t2048-ducc-fixed-ftol/summary.csv`](data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/summary.csv)
and [`validation-summary.json`](data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/validation-summary.json).
The T2047/T2048 q1/q8 full-transform checks passed finite-output, reset,
zero, extreme, subnormal, basis, roundtrip and adjoint checks; maximum random
relative L2 error was `1.18e-13` for analysis and `1.28e-13` for synthesis.
Release tests passed (230); no-default-feature release tests passed (192);
formatting, all-target/all-feature Clippy and the T2047/T2048/T3072 route test
passed. Next, profile the remaining q8 full-transform cost; do not repeat the
cutoff or tile experiment.

## Current T720 production/candidate A/B (2026-10-20)

Source `b25c011d`; identical f64 GL scalar A/S,
`mmax=180/720`, odd-minimum and even-smooth longitude shapes,
q1/q8. Three interleaved process pairs, 30 timed batches per
backend/shape. Compared production, fixed aligned scratch,
packed complex reduction, and their combination against pinned
DUCC native V3. Maximum relative L2 error `5.20e-14`.
[Full results, including first calls and uncertainty](data/scalar-analysis-cumulative-2026-10-20/t720-ab-current/).

- Fixed scratch alone changed **analysis** latency by approximately
  -2.8% to +0.2% across T720; it had an independently confirmed
  **q8 synthesis** improvement of **11.4–18.0%** over four cases
  (five interleaved process pairs, 50 batches/case, low runqueue wait).
  q1 synthesis was near-neutral. This is still an **opt-in candidate**;
  its mechanism, total workspace/memory cost and generality are unresolved.
- Packed complex reduction improved the isolated kernel by a little,
  but full-call analysis was only -2.0% to +0.4% alone, inconsistent;
  **removed**.
- Direct coefficient accumulation caused large T720 analysis
  regressions in an earlier cumulative experiment; **removed**.
- Safe indexed loop reduced instructions without a verified
  whole-transform performance advantage; **removed**.
- Sectoral seed-prefix and scaled-epoch/preflight reuse are in
  current production and common to the matched builds; their
  *individual* benefits were not measured in that A/B.

A combined earlier candidate build with retired features gave
large regression on several T720 analysis shapes. **Do not conflate
that obsolete build with current production or resurrect its flags.**
Review the full archived [experiment narrative](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md)
only when a specific source or provenance question requires it.

## Further historical experiments and provenance

- [Archived exact 2026-10-20 cumulative log](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md):
  earlier kernel, reduction, range, direct-output, epoch,
  setup and route-profiling experiments, including rejected
  implementations and their source/benchmark conditions.
- [Post-upgrade GCC/Clang baseline](archive/experiments/ducc-upgrade-baseline-2026-10-20.md)
  and [historical full-bandlimit sweep](archive/experiments/ducc-full-transform-parity-2026-10-20.md)
  use separately specified source/flag versions; do not merge
  their values into a current-source trend without matched reruns.
- [Current factual checkpoint](../PERFORMANCE_STATUS.md),
  [active Plan 11](../plans/11-ducc-equivalent-scalar-analysis.md).

No archived experiment or older "next" paragraph is authorization
to divert from the current cutoff investigation or to retry a
known rejected tile/reduction experiment.
