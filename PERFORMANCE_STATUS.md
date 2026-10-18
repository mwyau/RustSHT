# RustSHT performance status

Snapshot: `opt/sht-performance-parity` at `53c6772a` (2026-10-18). DUCC remains faster in the matched atmospheric workloads. The scalar tile-128 and scaled-prefix candidates have not entered production.

## Production architecture

Scalar synthesis uses ring-group workers, an order loop, the DUCC-derived paired-degree recurrence, a Pulp SIMD ordinary path, and a scaled per-lane recurrence fallback. Mixed latitude blocks can be compacted into worker-owned scratch. The compact scalar Legendre table is separately dispatched for measured small-band GL cases. Spin synthesis uses the general scaled recurrence, with measured scalar-conversion and hemisphere-pair routes for selected spin-1 cases. Analysis uses paired scalar and spin recurrences, mirrored-ring handling, and reusable worker scratch. The core contains GL, CC, F1, f64/f32, and spin 0–4 paths; test coverage by combination is documented separately.

The production synthesis loop is ring-group-owned: each worker traverses orders for its ring group. This exposes latitude parallelism and repeats some per-order coefficient setup across groups. DUCC's synthesis path prepares an order and processes its active rings in batches. The direct source comparison identifies coefficient setup and active-lane density as testable differences; it does not establish that either schedule is faster in isolation.

## Performance and numerical evidence

- The 29-grid atmospheric study covers T42–T2047 and reports 232 native f64 spin-0/spin-1 synthesis and analysis rows at q1/q8. It found DUCC faster in every matched comparison. Its binary came from an earlier revision; use it for workload trends, not as the current branch baseline.
- In the current campaign's matched T2047 baseline (DUCC 0.42.0, source `80388904`), q1 RustSHT/DUCC medians were 3355/370 ms for scalar synthesis, 1093/243 ms for scalar analysis, 4124/776 ms for spin-1 synthesis, and 1768/497 ms for spin-1 analysis. At q8, the corresponding ratios were 13.97, 5.42, 8.12, and 4.40. RustSHT relative-L2 errors against DUCC were between `1.7e-13` and `3.7e-13` for the q1 transforms.
- At T2047 q8, the atmospheric phase profiles attribute 84–97% of synthesis time and 68–90% of analysis time to Legendre recurrence and coefficient accumulation. Inverse FFT is at most 7.4% for scalar and 14.5% for spin 1 in the measured synthesis cases. RustFFT replacement is not the current bottleneck.
- The current f32 scalar recurrence uses normalization-skip dispatch only at the paired-screened T1023 and T1279 truncations. Those outputs were bitwise equal to the fallback; measured scalar synthesis and analysis improved at both sizes. Broader f32 performance limits remain unestablished. The reusable f64 bridge reduced error but remained as much as 13x slower than DUCC f32 at T1023 scalar q8 synthesis.
- Numerical reports identify DUCC 0.41.0 or 0.42.0 and provide analytic, grid, adjoint, high-bandlimit, and full-array comparisons for their stated cases. Those results do not establish untested configurations or universal limits.

## Technique findings

