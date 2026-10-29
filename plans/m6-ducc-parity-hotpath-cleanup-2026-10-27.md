# M6 — DUCC parity engineering and repository consolidation

## ACTIVE OWNER OVERRIDE — 2026-10-29: close the native-f32 DUCC gap

The reusable guarded f32-I/O/f64-internal scalar-synthesis backend is in
production on the four measured GL T720 spin-0 synthesis cells at
`0.775–0.961x` native-f32 DUCC; conversions and writeback are included. Twelve
other spin-0/spin-1 T720 analysis/synthesis cells remain GAP. The Stage B
16-cell diagnostic and production scalar-synthesis evidence remain in the
[single M6 ledger](../reports/DUCC_PARITY_M6.md).

The isolated native-f32 dual-channel reflected spin-1 leaf at exact parent
`5d6e5e34f92ae321d5754d120258e2fa63c79cb3` is rejected. Its q8 full/reduced
whole-call times were 2.196x/3.145x its parent and 74.735x/125.054x DUCC. Its
q1 result was also not a substantial gain (full was 5.7% slower; reduced was
neutral). Every attempted block used the intended route, so this is not an
activation failure. Callgrind found 32.9% fewer Ir at full q1 but no wall-time
improvement; scalar scaled-packet accumulation dominated the candidate profile.
The exact patch, complete timings, route counters, f64 control, accuracy delta,
Callgrind and assembly are in the [V1 evidence package](../reports/data/ducc-parity-m6-2026-10-29/f32-spin1-native-f32-reflected-leaf-v1/README.md).

Next run a fresh bounded V2 experiment: accumulate complete safe scaled
packets on the existing even/odd coefficient rows with the checked
`try_accumulate_f32_scaled_synthesis_packet` helper, retaining exact scalar
handling for tails/unsafe packets and transactional decline for an unsafe full
packet. Reuse the shared typed recurrence and do not alter f64 dispatch. Prove
range handling and route activation first, then compare complete public GL T720
spin-1 f32 synthesis q1/q8 full/reduced against parent and native-f32 DUCC. If
this does not deliver a substantial whole-call improvement, preserve the
negative result and continue with the measured remaining f32 gaps. M6 remains
open.

## SUPERSEDED OWNER OVERRIDE — 2026-10-28: native-f32 family follow-through after synthesis blocker

The accepted shared f32/f64 scalar-synthesis kernel, 32-ring scheduler and
checked native-f32 packet leaf remain in production. The later corrected
scaled-prefix-to-fused-suffix candidate at exact parent `2b531d73` found no
safe nonzero-exponent handoff and caused a repeatable adjacent f32
scalar-analysis regression. This is a quantified blocker for that transition;
do not repeat either suffix trial unchanged. Scalar synthesis remains far from
DUCC at **32.60×/36.12×** full and **21.84×/42.15×** reduced q1/q8 on exact parent 746c7e60. The latest exponent-correction suffix screen completed useful work but slowed all four cells by 8.2–27.0% and the Legendre stage by 7.1–26.9%; see the linked screen in the single ledger. Do not repeat either suffix route unchanged.

The owner's next priority after that blocker is the other native-f32 transform
families, before returning to the remaining f64 gaps. A bounded native-f32
scalar-analysis accumulation leaf has now been accepted on exact parent
`4770e1c7504d084a979a62ca0d8ef57e9fd293f8`. Across fifteen paired comparisons
per cell, its candidate/DUCC ratios are **32.44×/26.73×** full and
**24.37×/19.06×** reduced q1/q8. All remain GAP. The candidate lowers first-call
Callgrind Ir by 21.5% full and 31.1% reduced; the f64 analysis override and
scaled exceptional callback remain unchanged. Protected f64 GL T1023 spin-1
synthesis did not show a meaningful regression. Exact timings, numerical
errors, per-worker counts and AVX2 disassembly are in the
[single M6 ledger](../reports/DUCC_PARITY_M6.md) and
[candidate evidence](../reports/data/ducc-parity-m6-2026-10-28/f32-scalar-analysis-simd-4770e1c7/README.md).

An earlier q1 profile at source `0f821099` measured full GL T720 f32 spin-1
synthesis at 22.433B Rust versus 520.827M pinned-DUCC Ir (43.07x), and spin-1
analysis at 15.752B versus 430.972M (36.55x). The synthesis visitor makes two
indirect `scaled_basis_value_f32` calls for each of 161.324M useful
degree-latitude terms. DUCC's high-Lmax spin-1 route converts to two scalar
channels, evaluates 361 north/equator ring records for this 721-ring grid,
and mirrors the results; Callgrind confirms the scalar AVX2 double-FMA kernel
runs while the spin kernel does not. Rust's f32 `supports_paired_spin` policy
keeps this transform on the general spin callback.

The narrow f32 spin-1 dispatch into the existing separate-channel paired-scalar
path was screened at exact parent `092fe72f` and was not promoted. The initial
three-pair matrix was mixed; a 15-pair reduced-q8 extension gave paired
candidate/parent median `1.006`, 95% bootstrap interval `[0.823, 1.036]`, and
only 6/15 candidate-faster pairs. The run did not capture separate-route
success counts or candidate-specific instruction attribution. Exact paired
timings and the retained patch are in the [single M6 ledger](../reports/DUCC_PARITY_M6.md)
and [screen report](../reports/data/ducc-parity-m6-2026-10-28/f32-spin1-paired-scalar-screen-092fe72f/README.md).

A current-parent test-only route census later showed that this one-gate
candidate left the f32 paired-scalar route inactive: its scalar scratch was
empty and the f32 prepared-block hook returned `Ok(false)`. An isolated
adapter using the shared prepared f32 kernel did improve q1, but slowed q8
(candidate/parent `1.018` full and `1.417` reduced); reject the unconditional
route. Its counts and three-pair screen are in the
[single M6 ledger](../reports/DUCC_PARITY_M6.md).

The current-source full-mmax f32 spin-1 analysis baseline at `e094d4f4` is
36.932x/35.138x DUCC q1/q8 after the accepted scalar-analysis leaf; all cells
remain GAP. Exact paired timings and numerical checks are in the
[single M6 ledger](../reports/DUCC_PARITY_M6.md).

