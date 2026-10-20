# RustSHT — performance and implementation handoff

**Updated:** 2026-10-19. **Branch:** `opt/sht-performance-parity`.
**Objective:** implement DUCC-class native numerical execution while preserving
RustSHT's API, numerical safety and supported operation families. DUCC remains
the external reference; detailed methods and samples stay in the linked reports.

## How to use this summary

Read this file before revisiting historical experiments. It records current
production paths, the accepted interpretation of earlier screens, and the next
implementation work. Consult linked reports only when their details affect a
specific engineering decision. Do not combine timings from different commits,
workloads, operation boundaries or compiler settings.

## Current production paths

- Scalar f64 GL synthesis uses the order-major kernel in
  `src/legendre/scalar_ordered.rs` when eligible. It packs active/reflected
  rings by order, prepares coefficients per order, uses SIMD recurrence and
  dynamic order scheduling, and stages spectra before committing output.
- Unsupported or unsafe scalar synthesis cases retain the ring-group paired
  recurrence and checked scaled fallback. The old tile-128 accelerator and its
  tile-only workspace arrays are retired; archived source remains available.
- Scalar analysis uses active reflected-ring packing, compacted SIMD blocks,
  dynamic per-order work and a fused ordinary recurrence/accumulator. It falls
  back to the checked scaled path when range validation declines a block.
- Spin-1 synthesis retains a narrow, measured reflected-hemisphere route and
  scalar-conversion paths; other shapes use the general spin recurrence.
  Analysis remains operation-specific. Spin 2–4 and their supported adjoints
  remain on established paths.
- Native f32 recurrences are production. The scalar normalization-skip
  selector is restricted to tested truncations; f64 bridging is not the general
  f32 route. GL, CC and F1 retain their distinct rules and resampling behavior.
- RustFFT remains the root FFT engine. Reusable workspace scratch is bounded
  by worker and transform dimensions; public APIs are unchanged.

## Latest matched analysis profile

The T720 f64 GL profile at `fbefc60d` found recurrence and coefficient
accumulation to be the largest Rust excess. Full reused-call analysis measured
41.822/19.443 ms (2.15×) at q1 and 7.507/3.245 ms (2.31×) at q8, Rust/DUCC.
The instrumented recurrence-plus-accumulation stage measured 34.166/11.12 ms
at q1 and 5.123/1.76 ms at q8. Relative L2 error was `6.039e-14`.

FFT, coefficient preparation and ring packing did not explain the gap; q8
worker use was about 98% over the recurrence interval. `perf` hardware events
were unavailable under the host's existing permissions. Temporary counters
found 474 T720 declines, all during sectoral initialization; no recurrence
declines or discarded degree work occurred. The 11.8 ms callback time is
therefore mostly required mixed/scaled recurrence, with only a repeated seed
attempt. A bounded normal-route change preweights opposite-parity spectra once
per active block, improving the same-source full-call screen 2.5%/1.7% at T720
q1/q8 and 1.1% at T300 q1; T300 q8 was within screen noise. The T720 Rust/DUCC
ratios remain about 2.0–2.1×, with different setup boundaries. The rejected
two-pair implementation remains archived. See the
[profile](experiments/scalar-analysis-stage-profile-2026-10-18.md).

The following spin-1 GL analysis change precomputes ring scales once per
transform and reuses them across orders and Q/U. Full-call q1 screens fell
2.6% at T511 and 3.1% at T720; reduced T300 was nearly unchanged. q8 results
were within host variation. See the
[ring-scale screen](experiments/spin1-analysis-ring-scale-screen-2026-10-18.md).

