# RETAINED historical owner plan — f64 CC algorithms from GL and DUCC; then spin-1

**Status:** SUPERSEDED AS EXECUTION QUEUE on 2026-10-23 by [performance-first scalar CC/F1 implementation](f64-scalar-cc-f1-ducc-parity-implementation-2026-10-23.md). Preserve its existing benchmark specifications/reviews as history, but **do not block production kernel improvements on sweep recovery, seven figures, repository cleanup or GL table policy**. **Historical status:** ACTIVE, owner update 2026-10-22. **Branch:** `opt/sht-performance-parity`. **Implementer:** Luna Max. This plan supersedes the execution order in [the prior scalar CC/full-band and GL crossover directive](scalar-f64-cc-full-band-and-gl-table-crossover-2026-10-22.md), while retaining its exact grid, degree coverage and numerical contract. [AGENTS.md](../AGENTS.md) and later explicit owner instructions take precedence.

**Goal:** Reuse RustSHT's already fast f64 GL scalar algorithms for mathematically compatible CC work, using pinned DUCC 0.42.0 as a source-level algorithm and correctness reference. Finish trustworthy native f64 scalar CC evidence first; optimize CC synthesis and analysis separately; then resolve the scalar GL precomputation policy; then investigate the remaining f64 spin-1 gaps. **Do not resume f32 until the important preceding f64 scalar and spin-1 workloads are consistently near DUCC parity or faster.**

## Immediate first actions and priority queue

1. **Finish and validate the unfinished f64 scalar CC native benchmark campaign**, including missing results, independent numerical acceptance/coverage, final summaries and the requested seven SVG+PNG figure pairs.
2. **After benchmark closeout, inventory and clean non-checked-in repository-local files:** commit required source/tests/data/reports/figures; relocate potentially useful temporary large artifacts outside the repository with recorded provenance; delete only verified disposable logs/caches/build outputs/duplicates; preserve ambiguous or concurrent work. Commit cleanup separately.
3. **f64 scalar CC synthesis:** A/B scheduling grain, then adapt proven GL ordered/reflected scalar synthesis to CC, verifying DUCC's actual recurrence and SIMD dataflow.
4. **f64 scalar CC analysis:** reuse the proven bounded parallel CC/F1 spin-1 meridional resampler in the scalar path, then profile and improve recurrence if still dominant.
5. **f64 scalar GL analysis table-versus-recurrence policy:** reconsider 20 MiB using actual table SIMD dispatch, cold-plan and reuse evidence.
6. **f64 spin-1 GL/CC/F1:** investigate DUCC's reflected-pair scalar conversion and scaled recurrence versus Rust's checked fallback; promote only demonstrated improvements.
7. **f32 Plan 17 remains PAUSED.** If substantial systematic f64 gaps remain, continue f64 rather than starting f32.

The CC campaign's completion is an explicit immediate task. Do not use an optional stage-profile campaign to delay the handoff to implementation after the required benchmarks/reports are closed.

Before work: inspect branch/status/untracked files, running measurement processes and artifacts; fetch origin; preserve frozen binaries and concurrent work; read `AGENTS.md`, `plans/README.md`, `PERFORMANCE_STATUS.md`, both scalar CC plans, `INDEPENDENT_REVIEW_STEERING.md`, [Sol's f64/DUCC review](../reports/f64-ducc-independent-review-2026-10-22.md), [Astra's repository-state review](../reports/repo-state-review-summary-2026-10-22.md), and Plan 15/16 positive and negative experimental evidence. Reconciling documentation does not constitute performance measurement.

## M0 — Recover and publish the complete f64 scalar CC baseline

### M0.A Independent coverage and frozen-source recovery

At Sol's review checkpoint, **13 of 14** local CC shards had saved result JSON. The independent manifest expects **632 distinct geometries / 2,528 operation×thread groups**. It found **2,456 matching saved**, **72 missing**, and **16 historical extra** groups. Of the missing groups, **24** concern T1–T3 quarter order `mmax=0`, and **48** concern the unsaved final T2049–T3072 shard. Saved measurements span every T0–T256 and higher cases through T2048. These are historical audit counts; **check if later work already recovered any cells** before acting.

Preserve all 13 saved shards and frozen binaries. Verify the actual native binary SHA-256 against the independent-review metadata. Never invent source/build provenance or substitute a later binary. Recover the missing groups on a quiescent host without rerunning the entire T0–T256 campaign. Preserve the 16 extra historical rows, but do not mix them into the principal required CC series.

