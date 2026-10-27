# Performance evidence

**Current performance work:** [M6 DUCC parity: current-source profiling, disassembly, bounded optimizations and repository cleanup](../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md). The first selected current-head screen is in [the M6 report](DUCC_PARITY_M6.md) and [per-cell ledger](data/ducc-parity-m6-2026-10-27.csv); full-matrix parity remains unverified. [Candidate register](PERFORMANCE_CANDIDATES.md) retains source-recoverable ideas and negative experiments; [rolling progress](PROGRESS.md) carries concise checkpoints. Do not delete raw provenance to simplify the reports tree.

**For current work:** [AGENTS.md](../AGENTS.md) · [active f64 parity plan](../plans/f64-ducc-parity-all-cases-2026-10-24.md) · [accepted-results status](../PERFORMANCE_STATUS.md) · [rolling checkpoints](PROGRESS.md).

**For optimization selection and avoiding lost work:** [recoverable candidate backlog](PERFORMANCE_CANDIDATES.md) · [retired implementation notes](NEGATIVE_EXPERIMENTS.md) · [2026-10-25 historical audit and file coverage](ARCHIVE_METHOD_AUDIT_2026-10-25.md).

Historical reports and reviews are under [archive](archive/README.md). Raw benchmark records and preserved patches remain in `reports/data/`; inspect only relevant provenance. Archival **production** labels refer to the historical source revision and are not current-HEAD acceptance. A reverted patch can still be an instruction-efficient or near-neutral candidate; consult the candidate ledger before discarding its idea. The `torch_harmonics_bench.py` script remains available.
