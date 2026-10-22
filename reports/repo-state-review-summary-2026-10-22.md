# Repository state review — 2026-10-22

Static review of `opt/sht-performance-parity` at
`2ecd5a5592732fdc4f5b3808716090e55f9831e4`, focused on the latest eight
check-ins. Local HEAD matched the locally recorded origin branch; no fetch
was performed. No project code, tests, builds, benchmarks or audit scripts
were executed. Existing validation results below are historical evidence,
not results reproduced by this review.

The latest commits improve documentation and evidence, but the active scalar
CC directive remains incomplete. The main open findings from the preceding
independent review are still present in the current runner source.

## Findings

1. **High: the CC coverage generator still omits required low-degree bands.**
   `scripts/benchmark_ducc_full_transform_parity.py:318` uses
   `max(1, lmax // 4)` instead of the requested floor-quarter order. T1–T3
   therefore lack their required m=0 measurements. The coverage audit at
   line 1513 derives expectations from the same generator and repeats the
   clamp in its low-degree check, so its success cannot establish compliance
   with the owner manifest. Correct both expectations and generation for
   CC while preserving the historical GL contract. The committed independent
   audit records 2,528 required groups, 2,456 matching saved groups, 72 missing
   groups and 16 extra groups. Of the missing groups, 24 concern these
   low-degree shapes and 48 concern the unsaved high shard. These are recorded
   audit counts, not a fresh enumeration.

2. **High: measurement completeness still does not imply numerical acceptance.**
   The runner records finite error metrics without applying numerical
   tolerance gates. Its validation failure check at line 744 tests finiteness
   and reset consistency; the coverage audit accepts any non-null relative
   L2 result and can mark a group complete despite recorded failures.
   Separate timing coverage from numerical acceptance and reconcile stored
   random, basis and adjoint errors against established criteria before
   closeout. This is a reporting/validation gap, not evidence of a production
   operator defect. The previous review's cancellation analysis is relevant
   when interpreting small-dot adjoint residuals.

3. **Medium: interrupted shards remain vulnerable to loss of completed work.**
   `run_matrix` accumulates results until it returns; `write_artifacts`
   writes `results.json` afterward. The latest committed review records the
   final high shard stopped without saved JSON. Earlier untracked handoff
   text describes it as running and must be treated as a historical snapshot.
   Persist completed groups atomically and resume against frozen binary and
   configuration identities before another long recovery run. This review
   did not inspect live processes or determine the interruption cause.

## Latest check-ins and retained state

- `2ecd5a55` adds the detailed independent review, diagnostics and coverage
  audit without changing production kernels. Its findings are advisory;
  spin-1 investigation proposals do not supersede the active scalar CC
  sequence.
- `7b1629d4` preserves scalar benchmark summaries. These artifacts do not
  complete CC coverage or establish a replacement GL table policy.
- `a317ec46` resolves the stale `spharm.` references flagged after
  `b3ef6873`. Static inspection found no remaining such references in the
  compatibility tests; wheel and sdist configuration both name
  `spharm_rs.py`. Import/build behavior was not exercised.
- `75cf62ec` and `a81e0e41` document nominal SPHEREPACK versus exact CC degree
  limits. They do not change production numerical limits.
- `cd8beaa3` provides explicit CC worker selection and exact
  `ntheta=lmax+2` fixtures while retaining omitted-GRID GL support. Its
  coverage and persistence limitations above remain unresolved.

Production scalar tables still require GL, and the 20 MiB policy remains
in place. The table analysis SIMD branch additionally requires
`ntheta % 4 == 0` (`src/legendre/scalar_table.rs:589`), supporting the latest
review's recommendation to stratify future crossover evidence by executed
dispatch and cold/reuse behavior. It does not establish a new cutoff.
Plan 17 explicitly remains paused. Owner-approved Python integration and
the guarded paired FFT route remain intentional.

At inspection, tracked files were clean; numerous untracked benchmark
artifacts and a handoff were present and left untouched. This review adds
only this file. No commit, push, production change or new performance claim
was made. The next authorized implementation work remains CC manifest and
numerical closeout, followed by the measured GL crossover assessment.

Evidence: [active directive](../plans/scalar-f64-cc-full-band-and-gl-table-crossover-2026-10-22.md),
[latest independent review](f64-ducc-independent-review-2026-10-22.md),
[saved audit](data/f64-ducc-independent-review-2026-10-22/audit.json).