The exact scalar-synthesis source is `092fe72f` (latest dev head `e094d4f4`
adds evidence only). Matched parent/DUCC
ratios are 32.08×/36.94× full and 22.77×/42.43× reduced q1/q8. A 30-sample
test-only phase profile puts 99.68–99.82% of the call in Legendre recurrence
and spectral accumulation.

The scaled-prefix-to-fused-suffix route is a measured blocker. The production
paired-synthesis and scalar-transform source blobs at `092fe72f` are identical
to exact suffix-screen parent `746c7e60`. That isolated candidate completed
13.695M–33.598M checked suffix contributions per cell, yet regressed all four
whole-transform cells by 8.2–27.0% and the Legendre stage by 7.1–26.9%.
Recurrence-state declines were zero; later coefficient/product/output checks
caused 159–940 block declines, and another 93–131 blocks found no transition.
Declined work replayed through the original fallback. Do not repeat that
block-check/replay route unchanged; keep the exact polar, subnormal, nonfinite,
overflow and tail fallback.

The separate f32-I/O/f64-internal diagnostic at transform source `54b166f2`
measured the existing f64 reflected order-major route at 0.746–0.779× DUCC;
forcing that build through the all-ring paired route cost 3.714–13.442× DUCC.
This identifies a large dataflow cost beyond native-f32 exponent range. After
this documented suffix blocker, continue with the remaining native-f32
scalar/spin families, then return to the reflected-ring scalar-synthesis
opportunity without changing scalar-analysis or f64 dispatch. See the
[single M6 ledger](../reports/DUCC_PARITY_M6.md), [exact baseline
evidence](../reports/data/ducc-parity-m6-2026-10-28/f32-scalar-synthesis-baseline-092fe72f/README.md),
[suffix screen](../reports/data/ducc-parity-m6-2026-10-28/f32-scaled-prefix-suffix-746c7e60/README.md),
and [precision diagnostic](../reports/data/ducc-parity-m6-2026-10-28/f32-internal-double-current-source/README.md).
M6 remains open.

## HISTORICAL OWNER OVERRIDE — 2026-10-28: DUCC algorithmic execution parity first (superseded above)

**Continue native-f32 GL T720 scalar synthesis before returning to f64.** Preserve the accepted 32-ring scheduler (`335ce4c8`) and scaled-fallback packet leaf (`acc625d7`; current HEAD `2b531d73`). At that exact parent, matched Rust/DUCC medians are **33.37× / 34.94×** full mmax q1/q8 and **22.75× / 40.42×** reduced q1/q8; all four remain **GAP**. The guarded zero-exponent prefix-to-fused-suffix candidate at earlier parent `ef11ce0a` was rejected after regressing three cells and raising full-q1 Callgrind work by 4.77%. A later corrected packet suffix screen at exact parent `2b531d73` moved up to 61.2M terms into fused suffixes but found no safe nonzero-exponent handoff; it was rejected after 15-pair adjacent f32 scalar analysis showed a repeatable 3.9% regression in q1 and q8. Details for both screens are in the [single active ledger](../reports/DUCC_PARITY_M6.md) and their linked evidence. Continue toward a measured algorithmic reduction, not another isolated 1–3% shortcut. A single-digit Legendre-stage ratio is an intermediate target, not an M6 completion criterion.

### Grounded algorithm difference

Pinned DUCC 0.42.0 (`42ce8ab5`) in `src/ducc0/sht/sht_inner_loop.h` uses `init_lambda` → `iter_to_ieee` → `calc_alm2map` → `alm2map_kernel`: advance the scaled paired-recurrence prefix, accumulate through the scaled-to-ordinary transition, apply a one-time correction to the remaining recurrence states, then execute the degree suffix as an unrolled fused SIMD recurrence/accumulation loop. `src/ducc0/sht/sht.cc::alm2leg` dynamically schedules harmonic orders and batches active latitudes. DUCC's f32 input/output adapter uses f64 internally for the Legendre work.

RustSHT already has shared f32/f64 recurrence mathematics and a native-f32 fused normal-range SIMD kernel. But `src/legendre/scalar_paired/synthesis.rs::synthesize_paired_scalar_order_prepared_f32` enters that kernel only on initially all-normal blocks. Initially scaled blocks keep per-degree callback/scaling work; `F32SynthesisAccumulator` and the new packet leaf only reduce some of that cost. Exact-parent route counters estimate 45.164M / 18.740M full/reduced fallback contributions, 10.195M / 3.213M with nonzero exponent, with about 5.001M / 1.560M nonzero-exponent contributions eligible for packet SIMD. Historical matched whole-call Callgrind was ~8.37B Rust vs ~0.253B DUCC Ir **at its recorded earlier source**, not current-HEAD time or cycles. The same recurrence equations do not guarantee the same execution strategy, cache behavior, instruction counts, or numeric range.

### Historical required bounded execution — updated after suffix rejection

