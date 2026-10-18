# P01 shared recurrence step redesign screen

This rejected candidate factors the recurrence update used by the scalar and
paired scaled-prefix transitions into an out-of-line SIMD helper. The scalar
normalized suffix remains inlined. The candidate is based on commit `ed63cdb`
plus the archived P01-only change; P02 is excluded. The complete changed
source is in `source/`, and `candidate-source.patch` reconstructs it against
the base commit.

## Paired synthesis measurements

Each fresh process used five warmed samples. The table reports the median of
paired candidate/control ratios and a deterministic 95% bootstrap percentile
interval over the eligible paired ratios. CPU affinity was one CPU core, q1, GL
STANDARD f64, full `mmax`, `phi0=0.37`, and requested-only output. The benchmark
used a x86-64 CPU and Rust 1.99.0. Structured timing samples and RSS summaries are retained here; raw process, GNU-time and host captures remain local.

| Comparison | Eligible pairs | Candidate/control ratio | Result |
|---|---:|---:|---|
| T1023 scalar synthesis vs pre-P01 | 6 | 0.73059 [0.72670, 0.73164] | Scalar gain retained |
| T511 spin-1 synthesis vs pre-P01 | 4 of 6 | 1.09995 [1.09419, 1.10291] | Fails the 1.02 control guard |
| T511 spin-1 synthesis vs original P01 | 6 | 1.07780 [1.07426, 1.09262] | Shared-step change adds a repeatable slowdown |

For the pre-P01 comparison, pairs 5 and 6 at T511 were excluded because a
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
The direct comparison with the original P01 candidate had all six pairs below
the 8% PSI screen. That comparison isolates the shared-step source change and
shows a 7.78% median T511 spin-1 slowdown relative to original P01. The source path explains the regression in this redesign: the out-of-line
helper replaced the recurrence step in both transition implementations. The
paired transition is used by spin-1 synthesis, so it now makes an additional
function call at each recurrence step. The exact source path is visible in
`source/scalar_paired.rs`; spin-1 calls the paired transition in
`src/spin_transform.rs`. The profile and paired timings agree with that
mechanism. This does not identify the separate final-link layout effect seen
with the original P01 candidate.

## Route profile and numerical checks

The T1023 q1 profile recorded fused/visitor/transition timer sums of
54.017/70.317/59.964 ms, with 225,005,258 committed route units. Enabling
counters changed wall time from 206.775 to 207.514 ms (+0.36%). Compared with the
committed baseline's 55.352/191.910/13.420 ms route sums, visitor work remains
lower, but the shared transition is substantially slower than the original
P01-only profile's 54.404/69.752/39.293 ms. The following targeted checks passed:

- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo test --release -p rustsht --features parallel --lib scalar_synthesis_transition_matches_prepared_reference_with_tail -- --nocapture`
- `cargo test --release -p rustsht --features parallel --lib transition_ -- --nocapture` (9 passed)
- `RUSTSHT_ROUTE_LMAX=1023 RUSTSHT_ROUTE_THREADS=1 cargo test --release -p rustsht --features parallel --lib m6_scalar_synthesis_routes -- --ignored --nocapture`

The full control matrix was not run. T511 spin-1 already exceeds the declared
2% guard in the clean pairs, so reject this source design. P01 remains
unaccepted; the measurement does not authorize production use or a parity
claim. The detailed pair rows are in `process-pairs.csv`, and
`summarize.py` regenerates the summaries from local raw captures; structured scientific results are committed.

## Reproduction

The ordinary-link benchmark binary is archived at
`target/m6-p01-shared-step-redesign/m1b_bench`; its SHA-256 is in `sha256.txt`. The pre-P01 baseline and original default-link P01 executable hashes are
recorded there as well. `run-matrix.sh` records six alternating baseline vs
redesign pairs for T1023 scalar and T511 spin-1 synthesis. `run-causal-ab.sh` records six alternating pairs of the original P01 and the
shared-step redesign at T511 spin-1. `run-one.sh` records local process configuration and host telemetry, plus benchmark timing and RSS. Git retains only the sanitized workload configuration and structured results.