| Technique | Status | Result | Reason | Reference | Next action |
|---|---|---|---|---|---|
| Ishioka paired-degree scalar recurrence | Production | DUCC-derived recurrence supports fused SIMD and scaled fallback. | It is the current scalar basis kernel; recurrence correctness is not the bottleneck hypothesis. | [`scalar_paired.rs`](src/legendre/scalar_paired.rs), [`source comparison`](reports/data/m6-legendre-redesign/source-comparison.md) | Keep it as the reference while changing accumulation and work organization. |
| SIMD scalar and spin recurrences | Production | Pulp dispatches ordinary lanes; scaled lanes use the robust recurrence. | Mixed and extreme-range lanes need the scaled path. | [`scalar_paired.rs`](src/legendre/scalar_paired.rs), [`spin_fast.rs`](src/legendre/spin_fast.rs) | Improve production hot paths without removing scalar and scaled fallbacks. |
| Scalar north/south hemisphere pairing | Rejected | T511 was 10.4% slower at q1 and 6.2% slower at q8. | Safe fully active blocks covered only part of the workload; coefficient setup and scatter exceeded saved recurrence work. | [`hemisphere experiment`](reports/hemisphere-paired-synthesis-2026-10-14.md) | Do not repeat the coarse block design. |
| Spin-1 hemisphere pairing | Production, restricted | Current dispatch covers GL, full `mmax`, T319–T511, and 4–8 workers. | The measured route is useful only in a bounded geometry and worker range; checked conversion falls back to the general recurrence. | [`spin-1 route`](reports/spin1-hemisphere-pair-synthesis-2026-10-14.md), [`dispatch`](src/spin_transform.rs) | Preserve its numerical checks; broaden only with matched evidence. |
| Order-owned versus ring-owned synthesis | Order-owned prototype rejected | The prototype regressed, especially at T511 q8. | Its order loop and scheduling cost outweighed reuse; the production ring-group loop also repeats order preparation, which remains separately measurable. | [`order-owned screen`](reports/order-owned-synthesis-2026-10-15.md), [`source comparison`](reports/data/m6-legendre-redesign/source-comparison.md) | Keep ring ownership until a redesign removes the measured setup and scheduling costs. |
| Active-ring packing and compaction | Block-local compaction in production | Mixed blocks compact active lanes; fully inactive blocks are skipped. No order-wide packing candidate is accepted. | Compaction and active-lane density have not been separated from recurrence time. | [`source comparison`](reports/data/m6-legendre-redesign/source-comparison.md), [`transform.rs`](src/transform.rs) | Measure active counts and compaction time before changing layout. |
| Scaled transition and robust fallback | Production recurrence and fallback; tiled version experimental | The production path keeps scaled recurrence for unsafe lanes. The experimental T2047 q1 profile assigned 51.91% of the transform to tile transition work and 39.41% to the SIMD suffix. | Transition and scaled-prefix work are the main cost in the strongest scalar candidate. | [`campaign profile`](reports/m6-cpu-performance-parity.md) | Optimize the integrated scaled-prefix path while preserving checked fallback. |
| Tile-128 scalar synthesis | Experimental | A T511 q1 screen measured 19.59% lower latency; its host-pressure gate failed. Later tile-128 accumulator comparisons are listed below. | The positive T511 screen is not accepted evidence and the source is not in production. | [`tile-128 campaign results`](reports/m6-cpu-performance-parity.md) | Use the archived code as a starting point, not as production code. |
| Scaled-prefix accumulator | Experimental, strongest candidate | Six-pair q1 candidate/baseline ratios were 0.568982 at T1023 and 0.407892 at T2047; T2047 candidate latency was 1371 ms and relative-L2 error was `3.015e-13` versus DUCC. | T511 stopped at an unstable DUCC control; q8, spin-1, and broad regression gates did not run. | [`T1023`](reports/data/m6-legendre-redesign/route-timing/scaled-prefix-accumulator-t1023-q1.md), [`T2047`](reports/data/m6-legendre-redesign/route-timing/scaled-prefix-accumulator-t2047-q1.md) | Integrate with transaction-safe fallback, then validate representative operations and truncations. |
| Exponent-epoch accumulator | Experimental; no clear improvement | Six-pair q1 ratios were 0.594264 at T1023 and 0.414702 at T2047, weaker than the preceding accumulator. | It rebases within exponent epochs but did not improve the accepted high-resolution comparisons; T511 was unresolved. | [`epoch q1 results`](reports/data/m6-legendre-redesign/route-timing/exponent-epoch-q1-screen-v1-results.md) | Prefer the preceding accumulator unless a new profile supports epoch rebasing. |
| Prefix normalization/accumulation fusion | Experimental, matrix incomplete | One T2047 q1 process was 11.27% faster than the accumulator source with bitwise-equal output. The T1023 matrix stopped at an unstable control. | No completed comparison against clean production or a full regression gate. | [`fusion screen`](reports/data/m6-legendre-redesign/route-timing/prefix-lane-pass-fusion-screen-t2047-q1.md), [`matrix stop`](reports/data/m6-legendre-redesign/route-timing/prefix-lane-pass-fusion-t1023-q1-v2-stopped.md) | Recheck only after the accumulator is integrated and a stable control is available. |
| SIMD prefix and range prechecks | Inconclusive or rejected | The SIMD-prefix timing lacked a valid DUCC mate. A range-precheck diagnostic reduced the prefix estimate 6.41% and whole-call estimate 2.62% in one process. | Neither measurement has a valid paired acceptance matrix; corrected lane replay found very little work safe across every degree. | [`prefix and precheck findings`](reports/m6-cpu-performance-parity.md) | Keep SIMD packing out of the first integration; retain prechecks only if they simplify the production kernel and pass whole-call comparisons. |
| Native f32 recurrence | Production; limited performance selector | Exact T1023/T1279 scalar skips were bitwise equal to fallback and improved measured S/A at q1/q8. T720 screens rejected a general selector. | Benefits depend on the recurrence range; a broad scan regressed T720. | [`f32 study`](reports/f32-scalar-normalization-skip-2026-10-16.md), [`T1279 study`](reports/f32-scalar-normalization-skip-T1279-2026-10-16.md) | Preserve the tested selector; improve f32 kernels after f64 synthesis integration. |
| Reusable f64 bridge for f32 | Rejected as a general route | It was 3.49–31.74x faster than native f32 in an unaccepted screen but up to 13x slower than DUCC f32; T1023 spin-1 needed a 48 MiB temporary payload. | Conversion cost and memory do not close the DUCC gap; the screen lacked paired repetitions and odd-longitude controls. | [`bridge screen`](reports/m6-cpu-performance-parity.md) | Keep native f32 recurrence as the implementation target. |
| Scalar analysis | Production; slower than DUCC | Current T2047 q1/q8 ratios were 4.49x/5.42x. Recurrence and accumulation dominate high-resolution q8 profiles. | Synthesis-only work does not automatically improve the separate analysis operator. | [`analysis baseline`](reports/m6-cpu-performance-parity.md), [`analysis profile`](reports/current-analysis-phase-profile-2026-10-15.md) | Continue range-check and accumulation work after scalar synthesis. |
| Spin-1 analysis | Production; slower than DUCC | Current T2047 q1/q8 ratios were 3.56x/4.40x. | The path has channel preparation and spin-specific recurrence costs; synthesis improvements do not transfer automatically. | [`spin-1 baseline`](reports/m6-cpu-performance-parity.md), [`spin-1 analysis report`](reports/spin1-analysis-ducc-refresh-2026-10-15.md) | Reuse proven scalar kernel changes where the spin basis permits. |
| FFT and multicore scheduling | RustFFT production; Legendre work dominates | Row FFT parallelism reduced q6 T1023 analysis by 36.1% for scalar and 43.9% for spin 1. Prior leaf overpartition regressed T511 by 3.19% and T720 by 11.55%. | High-resolution FFT shares are smaller than recurrence/accumulation; balanced estimated leaf work still had 1.6–2.5x timing spread. | [`FFT scheduling`](reports/row-fft-parallelism-2026-10-15.md), [`atmospheric study`](reports/atmospheric-resolution-performance-ectrans-2026-10-16.md) | Keep RustFFT; improve recurrence scheduling only after the single-thread kernel. |
| Atmospheric-resolution comparisons | Historical matrix; current baseline separately measured | The 29-grid T42–T2047 matrix found DUCC faster in every matched case. Current T2047 q8 gaps are 4.40–13.97x by operation. | The 29-grid binary predates this branch snapshot; do not merge its timings with current-baseline measurements. | [`29-grid study`](reports/atmospheric-resolution-performance-ectrans-2026-10-16.md), [`current campaign`](reports/m6-cpu-performance-parity.md) | Repeat a representative resolution panel after major kernel integration. |

