# Owner steering — final RustSHT architecture and shared-kernel refactor

**Priority:** Active owner instruction, 2026-10-26 14:51 UTC onward. This supplements and, where inconsistent, supersedes earlier layout and compatibility instructions in AGENTS.md, the shared-kernel plan, the M0 route inventory, and older progress entries. Work on `dev` directly. Do not modify `main`, force-push, reset shared history or disturb parked experiments. Do not restart completed work.

**Objective:** One coherent RustSHT core with compile-time-shared algorithms across f32/f64, scalar/spin, and GL/CC/F1 where mathematically valid; limited typed SIMD leaves; fewer duplicate implementation bodies, dispatch modes, thresholds, scratch variants and special cases. Maintain numerical correctness, distinct adjoints, and transparently measured end-to-end performance; accept a justified few-percent slowdown for substantial simplification. **Do not optimize for minimum file count or line count alone.**

**Current progress (not a restart request):** The refactor already introduced `src/legendre/numeric.rs` / `LegendrePrecision` and shared spin factor preparation (`src/legendre/spin_factors.rs`), recorded fresh f32 baselines and an operator route inventory, and retained the validated f64 paths. These changes are on `dev`. Fetch and read HEAD before continuing. Advance from that state; do not redo baseline collection, factor extraction, or archived-source searches without a new reason. Baseline SHA: `062218639bc2f097f004c617da0ce46a3a4d2941`; all current benchmark comparison claims must specify the exact candidate SHA.

**Latest owner clarification (2026-10-26 17:13 UTC):** The earlier symmetric 0–5% performance trade-off below now applies principally to **f32**, not f64. **f64 correctness and important end-to-end performance are highest priority**, and f64 hot kernels must retain fast specialized leaves if sharing introduces a measurable regression. Follow the [rejected shared-kernel candidate recovery steering](rejected-shared-kernel-candidate-recovery-2026-10-26.md) for ranked B32–B35 re-evaluation. **B36 was already restored** to `dev` by `c908452a`; do not reintroduce it. Do not restart completed validation. The older symmetric acceptance bands below are retained as history only where inconsistent.

## Earlier owner performance-vs-maintainability acceptance policy (2026-10-26 17:02 UTC; revised 17:13 UTC)

**Priority correction:** The owner explicitly accepts a few percent slower execution in exchange for a **large, real reduction in source duplication and special-case maintenance**. The earlier rule "any measured regression in a validated fast path means revert" is superseded for this refactor. Maintainability is an objective, not merely a noise-level tie-breaker.

Decision guidance for correct, statistically credible **whole-transform** regressions:
- **0–2%:** ordinarily keep a meaningful shared implementation.
- **2–5%:** keep when it materially eliminates distinct recurrence bodies, substantial duplicated numerical branches, unsafe/fallback code, or persistent dispatch complexity.
- **5–10%:** require a demonstrably large maintainability benefit, review the affected workloads, and consider a small specialized SIMD leaf instead.
- **Over 10%:** normally do not accept; restore the fast leaf and share the surrounding algorithm.
These bands are guidelines, not automatic acceptance thresholds. A macro that hides but does not remove actual duplication, adds tangled abstractions, or complicates debugging is **not** sufficiently beneficial by itself.

**Measure the total result:** Compare each affected cell against both (a) the exact previous candidate parent and (b) the immutable pre-refactor `dev` baseline `062218639bc2f097f004c617da0ce46a3a4d2941`. Do not accept four successive 4% slowdowns by resetting the acceptance budget at each commit. Track f32 and f64, q1/q8, scalar/spin, full/reduced and selected GL/CC/F1 cells separately; avoid averages hiding outliers. Preserve explicit DUCC relative ratios and labels PASS/GAP even when the owner chooses a documented maintenance trade-off. No numerical correctness/adjoint/exceptional-input waiver.

**Source structure benefit should be concrete:** Report reduction in independently maintained recurrence/normalizer/accumulator implementations, duplicate LoC excluding comments/tests, number and complexity of dispatch gates, state/scratch variants, and new abstractions. Avoid arbitrary percentage requirements but articulate what code future maintenance no longer needs to duplicate. Prefer a 2–3% slower shared numerical algorithm if it genuinely replaces multiple precision-specific bodies and remains straightforward to optimize. Keep separate f32/f64 SIMD leaves only where required by numeric representation or a larger measured performance impact.

