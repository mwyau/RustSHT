# B3 f64 Callgrind attribution — 2026-11-02

## Scope and provenance

This diagnostic profiles the current B3 spin-1 analysis path at RustSHT `fb3ace0f87fdf542ae26d43213f24ac48b13e65d`, DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The active Rust source blob for `src/legendre/scalar_paired/analysis.rs` is `a8a8f18842af6d7507d1c5174be4aa0331e893c3`. The production worktree was not modified.

The harness performs two warm calls, verifies repeat and reset output bits, then profiles one complete transform. Fixtures are deterministic GL T720 maps; full uses M720/Nphi 1458 and reduced uses M180/Nphi 378. The machine was an AMD Ryzen 9 5950X, Rust 1.99.1/LLVM 23.1.1, and Callgrind 3.26.0. Hardware performance counters are unavailable (`perf_event_paranoid=4`).

The source matches the measured B3 implementation at the prior `85f74d6d` screen. Current whole-call f64/DUCC f64 ratios remain full q1 `1.109 [1.094, 1.120]`, full q8 `1.139 [1.108, 1.164]`, reduced q1 `1.095 [1.046, 1.151]`, reduced q8 `1.130 [1.058, 1.185]`; those are separate three-pair elapsed-time screens, not Callgrind results.

## Superseded debug-info profile

The first profile used a release binary with debug line tables. A later machine-code comparison found that its active V3 leaf was 11,029 bytes/2,216 instructions, while the official release benchmark leaf is 11,160 bytes/2,227 instructions. The debug-info build changed codegen, so the instruction shares and total counts below are retained only as historical diagnostic results; use the no-debug profile that follows for source-matched attribution.

| Grid | q1 collected Ir | q8 collected Ir | q8 excess |
|---|---:|---:|---:|
| Full M720 | 574,632,875 | 579,883,477 | 0.91% |
| Reduced M180 | 233,545,455 | 235,048,494 | 0.64% |

The original debug-info run attributed 260,221,946 inclusive Ir to the ordinary `FusedScalarAnalysisPair<true,false,true,true>` path (45.3% full, 57.3% reduced) and 54,612,681/19,156,462 Ir to the checked scaled path (9.5%/8.2%). These counts do not describe the official release codegen and are superseded below.

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

## No-debug matched-codegen cache and branch profile

The profile harness was rebuilt without debug info and checked against the official Rust release benchmark. Its active Pulp V3 leaf is byte-identical: 11,160 bytes, 2,227 instructions, instruction-byte SHA-256 `8da541e7f4207570321326aa904381d02884b37e7ad4f6b2454af003625ac119`. The Rust no-debug harness SHA-256 is `16e23f1590acc09c16813e2b5203cbac5804bbde9c20af4991df2a5868ff6951`; the official benchmark is `121e72638ea3038a0ab7db0d7ea6a08c81d405a37ac7df96986707786ec31ee5`. DUCC uses the pinned 0.42.0 source and a Callgrind-marked driver. Its checkout has unrelated B1 timer code guarded by `DUCC_B1_STAGE_PROFILE`; the compile command did not define that macro, so those blocks preprocess out.

| Cell | Rust whole-call Ir | DUCC whole-call Ir | Rust/DUCC Ir | Rust fused block Ir | DUCC `map2alm_kernel` Ir | Fused/kernel Ir |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 577.66M | 429.39M | 1.345× | 345.63M | 223.15M | 1.549× |
| Full q8 | 581.93M | 429.58M | 1.355× | 346.02M | 223.15M | 1.551× |
| Reduced q1 | 235.07M | 173.17M | 1.357× | 162.99M | 112.05M | 1.455× |
| Reduced q8 | 236.23M | 173.31M | 1.363× | 162.99M | 112.05M | 1.455× |

The ordinary vector leaf accounts for 262.90M Ir full and 135.18M reduced. The checked scaled leaf contributes 54.74M full and 19.20M reduced; both counts match between q1 and q8. These are nested function costs and must not be added to their inclusive wrapper or whole-program totals. In the Callgrind model, Rust has fewer data references but more D1 misses (2.14%/2.18% full and 1.85%/1.87% reduced, versus DUCC 0.79%/0.78% full and 0.46%/0.47% reduced). Simulated branch-mispredict rates are 1.83–1.99% for Rust and 0.99–1.89% for DUCC; last-level data misses are similar. I1 misses are much higher in Rust as a count, but remain below 0.05% of its instructions. This suggests branch and L1-locality hypotheses worth checking, but these are simulator events, not hardware cycles, and do not identify the cause of the 9.5–13.9% elapsed-time gap.

Machine-readable event totals, exact function costs, fixtures, binary hashes, and hashes of the eight raw profiles are in [`nodbg-cachebranch-summary.json`](nodbg-cachebranch-summary.json). Raw traces remain in the isolated profile worktree; they are not committed because they contain process-local paths and PIDs.

## Artifacts

- `m6_b3_analysis_callgrind_f64.rs`: one-transform profiling harness.
- `noinline-split.patch`: reversible diagnostic-only inline attribute changes.
- `profile.json`: environment, counts, fixture hashes and executable hashes.
- `nodbg-cachebranch-summary.json`: corrected release-codegen program and kernel events, provenance, and raw-trace checksums.
- The six original raw profiles remain untracked in the separate exact-source diagnostic worktree; `profile.json` records their SHA256 hashes. Callgrind's raw `summary:` field underflows after the client `ZERO_STATS` request; collected totals above are the tool's reported `Collected` values. Treat raw function/call records as stage instruction attribution, not cycles.
- The committed package keeps the numerical summary, source harness, reversible patch and provenance; it excludes process-local raw traces. The isolated worktree retains the original captures unchanged.
- `SHA256SUMS`: checksums for this package.