## Follow-up step 1 — recovery commands and provenance

The follow-up review remains at `2ecd5a55`. A read-only diff from `cd8beaa3`
through HEAD found no changes to `src/`, Cargo manifests/lockfile, either
native scalar worker or the scalar benchmark runner. The findings below
concern the current harness and its new handoff, not a newly introduced
production numerical regression.

- **Medium, concrete handoff error:**
  `reports/handoff-f64-independent-review-2026-10-22.md` recommends
  `--fixed-mmax 0`. The parser at
  `scripts/benchmark_ducc_full_transform_parity.py:2176` exposes `--mmax`,
  with no `--fixed-mmax` alias. The documented recovery command will be
  rejected. Its corrected selection fragment is
  `--mmax 0 --lmax 1 --lmax 2 --lmax 3 --grid CC --dense-sweep`.
  This can select the six missing low-degree geometries without changing
  worker binaries or repeating the existing low-degree sweep. No command
  was executed in this review.
- **Medium, recovery/merge mismatch:** `merge_shards` requires identical
  `rust_source_revision` across shards (line 2069), while metadata obtains
  that value from the checkout's current HEAD even under `--skip-build`
  (line 815). The saved campaign used harness revision `cd8beaa3`; a recovery
  launched from current HEAD records `2ecd5a55`. Consequently, the documented
  frozen-binary recovery from the advanced checkout cannot merge with those
  older shards under the current invariant, even if binary hashes and
  production code match. Resolve this before expensive recovery work.
  Preserve each shard's real runner revision and explicitly validate
  permitted harness-only differences; retain strict equality of worker
  hashes, production baseline, DUCC revision, geometry and measurement
  protocol. Do not relabel a new run as an old source revision or silently
  weaken all merge checks.
- **Provenance limit:** `--skip-build` records current toolchain/flags and
  binary hashes but does not establish that an existing binary was built
  from those sources with those tools. Compare against the frozen build
  record before reuse, keeping build identity separate from the source
  revision of the reporting process. The independent review provides the
  expected frozen hashes; no new hash verification occurred here.

Next action: correct the handoff selector and define recovery provenance
before starting missing cells. Keep all thirteen original shards immutable.

## Follow-up step 2 — numerical acceptance and measurement reporting

Further static inspection narrows the acceptance gap:

- Workers calculate `reset_consistent` for every timed operation/process,
  but the runner checks it only through `compare_validation` when
  `operation == "analysis" and pair == 0` (lines 601 and 744). A synthesis
  reset failure, or one in later analysis pairs, is therefore recorded in
  samples without automatically becoming a failure. The existing audit
  reports all saved reset flags true, so this is a missing future acceptance
  gate, not a claim of corruption in the saved campaign.
- `compare_validation` records `edge_cases.zero.exact_zero` but does not
  include it in its failure decision. Roundtrip arrays and adjoint residuals
  also lack explicit finite-value checks in that decision. Require all
  expected validation records and their individual acceptance results;
  finite edge arrays alone do not establish numerical acceptance.
- `aggregate` publishes one `batch_count`, taken only from Rust, and forms
  ratio percentiles by zipping Rust and DUCC batch lists (lines 902–903 and
  962). Each worker independently chooses five long-call or ten short-batch
  observations. When they differ, `zip` silently leaves some observations
  out of the ratio distribution although both latency distributions retain
  them. The latest audit records actual counts of 15/20/25/30. Publish counts
  per backend and the timing mode per process; distinguish ratios of process
  summaries from ordinal batch ratios. Thirty batches within three processes
  are not thirty independent process runs.
- `measurement_status` and the coverage audit currently measure matched
  process presence, not passing numerics or compliant batch durations.
  Keep these separate in the machine-readable output and figure annotations.
  Partial rows should remain available, but should not appear indistinguishable
  from complete accepted rows in a final comparison.

Next action: reconcile existing saved metrics without rerunning transforms
first. Add explicit acceptance outcomes using established tolerances, exact
zero/reset requirements and both adjoint residual definitions. Preserve
outliers, host-contended observations and the original raw sample counts.
Rerun only cells whose missing evidence or measurement quality requires it.

## Follow-up step 3 — scope and evidence from the newest review commit

