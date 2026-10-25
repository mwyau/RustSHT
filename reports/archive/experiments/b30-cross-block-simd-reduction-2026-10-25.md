# B30 — cross-block SIMD partial retention

**Disposition: rejected.** The implementation has been removed from production source. Its source remains recoverable through Git history (`8d736e4b`–`c52d4118`).

## Objective and mechanism

B30 tested whether `FusedScalarAnalysisPair` could reduce horizontal-sum overhead by retaining its eight AVX2 vector partials across adjacent reflected-latitude blocks, then reducing them together. It was motivated by earlier q1/q8 attribution that showed frequent horizontal reductions and by an isolated accumulator probe that ran faster. The production experiment kept Q and U channel contributions in a recurrence workspace buffer, threaded a reduction mode through the scalar trait and fused kernel, merged retained vectors on the next eligible block, and flushed pending vectors before fallback or at the end of the order.

The exact comparison was baseline `95125789` versus the corrected opt-in candidate `c52d4118` (initial implementation `8d736e4b`; the U final-degree flush fix is `c53de984`). The intervening B30 commits contain the experiment, its tests/fix, cfg registration, and documentation. Accepted A06 suffix-only accumulator clearing `f1b14541` is already present in baseline `95125789` and remains in production.

## Isolated probe and production attribution

The preallocated isolated AVX2 probe retained partials over adjacent 64-latitude-pair blocks. Across 12 interleaved CPU-2-pinned runs of 100 repetitions, it halved reduction invocations (8,192→4,096), with candidate/parent median time ratio 0.8788 and matching checksums. Probe total Callgrind Ir slightly increased (3.993444M→3.995002M, +0.04%); its main function self Ir fell only 0.34%. The probe omitted recurrence, checked decline/fallback, workspace handling, and output staging, so it did not predict production cost.

In the whole GL T720 full spin-1 analysis profile, each channel's eight-vector reduction groups fell 44.8% (2,064,951→1,139,418). The candidate used 213,885 immediate groups plus 925,533 merged groups; the retained-first blocks themselves did not reduce. Despite that change, whole-program Callgrind Ir rose materially:

| Threads | Exact parent `95125789` | Candidate `c52d4118` | Change |
|---|---:|---:|---:|
| q1 | 5.737B Ir | 6.757B Ir | +17.8% |
| q8 | 5.803B Ir | 6.827B Ir | +17.7% |

The reduction-count decrease therefore did not translate into lower total work. Retention adds mode checks, partial-buffer validation and copies, pending-state management, and transactional flush/fallback paths. Callgrind Ir is an instruction count, not a cycle or elapsed-time measurement.

## Whole-transform timing

Three interleaved parent/candidate pairs were run for each GL full spin-1 analysis cell; the table gives the median candidate/parent ratio. A ratio above 1 is slower.

| Workload | q1 candidate/parent | q8 candidate/parent | Evidence limits |
|---|---:|---:|---|
| T511 | 1.142× | 1.117× | Three pairs; host runqueue contention affected q8. |
| T720 | 1.186× | 1.230× | Candidate was slower in every q1 and q8 pair. |
| T1023 | 0.996× | 1.301× | q1 was effectively neutral; q8 and runqueue contention make the estimate noisy. |

The interleaved protocol included pinned DUCC 0.42.0, but the retained rolling record contains parent/candidate ratios rather than reliable absolute per-backend medians. These data establish no DUCC improvement. The T720 regressions and +17% whole-program Ir are sufficient to reject this implementation; q1 T1023's neutral result does not offset the affected-cell regressions.

## Correctness and validation evidence

The B30-enabled focused test matrix passed odd/even block sequences, single and final partial blocks, skipped blocks, Q/U channels, scalar SIMD tails, workspace reuse, polar declines, and checked-range decline before retained contributions were committed. Candidate-enabled and default release `spin1_` test runs passed (48 passed, 4 ignored, plus 2 f32 integration tests); the spin adjoint identity test passed. T720 candidate output differed from the parent by relative L2 `1.06e-15` and from pinned DUCC by `4.76e-14`; T511/T1023 q1/q8 checks remained within the existing tolerances. No tolerance was changed.

The local workflow on the candidate-containing `dbb4bacd` passed formatting, workspace/all-targets/all-features check, strict Clippy, Rust tests (290 unit, 4 extreme, 11 f32, and 1 workspace test passed; 37 ignored), and Python tests (20 passed). GitHub Actions run [40229003400](https://github.com/mwyau/RustSHT/actions/runs/40229003400) was reported green on Ubuntu, Windows, macOS, and Python. These validate the experiment's then-current code; cleanup validation is recorded separately in `reports/PROGRESS.md`.

## Complexity and final disposition

The implementation added a public-to-crate reduction-mode abstraction, extra `ShtScalar` arguments/methods, Q/U retained-vector handling in the fused pair kernel, a scratch allocation in `RecurrenceWorkspace`, per-order pending state, and flushes before fallback and order completion. Its tests found a real U final-degree omission before the corrected candidate was profiled. That state and fallback surface is not justified by the isolated reduction win, the increased whole-program Ir, and the measured q1/q8 regressions.

**B30 is rejected and removed from production source.** Revisit only if new evidence identifies a fundamentally different way to amortize reductions that avoids per-block retention state, buffer traffic, and transactional flush machinery, and demonstrates a whole-transform benefit with matched q1/q8 correctness and timing. Do not restore the same cross-block retention implementation unchanged.