**Previously reverted work is eligible to reconsider, not auto-accept:** Review source commits `f974d1d5` (scaled arithmetic), `75cf5472` (callback traversal), `8e57a108` (compile-time scalar traversal macro), and `467d73ab` (spin-step macro), plus recorded reverts. They passed available output parity/focused tests but produced different slowdowns: the callbacks were slower on multiple affected scalar cells; the scalar macro slowed f32 T720 synthesis by roughly 2.5–3%, and spin-step macro slowed protected f64 T1023 synthesis q1 by ~1.25%. Decide whether the *actual simplification* justifies those costs. Do not reinstate them simultaneously. Reuse saved benchmark data as initial evidence; run only narrowly scoped further tests for changed code, and a cumulative regression screen at a suitable family milestone. A trivial macro with no net maintenance improvement need not be restored solely because its slowdown is allowable.

**Validation economy:** No 15-triplet benchmark required to prove that a 1–3% regression exists if prior tests already established it. Do focused correctness and small performance screens after a coherent kernel family; reserve extensive benchmarks for larger changes, uncertain performance, and final cumulative results. The main goal is completing the shared numerical and module architecture.

**Standing owner rule — review authorization and experiment cleanup (2026-10-26 17:36 UTC):** [Experiment lifecycle and disk-space hygiene](../docs/EXPERIMENT_HYGIENE.md) supersedes earlier optional Sol reviewers and reusable experimental build environments. **Never invoke Sol unless the owner explicitly requests that one specific review; authorization is single-use.** After every experiment, persist/verify compact results and necessary data outside scratch, then immediately delete all experiment-owned build artifacts, local environments, caches, temp worktrees and profiler files. Prefer bounded `/tmp` scratch only after checking free space/inodes; avoid exhausting it. No saving binaries for later experiments. Do not delete any other agent's paths or unpreserved raw evidence.

## Final design principle: organize by mathematical/execution responsibility

**Target topology (conceptual, not an order to generate empty directories):**

```text
src/
  lib.rs                    public reexports only
  error.rs
  harmonics.rs              HarmonicSpec, packed coefficient indexing
  plan.rs                   ShtPlan configuration and static routing
  workspace.rs              shared allocated buffers / execution capacity
  fft.rs                    existing longitude FFT implementation
  grid/
    mod.rs                  GridSpec, GridKind, latitude/ring descriptors
    quadrature.rs           GL/CC/F1 nodes and weights, if splitting pays off
    resampling.rs           current meridional.rs: CC/F1 prepared operator
    geodesic.rs             formerly src/pyspharm/geodesic.rs
  legendre/
    mod.rs                  small internal exports and trait wiring
    numeric.rs              existing LegendrePrecision, typed scaling policy
    scalar.rs               shared normalized scalar mathematics
    spin.rs                 shared native-spin mathematics, ±spin
    spin_factors.rs         already introduced; consolidate if still needed
    coefficients.rs         current generic spin_fast.rs adapter functions
    paired/
      mod.rs
      analysis.rs           paired Legendre analysis kernels when large enough
      synthesis.rs          paired Legendre synthesis kernels when large enough
    ordered.rs              order-major/reflected kernel
    table.rs                prepared-table algorithms, when retained
    pointwise.rs            arbitrary-latitude associated Legendre evaluation
  transform/
    mod.rs                  public ShtPlan methods + shared execution
    scalar.rs               scalar orchestration, not scalar recurrence
    spin.rs                 spin orchestration, not duplicate spin recurrence
    adjoint.rs              distinct adjoint orchestration where useful
    scheduling.rs           common task partitions/cache/worker decisions
  evaluation.rs             arbitrary-point spectral interpolation/synthesis
```

This tree is an **architectural destination**, not a mandate to split every small module. Keep simple files flat when splitting would only create indirection. Splitting the currently very large `scalar_paired.rs`, `spin_transform.rs` and `transform.rs` is more valuable than moving already-clear small modules. Treat names as implementation guidance; correctness and reduced coupling control the actual split.

