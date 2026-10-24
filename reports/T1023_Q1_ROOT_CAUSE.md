# GL T1023 full-mmax spin-1 synthesis q1: root cause

**Finding.** The old q1 dispatcher bypassed RustSHT's existing reflected-component adapter because it only admitted a one-column spectrum. It therefore used the dual-Q/U physical-latitude kernels for nearly all recurrence work. A guarded q1 route now sends all 1,024 orders through the adapter, which processes reflected latitude pairs one scalar-converted component at a time. Matched native runs measured 90.402 ms versus 173.557 ms on the exact parent and 124.899 ms for pinned DUCC. This is a validated improvement and puts this cell below DUCC (0.72×). The old-route profile explains why the change mattered; it does not attribute the candidate's new time by function.

## Revisions, machine and reproduction

| Item | Identity |
|---|---|
| Old-route Callgrind source | Rust 58aa2e09, production-equivalent to exact parent bff7ed62; instrumented old q1 route |
| Candidate source | 7ef6d6e1; current docs checkpoint 5b653a99 |
| DUCC source | 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b (0.42.0) |
| Rust compiler / flags | rustc 1.99.1, LLVM 23.1.1; release; -C target-cpu=x86-64-v3 -Aunexpected_cfgs |
| DUCC compiler / flags | GCC 15.2.0; SHT TU -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3 |
| Host / ISA | AMD Ryzen 9 5950X; x86-64-v3, AVX2, FMA; all A/B variants used the same fixed affinity |
| Rust exact-parent binary SHA-256 | d16109d74612dacdaded495470cb6fa2e8b33bb2def597a51e731edcf469094e |
| Rust candidate binary SHA-256 | 88b952a36052bda8e9d94a9f0914878b76a335d8d462eea52660b9ab03eadb1b |
| Pinned DUCC binary SHA-256 | 671986a60c09591c8fc8219a5522006fd883d57d49a2f7d06e6eb468a670b7e5 |
| Rust Callgrind harness binary SHA-256 | ba0e06b824d4d36dbbe0a26ebf0ed99390d6ef3ca1caba030eb3367700caab6e |
| DUCC Callgrind binary SHA-256 | 6bcbf67a8b5d20e8c8b2de1febc89600cb39b7d7f47a65752732bfef41cf3b27 |

Native A/B procedure: run the existing exact q1 worker for three interleaved rounds with all variants pinned to the same logical CPU.
The workload is GL, Lmax=Mmax=1023, 2,048 longitude samples, phi0=0.23, fixture seed 1436943300, and target_ms=25.
Each worker uses repeat/reset; compare exact-parent bff7ed62, candidate 7ef6d6e1, and pinned DUCC.
Median candidate/parent paired ratio is 0.520; candidate/DUCC is 0.722.
Candidate-vs-DUCC output rel-L2 is 2.60e-13, max absolute difference 5.41e-9; candidate-vs-parent rel-L2 is 2.76e-13.
All nine worker outputs were finite and repeat/reset checks passed.

Callgrind used optimized binaries with debug symbols and the same fixture.
Rust ran the existing ignored profile_spin1_standard_synthesis_phases test with RUSTSHT_PROFILE_LMAX=1023, MMAX=1023, GRID=GL, THREADS=1, REPETITIONS=1, pinned to one logical CPU.
The Rust profile harness used CARGO_PROFILE_RELEASE_DEBUG=2 and target-cpu=x86-64-v3; its executable SHA-256 is listed above.
The tool options were valgrind --tool=callgrind --instr-atstart=no --dump-instr=yes --collect-jumps=yes --cache-sim=yes --branch-sim=yes.
The existing harness toggled instrumentation between PROFILE_CACHE_STATE and PROFILE_TOTAL using callgrind_control, excluding fixture/cache setup.
DUCC ran ducc_spin1_full_transform_parity --worker synthesis GL 1 1024 2048 1023 1023 0.23 ... under Callgrind; its symbol-enabled optimized executable SHA-256 is listed above.
Raw profiles remain under /tmp: rustsht-t1023-q1-matched-callgrind.1035889 and ducc-t1023-q1-callgrind.1033000.
Reproduce annotation with callgrind_annotate --inclusive=no and inspect codegen with objdump -dC -Mintel.
Hardware perf stat/record counters were unavailable because perf_event_paranoid=4.
No PMU cycles or hardware cache-miss counts are claimed.

## Executed paths and equivalent work