**Medium, conflicting next-step guidance:** the newest detailed review ranks
spin-1 first under the phrase "owner's priority order", and its compact
handoff says later owner steering prioritizes spin-1. The owner instructions
provided for this session and the active directive instead require scalar
CC closeout followed by GL crossover, with unrelated spin-1 performance work
closed. A statement inside an advisory handoff is not itself a new owner
instruction. Preserve the spin-1 evidence, but reconcile these next-step
paragraphs against explicit owner direction before using them as an execution
queue. No spin-1 implementation is recommended as the next action here.

The production source supports the scalar bottleneck distinctions:

- `src/transform.rs:1135` limits ordered scalar synthesis to GL; CC falls
  through to the general recurrence route.
- `src/transform.rs:1210` applies
  `with_min_len(parallel_groups.div_ceil(execution_threads))` to scalar
  synthesis groups. This is consistent with the previously recorded
  four-worker q8 observations; source inspection alone does not establish
  an improvement from changing it.
- `src/transform.rs:2199` calls serial `to_prepared` for scalar CC analysis.
  The reported T720 resampling floor therefore has a concrete source basis.
  Its diagnostic interval also includes normalization/latitude setup and
  must not be equated to DUCC's narrower resampling interval.
- `src/legendre/mod.rs:20` retains the required 256-pair analysis tile.
  No evidence from this review justifies revisiting rejected tile sizes,
  FFT changes, loop reversal or the prior packed/direct-accumulation work.

Next action: finish scalar CC evidence and attribution before GL crossover.
Treat task-grain, ordered-synthesis adaptation and parallel scalar resampling
as separate possible later experiments, each requiring its own numerical
controls and whole-transform A/B; none was implemented or benchmarked here.

## Follow-up step 4 — failure preservation and figure correctness

**Medium, nonfinite validation can prevent saving the entire shard.**
`compare_validation` calls `error_metrics` on basis/edge arrays before
checking their finiteness (lines 484–507). An edge-only NaN or infinity can
therefore enter the returned metrics even though `finite_outputs` is false.
`write_artifacts` then serializes the whole result with `allow_nan=False`
(line 1647), which rejects those metrics before writing `results.json` or
the later failure CSV. This defeats the intended failure-recording path.
Unlike edge validation, the main random-output comparison already represents
nonfinite metrics as `None`. Apply the same explicit failure representation
to all sidecar metrics and preserve failed records independently of final
aggregation. This is a statically traced failure path, not the established
cause of the historical missing high shard.

Two plot-selection defects matter when recovering the requested cases:

- **Medium:** reduced-order ratio plots select every row with
  `mmax != lmax` (line 1275), and the CC/GL figure labels the same selection
  "quarter mmax" (line 1485). Keeping the existing extra T2/T3 m=1 data and
  adding their required m=0 rows produces two different spectral bands at
  the same reduced-series degree. Filter the principal CC reduced series
  explicitly by `mmax == lmax // 4`; retain historical extra cells as labeled
  supplementary data. The auxiliary reduced/full plot also retains the
  original `max(1, T // 4)` predicate at line 1702 and needs the same
  grid-aware correction. Explain how shared T0 is represented.
- **Medium for noncanonical inputs:** the CC/GL join key omits `nphi`
  (lines 1471–1473), despite the chart subtitle promising matching longitude
  counts. A legal `--fixed-nphi` value can still have the `even_smooth` label,
  so equal labels do not guarantee equal geometry. Match exact `nphi`,
  verify grid/latitude conventions, and reject duplicate join keys instead
  of silently retaining the last GL row. No mismatch in the existing
  canonical saved cells was demonstrated here.

The runner writes SVGs only. The requested seven CC figures also need PNG
companions; the prior GL report documents a separate SVG rendering step.
Include that explicit artifact step in closeout rather than treating a
successful merge or `--plot-only` as all required figures completed.

Next action: use small saved-record fixtures to check nonfinite failure
serialization, required-versus-extra band selection, unequal `nphi`, duplicate
joins and partial-row labeling before regenerating final figures. These
checks need no transform workload when implementation resumes; they were
not run during this review.

## Follow-up step 5 — Python rename and actual test coverage

The rename is internally consistent after `a317ec46`: the module, test
imports, wheel inclusion and sdist inclusion use `spharm_rs`. Root Maturin
configuration excludes `python/pyspharm-rs/**`, preserving the separate
distribution. The compatibility module's `__version__ == "1.0.9"` is an
existing compatibility value, whereas the distribution version remains
`0.1.0.dev0`; this review does not treat their difference as a rename defect.

