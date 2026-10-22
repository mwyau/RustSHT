# Current-source packed-reduction A/B

This directory contains sanitized summaries for the current-source default
analysis kernel and the same source built with
`rustsht_packed_complex_pair_reduction`.

## Source and build

- RustSHT source: `ef238290f28d5192ca54de8e442c77640549828a`; both source
  worktrees were clean. Only the compile-time reduction cfg differs.
- Rust: 1.99.1, LLVM 23.1.1, Pulp 0.22.3; strict floating point and
  `-C target-cpu=x86-64-v3`.
- DUCC: 0.42.0, pinned source
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`; GCC 15.2. DUCC SHT source
  flags were `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and its
  driver was compiled with `-O3 -fno-fast-math -ffp-contract=fast
  -march=x86-64-v3`. The DUCC binary hash was identical in both runs.
- Host: AMD Ryzen 9 5950X, AVX2/FMA. q1 pinned to CPU 0; q8 pinned to CPUs
  0–7 in one L3 domain.

## Timing protocol

The full-transform table contains T720 GL STANDARD analysis at `mmax=1`,
`180`, and `720`, q1/q8. Each binary used three interleaved Rust/DUCC
process pairs and ten calibrated batches per case, targeting 25 ms per batch.
Plans and workspaces were reused; first-call/setup fields are retained in the
CSV, and output comparison/reset checks were outside timed batches.

The kernel table is the range-proof-cached `unchecked` fused route, including
the final coefficient commit, for 64/128/256 active lanes. It used ten
interleaved process pairs and six calibrated 20–100 ms batches per process
and backend, pinned to one physical core. Reset-only cost was measured
separately. The report uses paired Rust/DUCC process ratios from the harness.

## Files

- `full-transform-ab.csv`: warm and first-call latencies, p10/p90, paired
  ratios, numerical errors, setup and RSS summaries for all six T720 cases.
- `kernel-ab.csv`: direct-kernel latency distributions, paired ratios,
  absolute excess, and numerical errors for each lane size.
- `summary.json`: compiler/source hashes, timing protocol, host summaries,
  and numerical validation summary.

Raw per-batch JSON and process logs remain in local scratch and are not
included here. They were not merged into medians or represented as samples.
