# RustSHT performance agent — active instructions

**Only active task:** reach or beat pinned native DUCC 0.42.0 (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) for **every supported representative f64 spin-0 and spin-1** GL/CC/F1 SHT analysis and synthesis workload, including reduced/full mmax and q1/q2/q4/q8. No 2× or "near parity" stopping rule. S*/A* numerical/adjoint correctness remains mandatory; measure distinct adjoint kernels if needed.

**Read at startup:** this file, [short status](PERFORMANCE_STATUS.md), [active plan](plans/f64-ducc-parity-all-cases-2026-10-24.md), then source touched by the next candidate. **Do not auto-read** dated plans, reports, archived status, raw benchmark results or the [retired-experiment index](reports/NEGATIVE_EXPERIMENTS.md). Those are **evidence only**, not instructions; consult the negative index once before reviving an old design. Separate Plan 18 (Python/FFI) is NOT part of this task.

## Execution — production code first

- Continue any genuine in-flight source experiment, then prioritize **GL T1023 spin-1 synthesis q1** (~169.8 vs 120.8 ms DUCC), **GL T720 analysis q1/q8** (~1.7×/~1.6× DUCC), and the remaining CC/GL/F1/scalar gaps. Choose by **absolute milliseconds and reusable hot loop**, not solely ratios. Latest ratios are measured *snapshots* at different sources.
- Study the actual DUCC/Rust numerical kernels and generated code when a mechanism is unclear. Make **one distinct code change**; run focused existing correctness/poison/extreme/adjoint checks and a short matched exact-parent/candidate/DUCC A/B for the affected case. Keep repeat/reset and safe fallback.
- Accept a reproducible win and push; otherwise investigate one specific cause/adjustment, then **change mechanism or target**. No repeated neutral micro-variants, full-matrix sweeps between edits, broad testing campaigns, new benchmark infrastructure, long reports, catalogs, manifests or documentation-only performance checkpoints.
- Preserve existing f64 scalar/spin-1 gains and the numerical contract, including GL/CC/F1, polar/equator, phase/Nyquist, subnormals/nonfinite, scratch reset, transactional fallback. **Never loosen tolerances.** Two reduced-order CC/F1 one-vs-eight-thread assertions predate the last optimization; note them, do not block unrelated performance work, and do not claim full validation while they remain unresolved.
- **Evidence:** update `PERFORMANCE_STATUS.md` in **at most a few lines** for an accepted measured change; do not commit large per-experiment telemetry unless essential to reproduce an unresolved numerical issue. Run the representative all-cases comparison **once performance has converged**, not as the main work.
- **Agents:** main Luna Max owns source design and decisions. **Never use Sol**. At most **one Luna subagent at a time** for bounded mechanical profiling/benchmarking/builds; avoid simultaneous CPU-heavy jobs or interfering with other agents.
- **Git:** branch `opt/sht-performance-parity` only; inspect working tree, remote and active jobs first. Fetch/reconcile before pushing, preserve unpublished and concurrent work, no force push or reset. Commit and push useful validated source checkpoints promptly. Do not touch main.

Archived guidance is not binding; this file plus the active plan control this performance branch.