## Operation status

- **Scalar synthesis, f64:** GL, CC, and F1 paths use the paired recurrence, ordinary SIMD route, and scaled fallback. High-bandlimit DUCC and analytic checks pass for the documented test cases. The isolated scaled-prefix candidate is the best measured route, but is not production code.
- **Scalar analysis, f64:** GL, CC, and F1 analysis and the separate adjoint are implemented. The current high-resolution profiles place most q8 time in Legendre recurrence and coefficient accumulation. Scalar synthesis changes require separate analysis tests.
- **Spin-1 synthesis, f64:** The general recurrence, scalar conversion, paired Q/U route, and restricted hemisphere route are present. E/B conversion and polar fallbacks are checked for extreme finite coefficients.
- **Spin-1 analysis, f64:** Paired scalar and fused channel paths are present. DUCC comparisons and thread screens cover selected truncations; synthesis timing gains do not establish analysis gains.
- **f32:** Native scalar and spin recurrences are present. Scalar normalization-skip dispatch is restricted to T1023 and T1279; the reusable f64 bridge remains experimental and is not the general f32 solution.
- **Spin 2–4:** Existing transform operations, supported adjoints, and scaled fallbacks remain. Recent C03 checks cover spin-2 coefficient-conversion overflow cases; performance parity has not been established.
- **Grid and operator coverage:** The project supports GL, CC, and F1, with distinct S, A, S*, and A* contracts. See the numerical specification and DUCC crosswalk for exact sampling, parity, normalization, and adjoint rules.
- **Test limits:** Report the tested truncation, grid, precision, spin, operation, longitude count, and worker count. The campaign results do not establish a universal maximum or a general f32 accuracy threshold.

