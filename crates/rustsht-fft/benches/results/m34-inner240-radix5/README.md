# M34 radix-five candidate captures

These captures compare three temporary dispatch forms of the symmetric-pair AVX2 radix-five kernel with the accepted M32 binary. Each full capture runs six alternating process pairs over the 34-cell workload with seven warmed samples per row on CPU 30. The candidate source diff and binary hashes are recorded in each JSON configuration.

The directory-root files contain candidate 1. Candidate 2 routes the kernel through the common AVX2 pass dispatcher. Candidate 3 reuses the low-`ido` radix-four dispatch slot. A focused six-pair N=722 capture and Callgrind profiles accompany candidate 3. All candidates were rejected after repeatable unrelated regressions; their code was removed.

The phase method, paired results, rejection rationale, and accepted M32 cross-library status are documented in [`docs/m34-rader-phase-and-radix5.md`](../../../docs/m34-rader-phase-and-radix5.md). Raw paired process results and the summarized intervals are kept beside each candidate configuration.
