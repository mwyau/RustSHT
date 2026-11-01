# M6 — f64 scalar and spin-1 analysis performance

**Owner direction (2026-11-01): work on f64 B1 scalar analysis and f64 B3 spin-1 analysis.** Both are active, with one bounded experiment at a time. Do not continue B1 f32 performance tuning, attempt f32/f64 timing equality, or reopen B0 scalar synthesis or B2 spin-1 synthesis. This is the **only active M6 plan**; read [AGENTS.md](../AGENTS.md) and update the [live ledger](../reports/DUCC_PARITY_M6.md). The [B1-only plan](../reports/archive/plans/m6-b1-only-before-f64-b1-b3-focus-2026-11-01.md) and earlier three-way-parity plan are archived and superseded.

## Targets and source-specific baselines

Compare warm native Rust f64 whole transforms to **pinned DUCC 0.42.0 f64** (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) on GL T720, full M720/reduced M180, q1/q8. Target Rust f64 / DUCC f64 ≤1.00 **per cell**, with paired uncertainty. Do not interpret a geometric-mean improvement as universal parity. The reference measurements come from separate source revisions; refresh a B3 control before inferring an effect from B1 changes.

| Family | Full q1 | Full q8 | Reduced q1 | Reduced q8 | Evidence |
|---|---:|---:|---:|---:|---|
| B1 scalar analysis f64 / DUCC f64 | 1.136 | 1.132 | 1.142 | 1.111 | 15 pairs; SIMD seed-init candidate based on `80959a30` |
| B3 spin-1 analysis f64 / DUCC f64 | 1.213 | 1.346 | 1.236 | 1.295 | 15 pairs; older B3 fusion screen against `95bcbecc` |

B1's vectorized f64 seed initialization saved 8.3–10.6% whole-transform time and 6.5–7.6x in the isolated seed microbenchmark. The B3 fused scaled recurrence at `094d3162` changed **f32** only; it did not close B3's f64 gap. Older results remain valid for their source, not automatically for the current `dev` binary.

## Execution order

1. **B1 f64: bounded finishing investigation.** Profile the current f64 ordinary two-pair recurrence/accumulation, remaining initialization, reduction, and SIMD tails against DUCC f64's corresponding kernel. Try at most one or two evidence-backed changes before reassessing. Do not chase the last 11–14% indefinitely.
2. **B3 f64: main follow-on effort.** Refresh a small current-source baseline, then inspect the executed spin-1 analysis hot path, scalar/spin recurrence reuse, two-channel accumulation, normal/scaled transitions, seed initialization, memory layout, and q8 scheduling. Compare to DUCC f64. Favor an optimization that removes a substantial measured cost rather than multiplying fine-grained special cases.
3. **Shared f64 improvement:** if a B1 change is mathematically and architecturally applicable to B3, test that hypothesis against B3's actual path. Do not assume scalar and spin-1 use identical work/dispatch. Retain shared generic f32/f64 mathematical algorithms, with only justified narrow f64 SIMD specializations.
4. **Optional SIMD-tail experiment:** first measure recurrence-weighted tail frequency on actual B1/B3 grids. Only if tails consume meaningful time, microbenchmark greedy AVX-512 8→4→2 and AVX2 4→2→scalar f64 strategies (or a correctly masked partial vector) against the existing scalar tail. Verify inactive lanes do not enter reductions or scale checks. Do not implement a global width policy on speculation.

## How to work

- **Investigate first:** use pinned short microbenchmarks, `perf` cycle counters if available, direct wall-time phase instrumentation, and AVX2/FMA disassembly. Current host previously denied hardware PMU access (`perf_event_paranoid=4`); report this limitation rather than treating Callgrind Ir as cycles. Compare exact Rust and DUCC f64 paths, kernel bytes, SIMD lanes, spills, branch structure and usable work per degree/ring.
- **Screen narrowly:** use an isolated worktree, an explicit bottleneck hypothesis, affected correctness tests and approximately three interleaved warm whole-call f64 pairs against the exact parent and matching DUCC. Reject non-winners quickly; do not repeat rejected f32 tile, factor fill, recurrence-fusion, scaled-accumulator or suffix-screen variants without new evidence.
- **Promote deliberately:** only a candidate worth retaining needs 15+ paired warm f64 B1 or B3 whole-transform measurements for full/reduced q1/q8, uncertainty, hashes and output comparisons; formatting, strict Clippy, relevant Rust/Python regression suites, exceptional/adjoint/reset tests, protected f64 T1023 spin-1 synthesis and CI. For shared source changes, run focused f32 regressions too. Preserve all current numerical tolerances.
- **Keep other families parked:** B0 scalar synthesis already beats precision-matched DUCC. B2 spin-1 synthesis f64 is near parity in older three-pair screens; do not retune it now. f32 performance of B1/B3 and f32/f64 timing comparisons are **deferred**, not failed acceptance gates.
- **CI hygiene:** Ubuntu has previously failed `f32_uniform_minus_80_simd_scaling_matches_exact_scalar_scaling_and_tail` when eight exceptional lanes take the scalar tail on a 16-wide backend. If still failing, fix this narrow SIMD-width nonfinite contract and add targeted tests, without turning it into a separate f32 performance project.

## NEXT

Fetch/reconcile `dev`, verify current B1/B3 f64 paths and exact-source benchmark provenance, then **screen one or two bounded B1 f64 hot-loop/tail improvements and proceed to B3 f64 profiling**. Use short measurements and disassembly before broad validation. Push only verified improvements and concise updated findings to the existing `dev` branch, without subagents or history rewrites.
