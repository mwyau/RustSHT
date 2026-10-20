# Active plan — close scalar-analysis SIMD performance gap with DUCC

**Branch:** `opt/sht-performance-parity`  
**Updated:** 2026-10-20. **Reviewed production profile:** `d36b1a63` (status at `cf96d2d`); inspect current remote HEAD before acting.

**Reference:** historical benchmark pin DUCC `80388904b8cfc32400ba84af8a6a45aabfe60f86`; after the current comprehensive report, pin latest fetched `ducc0` branch HEAD (verified `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` on 2026-10-20). Source `src/ducc0/sht/sht_inner_loop.h` (`fill_m2a`, `init_lambda`, `iter_to_ieee`, `calc_map2alm`, `map2alm_kernel`) and surrounding `sht.cc`.

## Current checkpoint — matched flags and T64 selector

The standalone ordinary-kernel comparison now uses the pinned CMake v3 SHT
flags for DUCC's numerical translation unit (`-O3 -ffast-math
-ffp-contract=fast -march=x86-64-v3`) and a separately compiled strict driver.
The actual GCC 15 / Clang 21 commands, matched timings and numerical errors are
recorded in `reports/ducc-upgrade-baseline-2026-10-20.md`; inspect those exact
objects before assigning the remaining compiler-dependent gap. Rust keeps its
strict V3 target without fast-math.

The current-source recheck at `be608eb2` confirmed the same flags in the
standalone commands: the DUCC numerical object uses
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, while the separately
compiled driver uses `-fno-fast-math`. The runner now rejects overrides that
break this split. Actual upstream multiarch `compile_commands.json` entries
for `sht.cc` contain `-O3 -ffast-math -march=x86-64-v3` for the v3 object;
they do not spell out `-ffp-contract=fast`. See the current-source rerun and
the exact command summary in `reports/data/ducc-kernel-flags-rerun-2026-10-20/`.

A separate scalar dispatch experiment rejected a global T64 recurrence
cutoff because it regressed q1 analysis. Production applies the T64 cutoff to
f64 scalar synthesis only; analysis and spin-1 selectors remain unchanged.
The full comparison and final-policy checks are in
`reports/scalar-t64-recurrence-transition-2026-10-20.md`.

## Cumulative output-staging experiment (2026-10-20)

The direct-output accumulator is restored as an opt-in candidate under
`rustsht_scalar_analysis_direct_accumulation`; staged output remains the
production default. The candidate requires a cached checked recurrence proof
for the plan/order/tile and a product bound over the full reflected-latitude
count. It declines before output writes for scaled or unsafe inputs. The
combined tile-256, fixed-scratch, packed-reduction, indexed-loop and
direct-output build passed release library tests with and without default
features, strict all-target Clippy, DUCC/analytic, adjoint, edge-input, SIMD
tail and thread-count checks. T511 direct-versus-staged output relative L2 was
`2.5974e-16`. The earlier direct-only T720 screen regressed, so this combined
candidate needs its own matched performance run before selection. No timing
was collected during current host CPU saturation. The next measured work is
T720 full/reduced `mmax`, q1/q8, with the 64/128/256-lane cumulative variants.

## Non-negotiable objective

**Close the observed roughly 2× RustSHT-versus-DUCC f64 GL scalar-analysis Legendre performance gap through a source- and assembly-grounded implementation.** The existing measurements are *directional*, because stage boundaries differ; first establish a defensible apples-to-apples gap and then attack its largest contributors.

**Do not treat a sequence of 2–5% wins, fewer static instructions, a smaller stack frame, or merely producing another profiling report as completion.** After each validated improvement, remeasure the residual and continue with the highest attributable bottleneck. If parity proves unattainable in the available scope, the final result must isolate the concrete unresolved cause and test a viable next design—not merely say the gap remains.

This is a *goal*, not a guaranteed result or a license to weaken accuracy. Do not impose artificial truncation-specific or 10–20% acceptance thresholds. Completion requires either convincing near-DUCC behavior under equivalent workloads or a thoroughly measured, technically bounded explanation of the residual, including tested remediation attempts.

**Active work is scalar f64 GL STANDARD analysis (A) only.** Leave spin-1/f32/CC/F1 and FFT redesign to a separate goal; preserve their existing correctness and API behavior.

## Owner priority (2026-10-20) — stop optimizing the checked hot loop

**Do not pursue further *performance* rewrites of the per-group checked ordinary recurrence** (range-scan/mask/fused-check variants). It is a correctness and cold-start/fallback mechanism, **not the DUCC-parity target**. Keep the existing checked-first range proof/cache and robust scaled/exceptional fallback; do not remove required validation, weaken numerical behavior, or assume an unsafe unchecked path is valid for arbitrary inputs.

**Optimize the warmed, proven-safe ordinary AVX2/FMA path next**, using pinned DUCC's `map2alm_kernel` as the equivalent-work reference. At current source `be608eb2`, the matched T720 `m=1` warmed Rust/DUCC ratios are 1.172/1.107/1.089 with GCC and 1.161/1.094/1.078 with Clang at 64/128/256 lanes; the checked route remains substantially slower. Attribute the remaining warmed-path excess, starting by isolating SIMD horizontal reductions from recurrence arithmetic; continue with source/assembly-grounded changes, scaled-prefix work when its whole-transform contribution justifies it, and cumulative ablations. Do not restart already-tested checked-loop mask experiments.

Benchmark **warmed and cold** full transforms separately (T720 full/reduced `mmax`, q1/q8) and retain checked-path cost in reports as a safety/startup cost, **not a performance optimization objective**. Validate cache eligibility/invalidation, the first checked call, reused workspaces, changing maps/plans, extreme/subnormal/cancellation cases, SIMD tails, and DUCC/analytic/adjoint correctness. Revisit the checked path only for a demonstrated correctness issue or evidence that cold-start cost materially dominates a relevant real workload. This owner priority supersedes older milestones wherever they emphasize optimizing checked-loop throughput.

**Current cumulative candidates (unmeasured):** scaled sectoral initialization
reuses contiguous ordinary SIMD seed vectors before the first vector that
requires exponent scaling. The scaled two-pair prefix carries its all-negative
epoch state from the SIMD update pass rather than rereading the epoch slice.
Fallbacks, lane range checks and tail scaling remain unchanged. Release and
no-default-feature library tests pass; timing is deferred under host CPU
saturation. Keep both candidates experimental until matched 64/128/256-lane
and representative full/reduced-order q1/q8 comparisons measure their
contribution.

## Next sequence — comprehensive benchmark, toolchain/reference rebaseline, then kernel attribution (2026-10-20)

**Order matters.** First FINISH and PUSH the already-running comprehensive native DUCC-vs-RustSHT analysis **and synthesis** sweep across complete `l=0..lmax`, `m=0..min(l,mmax)` triangles and many bandlimits (not only T511/T720 or m=1). Include full/reduced `mmax`, q1/q8, numerical errors, warmed/cold execution, compiler/CPU details, reproducible commands and machine-readable results. Use the benchmark's original compiler and pinned DUCC revision so the finished results remain interpretable. Commit benchmark harness and report in separately reviewable commits and push **before starting new optimization or version changes**. Do not stall on infeasible highest degrees; record limits and skipped cases.

**Then upgrade and pin reproducibly, in separate commits:**

1. Update Rust to **1.99.1**, released 2026-10-15: change `rust-toolchain.toml`, update other genuine version pins as appropriate, record `rustc -Vv`/LLVM version, and run relevant release/no-default-features/Clippy tests. Keep the preceding 1.99.0 benchmark as an explicitly historical comparison; do not silently merge results from different compiler versions.
2. Fetch `mreineck/ducc` branch **`ducc0`** and pin its then-current full commit SHA, not a floating branch or a PyPI version. At owner review on 2026-10-20 the branch HEAD was **`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`**; if it has advanced, use the newer verified HEAD and record it explicitly. Update native fixture source checksums, benchmark `DUCC_REVISION`, scripts and active documentation consistently, retaining legacy `80388904b8cfc32400ba84af8a6a45aabfe60f86` results with their original pin. Adapt changed interfaces deliberately and validate numerics before timing.
3. Build the pinned DUCC source on Linux x86-64 using **`DUCC0_OPTIMIZATION=multiarch`** (the DUCC CMake option selected via this environment variable; it defines `DUCC0_MULTIARCH` and builds x86-64 **v1/v3/v4** object targets). Record actual compile commands and flags. Benchmark the **dispatched v3 AVX2/FMA implementation** on the Ryzen 5950X; compile and **disassemble v4 AVX-512 only** using `objdump`/`llvm-objdump`. **Never execute or benchmark v4** on the AVX2-only host. Verify v4 symbols belong to the real SHT specialization, not just unrelated objects. If the native standalone benchmark does not inherit the multiarch build, compile its v3 and v4 SHT kernels separately with explicit `-march=x86-64-v3`/`-march=x86-64-v4` and keep their source/flags matched.
4. Re-run representative full-transform and matched kernel benchmarks using **Rust 1.99.1 + the new fixed DUCC commit**. Include the same Rust source under 1.99.0 versus 1.99.1 as a controlled compiler comparison if feasible. Build matched DUCC v3 kernels with **GCC and Clang** to isolate C++ codegen effects from Rust LLVM. Document CMake multiarch `-ffast-math` and native-driver FP flags explicitly; different FP flags are not equivalent-work/codegen evidence without controls. Keep 20–100 ms calibrated batches and process interleaving for microkernels.

**Current warmed-kernel evidence (2026-10-20):** the flags-confirmation rerun at RustSHT `be608eb2` measured current-source T720 GL `m=1` kernel-plus-commit ratios of `1.172/1.107/1.089` with GCC and `1.161/1.094/1.078` with Clang at 64/128/256 lanes. Rust's absolute excess was `0.676–1.422 µs`; relative L2 error was `5.928e-14..1.628e-13`. The earlier `4c6945fc` and `d6382614` kernel measurements use different source/attribution wrappers and remain separate. The current result does not establish whole-transform, multi-order or q8 parity.

- In the corrected same-dispatch component probe at `be608eb2`, recurrence-only work measured `2.695/5.209/10.245 µs`; four-spectrum accumulation plus horizontal reduction/coefficient construction added `2.123/3.649/6.996 µs` at 64/128/256 lanes. This is about 40% of measured ordinary kernel time, but the probe does not separate accumulator chains from horizontal reduction and coefficient conversion. A retained-vector attempt through `std::hint::black_box` added eight YMM stack stores per pair group and gave deltas of inconsistent sign; the timings are invalid and the source patch is archived in the cumulative report data. Replace it with a measured scratch sink or independent reduction microbenchmark. Prior private AVX2 reduction gains were below 1%, and grouped reductions regressed.
- The GCC and Clang DUCC medians in the current matched-flag rerun were within 1% across all lane counts. Run-queue wait was low; three GCC batches slightly exceeded 100 ms and one GCC process spread triggered the stability flag. No samples were discarded. After the bounded reduction attribution, profile a larger-order q8 full transform and separate recurrence, packing, FFT and scheduling. Do not infer q8 or whole-transform parity from this `m=1` q1 kernel.
- **Then test loop form** in `fused_analysis_two_pair_steps_with_simd`: present eight zipped SIMD slices versus a straightforward indexed loop over validated equal-length vectors; compare executed v3 machine code for bounds/iterator overhead, address generation, loads/stores, spills, FMA dependency chains and register pressure. Do not assume iterator syntax is slower or use unsafe indexing without evidence.
- **Compare compiler/ISA output:** Rust 1.99.1 LLVM v3, pinned DUCC GCC v3, pinned DUCC Clang v3; inspect DUCC v4 AVX-512 disassembly strictly as a source/dataflow reference, never as executable timing evidence. Identify useful algorithmic/loop differences without copying v4-specific requirements into the portable runtime path.
- **Implement one justified change at a time**, test numeric and exceptional cases, remeasure the warmed kernel and representative full/reduced-order q1/q8 analysis/synthesis; commit/push each validated change. Retain correct-but-slower experimental results for final cumulative attribution, then clean up unsuccessful production variants. No checked-loop performance rewrites; preserve cold checked proof, range-cache validity and robust scaled fallback.

**Checkpoint requirement:** Commit and push (A) the completed pre-upgrade comprehensive benchmark, (B) Rust/DUCC toolchain and reference pins + multiarch build/disassembly evidence, (C) post-upgrade comparable results, then (D) subsequent performance experiments. Update `PERFORMANCE_STATUS.md` after each milestone; keep branch `opt/sht-performance-parity` and obey the mandatory fetch/rebase/instruction-refresh rule. This sequence takes precedence over stale tasks and measurements below.

## Milestone 0 — Verify the *executed* Pulp backend first

**Urgent prerequisite (2026-10-19):** An agent hypothesized that specifying only `features = ["x86-v4"]` in RustSHT's Pulp 0.22.3 dependency disabled AVX2 on Ryzen 9 5950X. **The published Pulp manifest contradicts that inference:** `x86-v3` is a *default* feature, and RustSHT does not disable dependency defaults. Pulp `Arch::new()` tries V4, then V3, then Scalar. Adding explicit `"x86-v3"` should therefore be functionally redundant. The scalar-fallback diagnosis remains **unverified** until a runtime trace demonstrates it.

1. Inspect `cargo tree -e features -i pulp`, actual active Cargo features and any `default-features = false` unification context. Instrument the **actual production Arch value** used by the benchmark (one-off diagnostic of `pulp::Arch::new()` and the path through `dispatch`); inspect hot executed V3 symbols. Do not infer dispatch from a V4 symbol in `objdump`.
2. In *otherwise identical, optimized* before/after builds, compare explicit feature-list change alone. If both choose V3 and perform alike, reject the feature omission as the root cause and continue the equivalent-work analysis. If dispatch differs, explain **why** in terms of resolved features/compile flags/actual dispatched call.
3. Preserve this as a small, standalone reproducible result in the findings/report. Do not modify or disable numerical fallbacks to force SIMD. Treat the claimed ~2× stage gap as provisional until dispatched path and stage boundaries are both validated.

**Upstream evidence:** [Pulp 0.22.3 default features](https://docs.rs/crate/pulp/0.22.3/features) and [`Arch::new()` source](https://docs.rs/crate/pulp/0.22.3/source/src/x86.rs).

## Current verified facts — do not rediscover them

- Production analysis already has compact reflected-ring packing, dynamic order scheduling, SIMD sectoral initialization, cosine-parity preweighting, guarded insignificant-prefix skipping (`sht_ftol=2^-60` with an explicit error budget), mixed/scaled-to-ordinary execution, and single-pair fused recurrence/accumulation.
- The historical stage profiles found the main excess in recurrence/accumulation rather than FFT or scheduling. T720 had **2,910 ordinary and 474 initially scaled blocks**; **no** late normal-loop declines or discarded completed degree work. Scaled blocks do real work.
- `a707780` changed the ordinary SIMD accumulators from `vmulpd` + `vaddpd` to fused `vfmadd*` instructions. Rust-only whole-call gains were modest and not uniform across workloads.
- `96ffd14` moved per-block cosine-square and scaled-correction tables into reusable workspace and reduced approximately 689,643 ordinary per-pair mask reductions to approximately 3,384 per-block reductions for T720. Despite fewer YMM stack-memory operands and reduced frame size, its **short T720 whole-call screen regressed** (29.203→31.215 ms q1; 6.493→7.150 ms q8), while smaller T300 q1 improved. This means the optimization was not validated as a general speedup.
- The inspected Rust wrapper and DUCC `map2alm_kernel` have **different boundaries**; do not compare frame sizes/spills or instruction totals from nonmatching scopes. The prior naive two-pair/four-degree unroll, 256-lane packing and out-of-line scaled initialization regressed; do not replay unchanged.
- A matched ordinary f64 scalar-analysis kernel screen now measures `m=1`,
  `lmax=720`, q1 against pinned DUCC. Rust checked kernel-plus-commit / DUCC
  ratios are 1.736×, 1.582× and 1.549× at 64/128/256 active latitude lanes;
  each case uses ten interleaved process pairs and six 29–79 ms batches per
  process. Output relative L2 error is `6.670e-14..1.628e-13`. This is not a
  full-transform or q8 result and does not measure scaled/mixed recurrence,
  other orders, or setup/FFT/scheduling phases; those gaps still need matched
  attribution. See [the rolling status](../PERFORMANCE_STATUS.md). CPU hardware
  `perf` counters were blocked on the host; do not change system-wide security
  settings.

Evidence: [latest code-generation follow-up](../reports/archive/experiments/scalar-analysis-codegen-followup-2026-10-19.md), [initial AVX2 report](../reports/archive/experiments/scalar-analysis-avx2-codegen-2026-10-19.md), [stage profile](../reports/archive/experiments/scalar-analysis-stage-profile-2026-10-18.md), [mixed-fusion screen](../reports/data/scalar-analysis-mixed-fusion/README.md). Archived reports are evidence, not competing goals.

## Milestone 1 — Build a truly matched Rust/DUCC hotspot comparison

Create or refine a **small native kernel benchmark**. Do not substitute Rust whole-call versus DUCC high-level Python timings.

- Exercise the **production-dispatched** f64 GL `A` path, including the actual monomorphized Pulp AVX2/FMA kernel, and the matching optimized DUCC specialization. Confirm execution by symbols/disassembly and optionally temporary counters; do not analyze dead/generic branches.
- Match inputs, transformed recurrence coefficients, active latitudes, parity-weighted Fourier spectra, prepared cosine squares, useful lane-degree count, `m` range, scaled/ordinary distribution, CPU target/ISA and compiler optimization settings. Separate **prepare/pack**, **normal recurrence + accumulation**, **scaled prefix and mixed recurrence + accumulation**, **normalization/scatter**, **FFT**, and worker scheduling. Account for work moved outside measured regions rather than hiding it.
- Test representative high-bandlimit and irregular/reduced-`mmax` cases, odd/even longitude count and 1/8 workers. In T720, separately report matched blocks from ordinary and the initially scaled population, with useful degrees and actual transition points. Report q8 *wall elapsed* rather than summing worker CPU incorrectly.
- Interleave warmed release runs under reasonably uncontended conditions, record sample spread, and validate output. Choose only a few informative cases, not a broad 29-grid campaign. If a benchmark stage is not actually equivalent, **fix the stage correspondence** before claiming its speed ratio.
- Produce an attribution table with absolute **milliseconds**, Rust/DUCC ratio, call/loop count, lane utilization and fraction of full-transform time. A profiler that cannot apportion the excess is incomplete.

**Decision:** is the deficit primarily (a) ordinary recurrence and accumulation, (b) scaled/mixed work, (c) setup/reduction/commit, (d) compiler-generated instructions, or (e) remaining whole-call overhead? If multiple, rank by *absolute excess latency*, not code complexity or speculation.

## Milestone 2 — Diagnose the real machine code, not just source syntax

Inspect `objdump`/`llvm-objdump` output from the **exact optimized binaries actually timed**. Pin compiler and target arguments. Rust uses LLVM; pinned DUCC currently uses GCC. Where feasible, build the same C++ DUCC kernel with **Clang at the same AVX2/FMA target** as a control: distinguish algorithm/data-flow effects from LLVM-versus-GCC code generation. Do not equate equal flags with identical codegen.

For the *matched loop bodies and their callers*, annotate:
- `vfmadd*`/multiply-add chains, instruction/iteration counts, vector width and live accumulator dependency chains.
- Actual **hot-path** YMM spills/reloads, stack and buffer memory operands, stores of recurrence state, load/store address generation and vector input broadcasts.
- Range-check comparisons, mask reductions, scalarized exceptional-lane processing, branch frequency and per-pair safety work. Specifically test whether checked guards can be moved to a mathematically justified preflight or cheap slow-path exit **without removing safeguards**.
- Ordinary versus initially scaled blocks; insignificant-prefix skip work, exponent normalization, transition rates, exception/correction products.
- SIMD horizontal reductions, pair-degree loop organization, front-end/code-size effects and inlining. DUCC's two-pair progression is a hypothesis for instruction-level parallelism, **not** permission to restore the rejected register-heavy unroll unchanged.

Use targeted source+assembly evidence, annotated loop snippets, `llvm-mca` selectively as a throughput model if available, and separate equivalent-work timing to test the top hypotheses. Never infer a measured bottleneck from stack-frame size or instruction count alone. Hardware `perf` being unavailable is not a reason to stop.

## Milestone 3 — Attack the dominant cost with substantial implementation work

Once the cause is isolated, pursue the **largest plausible structural reduction in actual executed work or per-lane throughput**, not another series of superficial tweaks. Choose the design based on measured attribution; candidate classes include:

1. **Common ordinary hot loop:** remove proven redundant memory passes, unneeded loop-body checks and stores; restructure latency chains and coefficient reduction; keep a separate checked slow path where preflight can guarantee fast-path safety. Retain appropriate polar/subnormal checks and transactional output.
2. **Mixed/scaled blocks:** avoid repeated per-lane scalar correction and branch-heavy iteration when a safe vectorized/batched scaled-to-ordinary transition can preserve exact required contributions; quantify the number of blocks/degrees and their fraction of latency.
3. **Layout and degree grouping:** port DUCC's *effective* dataflow (prepared geometry, coefficient reuse, latitude batching and paired-degree accumulation), measuring register demand and spills before and after. An alternative layout/unroll is acceptable if it solves the **recorded causes** of previous regressions.
4. **Compiler specialization:** if source algorithms are equivalent but assembly demonstrably differs, change Rust layout/inlining/loop presentation, investigate Pulp boundaries, or selectively use a **small private AVX2 specialization with portable fallback** only after compiler output and benchmarks establish necessity. Avoid unstable Rust features or public ISA dependencies.

**Accept proven incremental improvements, but do not call the performance objective complete just because one passed.** Reprofile against DUCC and prioritize the next largest absolute excess. If a supposedly substantial change fails to move end-to-end latency, explicitly resolve that discrepancy; do not keep layering optimizations blindly.

## Milestone 4 — Numerical, performance and closeout gates

- Verify pinned DUCC numerical outputs **and** analytic/adjoint identities, including distinct `S`, `A`, `S*`, `A*`; GL weights, reflected parity, `m=0`, `phi0`, polar/high-`m` cases, longitudes/tails, truncated `mmax`, extreme finite inputs, subnormals, exponent correctness, scalar/parallel routes. Other grids/spins/f32 paths must not regress.
- Run `cargo fmt --all -- --check`, relevant `cargo clippy --workspace --all-targets --all-features -- -D warnings`, release tests and no-default-feature tests; Python tests if bindings changed. Keep minimal validated production paths, stable Rust, portable Pulp dispatch and bounded workspace; no unrelated FFT or scheduler rewrite.
- For each accepted/rejected experiment, record baseline/candidate revisions, source/ISA flags, **equivalent-work Rust versus DUCC times**, phase boundaries, numerical errors, actual assembly differences, repeatability and residual excess latency. Use a concise rolling report, not an ever-growing new handoff.
- **Do not stop after the first few-percent win.** Continue while a justified larger opportunity remains. If representative equivalent-work and whole-call latencies reach the same practical performance class, verify across several shapes/q1/q8 and document any residual. If a large deficit persists, the closeout must identify a specific bottleneck, prove why attempted solutions failed and propose an actionable next kernel redesign. A generic “remaining ~2×” is not a sufficient final finding.
- Scope stays scalar f64 GL analysis. Spin-1, native f32 and other grid improvements require a *later separate goal*.

## Repository sync and instruction authority

**Before every push:** inspect `git status`, commit or safely stash WIP, `git fetch origin`, then `git rebase origin/opt/sht-performance-parity` for **unpublished local commits only**; reconcile conflicts without discarding concurrent work. Once the remote contents are integrated, **reread the latest `AGENTS.md` and this plan**, plus any new owner instructions in the update, before continuing and before pushing. Revalidate affected tests, push normally, verify remote HEAD. **After every pull, rebase, merge or integration of newer remote changes, recheck current instructions.** Never force-push, reset away work, rewrite published commits or update `main` without authorization. If safe rebase cannot complete, preserve work and clearly report the conflict.

At most one Luna benchmark-only subagent; no Sol reviewer unless expressly authorized. Keep raw disassembly, process/host/PSI diagnostic logs local and untracked per `AGENTS.md`. Commit meaningful validated improvements and concise reproducible findings.

## Reference specifications (not active task queues)

[Numerics and grids](02-numerics-and-grids.md) · [DUCC operation crosswalk](06-ducc-contract-crosswalk.md) · [Licensing](01-ducc-provenance-and-license.md) · [Historical archive](../reports/archive/README.md).