Fix `scripts/benchmark_ducc_full_transform_parity.py::make_cases`, whose `max(1, lmax//4)` contradicts the owner's principal `mmax=floor(lmax/4)`. Use an **independent expected manifest** that does not invoke the generator being audited; preserve the legacy GL contract and T0 deduplication. The supported recovery argument is `--mmax 0`, **not** the incorrect `--fixed-mmax 0` from the old handoff.

Keep every integer `lmax=0..256` and sampled high T in the prior directive through T3072; both full `mmax=lmax` and true quarter `mmax=floor(lmax/4)`; analysis/synthesis; q1/q8; even-smooth longitude at every case and historical representative odd-minimum cases. For exact CC use `ntheta=lmax+2` and both poles, appropriate weights and phase `phi0=0.23`. Preserve actual `nphi` and grid identifiers in all records. An unavailable/unsupported case must be explicitly labeled, never presented as measured.

### M0.B Fix acceptance, durability and merge provenance before rerunning

Based on Astra's [line-specific review](../reports/repo-state-review-summary-2026-10-22.md) and Sol's [independent audit](../reports/f64-ducc-independent-review-2026-10-22.md):

- Treat **timing coverage**, **numerical acceptance** and **measurement quality** as distinct machine-readable statuses. Record existing tolerances/criteria and enforce them against each required random, analytic/basis, roundtrip and adjoint record; do not accept arbitrary finite errors. For ill-conditioned adjoint dots, report normwise residual and relative-to-dot with cancellation context without loosening tolerance.
- Check **every** reset flag (including synthesis and later process pairs), exact-zero outputs, edge cases, expected validation count and finite fields. Nonfinite secondary metrics must produce serializable failed records (no `allow_nan=False` crash that loses an entire shard); independently persist failures.
- Persist completed configurations/process pairs atomically and resume by identity, not by an empty run or in-memory only results. Frozen build/worker hashes, DUCC revision, geometry, protocol, process affinity and benchmark flags must remain compatible. Track **harness checkout revision** separately from **true source/build/binary identity**; `--skip-build` must not falsely claim that the checked-out source built the binary. Do not relabel a recovered cell with an older source SHA or silently weaken all `merge_shards` invariants.
- Publish actual batch counts for Rust and DUCC separately, with per-process mode and timing distributions. Do not zip unequal asynchronous batches into putative paired observations; use properly matched process-level summary comparisons. Thirty batches across three processes are not thirty independent process runs. Keep host contention visible; do not silently omit quality failures.
- Validate the revised harness/merger with saved-record fixtures (nonfinite failure serialization, interrupted recovery, mismatched hashes, too few batches, missing numerics). Retain already valid measurements, do not rerun all older cells merely because metadata/report code changed.
- Preserve historical primary-source data and errors. Continue without inventing measurement results if hardware, source checkouts or permission prevents rerun.

### M0.C Final CC figures/report

Use principal reduced `mmax==lmax//4`, not arbitrary `mmax!=lmax`. Handle T0 appropriately. Preserve extra low-degree m=1 records as labeled supplemental controls. In CC/GL comparison, join on exact `lmax,mmax,nphi,operation,threads`; verify distinct CC/GL `ntheta` and detect duplicate/mismatched keys. Unaccepted or partial cells must be distinguishable in plots and summaries.

Publish [`reports/ducc-scalar-f64-cc-full-sweep-2026-10-22.md`](../reports/ducc-scalar-f64-cc-full-sweep-2026-10-22.md) and data under `reports/data/ducc-scalar-f64-cc-full-sweep-2026-10-22/`, including manifest, independent audit, sanitized result CSV/JSON, validation outcomes, actual counts, missing/extra/failed records, build provenance and reproducing commands. Generate **four CC ratio charts** (analysis/synthesis × full/quarter), **two absolute CC latency charts**, and **one CC-vs-GL matched chart**. Produce **both SVG and PNG** for all seven; no smoothing, omitted outliers or unmarked incomplete cells. Avoid adding a second copy of every throwaway per-shard SVG.

Close M0 only with reported independent accepted coverage or explicit incomplete/unsupported cells. Existing saved numerical controls passed, but "zero recorded failures" from the old harness alone is not sufficient to claim the entire sweep accepted.

