# RustSHT — current performance and implementation status

**Updated:** 2026-10-18. **Active branch:** `opt/sht-performance-parity`. **Goal:** faithful native Rust implementations of DUCC's complete scalar/spin SHT execution algorithms, then measured CPU parity across atmospheric workloads.

**This is the short context index.** Do not preload `reports/`, the old milestone ledger or architectural reviews. Detailed pre-DUCC-port evidence is preserved in [the prior performance snapshot](reports/performance-status-pre-ducc-port-2026-10-18.md); consult individual linked reports only for a concrete implementation decision. The active roadmap is [plans/05-validation-and-milestones.md](plans/05-validation-and-milestones.md).

## Current production

- Stable Rust 1.99.0; f64/f32; GL/CC/F1; scalar and spin 1–4 operations, relevant adjoints and derivative modes. Existing numerical and precision support is **combination-specific**, not universal; see [numerical contract](plans/02-numerics-and-grids.md).
- Scalar synthesis: ring-group-owned workers, DUCC-derived paired-degree recurrence, `pulp` SIMD ordinary suffix, scaled/unsafe fallback. The **integrated tile-128 f64 GL path** at eligible `lmax >= 255` uses a scaled-prefix accumulator, fused normalization and accumulation, SIMD prefix arithmetic, order-level coefficient-safety reuse and checked single-scatter staging. It is **not** DUCC's complete scalar synthesis algorithm. The separate order-major DUCC-style candidate in `src/legendre/scalar_ducc.rs` now dispatches eligible f64 scalar GL through q1 recurrence or weighted order-range parallelism; q>1 output uses bounded order-major workspace scratch and commits to row-major spectra only after all order groups succeed. Unsafe inputs retain the robust ring-group/tile fallback.
- Scalar GL synthesis can precompute/share transformed coefficients across ring groups when transition, worker and buffer eligibility permit. This generalizes the initially T1023-only cache; only the earlier T1023 q8 screen clearly demonstrated benefit.
- Spin-1 includes existing restricted hemisphere pairing, scalar conversion and robust extreme-value guards. Scalar/spin analysis, f32 and CC/F1 use their respective established production routes. RustFFT is the root FFT backend; the separate RustSHT-FFT crate is not integrated.
- Public APIs and existing safe fallbacks should be preserved while implementing a separate DUCC-parity candidate.

## What is known (not an active benchmark queue)

| Technique | Status | Key finding |
| --- | --- | --- |
| Ishioka paired recurrence, SIMD suffix and scaled fallback | Production | Mathematically DUCC-derived, but work batching, prefix skipping and reflected output differ from DUCC. |
| Tile-128 / scaled-prefix / normalization fusion / SIMD prefix | Production f64 scalar GL route | Large historical T1023/T2047 experimental gains; current integrated local screens are encouraging but **not matched DUCC parity evidence**. |
| Generalized coefficient cache and reused coefficient safety preflights | Production | Remove repeated work across eligible ring groups and tiles; benefits across arbitrary shapes not yet established. |
| Scalar hemisphere pairing, whole order-owned scheduling | Historical prototypes rejected | Coarse pairing and scheduling overhead outweighed saved work; do not repeat the same designs. |
| Exponent-epoch accumulator, SIMD precheck, f32-to-f64 bridge | Experimental/inconclusive | No general demonstrated improvement over the preferred route; preserve archived evidence. |
| Spin-1 hemisphere pairing, small-band tables, selected f32 normalization skips | Production, restricted | Keep measured selectors and fallback while reworking common kernels. |
| Native analysis and spin-1 performance | Production, behind DUCC | Separate kernels and operator checks required; synthesis gains do not establish analysis gains. |

