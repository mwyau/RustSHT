# B3 scalar-tail code outline screen — rejected

Date: 2026-11-02. Base source: `ead3f5232d47d7fedffaabd218cbfc56feeb6297`.

## Question and outcome

Could moving rare scalar SIMD-tail work out of the ordinary fused B3 pair loop shrink or simplify its hot code without slowing the common vector path? The candidate moved the existing scalar-tail recurrence and accumulation, unchanged in operation order, into a `#[cold] #[inline(never)]` helper. It did not produce a useful leaf-level gain: the no-tail result was effectively neutral, and the one-tail result was 18.0% slower by median. No whole-call run was warranted, and no production source changed.

## Direct leaf screen

The ignored test-only `benchmark_b3_spin_analysis_leaf_components` ran the `q_plus_u` route at `m=1`, `lmax=720`, GL T720 ring start 256, V3 AVX2/FMA, and 1,000 repetitions per sample. It used 64 lanes (no scalar tail) and 65 lanes (one scalar tail), with five samples per binary. Runs used one pinned logical CPU on an AMD Ryzen 9 5950X; the exact CPU index is omitted from committed results. The test removed its state-reset time from adjusted leaf timing.

| Input lanes | Parent median | Candidate median | Candidate / parent | Reading |
|---:|---:|---:|---:|---|
| 64 | 8,469.798 ns | 8,356.174 ns | 0.9866 | Small difference; not a stable or whole-call gain |
| 65 | 9,242.090 ns | 10,908.014 ns | 1.1803 | Tail candidate slower by 18.0% |

The generated caller stores scalar accumulator values to its stack, executes `vzeroupper`, and calls the outlined helper. The helper has a register-save prologue and reloads its slice fields from the passed struct. This call/ABI work is consistent with the tail-case slowdown; the isolated screen does not apportion the slowdown among individual instructions. The raw disassembly files remain local because they contain machine-specific binary paths; the relevant caller/helper behavior is summarized above.

The direct probe compiled and ran successfully. It was a test-only timing probe, not a full correctness suite. Parent and candidate outputs were not independently validated in this experiment.

## Related accumulator outline result

A previous isolated candidate changed `AnalysisTwoBasisAccumulation::with_simd` from `#[inline(always)]` to `#[inline(never)]`. It reduced the reported fused V3 leaf body from 11,160 to 10,172 bytes, but the same production-width `m=1`, 64-lane direct probe regressed from 8,469.063 to 10,152.404 ns/call (1.1988×). Its test-binary SHA-256 was `391f412a1a470100169711e24648a7cce54eac83917fb87edc2689d1c5f8e20c`; the patch is preserved at `raw/accumulator-outline.patch`, while direct-probe logs remain local. That result also rejects code outlining as a current B3 optimization route.

## Provenance

- Rust source: commit `ead3f5232d47d7fedffaabd218cbfc56feeb6297`; parent `analysis.rs` blob `b0077a7236f7e2315fe0c6b07da74f3eb4e845ba`.
- Compiler: rustc `1.99.1` (`58cf8f9c3`), LLVM `23.1.1`; flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- Parent test binary SHA-256: `48d03a63642bcd18da730789bbbdf088b3c3bfe0060a0e5a0f5345d6477c46ba`.
- Tail-outline candidate test binary SHA-256: `c1ddf474148f4bf13ad75f9b6c75f2a18b26da16124072a44f35ea4129b6af36`.
- Tail-outline patch SHA-256: `ca9e1d50da74cd7a09b4eb91385626c6047492c2f695ef31e67b6caf1b11959d`.
- Test-probe patch SHA-256: `e39a15df65f5c473df8c292043831299c9984937fe1ccb500bcc17d8c913b906`.
- The sanitized per-case results are in `result-summary.json`; reproducible test patches are listed under `raw/`. Raw console logs and disassembly remain local. `SHA256SUMS` covers committed files.

The parent binary includes the same test-only direct-leaf probe and an unrelated ring-phase transform experiment; that transform patch does not touch this direct analysis leaf. `perf_event_paranoid=4` still blocks hardware PMU events. These measurements cannot establish a cycle-level explanation of the Rust-versus-DUCC whole-transform gap.
