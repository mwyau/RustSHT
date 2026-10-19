# Native Rust scalar-analysis execution parity with DUCC

**Active, focused implementation plan — 2026-10-18 US Eastern.**
**Branch:** `opt/sht-performance-parity`.
**Reference:** DUCC source `80388904b8cfc32400ba84af8a6a45aabfe60f86`, principally
`src/ducc0/sht/sht_inner_loop.h` (`fill_m2a`, `init_lambda`, `iter_to_ieee`,
`calc_map2alm`, `map2alm_kernel`, `inner_loop_m2a`) and `src/ducc0/sht/sht.cc`.
**RustSHT starting point:** `15f4d28` or newer production head: `src/transform.rs::analyze_symmetric_order_group`,
`src/legendre/scalar_paired.rs` and its fast/fallback accumulation helpers. Verify exact
source and dispatch before modifying it. DUCC names appear here only as external references;
new RustSHT symbols/modules must use neutral algorithmic names. Preserve GPL notices/provenance.

## Objective and scope

**Goal:** implement a faithful, efficient **native Rust** scalar-analysis execution path matching
DUCC's *whole algorithmic data flow*, not just its Legendre polynomial formulas. Close as much
as is demonstrably possible of the measured ~2× T720 analysis gap, preserving correctness,
range safety, bounded scratch and the existing public API. **Parity is a testable goal, not a
promised result.** No wholesale FFT port, scheduler redesign, new dependency, or spin rewrite.

Focus initially on **f64 scalar GL STANDARD analysis (A)**. Retain distinct `S`, `A`,
`S*` and `A*` operators, normalizations, `m=0` reality, polar and Nyquist rules, GL
weights, nonzero `phi0`, reduced `mmax`, and the existing strong finite/subnormal/overflow
fallback. Preserve other supported f32, CC/F1, spins and adjoints untouched until expressly
validated. Use `pulp` runtime SIMD/portable scalar and Rayon as applicable.

## Evidence and what not to assume

Source: [T720 analysis stage report](../reports/scalar-analysis-stage-profile-2026-10-18.md)
at `fbefc60d` and follow-up `9e9c567`; [handoff](../PERFORMANCE_STATUS.md).

- T720 f64 GL 721×1442 full-mmax, q1: uninstrumented Rust/DUCC
  **41.822/19.443 ms = 2.15×** before cosine preweighting; q8 **7.507/3.245 = 2.31×**.
  The later cosine-preweighting screen found **41.182/19.215 = 2.14×** q1
  and **6.317/3.118 = 2.03×** q8. These compare different setups and source
  checkpoints; never combine numerator/denominator across screens.
- Instrumented *recurrence + accumulation* q1 **34.166/11.120 ms** Rust/DUCC
  (3.07×); q8 **5.123/1.760 ms** wall stage (2.91×). Worker-CPU sums are
  **not** wall times; substage boundaries differ. FFT, input packing, coefficient
  setup and q8 worker utilization are not the primary measured gap.
- At T720, **474 of 3,384 active blocks declined the fast route at
  sectoral initialization**, 2,910 succeeded, **zero recurrence declines** and
  **zero completed degree loops discarded**. Declined blocks nonetheless
  computed ~6.61 million lane-degree pairs via checked mixed/scaled fallback;
  callback time was ~11.82 ms q1. This is real recurrence work, **not** mostly
  replay of failed ordinary loops. Do not optimize by removing necessary checks
  or by blindly eliminating the fallback.
- First fused four-degree/two-pair Rust analysis prototype worsened q1 by
  ~8% and q8 by ~12%. Archived in
  `backups/retired-kernels/scalar-analysis-four-degree.patch`. **Do not copy it
  unchanged.** Larger live SIMD state and repeated input processing are
  plausible causes, not hardware-counter-confirmed findings.
- Newly accepted SIMD sectoral initializer, opposite-parity cosine preweighting,
  analysis order scheduling/packing and spin-1 per-ring scales already exist.
  **Do not reimplement them.** No hardware `perf` events are accessible
  (`perf_event_paranoid=4`); do not change host permissions.

## Technical execution crosswalk

