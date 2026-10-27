# M6 source attribution — CC T720 reduced spin-1 synthesis q1 — 2026-10-27

## Result

One exact-source Callgrind run collected **613,272,869 instruction references (Ir)** around one warmed f64 synthesis transform on CC `lmax=720`, `mmax=180`, `ntheta=722`, `nphi=378`, `phi0=0.23`, q1. The output was finite and bitwise stable across the initial call, two warm repeats, and the profiled call. This is source attribution only: Ir is not elapsed time, cycles, or evidence of a candidate speedup. No independent numerical reference was run.

The prepared fused pair route is the largest source group: `synthesize_prepared_scalar_pair_block_dispatched` accounts for **289.8M inclusive Ir (47.26%)** over 1,784 calls; the Pulp V3 `FusedPreparedSynthesisPairBlock` vectorizer contributes 265.6M inclusive Ir (43.31%). Inclusive rows overlap. In direct source attribution, **61.7M Ir (10.06%)** map to `core::iter::adapters::zip` inside that monomorphized vectorizer, while FMA/SIMD implementation code accounts for much of its vector arithmetic. Compiler inlining splits this work across the Rust kernel and core/stdarch sources; these direct source rows should not be summed with inclusive rows.

The checked transition path is also substantial. `try_synthesize_paired_scalar_transition_dispatched` has 152.5M inclusive Ir (24.87%); its lower-level `dispatch_synthesis_pair_transition::<OriginalSynthesisPairCoefficients>` has 137.4M (22.41%). The polar-safe-lane route has 69.0M (11.25%) over 114 calls, and the per-degree fallback traversal has 43.8M (7.15%) over 114 calls. These inclusive values overlap and identify work locations, not additive costs. The existing route profile records 1,736 ordinary fused successes, 305 fallback entries, and 114 successful polar split attempts; its `synthesis.rs` file SHA matches this run.

**Next candidate hypothesis:** replace the nested `zip`/`enumerate` traversal only in `FusedPreparedSynthesisPairBlock::with_simd` with sequential pair-index reads from the six coefficient slices. This targets the 61.7M direct zip attribution while preserving pair order and every recurrence/FMA expression. The earlier scalar `FusedSynthesisBlock` zip experiment is a different kernel and had a q8 slowdown, so this prepared spin-1 candidate needs a fresh CC q1/q8 whole-transform screen plus GL T1023 f64 synthesis controls. The profile alone does not support promotion.

## Provenance and method

- Exact parent: `f70d9845edf6bba617b9d034b1f5e1d628c429e0` (`dev`); production source was unchanged.
- Production file SHA-256: `src/transform/spin/synthesis.rs` `8ef50c59d134f8c816793d8567198ff4120919503f9c243a8984ce6d13fef4e1`; `src/legendre/scalar_paired/synthesis.rs` `1870d0f166b8c06f9b20a5e13e32744246abffe9ebc118991f9a39557cbee560`.
- Rust/Cargo 1.99.1, LLVM 23.1.1; GCC 15.2.0; Valgrind/Callgrind 3.26.0. Release build used `CARGO_PROFILE_RELEASE_DEBUG=2`, `-C target-cpu=x86-64-v3`, `-Aunexpected_cfgs`, and `parallel`; Pulp V3 appears in the attributed vectorizer symbols.
- Callgrind used `--instr-atstart=no --dump-instr=yes`; a local C shim bracketed one transform after three warm transforms. The harness uses the deterministic generator already in the ignored synthesis phase helper (seed `0x7a31_9d42_8b6c_05e1`).
- Harness SHA-256 `6bcf5df94445389db5dfc4821681f5cede0cc61bfe12ebcc5562b05416cc4446`; C shim `6caa936803f86a346742c49fc205e4cc2c63053e1bd6dda70d478abba8e282ee`; worker binary `06b22b4fc32145cb2a91acfc6be585bedf5cd5279a64d366185ff2e09638a7e1`.
- Output hashes: first map `b8176bd012ac5019b193d9537ef933a7b1c136434ccf74623f9e6b9f07f0e123`; second map `f1ad9a115ec02751dec35c7de544e5202061f91cfc7b282f3c263d02053cd547`.
- Raw Callgrind SHA-256 `1848049f5f35426fc9efd6a06a9d447e6dbb9aa16798117b263bb556314c8ef2`; the summary table is [cc-t720-reduced-q1-source-attribution.csv](cc-t720-reduced-q1-source-attribution.csv).
- `perf stat` hardware events are unavailable on this host (`perf_event_paranoid=4`); no host policy was changed.

## Cleanup

All outputs needed for review were copied to this directory and their hashes verified before cleanup. Removed only owned scratch root `rustsht-m6-cc-q1-callgrind.2LeORQlk` (506,452,604 apparent bytes; 4,842 files), its exact registered worktree, and the experiment-owned root marker. `temporary filesystem` available bytes rose by 518,352,896 (48,574,509,056 to 49,092,861,952); free inodes rose by 5,598 (943,418 to 949,016). The root and registration are absent, and no profiler worker remains.
