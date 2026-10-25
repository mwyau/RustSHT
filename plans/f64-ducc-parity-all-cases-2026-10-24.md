# Active plan — per-cell f64 DUCC performance parity

## Acceptance

Match or beat pinned native DUCC 0.42.0 (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) for every supported representative f64 spin-0 and spin-1 GL/CC/F1 analysis, synthesis, and distinct adjoint (S/A/S*/A*) cell at T255/511/720/1023, full/reduced mmax, and q1/q2/q4/q8. Measure matched whole transforms on a quiet host. Preserve numerical, adjoint, extreme-value, pole/phase/Nyquist, and scratch-reset guarantees. No averaging across cases; label unsupported, noisy, and unmeasured cells explicitly.

## Current position

GL T1023 full spin-1 synthesis q1 is closed at its measured candidate: 90.402 ms Rust vs 124.899 ms DUCC. The existing [root-cause report](../reports/T1023_Q1_ROOT_CAUSE.md) documents its route, instruction attribution, and validated improvement. Do not spend more work there without evidence of a regression.

The next open high-priority cells are GL T720 full spin-1 analysis q1 (40.746 vs 29.405 ms, 1.386×) and q8 (5.798 vs 4.223 ms, 1.373×). Candidate `70c92d9c` combines two SIMD finite-mask reductions into one; it improves q1 by 3.0% over exact parent `669d13f0` across five paired runs and q8 by 2.9% across three pairs, while preserving parent output bits. The remaining gap is recurrence/accumulation. Profile the current source and inspect its optimized AVX2 loop before choosing another mechanism.

Other measured work includes CC T720 reduced synthesis q1 (27.547 vs 18.684 ms, preliminary), while measured CC T720 full/reduced q8 and CC T1023 full q8 are below DUCC. Use [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md) as the short accepted-results snapshot; do not infer parity for unmeasured cells.

## Immediate sequence

1. Finish GL T720 spin-1 analysis q1/q8 attribution against the exact current source. Normalize work by active latitude and degree updates; separate ordinary SIMD, mixed/scaled fallback, coefficient preparation, and FFT/setup. Treat Callgrind Ir and simulated branches/cache events as instructions, not CPU cycles.
2. Rank no more than three candidate mechanisms by removable time, numerical risk, and affected cases. Implement one bounded mechanism, run focused correctness/adjoint/extreme/polar/scratch checks, then benchmark exact parent, candidate, and pinned DUCC with matched fixtures and affinity.
3. Keep a winning change only with repeatable whole-transform improvement and no material regressions. After the primary analysis gap, continue through other measured gaps by absolute time saved; do not repeat failed variants.
4. When changes stabilize, run the representative all-cases matrix and publish an explicit PASS/GAP/UNMEASURED/INCONCLUSIVE ledger with exact revisions, protocols, medians, variation, and correctness results.

## Experiment protocol

Use `benchmark_ducc_spin1_native.py` for spin-1 and `benchmark_ducc_full_transform_parity.py` for spin-0. Match fixture, compiler/ISA, CPU affinity, thread count, warmup, and repeats. Use exact-parent/candidate/pinned-DUCC comparisons. Preserve existing checks and tolerances. Keep raw profiles and temporary diagnostics outside Git unless they reproduce a specific defect. Avoid broad sweeps, generic profiler infrastructure, unrelated FFT/Python work, and f32 or spin>1 tuning until the f64 parity matrix is complete.

Follow [AGENTS.md](../AGENTS.md) for correctness, CI, Git, checkpoint, and agent-use rules.
