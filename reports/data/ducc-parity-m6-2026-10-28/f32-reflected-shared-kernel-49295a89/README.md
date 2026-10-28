# Shared-kernel reflected f32 scalar-synthesis screen

**Status:** substantial candidate result; not promoted. The candidate is based
on `49295a89`, while current `dev` is `0ce209a1`. Reconcile it with current
f32 analysis changes and repeat the affected measurements before promotion.

## Source and implementation

The exact parent was `49295a89f2d13d8b02c2cbfa8566a8c5c21c6a80`; the pinned
DUCC 0.42.0 source was `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The
candidate patch SHA-256 is
`f62628d42c21db6bada73ee6b0d0ad5001b67e319ed5370c7b77fc2e01b3af31`.
The archived patch passes `git apply --cached --check` against the exact parent.
It dispatches only GL T720 f32 scalar synthesis at full/reduced mmax. The
reflected-order adapter advances the northern rings once and emits both
hemispheres. It reuses the shared `FusedSynthesisBlock`, paired recurrence
visitor, and `F32SynthesisAccumulator`, adding mirrored accumulators to the
existing fused and scaled-fallback leaves.

Rust/DUCC comparisons used `-C target-cpu=x86-64-v3` and
`-O3 -ffast-math -march=x86-64-v3 -ffp-contract=fast`, respectively. The
machine was an AMD Ryzen 9 5950X, rustc 1.99.1 / LLVM 23.1.1, GCC 15.2.0.
Workers were pinned to CPU 0 for q1 and CPUs 0,2,4,6,8,10,12,14 for q8.
Inputs came from `benchmark_ducc_f32_native.py make_fixture`, with `phi0=0.23`;
the GL cases used `ntheta=721`, `nphi=1458` at `mmax=720`, and `nphi=378` at
`mmax=180`. Each synthesis cell has 15 interleaved fresh-process
parent/candidate/DUCC pairs with warm calls before timed batches.

## Whole-transform results

Times are medians in milliseconds. Candidate/parent and candidate/DUCC are
medians of the per-pair ratios.

| Cell | Parent | Candidate | DUCC | Candidate/parent | Candidate/DUCC | Faster pairs |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 653.368 | 357.806 | 19.000 | 0.549 | 18.76x | 15/15 |
| Reduced q1 | 194.691 | 117.958 | 8.291 | 0.604 | 14.23x | 15/15 |
| Full q8 | 99.466 | 46.935 | 3.219 | 0.471 | 14.58x | 15/15 |
| Reduced q8 | 55.479 | 18.621 | 1.363 | 0.350 | 13.18x | 15/15 |

The latest production scalar-synthesis baseline recorded in the ledger remains
32.08x/36.94x full and 22.77x/42.43x reduced q1/q8. The screen above is an
older exact-parent result and is not a current-head candidate ratio.

## Route and instruction evidence

In a separate feature-instrumented route census, the full-mmax route handled
80.730M northern degree terms: 26.984M through the fused block and 53.746M
through the scaled fallback, across 3,384 blocks (1,221 fused, 2,163
fallback, zero fused declines). The reduced route handled 40.662M terms:
18.682M fused and 21.980M scaled, across 1,086 blocks (527 fused, 559
fallback, zero fused declines). These terms produce both reflected outputs.
The parent all-ring path counted 161.199M/81.210M useful output terms, while
DUCC counted 80.730M/40.662M northern terms. This candidate therefore executes
the same northern recurrence visit count as DUCC on these shapes.

The q8 instrumented route scheduled 181 four-order jobs full and 46 reduced;
all eight workers received work. Instrumented per-worker stage times ranged
46.52–46.74 ms full and 13.88–16.84 ms reduced. These counters are diagnostic
and are not production latency measurements.

One q1 full-mmax Callgrind run collected 6.612B parent Ir, 3.661B candidate
Ir, and 260.801M DUCC Ir. This is a 44.6% candidate instruction reduction
from the parent. Normalized by the same 161.199M Rust output terms, the counts
are 41.02/22.71/1.62 Ir per term for parent/candidate/DUCC. The mirrored DUCC
count accounts for both outputs from northern work. Ir counts are not cycles.
The candidate profile resolves to the shared reflected accumulator, and the
AVX2/FMA disassembly is retained here. The sanitized per-pair numerical data
is in `sanitized-paired-results.json`, adjacent controls are in
`sanitized-control-results.json`, and route counts are in
`sanitized-stage-route-results.json`. A separate warmed stage profile used
instrumentation patch ID `5b60814d`; its stage timings are diagnostic and are
not attributed to the timed patch `f62628d4`. The profile-parent and
profile-candidate instrumentation patches are also archived with SHA-256
`57798428...a3da3f` and `d211f55c...bcc2e6`, respectively.

## Numerical and adjacent controls

Across the 15 synthesis pairs, candidate/parent relative L2 stayed below
`1.14e-7` and maximum absolute difference below `2.09e-7`. Candidate/DUCC
errors were `1.7234e-4` relative L2 / `9.6441e-4` max absolute full and
`2.7255e-4` / `1.5479e-3` reduced, matching the parent to the reported
precision.

Three-pair adjacent screens produced bitwise-equal parent/candidate outputs.
Candidate/parent paired medians were `0.967/0.970` full scalar analysis q1/q8,
`0.980/0.965` reduced scalar analysis, `1.004/0.999` spin-1 analysis, and
`1.003/1.012` spin-1 synthesis q1/q8. Protected f64 GL T1023 spin-1 synthesis
medians were `0.984` q1 and `0.975` q8; one of three q8 pairs was `1.075`.
These controls are initial screens, not promotion-grade extensions.

On the isolated candidate worktree,
`cargo test --locked --test f32 f32_t720_ -- --nocapture` passed 2 tests,
covering full/reduced analytic modes, q1/q8 scalar adjoints, sectoral modes,
subnormal output, overflow/nonfinite decline, and workspace reset.
`cargo test --locked --lib scalar_paired::synthesis::tests` passed 5 tests.
These checks do not replace exact-current polar, extreme-value, tail,
transactional-reset, or protected-performance validation.

## Decision and next step

The data supports a material algorithmic improvement: it reuses northern
recurrence work for both hemispheres and cuts full/reduced whole-call time by
40–65% across the four cells. Rust remains 13.18–18.76x DUCC in this older
screen. Keep the candidate unpromoted until its dispatch and tests are
reconciled with `0ce209a1` (current changes since the parent include
`src/legendre/mod.rs` and `tests/f32.rs`), then repeat the four 15-pair
synthesis cells and extend f32 spin-1 synthesis q8 and protected f64 T1023
spin-1 synthesis q1/q8 to resolve the observed spread. Run the full
exceptional and workspace-reset matrix on the reconciled candidate. The exact
candidate patch, per-pair numerical summaries, stage diagnostics, sanitized
Callgrind annotations, assembly, and benchmark scripts are retained alongside
this report.
The parent and candidate Callgrind worker sources are retained as separate
files.
The archived scripts use `M6_EXPERIMENT_ROOT` for a fresh scratch directory
and `M6_REPO_ROOT` for the checkout; set both before rerunning them.
Unmodified Callgrind profiles, machine telemetry, and raw per-run captures stay
in the local untracked `raw-local/` directory. Committed summaries and
disassembly have local paths removed.
