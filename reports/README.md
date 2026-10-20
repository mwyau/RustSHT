# Performance findings and evidence

The [consolidated performance findings](PERFORMANCE_FINDINGS.md) analyze all 80 archived performance reports: accepted changes, rejected prototypes, mixed/noisy results, measured Rust/DUCC gaps, and a complete categorized source catalog.

**Current technical goal:** [Plan 11 — close the scalar-analysis SIMD gap](../plans/11-ducc-equivalent-scalar-analysis.md). **Current factual checkpoint:** [PERFORMANCE_STATUS](../PERFORMANCE_STATUS.md).

- [Consolidated findings and full report catalog](PERFORMANCE_FINDINGS.md)
- [Archived dated experiments and performance reports](archive/experiments/)
- [Archived prior reviews, plans, and long handoffs](archive/README.md)
- [Raw and structured benchmark data, kept in place](data/)

Older reports retain their original benchmark assumptions and source revisions. They are **evidence, not active tasks or speedup commitments**. Avoid comparing unlike compiler/ISA settings, timing boundaries, useful degree work, thread-count modes, or first-call/warm runs.