### M0.D Clean repository-local non-checked-in artifacts, after report closeout

Inventory **all** repository-local untracked/non-checked-in files and active processes; establish ownership before touching them.

| Class | Action |
|---|---|
| Required source, tests, useful benchmark samples, reproducibility metadata, figures and reports | Commit normally |
| Potentially useful temporary raw data/binaries or oversized instrumented files | Relocate outside repository; document absolute location, hashes and source provenance |
| Proven disposable build products, logs, caches, duplicate exports, failed intermediates | Delete only after validation |
| Ambiguous files, frozen experimental evidence or concurrently edited work | Preserve, do not silently delete |

Do not erase or rewrite historical evidence. Do not remove frozen binaries until retention/ownership is explicit. Commit/report cleanup **separately** from benchmark harness and production changes, with an inventory of committed/moved/deleted items. Do not make an unverified "clean" claim.

## M1 — f64 scalar CC synthesis: proven GL ordered algorithm first

**The measured opportunity is large**, unlike f32 historical results. Saved full-order f64 CC synthesis (Rust/DUCC):
- T255 q8 `3.128/0.299 ms` = `10.45×`.
- T720 q1 `90.315/20.435 ms` = `4.42×`.
- T720 q8 `47.916/2.951 ms` = `16.24×`.
- T1023 q8 `136.614/8.156 ms` = `16.75×`.
- T2048 q8 `1414.172/65.689 ms` = `21.53×`.

By contrast T720 GL q8 synthesis is approximately `3.397/3.070 ms`. These saved measurements have known host/protocol qualifications; do not treat them as a new optimized-source A/B.

### M1.A A/B task grain separately

`src/transform.rs::synthesize_scalar` has `with_min_len(parallel_groups.div_ceil(execution_threads))` in the general CC ring-group route. Review counter evidence of **four actual workers at q8** despite twelve T720 CC groups. Test one isolated finer-grain candidate, bounded scratch, same actual worker pool; instrument occupancy and wait time. Measure T255, T511, T720, T1023, full/quarter, q1/q8 with both backends and host controls. Do not turn one case into an unconditional scheduler rule. A successful scheduling change is a standalone commit; it cannot explain the existing q1 synthesis gap.

### M1.B Adapt existing ordered GL scalar synthesis to CC

The current ordered scalar engine is gated by `self.grid.kind()==GridKind::GaussLegendre` in `src/transform.rs`; CC takes the general ring-group visitor. Study the existing `src/transform.rs`, `src/legendre/scalar.rs`, `src/legendre/scalar_paired.rs`, corresponding prepared/reflected recurrence and Pulp SIMD paths. Compare with pinned DUCC `src/ducc0/sht/sht.cc` and `sht_inner_loop.h`: `alm2leg`, `inner_loop_a2m`, `fill_a2m`, `calc_alm2map`, `alm2map_kernel`, `extract_a2m`. Write a source-level GL/CC/DUCC dataflow crosswalk **before implementing**.

Make the **smallest grid-aware adapter** that reuses Rust's successful ordered scalar engine for CC. Separate grid-independent paired harmonic recurrence from grid-specific theta/ring geometry and pole handling; **do not simply remove the GL guard**, clone a second giant engine or allocate unnecessary transform-sized buffers.

Prove exact CC geometry (`ntheta=lmax+2`, two poles), north/south reflection and parity, equator singleton, odd/even theta count, SIMD tails, m=0, full/quarter order, complex coefficients and nonzero `phi0`, scaled recurrence and safe fallback. Use checked eligibility; unsupported shapes continue through the old verified visitor. On late decline, discard partial order output before fallback. Preserve workspace poisoning/reset and all finite/Inf/NaN classifications required by the existing safety contract.

### M1.C Source/assembly analysis and isolated acceptance

Inspect **executed** native AVX2/FMA for recurrence streams, coefficient broadcast/reuse, north/south pairing, SIMD occupancy and tails, register pressure/spills, memory traffic and scaled/ordinary seed transitions. DUCC is an algorithmic/normalization reference; **do not weaken RustSHT safety** to copy DUCC's approximate `sht_ftol` behavior. An instruction count is a hypothesis, not speed proof. Previous packed/direct-accumulation, loop reversal, compaction and scalar-analysis 64/128 tile attempts remain rejected absent a new distinct mechanism.