### Ownership rules and dependency direction

1. `grid` owns geometry, quadrature and meridional map preparation; no independent Legendre recurrence. CC/F1 prepared resampling and its transpose remain genuine operators. GL does not use CC quadrature.
2. `legendre` owns coefficient factors, ordinary/scaled recurrences, basis evaluation, SIMD kernels, scalar/spin mathematical adapters, and local accumulation. It should not perform whole-transform FFT, Python packaging or unrelated global planning.
3. `transform` owns operator orchestration, spectral staging, scheduling, output handling and API method wiring. It calls `legendre`, not vice versa. Spin and scalar drivers share scheduling/cache/worker code where their semantics match.
4. `evaluation` uses the same `legendre` basis and coefficient conventions to evaluate a harmonic expansion at arbitrary coordinates. It is not a second recurrence engine.
5. `plan` and `workspace` remain narrow owners of immutable planning and caller-reused memory. `fft` remains an independent backend, not part of the generic-kernel refactor.
6. Public interfaces remain coherent through `lib.rs`; internal file movement should not scatter reexports across unrelated modules. Avoid circular dependencies: if a shared type is needed by both `grid` and `legendre`, put it in the lowest coherent shared owner rather than having the two subsystems import one another.

### Explicit treatment of the former src/pyspharm directory

**Latest owner instruction supersedes older progress notes saying to preserve `src/pyspharm`. Remove that misleading Rust source namespace.** Fold the numerical implementation into first-class RustSHT modules:

- `src/pyspharm/legendre.rs` → `src/legendre/pointwise.rs`; reuse shared f32/f64 Legendre recurrence. Do not duplicate basis stepping or exponent/scaling logic.
- `src/pyspharm/interpolation.rs` → `src/evaluation.rs`; use pointwise basis and coefficient packing, not a second Legendre implementation.
- `src/pyspharm/geodesic.rs` → `src/grid/geodesic.rs`; keep geodesic mesh generation separate from the rectangular GL/CC/F1 `GridSpec`.
- Integrate the three old module test files into module-local mathematical tests or equivalent focused tests. Remove only tests that exist solely to prove external pyspharm-syl/windspharm API/result compatibility, as the owner already instructed. **Keep independent analytic and numerical correctness tests** for normalization, interpolation, packing, geometry, poles, input errors and f32/f64.
- Remove `pub mod pyspharm` from the Rust library. Expose intentional Rust numerical entry points under meaningful public paths (e.g. `rustsht::evaluation`, `rustsht::grid` or reexports as appropriate). The owner accepts removal of the old `rustsht::pyspharm` path; do not preserve it with a compatibility alias. Identify any affected public paths and explicitly document the API break rather than pretending it is internal.
- Retain legacy-specific mathematical conventions **where behavior depends on them**, e.g. non-Condon–Shortley basis normalization/phase, degree-zero normalization, original geodesic ordering and coordinate conversion, even if names become neutral. A rename is not permission to change the mathematics. Keep a small explicit convention adapter if needed; do not silently change normalization to the regular-grid default.
- Update internal consumers such as `python/src/pyspharm.rs` to import the new Rust paths. **Do not remove the existing Python `python/pyspharm-rs/` distribution or `spharm_rs` API merely because a Rust source directory moved.** The current request is to reorganize Rust implementation and remove external-reference-only tests, not to delete a separately packaged Python product. A later rename of the Python binding source file is optional only if it genuinely reduces confusion and leaves the Python import/API intact.
- Make the relocation one isolated semantic-no-op change (plus explicitly recorded Rust public path rename), with cheap compile and focused mathematical tests. **No performance benchmark for a pure relocation**, unless the refactor also changes the recurrence or dispatch.

## Implementation order — implement shared kernels first, reorganize files second

### Phase 0 — State reconciliation (short, no new audit)