Public synthesis enters ShtPlan::synthesize_spin_ring_group_orders (src/spin_transform.rs:1886), which dispatches ring groups to mode/order processing.
In the old route, q1's spectrum stride was 1,025, so the one-column component-order adapter gate failed.
The q1 caller then traversed physical-latitude blocks with fused Q/U accumulation, checked scaled transitions and a small polar fallback.
The adapter already existed and prepared its scalar-converted coefficients once per order; the bottleneck was route eligibility, not missing reflection code.

The candidate admits only the complete GL full-mmax, one-thread q1 shape.
It sends each q1 order to try_synthesize_spin1_prepared_component_order_to_strided_spectra (src/spin_transform.rs:3419).
It attempts all 1,024 orders: 1,024 success, zero declines.
If an order declines, the code clears only that output column before using the existing checked fallback.
The adapter processes reflected north/south pairs and both output components sequentially.
Test-only source counters counted 225,203,832 active component-lane × degree-pair updates across those passes.
The 130 polar blocks use its generic per-degree path for 175,584 updates.
Candidate per-m-bin work splits were not retained; this is unresolved.

Old-route test counters give the following physical-latitude × degree-pair updates. The generic polar count is shown separately; rounded totals may differ by 0.001M.

| m range | Ordinary fused | Checked transition | Generic polar |
|---|---:|---:|---:|
| 0–255 | 99.157M | 8.407M | 0.176M |
| 256–767 | 76.515M | 26.348M | 0 |
| 768–1023 | 7.314M | 0.716M | 0 |
| Total | 182.986M | 35.471M | 0.176M |

The old route thus did most work in the ordinary kernel; 16.2% went through checked transition, while generic polar work was 0.08%. Of 2,283 prepared-kernel declines, the ordinary attempt rejected its initial sectoral normalization before entering its degree loop; fallback recomputed the sectoral seed but did not replay a completed recurrence. Only 2.246M updates (about 1.0%) used scaled-prefix recurrence. No coefficient-safety decline was counted. Coefficient conversion/preparation was once per order, not once per latitude block.

Pinned DUCC's get_mlim on the same GL geometry yields 225.142M physical-latitude × degree-pair updates, about 2.9% more active work than the old Rust counter. This includes both transformed components and active reflected latitudes. Dividing total Callgrind Ir by those source-derived work counts gives a coarse whole-transform 15.05 Ir/update for old Rust and 5.35 Ir/update for DUCC (2.81×); this includes outer/setup/FFT instructions and is not an isolated-kernel metric. The candidate's component-lane counter is a different unit and is not used in this ratio.

## Disjoint instructions, runtime and code generation

Callgrind counted 3,291,119,678 Ir for one old-route Rust transform and 9,630,926,104 Ir across eight DUCC worker transforms (1.204B/transform; 2.73× fewer DUCC Ir). Ir are simulated instruction references, not CPU cycles or elapsed-time shares.

| Region / symbol | Old Rust Ir | Share of Rust total | Attribution |
|---|---:|---:|---|
| FusedPreparedSynthesisPairBlock | 1.307B inclusive; 823.463M self | 39.7% inclusive; 25.0% self | Ordinary prepared SIMD route; do not call 1.307B exclusive |
| try_synthesize_prepared_order_pair_transition_dispatched | 1.149B inclusive; about 1.055B self after 94.048M identified helper children | 34.9% inclusive; ~32.1% self | Checked transition; includes loop and suffix work |
| normalize_synthesis_pair_lanes_simd, scaled_power, scaled multiplies | 94.048M combined child instructions | 2.9% | Identified transition children; not additional to the inclusive transition |
| Range checks / reductions | 9.19M checks; ~37M reductions | 0.28% / ~1.12% | Vector validity tests and reductions |
| Polar safe-lane route | 151–163M inclusive, depending on callgraph edge | 4.6–5.0% | Edge totals overlap caller attribution; exclusive polar-only Ir unresolved |
| DUCC calc_alm2map → alm2map_kernel | 6.727B → 6.654B inclusive over 8 calls | 69.9% → 69.1% of DUCC total | Child is included in parent; 831.8M kernel Ir/transform |
| DUCC iter_to_ieee | 30.399M over 8 calls | 0.32% | 3.80M/transform |
| DUCC fill_a2m, extract_a2m | 632.091M, 173.844M over 8 | 6.56%, 1.81% | Ring/component preparation and extraction siblings |