The scalar-analysis candidate now advances insignificant initial pairs using
DUCC's `sht_ftol=2^-60` threshold and an eight-ulp accumulation budget. In
mixed blocks, vector masks identify exceptional exponent lanes; negative scale
epochs are omitted under that same gate, while positive epochs and abnormal
mantissas retain exact scaled multiplication. It is enabled only for eligible
f64 GL `A`; `A*`, CC/F1, spin and f32 retain their existing paths. Unsafe
inputs still use the checked fallback. A Rust-only release phase screen on
Rust 1.99.0 and x86_64 Ryzen 9 5950X (AVX2/FMA; reusable plans/workspaces;
aggregate RSS not measured) used one warmup and five timed calls. Relative to
prefix-only source `e58c37f3`, T720 GL (721×1442, full mmax)
recurrence/accumulation was 26.932/3.781 ms at q1/q8 and full analysis was
34.205/5.586 ms. Reduced T300 GL (301×603, mmax=150) measured 2.390/0.390 ms
for recurrence/accumulation and 3.770/0.852 ms end to end; q8 T300 full-call
latency rose from 0.736 ms while its recurrence fell from 0.407 ms, so that
non-recurrence difference remains unresolved. DUCC was not remeasured; the
earlier roughly 2× T720 full-call gap remains historical context, not a current
parity claim. Next work targets DUCC's scaled accumulation and ordinary loop.

The current single-pair fused SIMD analysis traversal was screened against
the preceding Rust source and pinned DUCC revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. The bounded run used Rust 1.99.0
with `x86-64-v3` code generation, GCC 15.2.0 for DUCC, and one warmup plus five
timed calls on an AMD Ryzen 9 5950X. Rust recurrence/accumulation medians were
23.335/3.507 ms at T720 q1/q8 and 2.065/0.321 ms at reduced T300 q1/q8; the
DUCC instrumented inner-loop wall medians were 11.19/1.78 ms and 0.949/0.207
ms. Their phase boundaries differ: Rust separately times order-spectrum
preparation, while DUCC's interval includes more preparation and output work.
The ratios (about 2.08/1.97× at T720 and 2.18/1.55× at T300) are directional,
not strict phase-equivalent claims. Recurrence/accumulation improved about
6–8% from the preceding Rust source. q1 output relative-L2 errors were
`1.77e-14` at T720 and `1.10e-14` at T300. q8 timings varied with host load;
hardware counters were unavailable under existing permissions. The accepted
change fuses one coefficient-pair accumulation with next-state recurrence and
range checking in a single SIMD traversal. The mixed-scale fusion and 256-lane
packing experiments regressed and were discarded; checked scaled fallback is
unchanged. The remaining stage gap calls for a broader execution-structure
improvement, not more local unrolling.

The mixed-scale recurrence now also accumulates the current coefficient pair
while advancing the next pair in one lane traversal. It retains exact scaled
products for lanes not covered by the prefix omission budget and normalizes
before entering the ordinary SIMD loop. A same-flags Rust phase screen against
`8f2c597a` used one warmup and five timed calls on a Ryzen 9 5950X with Rust
1.99.0 and `x86-64-v3`: T720 GL (721×1443, full `mmax`) recurrence and
accumulation fell from 23.508 to 23.023 ms at q1 and 3.463 to 3.306 ms at q8.
The profiled full-call medians fell from 35.233 to 34.250 ms and 6.191 to
5.902 ms. A separate exact-even full-call screen and the reduced T300 screen
were noisy and did not show a stable change. The screen measures the full
Legendre phase, not mixed blocks alone, and does not establish DUCC parity; see
the [bounded screen record](../data/scalar-analysis-mixed-fusion/README.md).

The SIMD code-generation follow-up moved cosine-square and scale-correction
arrays into reusable workspace scratch. Matched x86-64-v3 assembly reduced the
block wrapper's local reservation from 1088 to 896 bytes and YMM stack-memory
operands from 31 to 16. A bounded full-call comparison of the complete
candidate against `a707780`
improved reduced T300 q1 by 10.3%, while T720 q1/q8 were 6.9%/10.1% slower;
T300 q8 was 2.1% slower. Output differences from the baseline were below
`1.2e-17` relative L2. A no-inline initializer experiment regressed and was
reverted. The remaining T720 q1/q8 stage gap is directional at about
2.36×/1.96× against pinned DUCC because phase boundaries differ. See the
[code-generation follow-up](experiments/scalar-analysis-codegen-followup-2026-10-19.md).

## Technique findings