Use matched original-Rust/candidate-Rust/DUCC end-to-end A/B at T63/127/255/511/720/1023/2048 and selected high degrees, full/quarter order, q1/q8, relevant odd/even `nphi`, cold and warm, normal/scale/fallback, with GL controls. Report absolute medians, p10/p90, both candidate/baseline and candidate/DUCC ratios; promote only reproducible whole-transform gains with unchanged numerical criteria and no material GL regression. Keep changes small and attributable.

## M2 — f64 scalar CC analysis: reuse proven bounded parallel resampler

Scalar CC `src/transform.rs` currently calls serial `resampling.to_prepared`. Rust's accepted CC/F1 **spin-1** analysis already has bounded order-parallel resampling in `src/meridional.rs`, analogous to DUCC `resample_to_prepared_CC`. Reuse that implementation rather than writing a new FFT/resampling engine. T720 full q8 scalar analysis measured Rust/DUCC `34.557/12.292 ms`; diagnostic Rust `~26.5 ms` resampling/latitude setup and `~4.4 ms` recurrence (stage boundaries differ from DUCC and are not exact comparable kernel ratios).

Implement a separately benchmarked scalar CC bounded worker-buffer route preserving prepared theta/weights, north/south reflection, pole handling, longitude normalization, meridional complex phase, full/quarter mmax, adjoints, workspace reuse and exact zero/reset. Determine eligibility from controlled small/large cases, not arbitrary bandlimits. Do not regress spin-1 resampling. After the q8 resampling floor is addressed, reprofile CC scalar recurrence against proven GL reflected-pair analysis and DUCC prepared-grid analysis; look for scaled visitors, seed work, SIMD/fusion and memory traffic. Keep production scalar-analysis tile **256 reflected pairs**; do not reopen rejected 64/128 tiles or blindly enable CC prefix advancement without numerical proof.

## M3 — f64 GL compact-table versus recurrence crossover

The scalar GL q1 analysis **20 MiB** table cutoff is a resource heuristic, not an established optimum. `src/legendre/scalar_table.rs::F64TableAnalysisGroup::with_simd` runs the two-complex-lane SIMD accumulator only when `ntheta % 4==0` and adequate scratch exists; neighbors can execute a **scalar** accumulator. This explains much of T127/T159/T191 nonmonotonicity. Keep the old 20 MiB policy as baseline until evidence warrants change.

Reuse exhaustive forced-route evidence. Add focused current-source independent repeats at T104, T126–128, T159, T191–192, T215–220 including T217/218, full and production-eligible near-full `mmax >= lmax-lmax/4`, both longitude shapes, with selected q8/synthesis controls. Stratify by the actual vector/scalar dispatch. Measure **independent cold plans** (including a recurrence-only plan with no resident table), table build/first-use/warmed times, resident memory and total amortized cost at reuse 1/2/4/8/16/32/64+. No arbitrary new byte cap, degree exception, platform-cache guess or CC compact-table support. Retaining the current limit is valid if no robust better rule emerges.

## M4 — f64 spin-1 STANDARD synthesis and analysis (only after scalar work)

Preserve accepted Plans 15/16 and the already performant spin-1 CC/F1 resampler. Sol's source review confirms DUCC's `lmax>=500` STANDARD spin-1 path converts ordinary rings through `spin1to0`, processes **reflected north/south pairs** with scalar SIMD and independent parity accumulators, transitions scaled recurrence to ordinary, and reserves direct spin for near poles. Rust already converts to scalar rows and has an order-major queue, but large CC work handles hemispheres separately and incurs checked fallback work.

**M4.A Measure work first:** instrument weighted lane×degree updates, successful ordinary/scale transitions, early/late transition decline, repeated work, final scalar visitor, and actual direct-spin polar work. Counts and worker sums are not end-to-end time fractions. Examine T499/T500/T511 transition, T720/T1023 full/quarter q1/q8 and GL/CC/F1 controls. No transform-wide cache or new FFT project.

**M4.B New checked mechanism:** test a bounded adapter to the existing scalar ordered engine over globally active **reflected pairs** for each converted component, preserving E/B algebra, polar originals, parity, scale/late fallback and safe output reset. Distinguish it from rejected all-ring parity separation, loop reversal, coarse hemisphere prototype and coefficient-cache-only experiments. Validate one order with analytic/DUCC maps and assembly before a matched full-transform A/B; reject a slower/unsafe candidate cleanly.

