# Active plan — per-cell f64 DUCC performance parity

## Acceptance

Match or beat pinned native DUCC 0.42.0 (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) for every supported representative f64 spin-0 and spin-1 GL/CC/F1 analysis, synthesis, and distinct adjoint (S/A/S*/A*) cell at T255/511/720/1023, full/reduced mmax, and q1/q2/q4/q8. Measure matched whole transforms on a quiet host. Preserve numerical, adjoint, extreme-value, pole/phase/Nyquist, and scratch-reset guarantees. No averaging across cases; label unsupported, noisy, and unmeasured cells explicitly.

## Current position

GL T1023 full spin-1 synthesis q1 is closed at its measured candidate: 90.402 ms Rust vs 124.899 ms DUCC. The existing [root-cause report](../reports/T1023_Q1_ROOT_CAUSE.md) documents its route, instruction attribution, and validated improvement. Do not spend more work there without evidence of a regression.

The next open high-priority cells remain GL T720 full spin-1 analysis q1 and q8, now measured at 37.687 vs 29.342 ms (1.284×) and 5.478 vs 4.229 ms (1.296×). Latest candidate `bf938aba` vectorizes only four-of-four exceptional AVX2 lane groups using the existing checked exponent scaler; partial vectors and any unsafe product retain scalar fallback. Against exact source parent `cc754e60` (the parent binary was built from source-identical `7c641b05`), 15 interleaved pairs improved q1 by a paired median 3.3% (13/15 wins, bootstrap 95% interval [0.9585, 0.9768]) and q8 by 2.3% (12/15 wins, interval [0.9706, 0.9979]). Candidate output was bit-identical to parent in all 30 pairs; repeat/reset checks passed; candidate/DUCC rel-L2 was 4.76e-14. The focused release `spin1_` suite passed 48 tests, and a temporary q1/q8 route probe exercised 29,545 full-vector fast paths with no declines. The T720 analysis gap remains open; continue from the current-source exclusive profile and rank ordinary fused and checked-scaled costs by removable time.

Other measured work includes CC T720 reduced synthesis q1 (27.547 vs 18.684 ms, preliminary), while measured CC T720 full/reduced q8 and CC T1023 full q8 are below DUCC. Use [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md) as the short accepted-results snapshot; do not infer parity for unmeasured cells.

## Immediate sequence

1. Use the exact-source exclusive profile and optimized AVX2 disassembly to rank the remaining ordinary fused and checked-scaled recurrence costs by removable time; keep inclusive counts labeled and do not treat Callgrind Ir as cycles.
2. Select one bounded source mechanism, run focused correctness/adjoint/extreme/polar/scratch checks, then benchmark exact parent, candidate, and pinned DUCC with matched fixtures and affinity.
3. Keep a winning change only with repeatable whole-transform improvement and no material regressions. After GL T720 analysis q1/q8, continue through other measured gaps by absolute time saved; do not repeat failed variants.
4. When changes stabilize, run the representative all-cases matrix and publish an explicit PASS/GAP/UNMEASURED/INCONCLUSIVE ledger with exact revisions, protocols, medians, variation, and correctness results.

## Experiment protocol

Use `benchmark_ducc_spin1_native.py` for spin-1 and `benchmark_ducc_full_transform_parity.py` for spin-0. Match fixture, compiler/ISA, CPU affinity, thread count, warmup, and repeats. Use exact-parent/candidate/pinned-DUCC comparisons. Preserve existing checks and tolerances. Keep raw profiles and temporary diagnostics outside Git unless they reproduce a specific defect. Avoid broad sweeps, generic profiler infrastructure, unrelated FFT/Python work, and f32 or spin>1 tuning until the f64 parity matrix is complete.

Follow [AGENTS.md](../AGENTS.md) for shared correctness and workflow rules.