| DUCC implementation | RustSHT counterpart / gap | Required investigation or implementation |
| --- | --- | --- |
| `fill_m2a`: pack active ring pairs; stable `csq`; weight Fourier data, preweight opposite parity; SIMD padding | `prepare_symmetric_analysis_pair_block`, `analysis_same/opposite_*`, cosine preweighting | Preserve compact lanes and SIMD tails. Test precomputed stable `cos²(theta)` (near poles `(1-sinθ)(1+sinθ)`) and reuse without numerical regressions or needless repacking. |
| `init_lambda`, `mypow`, `Tvnormalize` | `initialize_analysis_sectoral_lanes`, `sectoral_value`; mixed/unsafe blocks decline | Use DUCC-equivalent mantissa/exponent lane state for both normal and scaled paths; maintain f64 finite and subnormal guarantees. |
| `iter_to_ieee` | Rust callback-based mixed/scaled handling; no equivalent shared insignificant-prefix transition | Advance negligible early degrees efficiently **only under a validated error policy**, while tracking individual SIMD scale epochs. Determine whether same block can transition to a fast ordinary path after scaling. |
| `calc_map2alm`: scaled accumulation, conditional `rescale`, correction factor, transition into IEEE-range recurrence | Rust generic callback evaluates declined blocks through every degree | Make scaled-to-ordinary transition explicit and efficient; preserve contribution of all numerically significant early degrees, no unsafe zeroing, no allocation per degree. |
| `map2alm_kernel`: two-pair degree progression and SIMD coefficient reduction | `FusedScalarAnalysisPair`, `AnalysisPairAccumulation` currently separate traversals and per-pair reductions | Match data reuse and useful per-degree work with **bounded register pressure**, beginning with a one-pair combined traversal. Explore two-pair scheduling only after actual assembly/benchmark evidence. |
| `inner_loop_m2a`: latitude batches and degree tiling `lstep`, coefficient adjustment | `analyze_symmetric_order_group` per-order blocks, output commit | Compare exact coefficient adjustment/parity and whether degree tiling improves cache behavior. Do not mechanically copy a tile size; only adopt if measured. |

## Work sequence and decision gates

### Gate 0 — Reproducible source and fair kernel comparison

1. Fetch current branch and identify the **actual production fast and fallback paths**.
   Inspect the latest report and accepted changes; do not assume `fbefc60d` is HEAD.
2. Build a **small matched native C++ DUCC scalar-analysis driver** and Rust
   private/test benchmark that accept equivalent *prepacked Fourier/weighted
   ring input*, harmonic order ranges, prepared coefficients and reusable scratch.
   Reproduce `A` for the same mathematics, with no Python wrapper inside the
   measured kernel. Report packed-state preparation **separately** so optimizations
   cannot merely move work outside the timer. Also retain end-to-end public
   analysis measurements.
3. Test **T720 GL 721×1442 full `mmax`**, q1/q8, and **T300 GL 301×603,
   `mmax=150`**, q1/q8. Within T720, record matched ordinary-only
   and mixed/scaled block samples, plus total lane-degree work, scale
   transitions, SIMD vector counts and fallback frequency. Establish identical
   output/normalization/phase, comparable CPU target and release flags, compiler
   versions, warmups and timings. No synthetic kernel-only shortcut that omits
   real packing, polar limits or final coefficient adjustment.
4. Separate **(a) pack/setup, (b) scaled-prefix and correction, (c) ordinary
   recurrence/accumulation, (d) output adjustment/commit, (e) FFT and q8
   scheduling**. Show Rust/DUCC *absolute times* and sample variability.
   No invented hardware counters; if codegen matters, inspect optimized symbolized
   assembly and spills where feasible.

**Gate 0 outcome:** isolate which *equivalent* work is slow (especially the ~474
initially scaled blocks). Do not write another broad 21-cell campaign.

### Gate 1 — Faithful scaled-to-ordinary transition (first structural change)

- Introduce an order/batch-local SIMD recurrence state carrying each lane's
  basis pair and safe scale/correction metadata; normalize and advance
  insignificant prefixes as in DUCC where mathematically justified. Accumulate
  contributions while scaled and transition to ordinary SIMD **when safe**.
- For typical mixed blocks, avoid generic callback/degree-level dispatch for
  the full remainder when the DUCC-like transition would allow direct SIMD.
  Where a lane cannot transition, retain checked scaled computation. Maintain
  transactional coefficient output and a **small robust fallback** for genuinely
  unsupported/unsafe cases; no unbounded buffers or per-degree allocations.
- Before promoting: compare mixed/scaled block kernel time and normal block
  performance separately; prove agreement for polar rings, very high `m`,
  true underflow/subnormals, large finite spectra, nonzero phase, tails,
  missing/inactive rings and `m=0`.

