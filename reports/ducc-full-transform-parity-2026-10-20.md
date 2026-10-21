# Historical full-transform RustSHT/DUCC baseline — archived

**Status: HISTORICAL EVIDENCE, NOT CURRENT-SOURCE PERFORMANCE OR AN ACTIVE TASK.**
The complete original 2026-10-20 report, including all figures,
tables, caveats and reproduction instructions, is preserved at
[the archived full-transform parity report](archive/experiments/ducc-full-transform-parity-2026-10-20.md).
Original [CSV/JSON measurements](data/ducc-full-transform-parity-2026-10-20/)
remain in place.

The baseline uses **Rust 1.99.0**, an earlier RustSHT source revision and
DUCC 0.42.0 pinned at `80388904b8cfc32400ba84af8a6a45aabfe60f86`.
The 214-configuration f64 GL spin-0 A/S q1/q8 sweep includes
T1536 and T3072, a wide range of `mmax` and latitude/longitude
geometries, cold/warm measurements and numerical checks.
The old T3072 analysis ratios (8.39× q1, 7.44× q8) are meaningful
**historical observations**, but **not measurements of the current
production source** or a standalone direction to rewrite the kernel.
Current fixed-geometry T2047 evidence isolates a latitude-dependent
prefix-route cliff: see [current status](../PERFORMANCE_STATUS.md)
and [Plan 11](../plans/11-ducc-equivalent-scalar-analysis.md).

## Results

Read the [preserved complete result tables](archive/experiments/ducc-full-transform-parity-2026-10-20.md#results).

## Scope and method

Read the [original scope and method](archive/experiments/ducc-full-transform-parity-2026-10-20.md#scope-and-method).

## Reproduction

Read the [original reproduction contract](archive/experiments/ducc-full-transform-parity-2026-10-20.md#reproduction).
