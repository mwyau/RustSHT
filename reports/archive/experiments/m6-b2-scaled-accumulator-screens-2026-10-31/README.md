# B2 scaled-accumulator SIMD screens

These isolated screens tested whether vectorizing the scaled-prefix accumulator improves spin-1 synthesis (B2). None is a promotion candidate. All used the same `de6fc98e04dcbf56de125f4e1c0ec81aa402cb0f` base, pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, Ryzen 9 5950X, `-C target-cpu=x86-64-v3`, q1 affinity `0`, q8 affinity `0-7`, and native f32/f64 transforms. Pair outputs remained bit-identical to the parent in measured cells.

## Candidates and conclusions

| Candidate | Patch SHA-256 | Result |
|---|---|---|
| Uniform exponent, normal output only | `64e32a4a1e654016ed7e79dd93fae4a0f83fac866c89179594d6666eebea7fbd` | Three-pair screen was about 0.8–1.6% slower across the four T720 cells; successful vector accumulations were zero because scale-range checks rejected the groups. |
| Uniform exponent with subnormal output support | `7e19e348d040a31766ef2fc5933b7eaff4e0ed0660cd785b190ef06aade2b3fe` | Three-pair outputs were exact. The apparent full-q8 gain came from a parent timing outlier; a follow-up q8 screen stopped after three pairs at 1.002–1.011 candidate/parent. Other cells were neutral or inconclusive. |
| Per-lane exponent scaling | `7d441758a22b104b4ff1f280f57ac500897cafe0e00e399019f3b248e2f6d10e` | Three-pair full q1 and q8 were 2.38% and 2.02% slower; reduced q1's first two pairs were 0.97% slower. Outputs were exact. A test-only profile showed more vector-lane work, but the hot helper grew from 5,639 to 17,261 bytes and added stack traffic. |

The scaled-prefix SIMD hypothesis is closed. Do not resume these variants without a new execution-level hypothesis.

## Evidence

- `source-*.patch.gz`: recoverable source changes against the base above.
- `results/`: normal-only three-pair data and per-lane results for full q1/q8 plus the first two reduced-q1 pairs.
- `results/uniform-subnormal.json`: complete three-pair subnormal-support screen for full/reduced q1/q8.
- `results/full-q8-partial-15/`: the first three pairs of a planned 15-pair confirmation; the run stopped after showing no gain and is not a 15-pair result.
- `run_b2.py`, `run_full_q8_15.py`: runners.
- The diagnostic Callgrind profile and generated-code captures remain in the isolated experiment worktree's ignored `target/b2-vector-scaled-prefix-screen-20261031/` directory. The report summarizes the relevant counts and code-size comparison; Callgrind instruction counts are not cycle measurements.

The run metadata records per-case fixture hashes, runtime ratios, dispatch, worker checksums, binary hashes, compiler, affinity, CPU and source/patch hashes. Full raw intermediate traces and binaries are intentionally not retained.

## Validation

The per-lane helper's exact-scaling unit test passed over normal exponent fields and representative fractions, including exponents -1024 through 1024. `cargo fmt --all --check`, the numeric and scalar-ordered release library tests, and `cargo test --release --test f32` passed. These checks do not establish a performance improvement, so no candidate was promoted.

Patch and Callgrind artifacts use deterministic gzip compression; decompress them to recover the exact bytes and verify the recorded uncompressed SHA-256 values.
