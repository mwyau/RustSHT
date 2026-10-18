# Matched RustSHT–DUCC phase and function profiling

**Initial profiling checkpoint completed (2026-10-18).** This was a temporary profiling assignment, **not** a new broad benchmark or standing optimization plan. Check the latest branch HEAD and current code before acting. The active source module is `src/legendre/scalar_ordered.rs`, **not** the retired `scalar_ducc.rs`; `DUCC` belongs only in external-reference/attribution names. Follow the repository's minimal-fallback and Git synchronization rules.

## Outcome — completed initial profiling; do not rerun unchanged

The initial phase profiling and source-based follow-up were completed at `41b309a`. Results: [measured stage report](../reports/matched-rust-ducc-stage-profile-2026-10-18.md). The largest **pre-fix** Rust q1 cost was sectoral initialization/scaled prefix (T720 initialization 1.79 vs 0.51 ms, scaled-prefix 8.29 vs 0.09 ms; buckets not strictly equal work). The latest empty-rescale shortcut lowered full-synthesis q1 medians 14% at T720 and 12% at T300. **Post-fix component timing is still outstanding**; q8 remains slower and too variable to ascribe a confirmed improvement to the fix. Linux `perf` events are blocked by `perf_event_paranoid=4`.

**This file is retained as the methodology and completed initial assignment**, not an instruction to rerun the entire profiling study. The active bounded follow-ups and implementation priorities are now in [plans/05-validation-and-milestones.md](05-validation-and-milestones.md), priorities P0–P4. Repeat only the two small current-source stage checks requested there; do not recreate the 21-cell matrix or add permanent hot-loop timers.

## Question and outcome

The matched 21-case full-scalar-GL synthesis screen at Rust revision `6b356cb1` against ducc0 0.41.0 reported Rust/DUCC ratios **1.89–3.88× q1, 1.98–5.77× q4 and 2.09–6.43× q8** with relative L2 error ≤3.95e-14. These are *whole-transform*, not separated phase measurements. The most recent branch has since changed analysis scheduling; profile **current actual synthesis source**.

**Deliverable:** a compact phase-by-phase and hottest-function comparison with absolute times/cycles, attributable excess latency, matching workload details and a ranked top-three engineering bottleneck list. Distinguish confirmed measurements from hypotheses. Do **not** keep redesigning the kernel before identifying the actual expensive stages.

## Workload selection (small on purpose)

Use two discriminating workloads from the existing matched benchmark:
- **T720 full mmax, GL 721×1442**, even longitude size, q1 and q8. This tests the high-bandlimit, real-FFT-friendly, multicore route.
- **T300 mmax=150, GL 301×603**, odd longitude size, q1 and q8. This exposes reduced-order overhead and the current odd-length FFT behavior.
- Optional one **T1279** case *only if* the first two cases give contradictory or resolution-specific explanations.

Use the **same** harmonic coefficients, normalization, phase `phi0`, array layout, map output semantics, FFT length and thread counts for each library; verify maximum/relative numerical differences. Log hashes, compiler flags, ISA and exact DUCC implementation/revision. Interleave measurements after warming both implementations. Keep trial duration and host workload bounded; reject obviously contaminated data without turning this into a prolonged measurement protocol. Do not mix historic sample timings with current-HEAD profiling.

## Stage correspondence: report measurements, not assumptions

| Conceptual stage | Current RustSHT sites | Reference DUCC sites | Comparability caution |
| --- | --- | --- | --- |
| Setup/plan and validation | `ShtPlan::synthesis_into`, workspace/fast-path preflight | `synthesis`, `synthesis_2d`, `make_ringdata`, setup | Report cached-plan and construction separately; Rust reuses plans, public DUCC call may rebuild geometry |
| Order coefficient preparation | `scalar_ordered::prepare_order`, `fill_factors` / coefficient conversion | `Ylmgen::prepare`, `inner_loop_a2m` coefficient transform | Make sure equivalent normalization work is counted |
| Eligible ring packing | `scalar_ordered` active-ring scanning, `packed_sin/csq` | `fill_a2m`, ring `mlim` | Compare actual eligible lane/order counts |
| Sectoral initialization | `initialize_sectoral_batch`, `power_lower_limit` | `init_lambda`, vector `mypow` | Vector and scalar-tail work may differ |
| Negligible prefix/scaling | `iter_to_ieee`, `rescale_batch`, `scaled_pair_step` | `iter_to_ieee`, `rescale`, `calc_alm2map` | Distinguish prefix degrees from ordinary degrees |
| Ordinary paired recurrence | `synthesize_vector`, `ordinary_two_pair_step` | `alm2map_kernel` | Primary SIMD/FMA hot loop; note actual vector width and spill/load traffic |
| Reflect/phase/output | `scalar_ordered` reconstructed spectrum, parallel tiled scatter | `extract_a2m`, parts of `leg2map` | Rust applies `phi0` and writes Fourier-spectrum layout at different stages |
| Longitudinal FFT and map write | `inverse_synthesis_rows`, RustFFT / real-FFT adapter | `leg2map`, DUCC FFT backend | Separate even and odd longitude behavior, FFT planning, parallel rows and output layout |
| Worker synchronization | Rayon dynamic order jobs, spectrum scatter, row FFT | `execDynamic` and FFT parallel driver | q8 wall time is **not** sum of worker CPU times |