| Technique | Status | Result | Reason | Reference | Next action |
|---|---|---|---|---|---|
| Ishioka paired-degree scalar recurrence | Production basis | DUCC-derived paired recurrence with ordinary SIMD and scaled fallback. | Recurrence mathematics is established; execution and accumulation differ from DUCC. | [`scalar_paired.rs`](../../src/legendre/scalar_paired.rs) | Retain as reference and improve measured loop structure. |
| SIMD scalar and spin recurrence | Production | Pulp handles normal lanes; checked scalar/scaled paths preserve unsafe cases. | Mixed lane ranges and exceptional coefficients require safe decline behavior. | [`scalar_paired.rs`](../../src/legendre/scalar_paired.rs), [`spin_transform.rs`](../../src/spin_transform.rs) | Reuse common machinery only where operator semantics agree. |
| Scalar hemisphere-pair synthesis | Rejected prototype | T511 was 10.4% slower at q1 and 6.2% slower at q8. | Few blocks were eligible; setup and scatter outweighed shared recurrence. | [Experiment](experiments/hemisphere-paired-synthesis-2026-10-14.md) | Do not repeat the coarse block design. |
| Spin-1 hemisphere pairing | Production, restricted | Selected GL full-mmax route covers T319–T511 at q4/q8; tested reductions ranged from 4.1% to 22.7%. | Benefit depends on eligible geometry, normal range and worker count; checked fallback remains. | [Results](experiments/spin1-hemisphere-pair-synthesis-2026-10-14.md) | Preserve selector; widen only with matched evidence. |
| Earlier order-owned vs ring-owned synthesis | Prototype rejected | Old order-owned prototype was 32.65% slower at T511 q8 and 1.87% slower at T1023 q8. | It added coefficient setup, transpose and order-imbalance costs; it was not DUCC's full active-ring algorithm. | [Screen](experiments/order-owned-synthesis-2026-10-15.md) | Do not repeat that prototype; current order-major path is a separate complete kernel. |
| Active-ring packing and compaction | Production | Order-major synthesis packs active rings; analysis compacts mixed SIMD blocks. | Dense per-order packing reduces inactive work but carries packing/scatter overhead. | [`scalar_ordered.rs`](../../src/legendre/scalar_ordered.rs), [`transform.rs`](../../src/transform.rs) | Improve only against measured active-lane and packing costs. |
| Scaled transition and safety fallback | Production | Unsafe transformed coefficients or recurrence lanes decline before output commit. | Extreme finite values and subnormals need the existing robust route. | [`scalar_ordered.rs`](../../src/legendre/scalar_ordered.rs) | Keep transactional output and checked fallback. |
| Tile-128 scalar synthesis | Retired from production | Historical screens showed useful T511/high-degree gains, but were not a stable general acceptance. | Ordered execution now covers the useful operation; tile-only buffers and dispatch were removed. | [Prior status](experiments/performance-status-pre-ducc-port-2026-10-18.md) | Keep source evidence archived; do not restore tile dispatch. |
| Scaled-prefix accumulator | Historical candidate, superseded | Isolated q1 candidate/baseline ratios were 0.569 at T1023 and 0.408 at T2047. | These were tile-route source comparisons, not the present integrated kernel or broad DUCC parity evidence. | [M6 campaign](experiments/m6-cpu-performance-parity.md) | Preserve numerical ideas only when they fit the ordered kernel. |
| Exponent-epoch accumulator | Rejected as an improvement | q1 ratios were 0.594 at T1023 and 0.415 at T2047, weaker than the preceding accumulator. | It added epoch handling without improving those measured cases; T511 evidence was unresolved. | [Epoch results](../data/m6-legendre-redesign/route-timing/exponent-epoch-q1-screen-v1-results.md) | Do not reintroduce absent a new profile-supported mechanism. |
| Prefix normalization/accumulation fusion | Historical tile implementation | Directional screens improved selected T511/T2047 cases; T255 was effectively unchanged. | The route is retired and screens lacked a controlled broad gate. | [Fusion screen](../data/m6-legendre-redesign/route-timing/prefix-lane-pass-fusion-screen-t2047-q1.md) | Do not restore the tile implementation; compare any reused primitive in current code. |
| SIMD prefix arithmetic | Partly retained, historical tile screen | Prior one-process screens ranged from 0.4% to 6.8% lower at selected T255/T720/T1279 cases. | Those results were Rust-only, ungated and tied to the retired route. | [Prior status](experiments/performance-status-pre-ducc-port-2026-10-18.md) | Keep current vectorized scaling; avoid claims from the retired screen. |
| SIMD range prechecks | Inconclusive | A diagnostic reduced one prefix estimate by 6.41% and whole-call estimate by 2.62%. | No paired acceptance; corrected lane replay found little work safe across all degrees. | [M6 report](experiments/m6-cpu-performance-parity.md) | Keep checked transitions; do not add a scan without a new whole-call case. |
| Vectorized shared prefix and empty-rescale check | Production | T720/T300 q1 synthesis fell 14%/12% in the bounded screen; later stage checks found prefix work substantially reduced. | Gains were q1-specific; q8 remained variable and behind DUCC. | [Post-fix profile](experiments/matched-rust-ducc-stage-profile-2026-10-18-41b309a5.md) | Stop q1 prefix tuning; address distinct q8 structure only with evidence. |
| Two-vector ordinary synthesis suffix | Production | T720 ordinary stage improved about 20%; q8 full-call screen improved about 21% versus the preceding source. | Pair-major register residency reuses coefficients while preserving lane state; DUCC gap remains workload-dependent. | [Source comparison](experiments/matched-rust-ducc-stage-profile-2026-10-18-41b309a5.md) | Keep and validate across irregular sizes and fallback cases. |
| Vector-block-major synthesis suffix | Rejected | Legendre stage was 5.2×/6.4× slower at T255/T300. | Repeated coefficient broadcasts per SIMD block outweighed the changed loop order. | [Current source screen](experiments/scalar-ducc-kernel-screen-2026-10-18.md) | Do not retry this layout unchanged. |
| Coefficient cache and safety preflight | Production, eligible fallback route | T1023 q8 median improved 8.5% in five paired process comparisons; broader grids were inconclusive. | Reuses coefficient work without changing ring ownership when workspace constraints allow. | [Prior status](experiments/performance-status-pre-ducc-port-2026-10-18.md) | Retain bounded eligibility; avoid extra per-grid selectors. |
| Scalar analysis fused recurrence/accumulation | Production, eligible f64 GL `A` | DUCC-threshold prefix advancement and masked negative-scale accumulation remain enabled. Ordinary and mixed-scale loops accumulate one coefficient pair while advancing the next recurrence state in a single SIMD lane traversal. | q1 output agrees with pinned DUCC to relative L2 `1.1–1.8e-14`; the T720 full Legendre phase screen improved 2.1%/4.5% at q1/q8. Exact-even full-call and reduced T300 screens were inconclusive. The checked scaled fallback remains. | [Plan 11](../../plans/11-ducc-equivalent-scalar-analysis.md), [profile](experiments/scalar-analysis-stage-profile-2026-10-18.md), [mixed-fusion screen](../data/scalar-analysis-mixed-fusion/README.md) | Isolate mixed-stage timing, then inspect the remaining ordinary-loop instruction flow; do not restore the rejected multi-pair kernel unchanged. |
| Spin-1 analysis | Production; behind DUCC | GL f64 paired analysis now reuses per-ring scales across orders; q1 full-call screens fell 2.6%/3.1% at T511/T720. q8 remained within host variation. | Channel preparation and spin recurrence remain distinct from scalar analysis. | [M6 campaign](experiments/m6-cpu-performance-parity.md), [ring-scale screen](experiments/spin1-analysis-ring-scale-screen-2026-10-18.md) | Profile recurrence and accumulation next; preserve E/B and adjoint semantics. |
| Native f32 recurrence and normalization skips | Production, restricted selector | T1023/T1279 scalar skips matched fallback and improved measured S/A; T720 rejected a broad selector. | Benefit depends on degree range and cannot be generalized from two truncations. | [f32 status](experiments/f32-scalar-normalization-skip-2026-10-16.md) | Preserve selector; optimize native f32 after f64 kernel work. |
| Reusable f64 bridge for f32 | Rejected as general route | Faster than Rust native f32 in an initial screen but up to 13× slower than DUCC; spin-1 temporary reached 48 MiB. | Conversion and memory costs do not establish a useful general route. | [Bridge summary](../data/m6-f32-bridge/summary.md) | Keep native f32 recurrence as the target. |
| FFT and multicore scheduling | RustFFT production; recurrence dominates | Parallel row FFT helped analysis; high-resolution Legendre work remains the largest phase. | FFT changes do not address the measured recurrence excess; order scheduling is already dynamic on current fast paths. | [Atmospheric study](experiments/atmospheric-resolution-performance-ectrans-2026-10-16.md) | Keep FFT stable; improve recurrence and worker balance from measurements. |
| Atmospheric-resolution comparison | Historical 29-grid matrix | DUCC was faster for all measured scalar/spin-1 S/A workloads from that source revision. | The matrix predates current kernels and its ratios cannot be transferred to this branch. | [Study](experiments/atmospheric-resolution-performance-ectrans-2026-10-16.md) | Use as workload context; compare representative grids at major milestones. |

