# ACTIVE — f64 scalar CC/F1 DUCC performance parity: implement, do not benchmark indefinitely

**Owner reprioritization:** 2026-10-23. **Branch:** `opt/sht-performance-parity`. **Implementer:** Luna Max. **Status:** ACTIVE, supersedes the execution order of [the 2026-10-22 CC/GL/spin-1 plan](f64-cc-gl-kernel-port-and-spin1-parity-2026-10-22.md) and the earlier [full-sweep plan](scalar-f64-cc-full-band-and-gl-table-crossover-2026-10-22.md). Their accepted measurements, mathematical constraints and historical audits remain references, **not blocking prerequisites**. Explicit new owner instructions still take precedence.

## Goal and nonnegotiable priority

**Make f64 scalar spin-0 CC and F1 analysis/synthesis at least as fast as native DUCC 0.42.0 in representative production workloads, without numerical regressions; then optimize f64 spin-1.** A missed performance target is a remaining issue to investigate, not permission to claim success. Maintain f64 GL performance.

Priority:
1. **First meaningful next deliverable: production f64 scalar CC/F1 synthesis code improvement**, reusing the already effective GL ordered/reflected kernel and DUCC's verified latitude-pair recurrence algorithm. Validate numerics and run a few matched end-to-end A/B tests. Do not perform another full sweep first.
2. **Scalar CC/F1 analysis:** share bounded parallel CC/F1 meridional preparation from the existing spin-1 route; then adapt reflected/ordered analysis recurrence if the remaining stage profile warrants it.
3. **Finish scalar CC/F1 near-parity:** iterate on the highest measured remaining *absolute* time excess, including q1/q8, full/reduced orders and appropriate grid-specific dataflow, one algorithmic mechanism per candidate.
4. **Then f64 spin-1 GL/CC/F1 STANDARD S/A**, applying proven reflected-pair and scaled-recurrence lessons, preserving accepted Plans 15/16.
5. **f32 remains PAUSED** until important representative f64 scalar and spin-1 workloads are broadly near DUCC parity or better. GL 20 MiB table-policy research, enormous benchmark matrices, extra plots and repository housekeeping are **deferred** unless a demonstrable production correctness blocker requires a narrow fix. No forced arbitrary 10% acceptance target replaces honest per-case reporting.

### Why this change is necessary

At the checkpoint `82aafa20` (2026-10-23 01:45 UTC), the previous plan `f580ef35` had been followed by ten committed changes to the **benchmark runner, numerical audit, tests and recovery metadata**, with **no production SHT kernel changes**. These are genuine quality improvements, but their marginal value is now outweighed by implementation.