**DUCC-kernel checkpoint:** `b484a8c7` added `src/legendre/scalar_ducc.rs`, with order-major 256-ring active/reflected batching, DUCC-like scaled initialization and negligible-prefix skipping, parity reconstruction, transformed coefficient reuse and fused SIMD recurrence. The candidate now advances a monotone polar active-ring boundary across orders and schedules weighted order ranges across workers; q8 uses existing workspace storage followed by a tiled spectrum scatter. T255 mixed coefficients match pinned DUCC samples and the retained ring-group spectra; tests cover reduced `mmax`, odd/even longitude counts, a 301-ring batch tail, subnormal fallback and unsafe transformed coefficients. The q8 path matches q1 and ring-group spectra, and its staged output remains uncommitted when a later order declines on subnormal input. Five interleaved release samples on an AMD Ryzen 9 5950X (x86_64, 16C/32T, AVX2/FMA; Rust 1.99.0) measured q1/q8 medians of 3.33/2.13 ms for full f64 T300 GL 301×603 synthesis (`lmax=300`, `mmax=150`), about 1.56× q8 scaling. The screen was ungated and compared RustSHT q1/q8 only; it is not a DUCC q8 comparison or parity acceptance. Earlier T255/T300 full-call screens measured the candidate 32.3%/8.3% slower than DUCC; rerun those matched comparisons at the next integrated milestone. GL high-degree sectoral checks passed at opt-in T511/T1023/T2047. See [source screen](reports/scalar-ducc-kernel-screen-2026-10-18.md).

**Historical performance context:** the archived 29-grid study (T42–T2047) found DUCC faster for all matched scalar/spin-1 S/A workloads from that source revision. A former six-pair isolated scalar T2047 q1 scaled-prefix candidate measured ~1,371 ms versus ~3,360 ms clean baseline, still ~3.67× DUCC. These are **not measurements of the latest integrated build**. The latest integrated tile/SIMD/cache sources still lack complete matched DUCC and cross-operation acceptance.

## Active next steps — DUCC-first

1. **Validate, refine and generalize** the newly implemented native Rust DUCC-style f64 GL Legendre synthesis execution: `fill_a2m`, `init_lambda`, `iter_to_ieee`, `calc_alm2map`, `alm2map_kernel`, `extract_a2m`, per-order coefficients, reflected north/south pairs, SIMD unrolling, and active-ring batching. See pinned [source comparison](reports/data/m6-legendre-redesign/source-comparison.md) and DUCC `sht_inner_loop.h` / `sht.cc`. Use the existing `src/legendre/scalar_ducc.rs` candidate, finish correctness for mixed/extreme inputs and close the remaining kernel/code-generation gap while preserving robust fallback.
2. Stabilize q1/q8 order scheduling across more irregular and larger grids, measure staging/scatter overhead, and compare **equivalent Legendre-stage** work plus full transforms with pinned DUCC. Diagnose remaining instruction/work/layout differences rather than making more arbitrary exact-T selectors.
3. Port and share appropriate execution machinery for scalar analysis, spin-1 synthesis/analysis, f32 and the existing GL/CC/F1 operator pipeline. Retain all established numerical and adjoint checks.
4. Update this file *only after meaningful integrated milestones*. Keep detailed numerical histories archived and out of routine context.

## Read only when needed

- [DUCC execution crosswalk](plans/06-ducc-contract-crosswalk.md) and [numerical specification](plans/02-numerics-and-grids.md): actual mode, grid, adjoint and accuracy contracts.
- [Prior status snapshot](reports/performance-status-pre-ducc-port-2026-10-18.md): details and citations for tile, prefix, SIMD, cache and candidate screens.
- [Historical validation/milestones](plans/05-validation-and-milestones-history.md) and [historical integrated review](ASTRA_INTEGRATED_REVIEW.md): provenance, not instructions.
- [Atmospheric comparison](reports/atmospheric-resolution-performance-ectrans-2026-10-16.md), [M6 campaign report](reports/m6-cpu-performance-parity.md): benchmark data, **not a standing test matrix**.

**Execution:** implementation and numerical correctness first; light smoke screens during coding, comprehensive DUCC comparison at major milestones. Do not default to T511/T1023/T2047.