**Medium, automation gap:** `.github/workflows/ci.yml:42` runs only
`pytest python/tests`. It neither installs the separate compatibility project
nor runs `python/pyspharm-rs/tests`. Thus the normal Python CI job would not
have caught the stale names fixed by `a317ec46`. The checked-in independent
review validation record contains Rust/native checks, not post-rename Python
package build/import evidence. This does not establish that no manual Python
validation exists elsewhere; it identifies what these committed checks prove.

The renamed windspharm test accurately checks consumption of arrays produced
by `spharm_rs`. It imports `windspharm.standard` independently and skips when
unavailable. It neither routes windspharm through RustSHT nor establishes
that `pyspharm-rs` satisfies windspharm's distribution dependency. Plan 18's
updated explanation correctly preserves that distinction.

Next action, when code execution is permitted: run the compatibility suite
in its own project environment, then build and install its wheel/sdist
against a built RustSHT wheel in a clean environment outside the checkout.
Check that the artifact supplies `spharm_rs.py`, does not supply `spharm.py`,
and does not bundle a second Rust extension. Add the existing compatibility
suite to CI so the accepted rename stays covered. Keep windspharm optional
and report its skip explicitly; no publication or numerical-kernel work is
needed for this validation. None of these checks ran during this review.

## Follow-up step 6 — what the CC and GL evidence still needs

The latest `scalar-stage-summary.csv` contains T720 timing breakdowns:
Rust analysis full/reduced q1/q8, Rust synthesis full q1/q8, and DUCC full
analysis/synthesis q1/q8. Its route-counter companion adds scalar synthesis
observations at T255 and T1023. This is useful attribution, but it is not the
directive's complete diagnostic-size coverage at T63/T127/T255/T511/T720/
T1023 and selected larger sizes. Reduced-order synthesis stage timings and
matched reduced-order DUCC stages are also absent from this particular
summary. Reconcile other existing evidence before collecting anything new;
do not infer stage coverage from end-to-end rows or spin-1 profiles.

The stage evidence has explicitly documented limits: the Rust timing
executable's capture-time hash was not retained, backend stage runs were not
interleaved, and some early runs overlapped another workload. The diagnostic
source patches are preserved and frozen production hashes are recorded.
Keep these distinctions in the eventual CC report; the diagnostics locate
work but do not establish a new optimization gain or an exact cross-library
kernel ratio. Never replace the missing timing-build hash with a later
counter-build hash.

The table source confirms three separate constraints that a GL study must
keep distinct:

| Constraint | Current source meaning | Consequence for the next study |
|---|---|---|
| Production eligibility | GL, current degree range, supported scalar type, and `mmax >= lmax - lmax / 4` | Forced quarter-order tables are diagnostics, not evidence for changing production eligibility. |
| Analysis selector | Exact-grid scalar q1 recurrence at estimated payload >=20 MiB | Reassess this decision without changing allocation/range safeguards. |
| Table execution | Cached two-complex-lane path requires `ntheta % 4 == 0` and sufficient scratch | Compare like dispatch classes; degree neighbors can execute different accumulators. |

The existing forced-route test eagerly builds a table, times the first table
call, then the first recurrence call on the same plan/workspace. Its report
already discloses this sequence. Those numbers cannot answer the requested
independent cold-plan/reuse question, even if the warm interleaving is sound.
A recurrence-only cold control must avoid keeping an already constructed
table resident. Retain the 20 MiB rule provisionally; the available evidence
does not establish a replacement threshold.

Next action after CC closeout: reuse the existing full-order evidence and
fill the missing focused GL cells, especially T192 and T215–220, including
T217/T218. Include eligible near-full order `mmax=lmax-lmax/4` at selected
T127/T128/T191/T192/T217/T218, both longitude shapes, and selected q8/synthesis
controls. Keep one current source/build identity, verify actual forced
routes, and collect independent processes with calibrated short batches.
Then compare fresh-plan total costs at reuse 1/2/4/8/16/32/64+, preserving
build time, first use, warm latency and resident bytes separately. Reserve
unfitted neighboring shapes in each dispatch class to check any proposed
simple rule. A documented decision to retain the cap is a valid outcome;
another arbitrary byte or degree cutoff is not required.
