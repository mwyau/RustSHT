# P02 two-chain degree-panel screen

This candidate changed only the ordinary scalar synthesis kernel
`FusedSynthesisBlock`. It processed two SIMD latitude vectors together and
unrolled two paired-degree contributions per panel. The M1 T1023 q1 profile
motivated this path: the ordinary fused route accounted for 55.352 ms of the
260.682 ms total of the three measured scalar routes, while the visitor
accounted for 191.910 ms. P01 remained unaccepted; this experiment used the
pre-P01 source.

## Paired whole-transform screen

The candidate failed the predeclared screen: require at least a 10% T1023 q1
scalar synthesis reduction and no more than a 2% regression in controls. The
measurements used fresh processes, five warmed samples per process, alternating
baseline/candidate order, one CPU core affinity, q1, GL STANDARD f64, full `mmax`,
`phi0=0.37`, and requested-only output on a x86-64 CPU with Rust 1.99.0.

| Cell | Pairs | Baseline median (ms) | Candidate median (ms) | Candidate/baseline |
|---|---:|---:|---:|---:|
| T1023 scalar synthesis | 6 | 295.952 | 306.273 | 1.03358 [1.02877, 1.03524] |
| T511 scalar synthesis | 4 | 40.384 | 41.771 | 1.03383 [1.03290, 1.03551] |

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
not run the remaining T511 scalar pairs or the spin-1 control. This partial
matrix is sufficient to reject the candidate: the primary target was 3.36%
slower, and the four clean T511 scalar control pairs were 3.38% slower. The
full control matrix was not run. Structured timing and numerical results are retained in `summary.csv`; raw stdout, GNU-time, preflight process snapshots, and refusal captures remain local.

## Numerical checks

Formatting and focused numerical validation passed:

- `cargo fmt --all -- --check`
- `git diff --check`
- Three fused-kernel tests covering scaled-reference agreement, high-order
  recurrence, and vector tails
- Three pinned DUCC 0.41.0 scalar synthesis tests: low-degree reference,
  T255 sectoral samples, and T255 mixed samples

The focused regression passed; its raw console output remains local and untracked. Full workspace suites were not
run because the performance candidate failed and was rejected. This result
rejects the tested two-chain/two-pair panel; it does not accept a production
change or make a parity claim. The exact candidate source is in `source/` and
`candidate-source.patch`.

## Reproduction

`run-matrix.sh` runs six alternating pairs for T1023 scalar, T511 scalar, and
T511 spin-1 synthesis, stopping when `run-one.sh` sees CPU PSI at or above 8%. The retained campaign stopped after the primary T1023 cell and four T511
scalar pairs. `summarize.py` regenerates the process-pair and summary CSVs. Hashes for the pre-P01 baseline, candidate ELF, source and raw evidence are in
`sha256.txt`.
