# Active plan — CI first, then per-cell f64 DUCC parity

## Acceptance

1. `dev` contains the preserved optimization work and current `main` fixes, with a clean worktree, ordinary push history and CI passing on Linux/Windows/macOS (Rust fmt/check/Clippy/tests and Python tests).
2. Every supported representative f64 spin-0/spin-1 GL/CC/F1 S/A/S*/A* workload, T255/511/720/1023, full/reduced mmax and q1/q2/q4/q8, matches or beats pinned native DUCC 0.42.0 (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) in matched quiet-host whole-transform measurements. Numerical, adjoint, extreme/pole/phase/Nyquist and scratch-reset behavior must remain correct. Mark gaps, unsupported and unmeasured cells explicitly.

## Immediate sequence

1. **Branch migration:** preserve current local experiments; fetch remote `dev`, `main`, and `opt/sht-performance-parity`. Rebase the local optimization commits onto `dev` without rewriting either published branch. Resolve conflicts by retaining current `dev` workflow/instructions and all valid optimization code, tests and measurement evidence. Keep a recovery ref; never force push. Fast-forward `dev`, switch the main worktree to it and stop pushing to the old branch.
2. **Fix CI before tuning:** run local Rust fmt/check/Clippy/tests and Python tests, then inspect every GitHub Actions matrix result. Latest completed `main` CI showed cross-platform failures in `spin1_cc_grouped_synthesis_reuses_checked_coefficients` and `spin1_f1_dynamic_synthesis_groups_match_serial_and_reuse_workspace` (assertions at `src/spin_transform/tests.rs:6116` and `:5983`); Windows also failed `scalar_ordered_parallel_sparse_tails_reuse_odd_and_even_latitude_workspaces` (`src/transform/tests.rs:3535`). Recent commits may already address some: inspect current failure logs before editing. Diagnose deterministic numerical and threading-contract causes, not arbitrary tolerance or CI weakening. Push fixes, await completed clean runs.
3. **Resume the original numerical-performance work:** GL T720 full spin-1 analysis q1/q8 had a verified ~1.54–1.55× Rust/DUCC gap; recurrence/accumulation dominated measured Rust time (~85% q1, ~82% q8). Profile the removable cost in the current source, not more scalar-scan/cosine-preweight/shared-scale variants. GL T1023 spin-1 synthesis q1 is already faster (90.402 vs 124.899 ms) and closed for that measured cell. CC T720 synthesis q8 reduced/full now passes in the latest optimization-branch ledger (2.552/2.666 and 5.582/6.082 ms); CC T720 reduced synthesis q1 is still a provisional ~1.47× gap. After the principal analysis fix, prioritize other measured gaps by absolute runtime.
4. **Final certification:** one appropriately controlled representative all-cases matrix, explicit PASS/GAP/UNMEASURED/INCONCLUSIVE ledger, exact reproducible commands and commit IDs. No premature full-parity claim.

## Experiment rule

Use `benchmark_ducc_spin1_native.py` for spin-1 and `benchmark_ducc_full_transform_parity.py` for spin-0. Match fixture, compiler/ISA, affinity, thread count, warmup, repeats and pinned reference. Distinguish Callgrind instructions from wall time. One mechanism per experiment, matched parent/candidate/DUCC, focused correctness and affected regressions; revert failed variants promptly. Avoid FFT redesign, Python/FFI Plan 18, unrelated cleanup, f32 tuning and new report catalogs.

Follow [AGENTS.md](../AGENTS.md) for Git cadence, branch safety, agent policy and reporting.