Saved full-order f64 scalar CC synthesis native warmed medians in [Sol's review](../reports/f64-ducc-independent-review-2026-10-22.md), Rust/DUCC: T255 q8 `3.128/0.299 ms` (10.45×), T720 q1 `90.315/20.435 ms` (4.42×), T720 q8 `47.916/2.951 ms` (16.24×), T1023 q8 `136.614/8.156 ms` (16.75×), T2048 q8 `1414.172/65.689 ms` (21.53×). By contrast f64 GL T720 q8 synthesis `3.397/3.070 ms`. These are sufficient to justify a **small, instrumented source change now**. The CC source's GL-only optimized selector and general ring-group route explain a credible large portion of the opportunity, but the eventual speedup has not been demonstrated.

**F1 scalar performance is not broadly established.** The native scalar runner now accepts F1, with exact `ntheta=lmax+1` geometry, and the focused T255/T720 full-order baseline is recorded in the [F1 report](../reports/ducc-scalar-f64-f1-performance-2026-10-23.md). Analysis was faster than DUCC at all tested thread counts; synthesis was near parity except T255 q1 at `1.148x`. F1 has **no sampled poles** and **mandatory half-grid meridional phase/resampling** for analysis, unlike exact CC's poles and conditional resampling. Never substitute CC measurements for F1.

## Immediate work: stop the benchmark-only loop, preserve work

1. Fetch and reconcile `origin/opt/sht-performance-parity`. Check current worktree, untracked/frozen benchmark artifacts and live jobs. Do **not** terminate another agent's valid benchmark process or delete results; let work in flight reach a safe checkpoint or persist it, but **do not start any new full-band/shard-recovery/stage-profile/GL cutoff campaign**.
2. Review the latest ten audit/validation commits. Keep their fixes. The CC campaign is **not necessarily complete**: historical 13/14 shards, 72 required groups missing, plus 16 extra. Record current missing/numerical status succinctly; preserve raw shards. Do not claim full coverage or numeric acceptance based on finiteness alone, and do not rerun it for this milestone. Resume the missing high shard only after the primary optimization milestones or explicit direction.
3. **Complete:** add F1 GRID support to the scalar Rust/C++ workers and Python runner, preserving existing GL/CC inputs. The F1 smoke and exact analytic/adjoint checks passed. The focused F1 baseline covers T255/T720 full order, q1/q8, analysis/synthesis, and pinned DUCC. No new 2,528-cell grid was run; CC remains represented by the saved short comparisons. Use T1023 or reduced-order cases only when a specific residual warrants them.
4. The **next implementation milestone must contain a Rust `src/` production algorithmic change** or a precise correctness failure preventing it, not another multi-hour benchmark/report pass. If a proposed change fails, record a compact negative result and move to the next independent code mechanism; do not iterate on reporting alone.

## M1 — Ordered/reflected scalar synthesis for CC and F1 (highest priority)

Inspect `src/transform.rs` and the current `synthesize_scalar` GL-only gate (around line 1135), `synthesize_scalar_ordered_parallel`, `src/legendre/scalar.rs` and `src/legendre/scalar_paired.rs`. Inspect the pinned DUCC 0.42.0 `src/ducc0/sht/sht.cc` and `sht_inner_loop.h`: `alm2leg`, `make_ringdata`, `inner_loop_a2m`, `fill_a2m`, `calc_alm2map`, `alm2map_kernel`, `extract_a2m`.

**Port the proven dataflow, not a new subsystem.** Create a minimal grid-aware adapter to reuse the existing fast ordered/reflected scalar path for CC and F1, separating grid-dependent ring geometry/polar handling from grid-independent recurrence and parity. DUCC's reflected north/south SIMD and scale-transition machinery provides the reference; preserve RustSHT's independent numerical guards. Begin with one scalar component and one order or small T fixture, then enable a checked subset of eligible whole transforms. Prefer a shared helper over duplicate CC and F1 kernels.

Numerical requirements: CC includes the two poles and has exact `ntheta=lmax+2`, F1 excludes poles and samples offset meridians with exact `ntheta=lmax+1`; reflection, parity, midpoint singleton, longitude phase `phi0`, m=0, full/quarter order, even/odd theta and longitude, SIMD tails, scaling/subnormals/extreme finite and NaN/Inf classification, reset/poisoning, serial/parallel and all four operators/adjoints must remain correct. Protect against partially written outputs on a checked fallback. Do not simply delete the GL guard without proving the grid-specific invariants; keep a safe old-path fallback.

**Candidate priority:** implement ordered-engine extension before large task-grain tuning. The existing ring-group path has only four effective workers in a T720 q8 scalar CC diagnostic, but task grain alone cannot explain q1's approximately 4.4× gap. If ordered reuse is blocked or insufficient, run a small isolated q8 task-grain A/B (modify only `with_min_len` and check actual worker utilization), then return to fundamental recurrence work. Do not combine schedule, recurrence and resampling changes into one benchmark.

At each accepted candidate: targeted unit/analytic/DUCC/adjoin tests, f64 GL controls and end-to-end comparisons original Rust/candidate Rust/pinned DUCC. Report absolute latency, ratio and process spread; keep only repeatable whole-transform wins. **Do not treat an assembly operation count as a measured speedup.**

## M2 — Scalar CC/F1 analysis: parallelize resampling, then recurrence

At T720 full CC q8 analysis, saved Rust/DUCC `34.557/12.292 ms`; Rust diagnostic resampling + latitude prep around `26.5 ms` and recurrence about `4.4 ms`, with nonidentical cross-library stage boundaries. Current scalar `src/transform.rs` calls serial `resampling.to_prepared` (around line 2197). Existing `src/meridional.rs::to_prepared_parallel` has a validated bounded worker method for spin-1 CC/F1; DUCC `resample_to_prepared_CC` schedules orders in parallel.

Reuse the existing Rust CC/F1 prepared-grid buffers and bounded order-parallel mechanism for **scalar** without introducing a new FFT engine or unbounded workspace. Preserve CC conditional resampling and F1's **always active resampling with half-grid phase**, complex normalization and weights, both adjoints, poles/equator and safe reset. Test A/S controls q1/q8 at T255/T720, full/quarter and CC/F1, then higher T if warranted. For short T avoid parallel setup overhead by evidence-based dispatch; no arbitrary hard-coded threshold.

After removing the serialization floor, stage-profile only if needed to locate material residual. The GL analysis reflected-pair path and DUCC prepared-grid analysis are source references, but preserve the 256 reflected-pair scalar analysis tile and safe scaled fallbacks. Don't reopen rejected 64/128 tile experiments or enable unsafe CC prefix skipping.

## M3 — Close scalar parity gap, then f64 spin-1

Use **small calibrated representative A/B sets**, expanding only where a confirmed regression or unexplained large gap remains. Include T63/T255/T720/T1023 as applicable; q1/q8; full/quarter mmax; even-smooth and selected odd-minimum nphi; GL controls; CC including poles; F1 excluding poles/mandatory phase. Distinguish what is measured from projected gains. Compare absolute excess milliseconds as well as ratios. Try the highest-leverage source-grounded candidate, validate, keep or reject, then proceed.

**Scalar completion target:** representative f64 scalar CC/F1 analysis and synthesis are at or faster than DUCC where attainable; no important systematic slowdowns remain. Near-parity (e.g. modest 10–20% measurement spread) can be recorded separately but not declared "at least on par" where ratios exceed one. If hardware/compiler-specific limitations persist, document them and continue best-evidence source improvements; do not weaken correctness or fabricate parity. Escalate a genuine blocked architectural decision instead of running endless measurements.

When scalar has undergone all high-leverage tests and remaining gaps are near parity or concretely explained, proceed to **f64 spin-1**, not GL threshold research:

- In pinned DUCC `sht.cc` T>=500 STANDARD spin-1, `spin1to0` converts ordinary rings; reflected pairs use two scalar recurrences, independent parity and a scaled-to-ordinary transition; direct spin handles near poles.
- Rust already scalar-converts and has order-major q8 scheduling. Sol's [source crosswalk](../reports/f64-ducc-independent-review-2026-10-22.md) observes excess all-ring hemisphere work and expensive checked fallback. First count weighted lane×degree work for successful normal/scale transitions, early/late declines, final visitor and polar direct-spin; distinguish worker-summed time from whole-call latency.
- Test a **new** bounded reflected-pair adapter for the two converted components reusing the scalar ordered engine. Begin with one order and DUCC/analytic validation, then T720/T1023 full/reduced q1/q8 GL/CC/F1 A/B, preserving original-coefficient fallback and partial-write safety. Avoid prior rejected loop reversal, parity-only all-ring variant, cache-only repeat and spin-1 resampler rewrite.
- Analyze spin-1 after synthesis, reusing GL reflected-pair machinery for CC/F1 only with a demonstrated recurrence-stage bottleneck; the existing spin-1 resampler is already about as fast as DUCC on selected cases.

**f32 stays paused** until f64 scalar and spin-1 representative CC/F1/GL workloads are broadly near parity or better, with major outliers addressed. Preserve validated Plan 17 work; do not restart it automatically.

## Deferred work and safeguards

- **Do not block M1/M2 on:** completion of all historical CC high-degree cells, seven polished figure pairs, GL q1 20 MiB table-crossover/cold-reuse matrix, duplicative benchmark reports, library packaging/CI cleanup or exhaustive repo-local artifact classification. Track these as a separate **deferred benchmark/housekeeping backlog** and preserve evidence; do not delete untracked material without establishing its origin.
- Retain current GL 20 MiB policy provisionally; `ntheta % 4 == 0` determines table SIMD dispatch and warrants eventual policy study, **but GL is already broadly competitive and is not this phase's dominant performance gap**.
- No new FFT subsystem/RealFFT, broader odd-packed FFT eligibility, spins 2–4 optimization, multi-field batching, APIs, arbitrary table cutoffs, cache topology redesign or unrelated Python work. The targeted F1 benchmark adapter is authorized. Preserve owner-approved `pyspharm-rs`/`spharm_rs` and source attribution for translated DUCC algorithms.
- Follow existing numerical acceptance thresholds and tests: f64 analytic/reference maps, random spectral cases, roundtrip, `S/A/S*/A*`, GL/CC/F1, spin 0/1, poles and offset, m=0, subnormal/extreme/nonfinite, poisoned workspace, serial/q8 fallback. Never trade a numerical defect for speed.
- For **rapid directional A/B**, use a bounded repeat count with correct matched affinity and a quiescent host; only repeat promising candidates with calibrated 20–100 ms short batches (>=30 across independent runs), and individually timed long operations with actual counts. Alternate process order; track host contention; compare original Rust, candidate Rust, and pinned DUCC. Do not claim definitive parity from a quick screen.
- Prefer a small implement/test/measure/commit cycle to a giant profiling pass. Keep measured progress in `PERFORMANCE_STATUS.md`; commit source and concise evidence after each accepted algorithmic change. Negative experiments should be recorded briefly without accumulating dead production kernels.
- Git: only `opt/sht-performance-parity`; before every push inspect worktree, `git fetch origin` and reconcile concurrent work, rebase unpublished commits only if safe, never force-push, hard-reset shared work or modify `main`. Push often; at most one **benchmark-only** subagent; no competing load on the same cores. Run `cargo fmt`, targeted release tests, strict Clippy and regression suites after changes; package-only no-default checks where shared paths are touched.

## Immediate completion checklist for next agent checkpoint

- [x] **At least one implemented and validated f64 CC or F1 scalar performance algorithm candidate** has been measured against original Rust and pinned DUCC; report accepted or rejected outcome. Do not submit only a new benchmark report. The ordered/reflected CC/F1 synthesis route is committed in `4d5dcf97`; T255/T720 CC results and remaining q8 excess are in the [M1 report](../reports/cc-scalar-ordered-synthesis-2026-10-23.md).
- [x] Small scalar CC and F1 baseline/control rows include both synthesis and analysis, q1/q8 and at least full order, with correct grids and provenance; selected F1 T255/T720 full-order results are in the [focused F1 report](../reports/ducc-scalar-f64-f1-performance-2026-10-23.md).
- [ ] A clear follow-up source-level bottleneck and **next code change** is identified; no new exhaustive benchmark runs are launched.
- [ ] Reports accurately distinguish `MEASURED` from `HYPOTHESIS`; missing historical CC coverage is preserved as an open task.
- [ ] `AGENTS.md`, `plans/README.md` and `PERFORMANCE_STATUS.md` point to this active performance-first sequence, with GL table/extra figures/deep cleanup explicitly deferred.