The table labels inclusive nodes; do not sum them. Callgrind's branch simulation counted Rust 402.12M conditional branches / 4.209M simulated mispredicts for one transform, versus DUCC 466.88M / 6.842M over eight (58.36M / 0.855M per transform). Its cache model reported Rust 39.63M D1 read and 4.00M write misses; DUCC 7.14M read and 7.10M write misses per transform. These are simulated counts with different internal buffering and do not replace hardware counters.

Optimized AVX2 disassembly (objdump -dC -Mintel) shows:

- Rust FusedPreparedSynthesisPairBlock uses four-wide YMM vectors. Its main vector body has ten packed FMA opcodes per pair-loop iteration over one four-latitude block. The ordinary loop has no hot YMM stack spills.
- Rust traverses latitude block outside the degree-pair loop, so scalar coefficient broadcasts recur for each latitude block. The fused Q/U kernel also combines even/odd coefficient factors for the two physical hemispheres.
- The checked transition's ordinary suffix has two YMM accumulator spills/reloads in its pair loop. Other stack references hold intentional normalization/mask scratch. Its per-pair range checks reduce vector predicates to scalar branches; check instructions are cheap alone, but the transition body processes 16.2% of old-route updates.
- DUCC alm2map_kernel loops degree outside nv2 latitude-vector blocks, loading/broadcasting coefficients before the latitude loop; it groups reflected north/south rings and accumulates one scalar component at a time. d.lam* and d.p* array traffic is intentional recurrence/output scratch, not compiler spills. iter_to_ieee scans/rescales the prefix before ordinary recurrence.
- Raw FMA opcode counts are not normalized to equal output-component and latitude work: Rust's body iteration and DUCC's degree unroll have different extents. Therefore the earlier “20 vs 10” FMA ratio is withdrawn; a precise dynamic per-output FMA count is unresolved. No scalar helper call or fallback dominates the ordinary vector loop.

DUCC source references at pinned commit: sht.cc::spin1to0 (line 473), alm2leg (576), spin dispatch (705); sht_inner_loop.h::iter_to_ieee (426), calc_alm2map (542), alm2map_kernel (458), fill_a2m (1383), extract_a2m (1426). Rust locations are the dispatcher above and src/legendre/scalar_paired.rs transition and fused kernels. Exact elapsed time by kernel is unresolved: Callgrind Ir and its caches/branches are not timing probes, and hardware counters were blocked.

## Ranked causes and controlled experiment

1. **q1 did not use reflected component processing.** This is the largest measured cause: enabling the already-existing adapter for all 1,024 orders saved 83.2 ms from exact parent and made the whole transform 34.5 ms faster than DUCC. The old route made more paired updates and used the dual-channel physical-latitude path. The result is whole-transform evidence; no candidate Callgrind breakdown was captured.
2. **Ordinary fused Rust recurrence has high exclusive instruction cost.** Its 823.5M exclusive Ir is 25.0% of the whole old transform. Repeated broadcasts and per-degree even/odd Q/U combinations are present in assembly. Their isolated millisecond cost is unresolved; the validated route change removes this path for the selected case.
3. **Checked transition is large, but “remove checks” is not supported.** It accounts for ~1.055B self Ir on 16.2% of updates. The range-check instructions themselves are only about 0.28% of total, and the scaled prefix only 1.0% of updates. Preserve the late-tail safety fallback; the evidence does not justify disabling checks.

**Selected experiment and result.** Prediction: allowing q1's strided full spectrum through the existing reflected adapter would eliminate most dual-channel physical-latitude recurrence while preserving the adapter's per-order decline/clear/fallback contract. Observed: 1,024/1,024 orders succeeded; exact-parent/candidate/DUCC medians were 173.557/90.402/124.899 ms; candidate is 0.520× parent and 0.722× DUCC. Focused executed tests passed: spin_transform::tests::spin1_gl_table_synthesis_matches_recurrence_and_adjoint (1 passed), spin_transform::tests::spin1_order_major_synthesis_reuses_workspace_across_changed_coefficients (1 passed), spin_transform::tests::spin1_scalar_transition_splits_polar_lanes_and_matches_general (1 passed), and the ignored T1023 phase profile test (1 passed). The output comparison above passed; no tolerances or numerical checks were weakened. Source is pushed as 7ef6d6e1.

**Remaining limits.** Hardware cycles/cache misses are unresolved; candidate disjoint instructions are unmeasured; the old-route instruction profile is explanatory, not a candidate attribution. The accepted T1023 q1 cell is now below DUCC, but all-case parity remains open. Next high-priority case is GL T720 spin-1 analysis q1, followed by q8 and all remaining gaps in the active ledger.
