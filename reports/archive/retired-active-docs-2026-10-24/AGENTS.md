# RustSHT — active agent instructions

**Branch:** `opt/sht-performance-parity`. Read **only** this file, the [active spin-1 plan](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md), the [current performance summary](PERFORMANCE_STATUS.md), and code touched by the current experiment. Historical plans, reviews and reports are **on demand**, not startup context. [Pre-update instructions preserved](reports/archive/context-2026-10-23/prepared-scalar-adapter/AGENTS.md).

## OWNER OVERRIDE — complete DUCC parity on ALL f64 spin-0 and spin-1 cases (2026-10-24)

**The finish line is reproducible native DUCC 0.42.0 parity or better in every covered workload**, not merely removing >2x gaps, achieving near-parity or finishing the latest experiment. Scope: GL/CC/F1; spin-0 and spin-1; synthesis and analysis; full and reduced mmax; representative T255/T511/T720/T1023 where supported; and 1/2/4/8 threads. Preserve S*/A* numerical/adjoint guarantees and measure adjoint paths that have distinct performance kernels. No faster cell offsets a slower cell. Do not call parity complete while any supported cell is GAP/UNMEASURED/INCONCLUSIVE.

**Immediate order:** finish any running bounded implementation experiment, then optimize the largest absolute gaps: GL T1023 spin-1 synthesis q1 (+51.1 ms reported), GL T720 spin-1 analysis q1 (+22.1 ms), GL T720 spin-1 analysis q8 (+3.05 ms). Subsequently close **all** residual gaps, including CC T720 full/reduced q8 (~1.24x/1.45x), GL T511 q8 (1.33x), CC T1023 q8 (1.13x), and any yet-unmeasured scalar/spin-1 combinations. Prioritize absolute time saved and shared hot-loop mechanisms, not ratio thresholds.

**One experiment at a time:** source-level hypothesis → focused correctness/poison/extreme/adjoint checks → matched exact-parent/candidate/pinned-DUCC A/B. Reject negative mechanisms after one targeted correction; switch to another still-open workload rather than declaring success. GL T720 q8 analysis is recurrence-bound: 599/3,579 blocks take checked mixed/scaled recurrence, across 475/721 orders and ~80% of estimated recurrence work. Investigate the **checked scaled recurrence/accumulation itself**. Do not repeat failed 256-pair widening, scan fusion, route-cache-only or channel replay. Preserve checked AVX2 exponent safety and polar fallbacks.

**Coverage:** follow [the all-cases parity ledger](plans/f64-ducc-parity-all-cases-2026-10-24.md). During development benchmark only decisive cells; when changes stabilize, run the full representative matrix on a quiet matched host. Track exact SHA, source/protocol, per-cell median, variation and correctness. Treat noise as INCONCLUSIVE, not PASS. The two reduced-order CC/F1 baseline q1-vs-q8 test failures remain open for diagnosis; never loosen thresholds.

**Reviewers:** no reviewer or other subagent unless explicitly requested by user. Keep f32, spin>1 optimization and unrelated changes paused until f64 spin0/1 parity is verified.

## Historical phase priorities (superseded by owner override above)

1. **Spin-1 analysis closeout is complete.** The current open GL T720 full q8 analysis case is **1.71×** DUCC (7.317 ms versus 4.267 ms on exact parent `53ef4f2b`); the earlier 2.08× figure is stale. Treat this as the next analysis investigation after the already-closed q1 analysis work.
2. Preserve all accepted synthesis changes. The checked AVX2 scaled-prefix accumulator improved GL T1023 full q1 to **176.605 ms** from **193.242 ms** on its exact parent; it remains **1.41×** DUCC. Current synthesis ratios are in [PERFORMANCE_STATUS.md](PERFORMANCE_STATUS.md); T720 F1/GL q8 are at or below DUCC, while CC T720 full/reduced q8 are 1.24×/1.45×.
3. Continue with one bounded, source-grounded mechanism at a time. Do not repeat the rejected q1 dispatch no-op, checked reflected-prefix mappings, or 256-pair spin-1 analysis block. Keep f32 and broad benchmark sweeps paused.
4. **Reviewer policy:** do not use reviewers or other subagents unless the user explicitly asks. If asked for a review, use only a Sol reviewer.

## Required correctness/implementation guardrails

- Preserve `S/A/S*/A*` and spin-0/1/other unaffected operations, GL/CC/F1, poles/equator, m=0, full/quarter orders, phi0, even/odd longitude/Nyquist, q1/q8, SIMD tails, all scaled/subnormal/nonfinite/extreme cases, poisoned scratch, output reset and fallback atomicity. **Never relax tolerances to create a fast path**.
- Keep stable Rust, current Pulp SIMD, bounded worker-owned scratch and existing licensing. Compare against pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Leave scalar CC/F1 improvements, approved Python API and odd-FFT behavior unchanged.
- **Known failures:** two independent full scalar-ordered calls ~15.056→24.029 ms at CC T720 q8; eight-accumulator reflected fusion spills; C64 category packing loses full f64 lane width; 8-degree tiling and checked reflected-prefix candidates regressed; the q1 order-major gate was a no-op. Preserve the accepted **internal prepared-component adapter** and the accepted checked AVX2 prefix accumulator; do not repeat rejected mappings unchanged.
- **Experiment scope:** the prepared-component adapter is complete. For the next case, run one focused correctness gate and one matched whole-transform A/B against exact parent and pinned DUCC. Expand only when promising; avoid broad sweeps and report concise measured results.
- Before push: inspect worktree/concurrent jobs and untracked evidence, fetch and reconcile origin, rebase **only unpublished** work safely. Never force-push, hard-reset others' work or change main. Keep `PERFORMANCE_STATUS.md` a **short replaced snapshot**, not append-only; detailed historical evidence is already in Git/reports/archive.