## Source-level opportunities

The pinned DUCC comparison covers `sht_inner_loop.h` and `sht.cc` at revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. It identifies concrete differences
between the two synthesis paths:

- DUCC gathers active rings into order-specific batches with `fill_a2m`; RustSHT
  currently skips inactive blocks and compacts mixed lanes in ring-group blocks.
- DUCC transforms scalar coefficients once per order before ring batches. RustSHT
  prepares paired coefficients inside each ring group's order loop.
- DUCC dynamically assigns orders to workers; RustSHT assigns contiguous ring
  groups and each worker traverses the full order range.
- Both implementations have scaled recurrence setup, an ordinary SIMD kernel,
  and scaled fallbacks. The likely gains are in repeated coefficient work,
  active-lane handling, and accumulation cost, not in replacing the recurrence
  with a generic dense transform.

The source comparison proposes measuring coefficient-preparation time and
active-lane density before changing the scheduler. Existing route profiles
show that recurrence plus accumulation dominates high-resolution workloads, but
they do not isolate either source-level cost. The rejected order-owned and
hemisphere prototypes also show that reducing recurrence counts alone does not
guarantee lower whole-transform time.

## How to interpret the measurements

- The current M6 baseline and isolated candidate results use different source
  builds. Candidate/baseline ratios apply only to the matched source pair and
  workload named in each report.
- q1 and q8 are separate execution regimes. A q1 improvement does not establish
  a q8 improvement, and per-worker CPU timer sums are not wall-time shares.
- The scaled-prefix accumulator has valid six-pair q1 results at T1023 and
  T2047, but its T511 baseline control failed the fixed stability gate. The
  valid high-resolution results justify integration work, not a full-operation
  acceptance claim.
- The exponent-epoch and prefix-fusion measurements compare experimental
  sources. They do not replace the accumulator's clean-production comparison.
- Retain every valid timing and failed gate in its report. Do not use diagnostic
  profiles, one-pair screens, or an invalid process as performance acceptance.

## Implementation order

1. Integrate the scaled-prefix accumulator into scalar synthesis with bounded scratch, checked output commit, SIMD suffix, and the existing scaled fallback. Validate T127/T255, T720 or T799, T1279, T511/T1023, and selected T2047 cases across GL, CC, F1, reduced `mmax`, odd/even longitude counts, `phi0`, and one/multiple workers.
2. Reduce measured transition and coefficient-preparation work. Test per-order preparation and active-lane costs before changing the ring-group schedule.
3. Extend shared recurrence work to spin-1 synthesis without changing E/B conversion, spin signs, polar behavior, or its checked fallbacks.
4. Improve scalar and spin-1 analysis, then native f32. Keep operation-specific adjoint and accuracy checks.
5. Run paired DUCC comparisons across representative workloads at integrated milestones. Report gaps and regressions by operation; do not infer parity from a single truncation.

Historical reports and artifacts under `reports/` provide full methods, raw scientific samples, and source patches. Read them through the references above when a specific implementation decision needs more evidence.
