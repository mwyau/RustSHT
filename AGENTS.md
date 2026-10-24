# RustSHT — active agent instructions

**Branch:** `opt/sht-performance-parity`. Read **only** this file, the [active spin-1 plan](plans/f64-spin1-ducc-reflected-fused-parity-2026-10-23.md), the [current performance summary](PERFORMANCE_STATUS.md), and code touched by the current experiment. Historical plans, reviews and reports are **on demand**, not startup context. [Pre-update instructions preserved](reports/archive/context-2026-10-23/prepared-scalar-adapter/AGENTS.md).

## Current owner priority (2026-10-24)

1. **Spin-1 analysis closeout is complete.** GL T720 q8 analysis remains the largest documented gap at about **2.08×** DUCC and is the next high-gap investigation after the current synthesis checkpoint.
2. Preserve all accepted synthesis changes. The checked AVX2 scaled-prefix accumulator improved GL T1023 full q1 to **176.605 ms** from **193.242 ms** on its exact parent; it remains **1.41×** DUCC. Current synthesis ratios are in [PERFORMANCE_STATUS.md](PERFORMANCE_STATUS.md); T720 F1/GL q8 are at or below DUCC, while CC T720 full/reduced q8 are 1.24×/1.45×.
3. Continue with one bounded, source-grounded mechanism at a time. Do not repeat the rejected q1 dispatch no-op or checked reflected-prefix mappings. Keep f32 and broad benchmark sweeps paused.
4. **Reviewer policy:** use a Sol reviewer only when the owner explicitly asks. Do not invoke other subagents.

## Required correctness/implementation guardrails

- Preserve `S/A/S*/A*` and spin-0/1/other unaffected operations, GL/CC/F1, poles/equator, m=0, full/quarter orders, phi0, even/odd longitude/Nyquist, q1/q8, SIMD tails, all scaled/subnormal/nonfinite/extreme cases, poisoned scratch, output reset and fallback atomicity. **Never relax tolerances to create a fast path**.
- Keep stable Rust, current Pulp SIMD, bounded worker-owned scratch and existing licensing. Compare against pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Leave scalar CC/F1 improvements, approved Python API and odd-FFT behavior unchanged.
- **Known failures:** two independent full scalar-ordered calls ~15.056→24.029 ms at CC T720 q8; eight-accumulator reflected fusion spills; C64 category packing loses full f64 lane width; 8-degree tiling and checked-prefix reflected candidates regressed. The new **internal prepared-component adapter** is a different hypothesis, not a proven gain. Do not repeat those failed implementations unchanged.
- First test **CC T720 full q8** correctness and one short old/candidate/DUCC A/B on the same source/host. Expand to reduced q8/q1 and F1/GL/T1023 only if promising; no full sweep, benchmark dashboard or broad documentation exercise. Commit accepted code and targeted tests, or note a concise specific negative result.
- Before push: inspect worktree/concurrent jobs and untracked evidence, fetch and reconcile origin, rebase **only unpublished** work safely. Never force-push, hard-reset others' work or change main. Keep `PERFORMANCE_STATUS.md` a **short replaced snapshot**, not append-only; detailed historical evidence is already in Git/reports/archive.
