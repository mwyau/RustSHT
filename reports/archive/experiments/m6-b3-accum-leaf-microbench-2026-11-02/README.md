# B3 scaled-accumulator leaf discriminator — 2026-11-02

This test-only experiment compares the active fused two-channel scaled accumulator with a sequential-channel leaf, while omitting recurrence advancement. It is a bounded helper discriminator, not a production candidate or whole-transform benchmark. Production sources and routing were unchanged.

## Result

On the isolated parent `3e20b4997033e4c78927c6226f4e4243769d1d3f`, the V3 AVX2/FMA fused accumulator measured a 454.09 ns median per call; the sequential-channel leaf measured 506.39 ns. The sequential/fused ratio was 1.11517 over nine interleaved samples of 100,000 calls each. Both outputs matched bit-for-bit. The fixture has 64 lanes, of which 23 carry nonzero scale exponents.

The sequential version was 11.5% slower. It repeats basis/exponent/mask work even though it holds fewer accumulator vectors live. This synthetic leaf therefore provides no evidence to build a whole B3 candidate. It does not prove that register spills are immaterial in the active recurrence: the active scaled AVX2 leaf still has a 2,728-byte frame and 200 YMM stack references. Recurrence/normalization and exceptional-lane costs remain unresolved.

## Reproduction

In the isolated worktree at the parent SHA, restore and apply the test-only patch, then run:

```sh
gzip -dc repro/test-only.patch.gz | git apply
export PINNED_CPU=<allowed-logical-cpu>
export RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'
taskset -c "$PINNED_CPU" cargo test --release --lib \
  benchmark_b3_mixed_accumulator_against_sequential_channels \
  -- --ignored --nocapture
```

The source file, compressed exact patch, nine structured timing samples and deterministic fixture-hash script are in `repro/`. The build used Rust 1.99.1 / LLVM 23.1.1 on an AMD Ryzen 9 5950X, with single-logical-CPU affinity. The exact CPU index remains in local experiment scratch; raw console output is also retained there. Binary SHA-256: `5642df183485bcd108d8ee743c98b536891233cf6d8754446656f33bac8ceb88`.

## Provenance and limitations

DUCC reference revision: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; no DUCC binary or whole-transform timing was involved in this helper-only experiment. B3 hot source blobs at the parent match the `cc60dbdc` scaled-prefix profile, so its three-pair current-source whole-call screen remains the nearest transform-level control; this microbenchmark does not update those ratios. See [PROVENANCE.json](PROVENANCE.json) and [SHA256SUMS](SHA256SUMS).

**Next diagnostic:** add sampled, test-only timings to the exact active scaled leaf to separate recurrence/normalization, masked ordinary accumulation, and exceptional-lane recovery. Use the smallest sampling rate that keeps instrumentation overhead bounded, report against an uninstrumented whole-call control, and do not add a production candidate until a distinct, material cost is isolated.