- Inspect current `dev` HEAD, new `numeric.rs` and `spin_factors.rs`, `reports/SHARED_KERNEL_REFACTOR_PROGRESS.md`, and `reports/SHARED_KERNEL_ROUTE_INVENTORY_2026-10-26.md`. Carry forward the actual completed steps. Do not replay M0 and the fresh 48-case f32 baseline.
- Respect existing ongoing uncommitted work; no new subagent writes to `dev`. Fetch/reconcile before each push. Keep the immutable parent SHA for all claims; use latest candidate SHA for new measurements.
- Add one short progress note listing the next three concrete code changes, then work on them; avoid additional source-only status churn.

### Phase 1 — Finish shared scalar precision/recurrence (existing work first)

- Unify duplicate normalized scalar factor and degree traversal in `scalar.rs` and `scalar_f32.rs` using `LegendrePrecision`. Keep exact f32/f64 factor order, thresholds and fallback. Prefer common typed functions returning primitive values to overly broad traits.
- Share ordinary and scaled recurrence state transitions and reusable scratch interfaces. Preserve precision-specific SIMD operations as leaf specializations. Both native f32 and f64 paths must run real typed computation, not automatic f32→f64→f32 conversion.
- Move f64-only paired/ordered implementations later; do not attempt a huge mechanical conversion before the basic scalar route is correct.
- Eliminate redundant definitions after their last caller migrates, not at the same time as first introducing new code. Run focused scalar tests and at most a small timing screen for the completed family.

### Phase 2 — Finish shared native-spin recurrence and adapters

- Reuse the already shared `spin_factors.rs`. Unify equivalent spin recurrence control, plus/minus state, normalization, buffer rotation and exceptional-lane structure from `spin.rs` and `spin_f32.rs`.
- Keep actual spin-dependent coupling and initial values explicit. Spin-1 scalar adapters in `spin_fast.rs` are already precision-generic: reuse them. Do not rederive coefficient transforms for f32.
- Distinguish STANDARD, GRAD_ONLY, DERIV1, S/A/S*/A*, and spins 2–4. Share the scheduler and numeric primitives, not non-equivalent mathematical operators.
- Keep f32-specific normalizer precheck and exponent limits. Keep direct-spin fallback where needed. Do not silently alter exceptional input handling.
- Check compilation frequently; focused spin/adjoint and unusual-value tests at the end of a coherent implementation slice.

### Phase 3 — Shared paired/reflected/order-major kernels, then f32 enablement

- Use shared scalar recurrence core and compile-time f32/f64 Pulp leaf specializations for `scalar_paired.rs` and `scalar_ordered.rs`. A small private macro emitting two concrete bodies is allowed if it is clearer and faster than an elaborate SIMD generic trait. Do not add dynamic dispatch or new runtime dependencies.
- For f64, preserve measured two-vector ordinary loops and compiled inlining; avoid introducing new bounds checks, YMM spills or extra code size as a side effect of generic wrappers.
- Implement typed f32 counterpart first; only then set necessary `ShtScalar` capabilities and enable f32 cache/reflected/order-major routing for eligible shapes. Do not turn gates on speculatively.
- Cache sizes must be based on `size_of::<Complex<T>>()`, geometry, active mmax and workspace worker capacity; polar declines and poison/reset must remain transactional.
- First validate f32 GL spin-1 exact and oversampled synthesis, q1/q8 with full/reduced mmax. Preserve native direct-spin route as baseline and fallback. Expand to CC/F1 only after correct grid-specific behavior and measured routing.
- Avoid explosion of T-specific special cases. A general, proven geometric/workload criterion is preferred over a sequence of equality checks.
- Separate paired analysis/synthesis source files only **after** their shared body and fast paths have stabilized.

### Phase 4 — Unify orchestration and grid adapters without changing mathematics

- Factor common scheduling, order caches, disjoint spectrum output ownership, parity geometry, FFT staging and scratch resets from `transform.rs` and `spin_transform.rs` into small internal helpers. Keep already-generic code rather than rewriting it.
- Move `meridional.rs` into `grid/resampling.rs` only when the module import/dependency layout is stable. The prepared CC/F1 resampling operators and transpose are not replaceable with a GL direct quadrature call.
- Split `grid.rs` into `grid/mod.rs` and optional `quadrature.rs` only if it clarifies responsibility; keep geometry independent of core Legendre kernels.
- Do not merge synthesis and adjoint synthesis into the same weighted operation. Test adjoint identities on the correct real-harmonic inner product (factor two for nonzero m) and distinguish quadrature weights.
- Integrate arbitrary-point evaluation and geodesic generation as above; preserve correct f32/f64 normalization. Do not create a new `auxiliary`, `compat` or `spherical` bucket merely to move files out of `pyspharm`.
- Prefer one transform entry-point family with shared private execution helpers rather than giant copy-pasted scalar/spin modules. Split `transform` only where it reduces size and repeated logic; do not force every operator into the same function.