**M4.C Analysis:** DUCC performs two reflected scalar analyses then `spin0to1`. Rust CC spin-1 analysis currently has all-ring visitor work, while GL has a reflected-pair route. Investigate shared reuse if an updated stage profile confirms recurrence is the residual. The accepted T1023 spin-1 CC analysis resampling `16.2 ms Rust vs 17.1 ms DUCC` is already comparable; do not rewrite it without contrary evidence.

Spin-1 starts after scalar CC synthesis has a validated improved path, scalar analysis bottlenecks are addressed, the scalar benchmark report is published and M3 has a justified disposition. Exact 1.00× parity for each scalar cell is not required to start M4, but remaining material scalar gaps must be explicitly explained and prioritized.

## M5 — f32 gate (strict)

**PAUSED** Plan 17 until representative **f64 scalar and spin-1** GL/CC/F1 (where applicable), analysis/synthesis, full/quarter order, q1/q8, low/intermediate/high degrees and odd/even longitude are broadly near DUCC parity or faster. A practical nonbinding guide is consistent ~10–20% deviation without material systematic outliers; do not hide 2×/3×/10× cells behind geometric means. Document unavoidable residuals and seek owner review; do not automatically resume f32 because of token/session availability. Preserve validated f32 work and port back only after f64 gate.

## Numerical, benchmarking and scope invariants

- Maintain the public API and independent `S`, `A`, `S*` and `A*` semantics. Preserve GL/CC/F1, both spins, other existing spins if shared kernels are touched, f32/f64, poles/equator, phase, weights, odd/even longitude, SIMD tails, subnormal/zero/extreme finite and NaN/Inf classes, adjoints, poisoned workspace, guarded allocation and fallback. No numerical tolerance relaxation or unsafe partial output.
- Pin matched native Rust/DUCC **0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`**, identical operations and input/reset conventions, compiler flags/ISA/threads; q1 to one physical core, q8 to matched cores on one CCD; interleave backend process order. Warm/reuse and cold recorded separately.
- Short operations: calibrated **20–100 ms** batches and >=30 timed batches across independent runs. Long calls >100 ms: individually timed interleaved samples, real sample counts. Show medians/p10/p90/process spread, candidate/base and Rust/DUCC, absolute excess, first/setup costs, numerical acceptance, host contention. Equivalent whole-transform speed is the production decision; stage timings and assembly are attribution evidence.
- Each candidate must isolate **one mechanism**, include independent analytic/DUCC and adjoint controls, held-out neighboring degrees/order/grid cases and GL regression controls. Run `cargo fmt`, strict Clippy, release all-feature and true package-only no-default tests, and relevant Python tests if modified. Never claim speed based on code shape alone.
- Stable Rust, existing Pulp runtime SIMD and bounded caller-owned scratch; licensed DUCC-derived algorithm attribution. No new FFT backend/RealFFT work, paired-odd FFT eligibility changes, batching API, spins 2–4 optimization, cache-discovery project, CC compact table, broad public API rewrite or unrelated Python feature work. Preserve owner-approved `pyspharm-rs` and `spharm_rs`. Add the existing compatibility tests to CI in a small isolated verified change when appropriate.

## Git, checkpoints and completion

Stay on `opt/sht-performance-parity`. Use default Git identity. Inspect worktree/untracked artifacts and active benchmarks before change; **fetch remote and safely reconcile before every push**, rebasing only unpublished local commits when safe. Preserve concurrently authored material; never force-push, hard-reset, touch main, or kill another process. At most **one benchmark-only Luna subagent**, and never compete for the same host cores.

Commit small verified milestones separately (benchmark correctness, recovery, figures, cleanup, CC scheduling, CC ordered synthesis, CC resampling, GL policy, spin-1 algorithm, report). Keep `PERFORMANCE_STATUS.md` and reports current with source hashes, validation, accepted/rejected hypotheses and remaining measured gaps. Record explicit `VERIFIED`, `MEASURED`, `HYPOTHESIS`, `INCOMPLETE`. Do not fabricate results or report successful tests not executed.

**Complete M0 before declaring the CC campaign done.** Afterward proceed with M1–M4 continuously, not f32, until the f64 gaps are near parity or exhausted with evidence. The final handoff must include all benchmark result files and figures, audit completeness, every repository cleanup action, accepted optimizations, failures, exact rerun commands, and next f64 bottleneck.