1. **Confirm the comparison.** Fetch/reconcile `origin/dev`; pin Rust parent and DUCC SHAs. Verify geometry, input/output types, `phi0`, sampling, ISA, affinity, q1/q8, and warm-call boundaries. Measure Legendre-only **and** whole-transform times separately, with useful degree×latitude work counts, instruction references, and inspected executed SIMD loops. Report CPU time/wall time separately. Do not add overlapping inclusive Callgrind counters.
2. **Instrument scaled→normal transitions before writing the candidate.** For active eight-lane packets and orders, record first degree where both recurrence states and accumulated output can be safely converted to normal native f32; later range declines; usable suffix contribution count/length; polar-mixed lanes, tails, zero/nonzero exponents, exceptional values, and required retry/rollback. Distinguish raw exponent eligibility from the full finite/subnormal/overflow proof. Compare full/reduced GL T720 q1/q8, plus a small analytic/extreme fixture. Preserve compact evidence; do not instrument timed production binaries.
3. **Suffix handoff screens completed and rejected.** Exact parent `ef11ce0a` was compared with an isolated guarded all-zero-exponent handoff. The candidate split each 32-ring fallback group into 8-lane traversals and generalized the shared fused kernel for arbitrary suffix starts. It slowed full q1/q8 and reduced q1, and increased full-q1 Ir by 4.77%; reduced q8 was inconclusive at three pairs. A corrected packetized route at exact parent `2b531d73` moved up to 61.2M terms into fused suffixes but still made no nonzero-exponent transition. It was rejected after 15-pair adjacent f32 scalar-analysis comparisons showed repeatable 3.9% q1/q8 regressions; reduced synthesis q8 was also neutral. Do not restore either candidate unchanged. Evidence is in the [first suffix rejection report](../reports/data/ducc-parity-m6-2026-10-28/f32-zero-exponent-suffix-screen/README.md) and [corrected-transition screen](../reports/data/ducc-parity-m6-2026-10-28/f32-corrected-transition-2b531d73/README.md).
4. **Next measure synthesis-only reflected-ring reuse.** DUCC's measured GL route computes both hemispheres from northern-ring recurrence work; the prior work census shows approximately half the useful recurrence updates of Rust's all-ring route. Design a scoped experiment that reuses the existing shared native-f32 recurrence and changes only scalar synthesis routing/accumulation. Keep scalar analysis and f64 dispatch untouched. Do not restore the prior duplicated reflected f32 implementation, which regressed reduced scalar-analysis q8. First collect current-source route/work counts and Legendre-stage timing; implement only if the measured work reduction survives polar, tail, and numerical constraints.
5. **Use the f32-I/O / f64-internal run as a diagnostic only.** It showed near-DUCC timing on the reflected order-major route and several-times-DUCC timing when forced through the all-ring paired route. This separates large dataflow and exponent/codegen effects; it does not justify widened production arrays or a second maintained implementation.
6. **Gate any promotion.** Start with 3 randomized/interleaved fresh-process parent/candidate/DUCC q1/q8 pairs; use 15+ paired runs for consequential acceptance. Cover full/reduced f32 T720 scalar synthesis, adjacent f32 scalar analysis and spin-1 analysis/synthesis, and protected f64 GL T1023 spin-1 synthesis q1/q8. Re-run focused tests for low-mode and sectoral analytic fields, coefficient recovery, adjoints, poles, tiny/subnormal/large/nonfinite cases, SIMD tails, finite/repeated/poisoned workspace, capacity, transactional decline and output reset. Do not weaken tolerances. Bitwise equality is preferred but not mandatory if the mathematical order changes; independently substantiate any numerical difference. Protect existing f64 speed wins and report any control regression.
7. **After measurable algorithmic improvement**, extend the same investigation to the f32 analysis/spin-1 families, then return to smaller f64 gaps. If the measured dataflow change does not reduce whole-transform work, preserve negative evidence and change algorithmic hypothesis instead of performing repeated minor tuning. Update only the single live per-cell ledger at `reports/DUCC_PARITY_M6.md` with exact revision, q1/q8 and PASS/GAP/INCONCLUSIVE/UNMEASURED. Profiling, one cell, or docs alone never completes M6.

**Execution checkpoint — 2026-10-28:** the accepted native-f32 fallback
packet vectorization lowered scaled-helper calls and whole-call q1 Ir, with
15-pair whole-transform gains in all four cells; it did not add a recurrence
suffix or establish parity. The guarded all-zero-exponent suffix candidate was
then screened on exact parent `ef11ce0a` and rejected after three of four
whole-transform cells regressed and q1 full Ir rose 4.77%. The earlier
37.62M/33.88M full and 16.70M/15.07M reduced suffix-slot counts are route
opportunity, not evidence that the candidate completed that work. DUCC's
reflected-ring work reduction is now the next measured hypothesis. Keep the
existing scaled path for polar, mixed, exceptional, tail, and declined cases;
do not restore the rejected shared early-stop visitor or duplicate reflected
implementation.

**Operations:** Continue directly on `dev`. No main merge or force-push; reconcile remote before push and commit verified meaningful code slices. Preserve exact experiment evidence before cleaning only owned inactive scratch; do not erase parked worktrees. Main Luna Max is the only production writer; **no Sol or Sol reviewer under any circumstances**. On quota/blocker, checkpoint tested code, concise NEXT and evidence, then stop.

---

## Prior owner direction — 2026-10-27 (historical, superseded above)

The generic f32/f64 fused scalar-synthesis kernel milestone is **complete**: shared recurrence/accumulation, zero-exponent paths, and native-f32 SIMD leaves are implemented and validated. Do not restart that work. Current source is `ec5d56ca29ec6ad4e34fe80443d934677b0515f8`, based on exact parent `3816adce3226a92669a5807241b03f80cb6d8cfd`, with pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. It adds a checked native-f32 SIMD leaf for scaled contribution packets after accepting the separate 32-ring q8 task split. Against the exact source parent, full/reduced q1/q8 candidate/parent is `0.9783`/`0.9754` and `0.9685`/`0.9901`; the reduced-q8 result is inconclusive. Candidate/DUCC remains `36.37×`/`40.07×` full and `25.11×`/`45.32×` reduced q1/q8. All four cells remain **GAP**. The previous scheduling screen established that q1 does not execute the parallel task route, eight q8 workers were active, 12→23 jobs reduced skew, workspace grew 738 KB, and scans were small. Exact timings, numerical criteria, ring costs and worker diagnostics remain in the [current ledger](../reports/DUCC_PARITY_M6.md), [32-ring report](../reports/data/ducc-parity-m6-2026-10-27/f32-32-ring-schedule-2026-10-27/README.md), and [packet-leaf report](../reports/data/ducc-parity-m6-2026-10-27/f32-scaled-packet-leaf-2026-10-27/README.md). **Next:** measure the first normal-range transition and useful suffix length for each active eight-ring packet; only then test a bounded f32-synthesis handoff that leaves the shared f32/f64 recurrence and f64 analysis route unchanged.

**Fallback-screen history — 2026-10-27.** At exact parent `0677b5bf`, the all-active normal-suffix handoff was not promoted because its shared early-stop traversal slowed protected f64 T720 spin-1 analysis q8 in all three pairs. The lane-level profile found only 12.9% of mixed pair-lane contributions could fit disjoint width-eight normal suffix spans under an optimistic partition. The global min-groups-per-task-one candidate was also rejected after protected f64 GL T1023 spin-1 synthesis q8 slowed in all three pairs. These rejected candidates do not change the current route. See the [mixed-span profile](../reports/data/ducc-parity-m6-2026-10-27/f32-mixed-span-route-profile-2026-10-27/README.md), [safe-suffix screen](../reports/data/ducc-parity-m6-2026-10-27/f32-safe-suffix-handoff-2026-10-27/README.md), and [global schedule screen](../reports/data/ducc-parity-m6-2026-10-27/f32-task-grain-global-screen-2026-10-27/README.md).

