# Archived RustSHT plans, performance experiments and prior reviews

**Everything indexed here is historical evidence or superseded
guidance, not an active optimization goal.** Current instructions:
[Plan 11](../../plans/11-ducc-equivalent-scalar-analysis.md),
[AGENTS](../../AGENTS.md) and
[PERFORMANCE_STATUS](../../PERFORMANCE_STATUS.md).
Source revisions, workload boundaries and compiler choices remain
those of the original reports.

## Consolidation of 2026-10-21

The latest active-document cleanup preserved complete previous
reports, copied before their former paths became short current
summaries or archive pointers:

- [Previous long Plan 11](plans/11-ducc-equivalent-scalar-analysis-pre-refresh-2026-10-21.md):
  earlier milestones, superseded directives and 64/128/256 tile
  experiments, **not current owner instructions**.
- [Previous long performance status](performance-status-2026-10-21-pre-refresh.md):
  full running log with earlier measured/unmeasured candidates.
- [Cumulative scalar analysis report](experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md):
  full source-by-source A/B history, including rejected/removed
  direct-output, packed-reduction and indexed-loop experiments.
- [GCC/Clang DUCC rebaseline](experiments/ducc-upgrade-baseline-2026-10-20.md):
  compiler and ISA provenance; comparisons may be host-contended.
- [Rust 1.99.0 full-transform DUCC sweep](experiments/ducc-full-transform-parity-2026-10-20.md):
  old baseline and T3072 ratios, not current source.
- [Historical 80-report consolidated synthesis](performance-findings-2026-10-19-consolidated.md):
  categorized source catalog and retired experiment review.

The original data directories under
[reports/data](../data/) are **intentionally kept in place**
for reproducibility. The accepted T64 synthesis-only Legendre
dispatch study remains available in [reports/](../).
Do not delete original measurements, cherry-pick obsolete cfgs
or rerun the rejected tile variants absent new evidence.

## Earlier archive

- [Integrated ASTRA review](reviews/ASTRA_INTEGRATED_REVIEW.md)
- [Independent SHT ASTRA review](reviews/ASTRA_SHT_REVIEW.md)
- [Independent FFT ASTRA review](reviews/ASTRA_FFT_REVIEW.md)
- [October 14 session handoff](reviews/session-handoff-2026-10-14.md)
- [Literature and licensing research](reviews/sht-literature-and-licensing.md)
- [Performance status before October 19 cleanup](performance-status-2026-10-19-pre-cleanup.md)
- [Older historical plans](plans/)
- [Dated experiments](experiments/)

The [plans index](../../plans/README.md) links directly to the nine
archived historical plans (00, 03, 04, both 05 documents, 07–10).
Their redirect-only stubs were removed from `plans/`.
The substantive provenance, numerical and DUCC contract references
remain there, as do completed Plan 11, active Plan 14 and deferred
Plans 12–13.
