# Plan 16 — DUCC-equivalent spin-1 recurrence and dataflow

**Status:** QUEUED after the current owner-directed scalar-analysis cumulative-ablation study. **Branch:** opt/sht-performance-parity. **Scope:** f64 single-vector-field spin-1 STANDARD E/B analysis and synthesis on GL, CC and F1; optimize the measured synthesis recurrence/spectrum residual first. This is a new investigation, not a reopening or retroactive reinterpretation of completed Plan 15.

## Execution order and handoff

1. **Finish the ongoing scalar task without interruption.** Follow reports/scalar-analysis-cumulative-ablation-2026-10-21.md and any newer remote instructions. Complete its remaining fixed SIMD workspace-layout experiment, the original/current/cumulative 64/128/256-tile comparison and required candidate ablations/interactions; attribute effects, validate all controls, commit a clear conclusion, and leave a known-good production default. Do not mix scalar and spin-1 code changes in an experiment or declare the scalar task complete prematurely.
2. **Then start Plan 16 on the same working branch**, after fetching and safely integrating the latest remote head. Read AGENTS.md, PERFORMANCE_STATUS.md, Plan 15's closeout, actual Rust and pinned DUCC sources, and recent native results. A changed upstream branch or a newer explicit owner instruction supersedes this queue; never overwrite concurrent work.
3. Work one hypothesis at a time. Publish reproducible small commits for accepted improvements and documented negative findings. A negative measurement is a valid result; do not force a new kernel into production.

## Existing evidence and important distinctions

The Plan 15 closeout (reports/plan15-spin1-synthesis-dataflow-closeout-2026-10-21.md) measured **exact T720 CC q8 spin-1 synthesis** at **23.596 ms Rust versus 6.087 ms DUCC full order (3.87x)** and **17.179 versus 2.647 ms at mmax=180 (6.48x)**. Rust's combined order scheduling/recurrence/spectrum phase was 18.994/15.641 ms while inverse rows/writeback were 1.205/0.181 ms. These Rust phases are **not** independently matched DUCC substages. F1 T720 q8 retained approximately 3.80x full and 5.63x reduced whole-transform ratios on a low-wait recheck. Do not blend these runs with the noisier integrated T720 screen (which reported larger ratios).

Already accepted: CC/F1 order-parallel **analysis** meridional preparation; GL/CC/F1 synthesis group scheduling and eligible cross-group scalar coefficient conversion reuse. Already tested and rejected: naive degree-pair-outer SIMD loop reversal (q1 regression 23–27%, q8 full regression ~5%); active-lane compaction (24% fewer fallback blocks but less than 1% whole-call impact). Do not retry unchanged versions. The previous pair-outer regression demonstrates that fewer coefficient broadcasts can lose to SIMD lane-state loads/stores. Repeated coefficient conversion is **not** an unexplored opportunity; only downstream paired-coefficient preparation reuse remains to test.

Rust source crosswalk:
- src/legendre/scalar_paired.rs, FusedPreparedSynthesisPairBlock and synthesize_prepared_order_pair_block_dispatched (around lines 5311–5455 and 663–755 at initial Plan 16 snapshot).
- src/spin_transform.rs, prepare_spin1_scalar_coefficients; synthesize_spin_ring_group_mode; synthesize_spin1_scalar_block; group scheduling (around lines 1179–1376, 1464–1544, 1646–1858 and 1951–2408).
- src/meridional.rs, to_prepared_parallel and restore_original, including temporary order tiles and ring-major scatter.
- src/legendre/spin_fast.rs, spin1_to_scalar and scalar_to_spin1.
Pinned reference: DUCC 0.42.0 commit 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b. Examine src/ducc0/sht/sht.cc (alm2leg, allocate_leg, spin1to0 and resample_to_prepared_CC) and src/ducc0/sht/sht_inner_loop.h (alm2map_kernel, extract_a2m, spin-specific kernels and scaled/ordinary transitions). Check which DUCC **scalar-converted** versus **direct-spin** path actually executes for each matched fixture. Do not equate DUCC's direct-spin kernel with its scalar spin-1 fast path.

