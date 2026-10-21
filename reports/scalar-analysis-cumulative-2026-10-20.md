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
DUCC's `iter_to_ieee`. Removing that gate has **not yet been
validated** as a safe production change. Independent input-range,
scale, polar/subnormal and fallback checks still matter.

**Tile conclusion:** 256 is the **retained production default**;
smaller tiles only recover a fraction of the disabled-prefix
slowdown and remain much slower than DUCC. The existing 64/128
comparisons are historical diagnostics, **not a request to run them
again**. A 512 trial is optional after addressing the prefix issue;
it requires expanding the fixed scratch capacity and ensuring that
cache/memory and range-cache changes are validated.

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