**Decision:** if only tiny fractions transition or the kernel slows the common
path, reject/archive the experiment and report the structural reason rather
than adding another dispatch layer.

### Gate 2 — Ordinary recurrence and accumulation layout

- First implement a **single-pair, single-pass latitude kernel** that shares
  basis, Fourier values, `csq` and safety checks without doubling the live
  two-pair accumulator set. Compare with the current code and DUCC at
  *equal useful work*.
- Retain register-resident state where worthwhile, use coefficient broadcasts
  across adjacent latitude vectors only when it lowers measured work, and
  inspect scalar tails and SIMD reductions for repeated loads, spills and
  hidden calls. Consider a small two-pair or multi-vector layout **only** if
  it corrects the rejected prototype's register-pressure/work duplication.
- Use matched absolute kernel stage timing and whole-call timing to choose
  a winner. Do not accept a normal-only speedup if mixed-case regression
  makes the public transform slower.

### Gate 3 — Integration and acceptance

- Integrate the winning native f64 GL analysis path behind an
  **algorithmic/private neutral Rust name**; prefer consolidating existing
  `scalar_paired.rs` machinery over permanent competing large kernels.
  Retain one minimal checked fallback for regimes where genuinely necessary.
  Archive rejected prototypes under `backups/retired-kernels/` with rationale.
- Correctness: pinned DUCC **and** independent analytic harmonics and
  weighted-adjoint tests. Verify `A`, `S*`, `A*` distinctions, quadrature,
  `m=0`, reflected parity, odd/even `nphi`, reduced `mmax`,
  nonzero `phi0`, polar SIMD tails, underflow/overflow, GL/CC/F1 nonregression,
  spin 1–4/f32 nonregression. Do not relax tolerances, or silently alter
  the public API and normalization.
- Tests: `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --release --workspace --all-features`,
  `cargo test --release -p rustsht --no-default-features`; Python checks only
  if bindings are affected. Explicitly state any unavailable tests.
- Performance: compare q1 and q8 at T720 and irregular/reduced T300 with
  matched optimized flags. **Stretch objective: within 10–20% of DUCC's
  comparable Legendre stage** on representative eligible cases, with no
  material safety/regime regressions. This is **an acceptance aspiration, not
  an expected or guaranteed speedup**. If still far behind, document
  *measured residual component gap*; use disassembly/instruction-flow
  comparison next, not arbitrary tuning.

## Progress and stop conditions

- Commit meaningful, tested milestones on `opt/sht-performance-parity`;
  check out/fetch upstream DUCC source read-only or in a separate local
  worktree. Never commit changed vendor/DUCC sources or permanent inner-loop
  timers to production. At most **one Luna benchmark-only subagent**; no Sol.
- Per checkpoint, record: exact Rust/DUCC source hashes, compiler flags,
  benchmark workload/operation boundary, active/mixed lane counts,
  correctness errors, elapsed wall time q1/q8, relevant substage times,
  decision (accept/reject), and smallest remaining measured bottleneck.
  Update `PERFORMANCE_STATUS.md` only after a real implementation milestone;
  keep raw host/PSI/process diagnostics untracked per `AGENTS.md`.
- **No stalls:** if profiling counters, permissions, or a backend are unavailable,
  use bounded timers and source/assembly analysis, record limits and continue
  through a valid small implementation experiment. If the expected gap cannot
  be closed, deliver a complete evaluated kernel and explicit residual instead
  of indefinite experiments.
- **Before every push:** inspect worktree and `git fetch origin`, compare
  branch ancestry, safely reconcile concurrent commits without force-pushing
  or dropping another agent's work, rerun affected checks, push normally and
  verify the new remote HEAD. No main-branch changes without authorization.
- After Gate 3, continue the roadmap's **spin-1 analysis/synthesis**, native
  f32 and appropriate CC/F1/spin-2 work. Share only mathematically compatible
  implementation pieces; preserve all established operators and grids.

## References

- [Analysis stage profile and decline counters](../reports/scalar-analysis-stage-profile-2026-10-18.md)
- [Latest performance handoff](../PERFORMANCE_STATUS.md)
- [DUCC operation/grid contract](06-ducc-contract-crosswalk.md)
- [Kernel retirement](08-kernel-naming-and-retirement.md)
- [DUCC algorithms, historical implementation](https://github.com/mreineck/ducc/tree/80388904b8cfc32400ba84af8a6a45aabfe60f86/src/ducc0/sht)