## Milestone A — establish an apples-to-apples executed-kernel crosswalk (required first)

For exact T720 CC full and mmax=180 q1/q8, plus representative GL and F1 cases, document precise stage timing boundaries in both programs: E/B conversion; scalar-pair preparation and checks; Legendre recurrence/accumulation; polar scaled/ordinary transitions; scratch and spectrum writes; scheduling; CC/F1 meridional preparation; longitude FFTs. Measure stage equivalents directly where practical, not by subtracting unrelated whole-call runs. Capture actual executed normal/fallback counts, recurrence degree×lane work, redundant coefficient preparations, vector width, instructions and spill/load/store behavior. Document the compilation and dispatch paths, not just source-level operations.

Use a small single-thread kernel fixture at matched order/latitude/degree boundaries (including low/medium/high m and a polar control) and an independent whole-transform cross-check. Treat compiler-policy differences (Rust strict FP vs DUCC's GCC -ffast-math/-ffp-contract=fast) as a confound; record flags and generated code, and use optional matched-policy diagnostics without weakening production numerical guarantees.

## Milestone B — parity-separated versus cosine-combined SIMD recurrence (highest-priority new hypothesis)

Rust FusedPreparedSynthesisPairBlock currently creates cosine-weighted coefficients inside the degree-pair loop for two complex channels. DUCC's converted scalar alm2map_kernel accumulates even/odd contributions separately and recombines with cos(theta) at extraction. **This is a hypothesis, not a demonstrated Rust defect or guaranteed speedup.** Derive and test the exact algebra and normalization, including north/south reflection, phase and 1/sin(theta) scaling, prior to modifying hot loops.

Compare separately and in order:
1. Current fused two-channel cosine-combined SIMD baseline.
2. Two-channel **parity-separated accumulation**, moving the cosine combination after the degree loop without changing lane-outer recurrence-state retention; guard numerical range and transactional fallback.
3. A **one-channel-at-a-time** parity-separated variant that resembles DUCC scalar accumulation, explicitly measuring repeated recurrence versus reduced live registers.

Do not repeat the rejected pair-outer loop reversal. Inspect V3 AVX2/FMA assembly for instruction mix, FMA count, stack spills, live register pressure, hoisting of cos(theta)^2, memory traffic and code size; verify portable Pulp fallback and SIMD tails. Compare isolated kernel timing and full q1/q8 transforms at T255/T511/T720/T1023 where justified. Keep a new kernel only if it provides repeatable whole-call gains with no unacceptable correctness, workspace, cold-call or other-grid regression. Remove or disable disproven variants, retain their concise result and generated-code interpretation.

## Milestone C — reuse fully prepared paired coefficients across ring groups (independent candidate)

Distinguish cached **converted scalar E/B rows** from the four **normalized/prepared even/odd coefficient streams** consumed by the paired recurrence. Instrument how often the latter are recomputed per Fourier order and how many bytes/time they cost. Test a bounded per-order/per-transform prepared-coefficient cache so preparation can be reused by multiple ring groups; preserve overflow/finite safety, coefficient-change invalidation, plan/workspace ownership and all fallbacks. Avoid globally cached input-dependent results, unbounded O(lmax×mmax) extra scratch or premature preparation for rejected/polar orders. Compare q1/q8 and full/reduced CC, F1 and GL before promotion. If Milestone B changes the coefficient representation, measure cache reuse on its best validated kernel rather than coupling experiments prematurely.

## Milestone D — order-major synthesis dataflow and bounded scheduling (only after A–C)

DUCC order-parallel synthesis processes compact latitude blocks and may allocate order-major intermediate Legendre data. Rust's current dynamic queue parallelizes latitude ring groups and writes ring-major spectra. Build one controlled prototype for order-major tasks and/or order-major temporary spectra. Measure recurrence locality, spectrum writes, conversion/transposition costs, polar compact packing, workload balance, memory footprint and final inverse-FFT handoff **end-to-end**. Preserve the already successful group queue as control. An order-major intermediary that speeds the isolated recurrence but slows the complete transform must not be promoted. Do not implement a broad new SHT architecture or public API.

## Milestone E — T1023 coefficient-cache eligibility and CC/F1 analysis overhead (conditional)

After identifying the preceding cost mechanisms, test whether Rust's current spin-1 coefficient-cache lmax<=720 limit and 17 MiB budget are unnecessarily restrictive for reduced/full T1023. Control actual allocation, cold/warm timing and worker occupancy; adjust a gate only if data supports it. Separately for CC/F1 **analysis**, isolate the order-tile-to-ring-major scatter and meridional FFT profile cost against DUCC's resample_to_prepared_CC; consider a layout-specific prototype only if the stage is still a material residual. Keep the analysis study separate from synthesis and do not broaden to scalar CC/F1.

## Matched measurement and validation protocol

- Baselines: preserve an immutable current-production Rust reference and pinned DUCC 42ce8ab5. For comparative measurements use the established Ryzen 9 5950X q1 physical-core pin and q8 CPUs within one L3 domain, reused plans/workspaces and identical fixtures, with alternating source/process order. Report toolchain, ISA/compiler flags, build hashes, geometry, output reset, first-call versus warmed, runqueue wait/host contention and confidence/spread.
- Short kernels and transforms: calibrated repeated 20–100 ms timed batches, at least 30 batches across independent runs for final claims. Long calls: repeated individually timed interleaved processes with explicit sample counts, median and p10/p90. Show absolute Rust and DUCC latency, Rust/DUCC ratio, candidate/baseline ratios and stage attribution. Do not invent an arbitrary speedup gate, infer a source gain from overlapping noisy data or compare unlike processing boundaries.
- Accuracy: native DUCC comparisons and internal analytic/adjoint identities, S/A/S*/A* checks, spin-1 STANDARD and GRAD_ONLY; preserve DERIV1, spin 0–4, GL/CC/F1, f32/f64, odd/even longitude controls, subnormal/extreme finite inputs, near-pole transitions, changed E/B coefficients, poisoned/reused workspace, SIMD tails, output reset and cold/warm repeat behavior. No altered tolerance, silent rounding semantic change or partial writes on a failed fast path.
- Engineering: stable Rust and existing Pulp SIMD, bounded owned scratch, no dependencies/public API changes. Run formatting, strict Clippy, workspace release tests and no-default-feature release tests after accepted changes, plus targeted cross-library checks. Keep license/source attribution when translating DUCC algorithms. Preserve rejected experiments in concise documents/data, not proliferating production kernels.
- Repository/workflow: work on existing opt/sht-performance-parity only; no new branch, no PR/main change, no forced push. Before every push inspect worktree, fetch origin, rebase **only unpublished** commits when safe, reread AGENTS and active plan after any remote integration, retest, push normally, verify remote HEAD. If concurrent changes cannot be safely reconciled, preserve work, report conflict and move to independent investigation rather than force-overwriting. At most one Luna benchmark-only subagent; no Sol reviewer unless specifically authorized. Commit validated, attributable milestones frequently and never push known regressions.
- **Owner scope hold:** do not optimize odd-length FFT, swap realfft, introduce custom rustsht-fft backends, or reopen Plan 14 FFT campaigns. Keep odd/even grids only as correctness and regression controls.

## Deliverables and stopping rule

For each milestone, commit a compact source-level DUCC/Rust crosswalk, matched numerical checks, timing distributions or sanitized summaries, assembly/instruction interpretation where relevant, candidate disposition and reproducible command. Update PERFORMANCE_STATUS.md with supported results, not speculative speedups. Finish the sequence or clearly close each unpromising hypothesis; do not extend work to unrelated kernels because one candidate failed. Summarize the final accepted changes, remaining q1/q8 GL/CC/F1 gaps and what evidence would justify further work. Plan 15's historical closeout remains unchanged.