### Phase 5 — Deliberate deletion and simplification

- Inventory each remaining old path: precision duplicate, scalar/spin duplicate, ring/worker duplicate, table fallback, special-case gate, scratch variant and dynamic eligibility condition. Identify whether it remains mathematically or empirically necessary.
- Remove only when all callers are covered and a narrowly scoped regression gate passes. Explain each retained exception in a short comment near the condition or in one compact architecture note.
- Shrink the very large Legendre trait boilerplate and oversized kernel files only insofar as it reduces implemented logic, not by inserting extra layers of routing. Code organized into more files but with unchanged duplicated algorithms is not success.
- Measure baseline-to-final source LOC (excluding tests/docs), maintained kernel body count, precision duplicate count, meaningful dispatch gates, duplicated buffer copies, unsafe writer sites, code size and extra dependency count. Avoid invented metrics or rigid percentage targets.
- Keep the old numerical experiment ledger/history; do not delete archived evidence. Do not resurrect negative experiments without a newly demonstrated reason.

## Validation cadence — light between small edits, strong at integration gates

**Do NOT run a full suite or 15-pair benchmark after every helper change.** Validate according to the size of the executed behavior changed:

- **Level 0 (small extraction/rename):** `cargo check --lib` or narrow compile; formatting after a related group; no benchmark, no full suite. Several compatible edits may be grouped into one coherent commit.
- **Level 1 (coherent numerical slice):** directly affected focused tests across both precisions, including one normal and one appropriate exceptional/adjoint/scratch test. No exhaustive geometry matrix.
- **Level 2 (one complete kernel family or new dispatch route):** targeted numeric/adjoint controls and three interleaved exact-parent/candidate/DUCC screens of affected q1/q8 workloads. Benchmark only the cells the source can affect; use a small and a representative large T, one reduced case when relevant.
- **Level 3 (retiring an old kernel, major stage or final merge-ready state):** broad Rust feature tests, Clippy/format, selected Python checks; 15+ interleaved pairs for material f32/f64 fast-path decisions or suspected repeatable regressions, with known f64 wins protected. Full cross-grid/spin/operator matrix once at final integration, not after each small patch.

Record tests actually run, not implied. If inherited parent tests fail the same way, document as inherited, do not silently weaken thresholds or delete the case. If numerical results differ due to accepted order of operations, demonstrate unchanged independent correctness and adjoints under existing tolerances.

### Protected performance cells (non-exhaustive)

- f64 GL T1023 spin-1 synthesis q1, GL T320 over-resolved full q8, GL T720 reduced q8, GL T1023 over-resolved full q8, CC T720/T1023 spin-1 synthesis q8.
- f64 GL T720 spin-1 analysis q1/q8, reduced T320 q8 synthesis and CC T720 reduced synthesis q1 remain known gaps; do not turn cleanup into isolated micro-optimization of these.
- f32 GL T255/T511/T720/T1023 scalar/spin-1 analysis and synthesis, exact and selected reduced mmax, q1/q8. Use the fresh baseline and paired-run artifacts, not October 22 historical timings.
- Include focused GL/CC/F1 and arbitrary-point numerical controls for touched algebra. No averaging away isolated regressions. Ir and code size are diagnostic, not runtime replacements.

## Agents, CPU ownership and Git discipline