Names are lookup starting points; inspect the current source (some helpers may be inlined/renamed) and annotate any stage boundaries that do not correspond exactly. Note fast-route rejection/fallback rate; do not compare an optimized DUCC kernel with a Rust fallback accidentally.

## Two-level profiling method

**A. Coarse phases first (absolute latency).**
- Establish total end-to-end time in optimized **uninstrumented** builds. For Rust, existing test-only `record_transform_phase` and bounded phase probes may be useful. For DUCC, build a **throwaway local C++ driver** at the pinned DUCC revision with symbolized optimized source and reusable prepared inputs. Measure top-level `alm2leg`, `leg2map`, setup and output operations separately, and if warranted bracket `fill_a2m`/coefficient prep, `calc_alm2map` and `extract_a2m` at **order/batch boundaries only**.
- **Never time each inner degree pair with `Instant`/`chrono`.** That distorts precisely the hot kernel under study. Separate instrumented phase builds from release timing builds; report overhead and prevent the diagnostic timers from changing numerical output.
- If a DUCC C++ driver is temporarily unavailable, use public `ducc0.sht` only for preliminary end-to-end context and label Python wrapper/setup contamination; do not draw detailed kernel conclusions from Python-level phase differences.

**B. Sampling/counters second (where instructions go).**
- On the Ryzen 9 5950X/Linux host, try `perf stat -e cycles,instructions,branches,branch-misses,cache-references,cache-misses -- <runner>` and `perf record -g --call-graph fp -- <runner>` followed by `perf report` / `perf annotate`; collect sufficient warm repeated calls to make sampling useful.
- Use **optimized, symbolized** builds of each library at comparable CPU target and optimization level; Rust `-C debuginfo=1 -C force-frame-pointers=yes`, and DUCC C++ `-O3 -g -fno-omit-frame-pointer`, with identical intended AVX2/FMA CPU targeting. Record actual flags/assembly; do **not** quietly compare native-tuned C++ against generic Rust. Hardware counters and sampling attribution are approximate, not an exact mutually exclusive wall-clock decomposition.
- If `perf` is unavailable because of `perf_event_paranoid` or permissions, note this and use reproducible phase timers; do not modify machine-global settings without owner authorization.
- Record function/self/caller costs, cycles per useful harmonic/ring work unit, instruction counts and IPC where supported. Analyze the top 2–3 kernel hot loops with annotated assembly for FMA throughput, excessive loads/stores/spills, branches and mask handling.
- For q8, record **elapsed wall time** and per-thread CPU/busy distribution separately. Attribute waiting, scheduling overhead, spin-scratch contention/false sharing and memory-bandwidth behavior cautiously. Do not use aggregated multi-thread CPU cycles as though they were sequential wall time.

## Output: one concise report, then implementation

Put the result in **one** short `reports/matched-rust-ducc-stage-profile-2026-10-18.md` (or an appropriately dated name). Include:
1. **Method**: both source hashes/compilers/flags, both workloads, q1/q8, warmup/repetitions, correct output checks, host caveats, fallback route.
2. **Phase table**: Rust time, DUCC time, ratio, approximate share of total wall-time gap, and `measured` versus `estimated/not directly comparable` status for setup, coefficient prep/ring packing, initial/scaled prefix, ordinary recurrence, spectrum scatter, FFT/map and scheduling.
3. **Hotspot table**: top CPU functions with sampled/self time/cycles and associated source line/ASM explanation. Explain any unmatched stages rather than attributing all wall-time excess to the Legendre recurrence.
4. **Ranked top three fixes**: specific Rust source sites, suspected mechanism, measurement supporting it, and smallest correct implementation experiment.
5. **Gaps/limitations**: what cannot be instrumented comparably, noise and unresolved costs. Do not invent unmeasured values.

The next code change should attack the **largest measured source of excess latency**, not simply the most visually obvious loop. Prefer immediate reusable improvements across scalar synthesis/analysis/spin where justified. If the FFT is dominant, isolate it first; do not automatically start a wholesale FFT library port. Keep `PERFORMANCE_STATUS.md` to a two-line link/summary at most; leave historical reports intact.

## Git and safety

Use a temporary local profiling worktree and temporary DUCC source instrumentation as needed, without pushing altered DUCC/vendor sources or keeping profiling hooks in production hot loops. If a helper must be `#[inline(never)]` to observe it, use **profiling-only configuration** and measure the release binary without that annotation too. Benchmarks must not add new always-on allocs, locks or branch checks to SHT hot paths.

Before **every push**, inspect local work, fetch remote, compare ancestry, fast-forward where possible or safely rebase **only unpublished** local work/merge if needed, run tests after reconciliation, push normally and verify remote HEAD; never force-push or discard concurrent agent changes. At most one Luna benchmark-only subagent; no Sol reviewer. The main agent can implement independently, but do not benchmark on a concurrently saturated host without noting interference.
