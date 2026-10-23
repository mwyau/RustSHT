# Atomic block claiming for ordered CC scalar synthesis

## Change

The parallel ordered scalar synthesis path no longer allocates a `VecDeque`
or locks a mutex to claim each four-order job. Workers atomically claim block
indices with `AtomicUsize`; the block boundaries and dynamic load balancing
remain unchanged. This targets per-transform queue allocation and the
contended lock taken for each of the 181 jobs at T720. A failed job still
causes the complete Fourier workspace to be cleared before the guarded
fallback runs.

## Correctness

The release `scalar_ordered` tests passed with the parallel feature (18 passed,
2 ignored) and with `--no-default-features` (13 passed). These include exact
CC/F1 comparisons against the general recurrence, parallel-versus-serial
output checks, high-degree cases, and exceptional-value fallback coverage.
Strict all-target/all-feature release Clippy and `cargo fmt --check` passed.

The native T720 CC q8 comparison produced three numerical comparisons per run,
zero failure records, accepted reset checks, and maximum relative L2
`6.81e-14`. The benchmark validation sidecar was incomplete because this
focused comparison did not run the broad validation matrix; the focused
numerical comparisons and Rust tests passed independently.

## Matched comparison

The baseline is commit `4447f432`; the candidate is the source patch recorded
in this commit, SHA-256 `f36a1ac7c76b8ca51b1f0476cc684b68c95a55dac2a3729283e6643ffd31e026`.
Both use pinned DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`,
with the same DUCC binary SHA-256
`3df1d4408407015c65a85e2909aa34219871bf29a75126b23e497bcbb396c89a` and
Rust `-C target-cpu=x86-64-v3` builds. The case was exact CC T720, `mmax=720`,
`ntheta=722`, `nphi=1536`, `phi0=0.23`, synthesis q8. Each run collected 30
calibrated batches per backend over three process pairs; all batches met the
20–100 ms target window. The second run reversed the source order.

| Source run | Rust warm ms | DUCC warm ms | Rust / DUCC | Paired-ratio p10–p90 |
|---|---:|---:|---:|---:|
| Baseline, first | 2.811 | 2.739 | 1.030× | 0.996–1.035× |
| Candidate, first | 2.786 | 2.735 | 1.014× | 1.012–1.026× |
| Candidate, reversed order | 2.768 | 2.715 | 1.018× | 1.016–1.021× |
| Baseline, reversed order | 2.827 | 2.731 | 1.046× | 1.025–1.047× |

The median of the two Rust run medians fell from `2.819 ms` to `2.777 ms`
(1.5%). The median Rust/DUCC ratio fell from `1.038×` to `1.016×`. This is a
small, repeatable whole-transform gain in this case. It does not establish
parity across other degrees, q1, reduced order, F1, or analysis.

Raw captures and local host telemetry are preserved in local temporary
directories and are not committed.
Rust binary SHA-256 values are `f19064dccdf352e2b11c33af7ed2a5744100b71d5cd4dd50f81a727157516cc2`
(baseline) and `23d9402ea44a62bf3cf3051eda677f2d489d2f0d1bd5d65ac964514bb4e68121`
(candidate). The runs used the existing focused harness and its compatibility
wrapper from `reports/data/cc-scalar-synthesis-grain-2026-10-23/`.