- **Main agent: Luna Max.** Owns production edits, architecture choices, test integration, commits and pushes to `dev`.
- **Benchmark subagent: one Luna named `benchmark`.** Read-only toward `dev`; use temporary isolated worktrees and an experiment-specific build environment under a bounded scratch root (prefer `/tmp` after space checks), plus the existing harness. It can compile benchmark workers and run q1/q8 parent/candidate/DUCC comparisons, collect timing/Callgrind and report exact provenance. Persist results first, then **immediately remove its entire experiment-owned build/venv/cache/worktree/profiler scratch**. Never retain compiled parent/DUCC binaries for the next experiment. It MUST NOT modify production, commit/push, cherry-pick, or instruct the main agent to weaken correctness.
- **Sol reviewer: prohibited by default.** Do not invoke Sol for architecture, kernel retirement, or any other review unless the owner expressly authorizes that **one** review. Any authorization expires after that one invocation; older optional-review directives are withdrawn.
- Benchmark agent and main agent coordinate exclusive CPU measurement: no concurrent compilations/tests during timing. No multiple agents editing shared repo files. Use quiet cores and record host contention. Measure whole transforms with reusable plans/workspaces.
- On active `dev`, fetch before writing; inspect new commits and changed files; only fast-forward/rebase safely for unpublished local commits; never force-push, reset others' work or rewrite remote history. Preserve current owners' untracked work. Main agent is the only branch writer.
- Push coherent verified changes regularly rather than dozens of docs-only checkpoints. Use `reports/SHARED_KERNEL_REFACTOR_PROGRESS.md` for actual code changes, exact SHAs, a concise test/benchmark summary, complexity removed and next source action.
- No new feature branch for this main implementation. Do not modify `main`. Do not open a PR from `dev` to itself.

## Explicit non-goals and prohibitions

- No new dependency or nightly Rust/SIMD rewrite; retain Pulp and the existing FFT subsystem.
- No GPU/MPI/batched APIs, new Python distributions, unrelated packaging changes, external compatibility test resurrection or new C/Fortran ABI work.
- No f32-wide computation via f64 bridge in the default hot path; no trait objects or dynamic polymorphism inside SIMD loops.
- No giant mechanical source migration that rewrites algorithms and file hierarchy simultaneously; no speculative forced inlining.
- No false scalar/direct-spin equivalence, GL/CC/F1 quadrature equivalence, or S/A/S*/A* interchangeability.
- No removal of polar/small-workload/exceptional fallbacks without coverage; no changed numerical tolerances or suppressed failures.
- No expanding experimental `lmax == ...` condition chains to hide a generic slowdown.
- No repeated costly full test matrices between small edits. No large new benchmark framework or accumulating raw telemetry in Git.
- No requirement to create every target module as a separate file; avoid pointless boilerplate and pass-through wrappers.
- No change to Python runtime import paths `rustsht` or `spharm_rs` merely because Rust internals move.

## Acceptance and final report

At completion, the actual running code (not just a tree diagram) must show:
1. Equivalent f32/f64 scalar and native-spin recurrence algorithms share maintained source, using typed leaf SIMD when needed.
2. Validated f32 and f64 use the same paired/order-major execution architecture where eligible; original f64 wins remain materially intact.
3. Scalar/spin drivers and GL/CC/F1 reuse the same grid-independent Legendre and scheduling machinery; their distinct mathematical adapters remain correct.
4. `src/pyspharm/` is removed, with its numerical capabilities integrated into `legendre/pointwise.rs`, `evaluation.rs` and `grid/geodesic.rs` or clearly justified equivalents. The obsolete Rust module path is not retained as a compatibility shell.
5. Supported operators/adjoints, f32/f64 extreme cases, scratch reuse, exact/oversampled/reduced and all supported grids pass appropriate numerical validation.
6. Duplicate kernel implementations, unnecessary source branches and special cases are demonstrably reduced; quantify before/after.
7. The final Rust/Python test suite and applicable CI pass; all accepted performance trade-offs or improvements have exact baseline/candidate and pinned-DUCC provenance.
8. Updated internal module documentation states why any remaining precision-specific kernels and grid/operator adapters are necessary.

**Immediate next action:** Continue the already-in-progress scalar/spin shared recurrence work on `dev`. Do not detour into moving all files now. First finish one coherent shared numerical family, then merge the old `pyspharm` functions into their long-term owners as a standalone no-arithmetic-change commit, then tackle the paired/ordered f32 enablement. Keep implementation moving and avoid frequent validation of minor edits.