## Operation and numerical coverage

- Scalar synthesis: eligible f64 GL order-major path plus a checked fallback;
  CC/F1, unsupported layouts and unsafe fast-path inputs keep established routes.
- Scalar analysis and its adjoint remain separate from synthesis and synthesis
  adjoint. Their weights, `m=0`, and rectangular-grid rules are unchanged.
- Spin-1 synthesis/analysis retain E/B conversion, spin-specific recurrences,
  polar handling and distinct adjoint behavior; the narrow hemisphere selector
  is not a general spin route.
- f32, CC/F1 and spin 2–4 remain supported on existing validated paths. Current
  timing evidence is combination-specific, not a universal performance claim.
- Numerical checks must include analytic fields and adjoint identities alongside
  pinned DUCC comparisons; roundtrips alone do not establish correctness.

**Focused scalar-analysis plan (active):** [Plan 11](../../plans/11-ducc-equivalent-scalar-analysis.md) defines a matched native Rust/DUCC kernel comparison, an integrated scaled-to-ordinary transition for the initially scaled blocks, bounded ordinary recurrence/accumulation changes, and numerical acceptance gates. No parity claim yet; use the existing handoff results as baseline, not a fresh benchmark campaign.

## Next implementation work

1. **Scalar analysis (priority):** inspect the mixed-path vector spill and remaining per-pair lane checks, then compare matched recurrence work against DUCC. Keep the single-pair recurrence until a measured change addresses the earlier unroll regression. Preserve exponent correction and checked fallback.
2. **Spin-1:** after a substantive scalar-analysis result, profile recurrence/accumulation and reuse compatible techniques; preserve E/B, pole and adjoint semantics. Per-ring scales already exist.
3. **Generalization:** after validated kernels, improve native f32 and applicable CC/F1/spin-2 paths without breaking spin 3–4.
4. Keep benchmark screens bounded and irregular/reduced cases represented; reserve broader DUCC comparisons for meaningful integrated milestones.

## Selective source and evidence index

- [Active validation roadmap](plans/05-validation-and-milestones.md): P0–P4.
- [DUCC operation contract](../../plans/06-ducc-contract-crosswalk.md) and
  [numerical/grid rules](../../plans/02-numerics-and-grids.md): consult for kernel changes.
- [Latest scalar analysis stage profile](experiments/scalar-analysis-stage-profile-2026-10-18.md): recurrence bottleneck and rejected loop prototype.
- [Scalar synthesis stage profile](experiments/matched-rust-ducc-stage-profile-2026-10-18.md): prefix and q8 findings.
- [Prior status snapshot](experiments/performance-status-pre-ducc-port-2026-10-18.md): archived experiment index.
- [Atmospheric study](experiments/atmospheric-resolution-performance-ectrans-2026-10-16.md): historical workload patterns, not a standing matrix.

**Evidence rule:** every performance number is tied to the report's source,
workload, operation boundary and worker count. Do not claim DUCC parity from
an isolated Rust-only comparison or from a previous source revision.