**New owner goal — 2026-10-27 04:46 UTC.** The former Luna session's “goal achieved” applied **only** to algorithm convergence and unit-test layout. It was completed and pushed at `b7bb2ac49b3b673bd344ad1caf154e52d3130070`, with 317 workspace tests passing, 37 ignored, zero failed, and temporary builds cleaned. **This new M6 goal is not achieved, and must not be declared achieved, by clean CI, successful profiling, documentation work, or completing a handful of short screens.**

**Current starting Git ref:** Fetch latest `origin/dev` before acting (at this plan's creation: `7629d25a5b4df928f9237158ca3cbbb872c42e6d`). Work on `dev` directly. `main` must not be modified or merged until the owner requests it.

## Historical initial execution priority override — 2026-10-27 12:41 UTC: native-f32 hot path first; fused milestone complete

The initial f32 scalar-synthesis implementation and matched baseline work below are complete history. The checked packet leaf has since been implemented and measured. The current next task is in the [owner direction](#prior-owner-direction--2026-10-27-historical-superseded-above): measure per-packet normal-range transition degrees and useful suffix coverage before considering a synthesis-only recurrence handoff. The older f64 GL T720 spin-1 analysis target remains the subsequent f64 acceptance gate. Overall per-cell DUCC goals are unchanged.

### Later M6 update — fallback attribution and task-grain decision

The shared fused algorithm described below was implemented and validated; its implementation is complete. The exact-parent mixed-block profile, accepted 32-ring task-grain candidate and reduced-mmax worker profile remain the scheduling evidence. The checked scaled-contribution packet leaf is now measured at source `ec5d56ca`; the next bottleneck is repeated per-degree scaled work for millions of contributions that remain outside exponent-safe packets. Measure transition degree and suffix coverage per packet before choosing another implementation. See `reports/DUCC_PARITY_M6.md` for the single current performance ledger and the linked experiment reports for route counts, timings, numerical errors, allocation, assembly and per-worker diagnostics.

### Verified baseline and source

Current-source matched reference: RustSHT `98adf1e6b5334021bfe6ae9abb618c6be167a822` vs pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. GL T720 full `mmax=720`, `ntheta=721`, `nphi=1458`, native-f32 scalar synthesis: **q8 373.030 ms / DUCC 2.660 ms = 139.613×**; **q1 1066.473 ms / 19.079 ms = 55.222×**. GL T720 reduced `mmax=180` f32 synthesis q8 is **123.947×** DUCC, so this is not confined to the full-`mmax` fixture. Three interleaved same-source measurements; numerical max relative-L2 vs DUCC for full synthesis was `1.72342e-4`, but the timing harness itself has **no T720 numerical tolerance gate**. Existing f32 Rust integration tests passed (11/1 ignored). Preserve their numerical behavior, never increase tolerance to accept a speedup.

The new exact-source, unmodified-code Rust-only q8 Callgrind profile at source `98adf1e6` reports **13,863,893,209 Ir** for one warm transform; evidence in `reports/data/ducc-parity-m6-2026-10-27/f32-scalar-synthesis-profile-q8-2026-10-27/{README.md,profile-summary.json}`. Major source-attributed costs include `src/legendre/scalar.rs::multiply_complex_scaled_f32`, `multiply_component_scaled_with_normal_factor_f32`, and the `src/legendre/scalar_paired/synthesis.rs::synthesize_paired_scalar_order_prepared_f32` callback. `fmaf` and `__powisf2` symbols also appear. **Callgrind source/inlining views can overlap; do not sum them into a claimed percentage of whole-call cost.** No matched DUCC Callgrind for this f32 case yet. The f32 timing benchmark used native Pulp V3 but *default* Rust release target CPU, whereas DUCC used x86-64-v3: run an equivalent-ISA Rust control before attributing differences to an algorithm, without pretending that alone explains 140×.

### Completed architecture milestone — one generic algorithm, native typed SIMD leaves (owner 2026-10-27 12:45 UTC)

The shared normal-range scalar-synthesis recurrence/accumulation kernel and native-f32 SIMD leaf described here are implemented and validated. The design notes below preserve mathematical provenance; they do not reopen that milestone. The scaled-contribution packet leaf is also implemented at `ec5d56ca`; the current bounded task is per-packet transition/suffix attribution as described at the top of this plan.

**Do not treat a one-line `exponent == 0` fast branch as the destination.** The critical architectural goal is to **share the *fast* synthesis algorithm between f32 and f64**, instantiated at compile time through Rust generics/traits/monomorphization, while allowing *small and focused* f32/f64 Pulp SIMD/normalization and range-policy leaves. **Never** compute an entire f32 transform using f64 arrays or widen every coefficient to f64 in the hot path just to reuse code. Keep f64 optimized codegen and performance.

**Confirmed in current source:** `scalar_paired::for_each_y_lm_paired_status_dispatched_typed<T,F>` already shares the mathematical recurrence and ring traversal. But `src/legendre/scalar_paired/synthesis.rs::synthesize_paired_scalar_order_prepared_f32` feeds every degree into a callback, then loops through scalar latitude lanes and calls `scalar::multiply_complex_scaled_f32` and separate normal-factor/exception handling per contribution. The f64 `FusedSynthesisBlock` and `FusedPreparedSynthesisPairBlock` compute recurrence and coefficient accumulation **together inside one register-resident SIMD loop** with type-specific Pulp `f64s` operations and a distinct checked/scaled fallback. The huge f32 whole-transform gap and 13.86B source-attributed Ir make **missing f32 fused accumulation and scaling orchestration** a serious mechanism to test; the exact share of time is **not yet causally demonstrated**. Avoid claiming an ordinary generic recurrence fixes SIMD fusion automatically.

**Design direction:**
1. **Share the owner/control flow**, not just small helper methods: one paired normal-range preparation and safety decision, one fused degree × latitude/block traversal, one transactional fallback contract and scratch/shape policy. Use compile-time generic `T` and a **narrow precision-policy trait** for typed SIMD vector conversion/FMA/complex accumulation, safe sectoral seed and exponent handling. Prefer existing `LegendrePrecision` / `PairedScalarRecurrencePrecision` / `ShtScalar` surfaces; add a new trait only if it eliminates genuine algorithm duplication rather than creating a framework.
2. Preserve the **existing f64 optimized `FusedSynthesisBlock` / prepared-pair codegen as the performance oracle**. First add the analogous native-f32 fused normal-range path using Pulp `f32s` lanes (8 lanes on AVX2 vs 4 f64 lanes), with no callback or scalar complex scaling per degree/latitude. Then converge mathematically identical **scheduling, degree loop, accumulation equation, dispatch and fallback** into one compile-time generic implementation **if the f64 and f32 end-to-end benchmarks stay sound**. It is acceptable to retain short optimized typed SIMD leaves for each precision; it is **not** acceptable to copy an entire f64 algorithm into a new f32 module and maintain both indefinitely.
3. The fast path must prove its validity **before mutating outputs**: normal/representable seed and coefficient range, zero or safely bounded scale exponents, finite accumulations, polar/small-sine constraints and trustworthy ring/order masks as applicable. Real f32 has a narrower exponent/dynamic range: do not blindly reuse f64 thresholds or assume every lane is normal. Keep the existing f32 scaled per-degree callback path as the correct **slow exceptional fallback**, and preserve transactional decline/replay, signed zeros, NaNs, infinities, subnormals, adjoints and poisoned workspace behavior. When a batch contains mixed normal and scaled lanes, explore masked/compacted SIMD vs fallback **only after measuring the distribution**; never silently drop exceptional work.
4. Instrument normal-lane / zero-exponent / fallback frequency, actual productive SIMD lanes and per-degree callback count **without timing instrumentation contamination**. Profile the emitted f32 AVX2/FMA in the new fused inner loop: no per-contribution `__powisf2`, scalar `fmaf` helper chains, repeated exponent splitting, or callback dispatch on ordinary safe lanes. Confirm matched q1/q8 whole-call gains, not just fewer source lines or lower Callgrind Ir. The old f64 fast kernel is a *reference design*, not permission to replace proven f64 optimizations with slower generic code.
5. **Stage implementation**, not another sweeping refactor: (A) evidence/branch eligibility; (B) one bounded fused f32 safe-lane kernel with fallback + rigorous tests; (C) accurate whole-transform parent/candidate/DUCC measurement and protected f64 controls; (D) consolidate f32/f64 fast algorithm source where it measurably preserves f64 speed and removes duplicative control-flow; (E) extend this pattern to f32 scalar analysis and spin-1 only after the synthesis win is demonstrated. One owner of a mathematical algorithm is the maintainability criterion; a common file containing two whole algorithms is not sufficient.
6. **Never reintroduce retired `scalar_f32.rs` or `spin_f32.rs`**; keep the test layout `<module>/tests.rs` and avoid destabilizing public Rust/Python APIs. Small precision-specific SIMD leaves are intentional. No extra framework/runtime dependency and no Sol agent or reviewer.
7. The 2× f32 whole-call speed milestone is a **minimum intermediate diagnostic**, **not** a reason to stop if the f32/DUCC ratio remains tens of times slower. Target a transformative reduction from 139.6× for GL T720 scalar synthesis q8, then work toward `<=1.00×` across required cells. Maintain a clear per-cell gap ledger.

### Historical first experiment — exponent-zero path completed; do not repeat

The exponent-distribution instrumentation and the narrow exponent-zero fast path described in this historical screen are complete. The current scaled-arithmetic task is the separate packet-level measurement described at the top of this plan.

1. Fetch `origin/dev`, inspect uncommitted/parked worktrees and `/tmp` ownership, preserve source/evidence, confirm no CPU-heavy job. Read *only* the f32 profile and its latest source functions, the M6 current-head rebaseline, `reports/PERFORMANCE_CANDIDATES.md` relevant f32 entries, and experiment hygiene. Do not perform a broad plan/history review.
2. In **isolated instrumentation only**, count calls into `multiply_complex_scaled_f32` within the prepared paired f32 synthesis callback, grouped by exponent **0 vs nonzero**, normal/zero/subnormal/nonfinite coefficient and basis, fast normal-product success vs exceptional fallback, and occurrence by m/latitude block if useful. Check whether `__powisf2`/`fmaf` originates in an actual dominant path; inspect optimized assembly/Callgrind executed PCs for these functions and their caller. Explain the cost **per contribution**, not merely the inclusive Rayon symbol. Do not commit general-purpose profiling counters to production.
3. **Conditional on evidence:** if exponent 0 is common and the multiply is safe with unchanged numerical semantics, implement a *minimal* `exponent == 0` fast case using the exact native f32 multiplication behavior and preserving signed zeros, NaNs, infinities, subnormals, overflow/underflow, and fallback behavior. **Avoid bypassing an existing guard just because an input is finite**. Preserve the old path whenever proof fails; if using any additional normal-range test, measure whether its branch/scan cost defeats the gain. No `unsafe` without demonstrated need and invariant tests. Do not add dynamically allocated work per coefficient.
4. If zero-exponent frequency is low, or that candidate gives negligible/mixed results, **do not repeat variations of the same tiny branch**. Profile/prevent repeated per-lane scaled-conversion and `powi`/compiler runtime calls, including whether per-ring exponent normalization can be amortized or deferred *without* changing exact/scaled semantics. Compare to DUCC's actual arithmetic and operation counts; maintain the existing shared typed f32/f64 mathematical owner. A successful redesign may require a batched/vector f32 contribution+accumulation kernel, but only if profile and tests show it is warranted. Do not copy f64 algorithms wholesale or recreate `scalar_f32.rs`.
5. Match exact production parent/candidate/DUCC **whole transforms** at f32 GL T720 scalar synthesis q1/q8, both full and reduced `mmax`; include f32 scalar analysis and spin-1 synthesis as nearby route controls, and important protected f64 synthesis/analysis cells if common code changes. **Validate matching C++/Rust ISA**, separately retain the project default build as a production-relevant measurement. Three interleaved fresh-process comparisons for triage, then 15+ for acceptance of material wins where time/noise warrants. Record throughput, paired ratios, source/compiler/ISA hashes, CPU wait/frequency, exact output differences from parent, DUCC rel-L2/max abs, and independent f32 reference/adjoint/polar/underflow/nonfinite/workspace tests. Avoid claiming an unchanged baseline if the codegen flags changed.
6. **Adopt only measured correct whole-transform wins** without repeatable protected-cell regressions. Commit/push one coherent source+test improvement to `dev` when validated, then the compact evidence checkpoint; cleanup exact experiment-owned `CARGO_HOME`, `CARGO_TARGET_DIR`, DUCC builds, worktrees, profile files and venv only *after* durable SHA-verified evidence preservation. If rejected, preserve a recoverable candidate patch and numerical/timing reason and advance. **No Sol of any kind.**
7. Continue through the highest-cost native-f32 operations: scalar synthesis q1/reduced, scalar analysis, then spin-1 analysis/synthesis, with shared numerical ownership and protected f64 routing. **Target <=1.00× DUCC per supported cell** eventually; prioritize first **at least 2× whole-transform f32 speedup on a meaningful measured cell without numerical regression** as an *intermediate breakthrough*, **not goal completion**. If a sound candidate falls below that milestone, retain it if measurable and continue. Once f32 bottleneck is materially improved or blocked by quantified evidence, resume f64 GL T720 analysis and CC reduced synthesis parity work; do not leave them permanently deferred.

### Quota/time, scope, cleanup, honest completion

A result with no production performance change after many f64 micro-experiments is **research progress**, not goal completion. Do not declare M6 “achieved” until per-cell DUCC parity acceptance is actually substantiated. At resource limits, push verified source/evidence, record the exact next source function/hypothesis and remaining ratios, delete only confirmed-owned inactive transient builds, and stop—**never spawn a Sol reviewer, Sol agent, Sol fallback or any other model**. Main Luna Max is the sole production writer; optional Luna-only read-only benchmark worker remains constrained. Never force push or touch `main`. Repo/doc/code cleanup remains an eventual *separate* M6 phase, not the excuse to avoid f32 improvements.

## Goal and measurable performance target

Make RustSHT **as fast as or faster than pinned native DUCC 0.42.0**, SHA `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, on the supported **equivalent warm whole-transform** representative workload matrix, while preserving correctness, adjoints, exceptional-value safety, tested performance wins and a clean maintainable Rust repo.

- **Immediate M6 target:** At source `ec5d56ca`, the checked scaled-contribution packet leaf improves three T720 f32 scalar-synthesis cells, while reduced q8 is inconclusive and all cells remain 25.11×–45.32× DUCC. Exact-parent route counts show millions of nonzero-exponent contributions outside exponent-only-safe packets. Measure each active packet's first normal-range degree and the remaining useful suffix before screening a bounded f32-synthesis-only handoff. Keep unsafe, polar and tail work on the existing exact fallback. After material f32 synthesis improvement, return to the **f64 GL T720 spin-1 analysis, full mmax, q1/q8** gaps; older figures around 1.23×/1.25× are historical, not latest-source certification. The target remains `Rust/DUCC <= 1.00` separately per cell.
- **Next measured gaps:** f64 CC T720 reduced-mmax synthesis q1 (~1.47× DUCC, preliminary), GL T320 reduced-mmax oversampled synthesis q8 (~1.25×), GL T511 reduced q8 (~1.016×); then fill scalar and spin-1 GL/CC/F1 analysis, synthesis and true adjoints across representative T255/511/720/1023, full/reduced, exact/oversampled latitude, and q1/q2/q4/q8 as relevant.
- **Native f32 is NOT exempt:** f32 GL T720 scalar synthesis is remeasured at source `ec5d56ca`, with 36.37×/40.07× full q1/q8 and 25.11×/45.32× reduced q1/q8. The scaled recurrence-to-suffix transition remains the measured priority. Continue to the other f32 analysis/spin-1 families after a material synthesis gain; preserve shared mathematical ownership and all protected f64 paths.
- A representative cell is **PASS** only with numerical comparability and no supported repeatable slowdown relative to DUCC (`Rust/DUCC <= 1.00` under credible paired uncertainty). **GAP, INCONCLUSIVE, UNMEASURED, UNSUPPORTED/NOT-COMPARABLE** must be distinct. Do not average a slow cell into another cell's fast result or claim “parity” from reduced instruction count alone. A borderline ratio must not be called PASS when uncertainty remains.
- **Protect established f64 wins**: GL T1023 spin-1 synthesis q1/q8 ~0.73× DUCC; important GL over-resolved spin-1 synthesis and CC full/reduced q8 synthesis fast paths. A new improvement must not materially degrade another important cell. Preserving f64 paths is more important than extra code uniformity.
- Follow `plans/ducc-performance-parity-continuation-2026-10-27.md` for the larger f64/f32 matrix and `PERFORMANCE_STATUS.md` for dated source-specific results; this M6 plan governs **execution order, profiling depth and repository/documentation cleanup**, superseding completion language from the 2026-10-26 refactor plans.

## Unconditional agent and hygiene constraints

1. **ABSOLUTE NO SOL.** No Sol model, Sol High, Sol reviewer, review agent configured as Sol, tool delegation to Sol, or Sol quota fallback, under any circumstances. Earlier optional/single-use permissions are revoked. Main **Luna Max** alone writes production on `dev`. One previously approved **Luna-only `benchmark`** subagent may do bounded, read-only-to-production measurements without delegation if resources permit; otherwise benchmark sequentially in main Luna. No other subagents. If Luna quota expires, checkpoint/push verified work, save provenance, remove owned transient scratch and stop; never switch to Sol.
2. **Always cleanup first:** reconcile `git status`, `git worktree list --porcelain`, active jobs, unpushed commits and `origin/dev`; check `df -h /tmp`, `df -i /tmp` and experiment scratch. Only delete **confirmed owned, inactive, evidence-saved** build targets, test venvs, DUCC objects/binaries, Callgrind/perf traces, transient worktrees and caches. Do not delete other workers' files, parked branches, shared package caches/toolchains or unique raw evidence. The last agent reports cleanup and a clean worktree at `b7bb2ac4`; still verify on this host, do not repeat blind cleanup.
3. Each experiment gets a uniquely identified bounded scratch root (prefer `/tmp` only with space/inode headroom); use isolated `CARGO_TARGET_DIR`, local build/venv/cache/profiler files and comparison source worktrees. **After** verified committed compact reports/CSV/JSON/source SHA, immediately delete **ALL** experiment-owned compiled artifacts and worktrees for that individual run; prove absence and reclaimed size. Don't stockpile compiled baselines across trials. Coordinate CPU exclusivity; host scheduler contention makes apparent 1% wins unreliable.
4. Fetch/reconcile before every production push. No force push, history rewrites, hard reset, overwrite of concurrent edits, `main` changes, silent removal of tracked experiments, or deleting unverified data. Commit tested meaningful source checkpoints and bounded evidence frequently; avoid docs-only commits per tiny experiment.
5. Preserve GPL/DUCC adapted-code provenance and correct SPDX/source-revision/license notices through any split/cleanup; do not conceal mathematical API differences, numerical errors or unsupported routes.
6. **No new runtime dependency or large framework rewrite** without a measured compelling cause. Avoid sweeping FFT reimplementation, f32/f64 re-duplication, new `*_f32.rs` owners, cosmetic file splitting, redoing the `module/tests.rs` migration, or enabling unsupported f32 paired-spin via capability flags alone. Linux/x86 AVX2 is the performance host, but Rust portability and CI on other targets must remain intact.

## Phase A — exact current-head performance triage

**Complete this before tuning source.** Reuse checked `scripts/benchmark_ducc_spin1_native.py`, `scripts/benchmark_ducc_full_transform_parity.py`, and `scripts/benchmark_ducc_f32_native.py`; inspect each script's real CLI options and build boundary first, **do not invent flags**. Use checked scripts/examples for identical operations/shape/precision/normalization/phase/output lengths. Confirm pinned DUCC Git hash, compiler versions, release flags, `target-cpu=x86-64-v3`/AVX2+FMA, Pulp selected backend, thread counts, fixture seed/hashes, warm-call boundary, process startup exclusion, and CPU affinity/threads. The historical f64 scalar-analysis output-length mismatch requires explicit protocol correction if seen; **never time two different mathematical problems and compare them as parity**.

Run limited 3-pair interleaved current `dev` vs DUCC q1 and q8 controls for GL T720 spin-1 analysis, GL T1023 spin-1 synthesis, CC T720 reduced synthesis q1 and one GL reduced oversampled q8. Add current f32 GL T720 scalar/spin-1 analysis/synthesis q1/q8 with same native f32 DUCC reference. For timings report medians, **paired ratios**, min/max or CI, source/binary hashes, host wait/frequency distributions, finite/repeat/poisoned-reset status and accuracy vs DUCC; do not fabricate unsupported checks. Record questionable/high-wait samples without silently discarding them.

Create/update **one compact, current-head per-cell ledger** `reports/data/ducc-parity-m6-2026-10-27.csv` (or JSON with a clear schema) plus `reports/DUCC_PARITY_M6.md`, with a unique cell key (precision, grid, spin, operator, T/mmax, latitude/longitude shape, threads, warm/cold) and status **PASS/GAP/INCONCLUSIVE/UNMEASURED/UNSUPPORTED**. Distinguish historical rows from fresh measured rows. This is the persistent scorecard for M6; don't build five overlapping dashboards or claim all cells have been measured after a small triage.

## Phase B — cause-based profiling, Callgrind, **actual disassembly**

**No speculative optimization before identifying the current bottleneck.** Start with reconfirmed f64 GL T720 spin-1 analysis q1, also examine q8's dominant route.

1. **Trace real dispatch:** Follow `src/transform/spin/analysis.rs` (`analyze_spin_order_group`, `analyze_spin_scalar_reflected_order`, `try_fused_spin_scalar_analysis_pair_block`) into `src/legendre/scalar_paired/analysis.rs` and `src/legendre/spin.rs`, including ring masks, recurrence suffixes, checked scaled/polar prefix, Q/U spectrum preparation, scalar-to-spin conversion, factor prep, scratch and FFT staging. Establish actual hot specialization and event counts; don't optimize a function unused in that cell.
2. **Normalize equivalent mathematical work in Rust and DUCC:** Count active order/latitude/degree pair-lane updates, component-channel accumulation, skip/checked fallback/replay, factor initialization and FFT work with identical definitions. q8 counters must be **per-worker/thread-safe**, not historical racing global integers. Use temporary low-overhead counters outside production benchmark builds; report both executed updates and useful updates, and map DUCC's `map2alm_kernel`, `calc_map2alm`, `fill_m2a`, `iter_to_ieee` and `spin0to1` to equivalent Rust stages.
3. **Disjoint profiling:** Run matched **Callgrind** samples of comparable bounded whole-transform workers where available, compiling both optimized Rust and DUCC with suitable debug symbols and identical ISA/fixtures, without extra instrumentation in timed builds. Use `valgrind --tool=callgrind` with `--callgrind-out-file` and, when feasible, `--dump-instr=yes --collect-jumps=yes`; analyze with `callgrind_annotate` or KCachegrind. Check valgrind version/support and bound transform counts so instrumentation does not exhaust memory/time. Report whole-program Ir, **disjoint inclusive/exclusive** stage and function costs, per-equivalent-useful-update Ir, major source lines and actual **executed basic blocks**. Account for nested calls exactly once. The old q1 matched order-stage excess and q8 3.38B-vs-1.79B selected-loop Ir are **leads, not current source proof**.
4. **Compare actual emitted machine code:** Use optimized binary and available `llvm-objdump -Cd --no-show-raw-insn`, `objdump -drwC -S`, `nm -C`, `addr2line`, and (if available/permissioned) `perf record/report/annotate`. Resolve addresses/symbols back to **executed hot PCs**, then show side-by-side Rust/DUCC SIMD inner-loop assembly for the specific dominant monomorphized `WithSimd`/Pulp specialization vs DUCC's template instantiation. Explicitly count relevant `vfmadd`/`vmulpd`, load/store, vector shuffles, spills/reloads, branch/bounds checks, scalar work, masking/scale-repair, data movement and loop-carried dependencies; map each to normalized update counts and whole-worker impact. Do not claim instruction counts are cycles, infer hotspots from code size alone, or use unexecuted disassembly as performance evidence.
5. If `perf` is unavailable because of kernel permissions, **do not block**. Use sampled-call boundaries, Callgrind/annotated source and assembly; record what evidence is missing. If Valgrind itself is unavailable or unusable, inspect optimized assembly and use compiler profiling/another installed profiler with limitations explicit; no disabling system security for profiling.
6. Write a **compact top-three hypothesis ranking**: exact current Rust source/function/address vs pinned DUCC equivalent, matched executed counts, Ir per useful update, stage share of whole call, specific assembly difference, estimated removable fraction and confidence. Choose **one** best-value bounded source experiment; **do not end the task after publishing a diagnostic-only report**. Avoid re-running already rejected unchanged `StepBy` iterators, oversized SIMD accumulators, scratch deferral, indiscriminate input scan fusion, large tile registers, old scalar B33/B34 or f64 pair-scaling changes. If evidence points to no useful gain in those, move on.

## Phase C — implement, benchmark, preserve or revert and repeat

- Each hypothesis is one small **reversible production code change** (e.g. eliminate a measured repeated layout/safety pass, precompute truly reused order geometry, avoid proven redundant lane work, tighten executed SIMD reductions/indexing, fix a confirmed route/schedule cutoff). **Do not pre-decide the mechanism before Phase B attribution.** Measure at whole-transform boundaries, not just helper loops.
- Before accepting: focused Rust accuracy with DUCC and independent mathematical reference, GL/CC/F1 supported shapes, full/reduced, poles/equator and exact/off-grid, spin-0/1 (higher spins where affected), E/B signs, S/A/S*/A* adjoint identities, under/overflow/nonfinite/subnormal, normal/checked fallback and poisoned/reused workspaces. Do not weaken tolerances or delete tests to pass. Preserve f64 fast paths and f32 native precision; no opaque runtime dispatch allocations in degree/SIMD loops.
- Match **exact parent → candidate → pinned DUCC** compiled in isolated scratch, with randomized/interleaved process order, stable affinity and diagnostics. First 3 triplets for rejection/triage; 15+ pairs or uncertainty estimates for acceptance of important cells. Test q1 and q8, plus adjacent T/mmax/CC/GL/F1 cells that exercise changed code. Reject/restrict a repeatable slowdown in a protected winning cell even if the headline T720 case improves. A reproducible whole-call Ir reduction with time-inconclusive outcome is a recoverable `PERFORMANCE_CANDIDATES.md` idea, **not a PASS nor grounds for a complicated new production route**.
- Make accepted code + tests a coherent commit, then compact evidence and cleanup; push to `dev` with latest-remote reconciliation. Keep exact source SHA, parent, DUCC, fixtures, raw per-pair medians, CI/wait/frequency caveats, correctness, source/assembly attribution, and decision. Revert only the experiment candidate on failure; do not force-reset unrelated or concurrent commits. Update M6 ledger and ranking to the next gap and **continue**.
- After the T720 q1 and q8 parity target is really verified (or progress blocked with a quantified open gap), proceed to the CC/reduced cases, f32 native paths and representative matrix until remaining gaps are closed. No “we ran three benchmarks” completion criterion.

## Phase D — targeted code/repository cleanup (without losing performance)

**Cleanup is required, but is subordinate to demonstrated performance work and must not become an escape hatch.** After a coherent optimization milestone (and at final handoff), make a separate no-algorithm-change maintenance pass:

- **Rust source:** find stale scaffolding/unused bench hooks or cfg/profiling counters, obsolete precision-module references, redundant safe wrappers, dead test adapters and duplicated dispatch paths **using concrete call sites, features and `cargo check`/Clippy evidence**. Remove only confirmed unused production code. Preserve useful test hooks and approved vector specializations. Do not flatten logical modules or recreate f32/f64 sibling algorithms merely to reduce LOC. Small repairs need an explicit reason; no broad automatic refactor.
- **Build/Cargo/scripts:** inspect `Cargo.toml` features/dependencies and benchmark scripts for clearly obsolete references and target paths; keep stable public APIs and lockfile unless a validated dependency change requires it. Repair broken paths/CLI/tests from earlier moves; do not delete still-used benchmark sources, archived provenances or integration fixtures. No blanket stale-cache deletion without local ownership checks.
- **Repository/docs:** `AGENTS.md`, `plans/README.md`, `PERFORMANCE_STATUS.md`, `reports/PROGRESS.md`, `README.md` and `reports/README.md` should present **one** authoritative current goal, complete/pending labels, working bench commands, reproducer provenance, clear operator/grid/precision scope, and links to M6 ledger and useful historic reports. Mark stale architectural plans **completed/historical**, and eliminate conflicting “active next step” wording from front pages; do not delete Git history or irreproducible evidence. Move bulky historical discussions to existing archive only when links, checksums and scripts remain intact. Keep the rolling progress to five meaningful entries rather than accumulating new speculative planning documents.
- **Writing quality:** use an available **technical-writing/documentation skill** for README and durable user-facing reports *if installed and permitted*; otherwise edit carefully without stopping. Concise scientific prose: define `q1/q8`, units, sampling, correctness tolerances, source and DUCC revisions; distinguish measured, inferred, and unmeasured. Avoid claims of parity from source-local or one-case wins, avoid repeated massive progress prose and unsupported publication language. Documentation skill is an **editorial aid**, never a numerical reviewer and **never Sol**.
- **Validation:** unit-test tree convention remains `<module>/tests.rs`; root `tests/` remains integration tests. Test count, ignored markers, feature gating and assertions must be preserved. Run check/Clippy/fmt and focused tests for cleanup; **no expensive benchmark for a proved non-production no-op**. If the source move affects optimized codegen or runtime dispatch, verify it is truly no-op or do a focused regression screen.
- **Final release gate** requires Rust workspace all-features and no-default-features checks, target-compatible CI, relevant Python binding tests if numerical APIs changed, accuracy/adjoint/extreme checks, representative f64/f32 current-head performance and documented remaining gaps. `main` stays untouched pending owner approval.

## Scope management, persistence and honest stopping rule

- Do not run extra Sol reviewer, do not exhaust quota re-auditing 170 archived plans, and do not run every Cartesian matrix cell before taking a promising targeted optimization. Use one main Luna implementation session, optional constrained Luna benchmark only, and no ongoing process that relies on unscheduled background work.
- Work in **measured cycles**: baseline → causal profile/ASM → one controlled patch → correctness → whole-transform q1/q8 A/B → decision → durable evidence → scratch cleanup → next highest gap. Commit/push meaningful verified progress on `dev` so a quota break is recoverable.
- Do not mark M6 “achieved” until **at least** f64 GL T720 spin-1 analysis q1 AND q8 have current-source validated DUCC PASS, and a representative f32 comparison ledger and subsequent important f64/CC/reduced gaps are addressed; the larger per-cell parity goal remains open until every agreed equivalent supported representative cell is PASS. **A partial target is explicitly “IN PROGRESS”, even after an excellent profiling report or 317 passing tests.** If progress is impossible with available time/tools/quotas, checkpoint verified work and a precise NEXT (hot PC/loop, evidence files, exact current gap, next tested hypothesis), then stop **without changing to Sol or claiming parity**.
