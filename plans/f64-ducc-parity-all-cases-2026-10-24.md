# Active plan — per-cell f64 DUCC performance parity

## Acceptance

Match or beat pinned native DUCC 0.42.0 (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) for every supported representative f64 spin-0 and spin-1 GL/CC/F1 analysis, synthesis, and distinct adjoint (S/A/S*/A*) cell at T255/511/720/1023, full/reduced mmax, and q1/q2/q4/q8. Measure matched whole transforms on a quiet host. Preserve numerical, adjoint, extreme-value, pole/phase/Nyquist, and scratch-reset guarantees. No averaging across cases; label unsupported, noisy, and unmeasured cells explicitly.

## Current position

GL T1023 full spin-1 synthesis q1 is closed at its measured candidate: 90.402 ms Rust vs 124.899 ms DUCC. The existing [root-cause report](../reports/T1023_Q1_ROOT_CAUSE.md) documents its route, instruction attribution, and validated improvement. Do not spend more work there without evidence of a regression.

The next open high-priority cells remain GL T720 full spin-1 analysis q1 and q8, now measured at 39.298 vs 29.447 ms (1.335×) and 5.749 vs 4.279 ms (1.344×). Candidate `ab3ec7da` adds a three-buffer rotation to sequential Q/U recurrence, avoiding the pre-step basis copy without changing accumulation order. Against exact parent `7fbb75cc`, q1 improved 2.2% over 15 pairs (13 wins) and q8 improved 3.6% over five pairs (5/5); outputs remained bit-identical and candidate/DUCC rel-L2 was 4.76e-14. Focused full/polar/scaled/fallback/scratch/adjoint checks passed. A matched q1 Callgrind profile fell 0.76% in instructions, so the remaining gap is still recurrence/accumulation. Continue from the current-source exclusive profile, prioritizing the ordinary fused loop and checked-scaled path by removable cost.

Other measured work includes CC T720 reduced synthesis q1 (27.547 vs 18.684 ms, preliminary), while measured CC T720 full/reduced q8 and CC T1023 full q8 are below DUCC. Use [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md) as the short accepted-results snapshot; do not infer parity for unmeasured cells.

## Immediate sequence

1. Use the exact-source exclusive profile and optimized AVX2 disassembly to rank the remaining ordinary fused and checked-scaled recurrence costs by removable time; keep inclusive counts labeled and do not treat Callgrind Ir as cycles.
2. Select one bounded source mechanism, run focused correctness/adjoint/extreme/polar/scratch checks, then benchmark exact parent, candidate, and pinned DUCC with matched fixtures and affinity.
3. Keep a winning change only with repeatable whole-transform improvement and no material regressions. After GL T720 analysis q1/q8, continue through other measured gaps by absolute time saved; do not repeat failed variants.
4. When changes stabilize, run the representative all-cases matrix and publish an explicit PASS/GAP/UNMEASURED/INCONCLUSIVE ledger with exact revisions, protocols, medians, variation, and correctness results.

## Experiment protocol

Use `benchmark_ducc_spin1_native.py` for spin-1 and `benchmark_ducc_full_transform_parity.py` for spin-0. Match fixture, compiler/ISA, CPU affinity, thread count, warmup, and repeats. Use exact-parent/candidate/pinned-DUCC comparisons. Preserve existing checks and tolerances. Keep raw profiles and temporary diagnostics outside Git unless they reproduce a specific defect. Avoid broad sweeps, generic profiler infrastructure, unrelated FFT/Python work, and f32 or spin>1 tuning until the f64 parity matrix is complete.

Follow [AGENTS.md](../AGENTS.md) for correctness, CI, Git, checkpoint, and agent-use rules.
