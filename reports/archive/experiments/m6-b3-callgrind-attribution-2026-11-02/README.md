# B3 f64 Callgrind attribution — 2026-11-02

## Scope and provenance

This diagnostic profiles the current B3 spin-1 analysis path at RustSHT `fb3ace0f87fdf542ae26d43213f24ac48b13e65d`, DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The active Rust source blob for `src/legendre/scalar_paired/analysis.rs` is `a8a8f18842af6d7507d1c5174be4aa0331e893c3`. The production worktree was not modified.

The harness performs two warm calls, verifies repeat and reset output bits, then profiles one complete transform. Fixtures are deterministic GL T720 maps; full uses M720/Nphi 1458 and reduced uses M180/Nphi 378. The machine was an AMD Ryzen 9 5950X, Rust 1.99.1/LLVM 23.1.1, release with debug line tables and `-C target-cpu=x86-64-v3`; q1 used one pinned CPU and q8 used eight pinned CPUs. Callgrind was 3.26.0. Hardware performance counters are unavailable (`perf_event_paranoid=4`).

The source matches the measured B3 implementation at the prior `85f74d6d` screen. Current whole-call f64/DUCC f64 ratios remain full q1 `1.109 [1.094, 1.120]`, full q8 `1.139 [1.108, 1.164]`, reduced q1 `1.095 [1.046, 1.151]`, reduced q8 `1.130 [1.058, 1.185]`; those are separate three-pair elapsed-time screens, not Callgrind results.

## Current inlined profile

| Grid | q1 collected Ir | q8 collected Ir | q8 excess |
|---|---:|---:|---:|
| Full M720 | 574,632,875 | 579,883,477 | 0.91% |
| Reduced M180 | 233,545,455 | 235,048,494 | 0.64% |

The dominant ordinary `FusedScalarAnalysisPair<true,false,true,true>` path accounts for 260,221,946 inclusive Ir (45.3%) full and 133,763,710 (57.3%) reduced. The exceptional `FusedCheckedScaledAnalysisPairChannels<false>` path accounts for 54,612,681 (9.5%) full and 19,156,462 (8.2%) reduced. These kernel counts match between q1 and q8; the small whole-run Ir excess is outside those counted B3 kernel paths. This is instruction attribution, not elapsed-time attribution. It cannot explain the 9.5–13.9% Rust/DUCC wall-time gap.

The ordinary path is in `FusedScalarAnalysisPair::with_simd` and `fused_analysis_two_pair_steps_with_simd` at `src/legendre/scalar_paired/analysis.rs` lines 4873–5153 and 4516–4790. That helper advances the shared recurrence while accumulating one channel; the second `AnalysisTwoBasisAccumulation` channel is accumulated from the saved basis vectors at lines 6529–6615. The range scan is `analysis_pair_group_state_in_range` at lines 4805–4857.

## No-inline diagnostic split

To expose two inlined bodies as separate cost centres, a diagnostic-only patch changed `fused_analysis_two_pair_steps_with_simd` and `AnalysisTwoBasisAccumulation::with_simd` from `inline(always)` to `inline(never)`. Full and reduced q1 output arrays were bitwise identical to the inlined parent. This changed code generation: total collected Ir rose 12.1% full and 14.2% reduced, so the split is not a production cost model.

| Grid | Helper calls | Recurrence + first-channel Ir | Ir/call | Second-channel Ir | Ir/call |
|---|---:|---:|---:|---:|---:|
| Full | 63,669 | 31,997,163 | 502.5 | 21,436,806 | 336.6 |
| Reduced | 150,945 | 81,393,180 | 539.2 | 55,721,340 | 369.1 |

The relative helper costs track the source's required vector FMA work (12 versus 8 FMAs per two-degree group). The split did not expose a missing or duplicate operation. The second-channel skip experiments also show that removing required output work shortens a call, not that Rust does work DUCC avoids. No performance candidate follows.

## Conclusion and next distinction

The fused ordinary path is the largest instruction-count hotspot, while the checked scaled path contributes a smaller but material count. The isolated no-inline cost centres and source confirm the normal recurrence and both channel accumulations are required. The remaining question is whether the whole-call gap comes from instruction front-end/code footprint, dependency or arithmetic throughput, or other non-kernel work. Callgrind counts instructions only; the sampled in-transform timer was too intrusive; PMU access is blocked. Stop speculative kernel edits. The next useful experiment is a PMU-enabled run of the exact source and fixtures, collecting cycles plus front-end/backend counters for q1 and q8; this distinguishes those mechanisms before another source change.

## Artifacts

- `m6_b3_analysis_callgrind_f64.rs`: one-transform profiling harness.
- `noinline-split.patch`: reversible diagnostic-only inline attribute changes.
- `profile.json`: environment, counts, fixture hashes and executable hashes.
- The six original raw profiles remain untracked in the separate exact-source diagnostic worktree; `profile.json` records their SHA256 hashes. Callgrind's raw `summary:` field underflows after the client `ZERO_STATS` request; collected totals above are the tool's reported `Collected` values. Treat raw function/call records as stage instruction attribution, not cycles.
- The committed package keeps the numerical summary, source harness, reversible patch and provenance; it excludes process-local raw traces. The isolated worktree retains the original captures unchanged.
- `SHA256SUMS`: checksums for this package.
