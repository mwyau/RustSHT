# Plan 16 Milestone B — parity-separated spin-1 synthesis A/B

**Conclusion:** neither parity-separated candidate is a production
improvement. Moving the cosine combination after the degree loop does remove
repeated cosine weighting, but the two-channel form spills SIMD accumulators;
the one-channel form removes those YMM spills while evaluating the recurrence
twice. The resulting end-to-end synthesis cost is neutral to substantially
higher. The production lane-outer cosine-combined kernel remains unchanged.

## Compared paths

The baseline is RustSHT commit `ba8a2003ab346a56321aa8912b07777f745b1352`
with the existing lane-outer `FusedPreparedSynthesisPairBlock`. The candidates
keep that lane-outer order and differ only in parity accumulation:

1. **Two channels:** eight SIMD accumulators hold even/odd × two complex
   channels; each parity is recombined with `cos(theta)` after the degree
   loop. This shares recurrence work, but the V3 specialization spills YMM
   accumulators to the stack inside the pair loop.
2. **One channel at a time:** four SIMD accumulators hold even/odd × one
   complex channel. The recurrence is run once per channel, trading duplicated
   recurrence work for lower live SIMD state.

Both candidates stage and validate both channels before committing spectrum
updates. Numerical safety checks and fallback behavior were unchanged.

## Whole-transform results

Exact T720 CC geometry used `ntheta=722`, `phi0=0.23`, full
`mmax=720, nphi=1458`, and reduced `mmax=180, nphi=378`. Timing reuses plans
and workspaces. The table reports pooled batch medians; the candidate/baseline
column is the median of three paired process ratios. Rust/DUCC ratios are the
medians of the per-process ratios. Full distributions, first-call/setup
measurements, host diagnostics and validation rows are in the
[machine-readable data](data/plan16-parity-separated-t720-2026-10-22/README.md).

| Operation | Order | Threads | Baseline Rust / DUCC ms | One-channel Rust / DUCC ms | Candidate / baseline | Rust/DUCC baseline → candidate |
|---|---:|---:|---:|---:|---:|---:|
| Analysis | 180 | 1 | 72.598 / 35.038 | 72.051 / 34.999 | 0.998× | 2.066× → 2.056× |
| Analysis | 180 | 8 | 12.534 / 10.514 | 12.721 / 10.539 | 1.001× | 1.202× → 1.223× |
| Analysis | 720 | 1 | 185.530 / 118.358 | 185.621 / 120.299 | 1.005× | 1.562× → 1.543× |
| Analysis | 720 | 8 | 32.350 / 21.287 | 32.380 / 21.176 | 0.994× | 1.524× → 1.532× |
| Synthesis | 180 | 1 | 45.276 / 19.104 | 55.699 / 19.248 | 1.232× | 2.380× → 2.847× |
| Synthesis | 180 | 8 | 17.523 / 2.674 | 17.739 / 2.696 | 1.018× | 6.540× → 6.496× |
| Synthesis | 720 | 1 | 96.207 / 44.596 | 119.423 / 44.860 | 1.234× | 2.152× → 2.641× |
| Synthesis | 720 | 8 | 23.885 / 6.199 | 25.508 / 6.161 | 1.080× | 3.865× → 4.150× |

The one-channel candidate is neutral in analysis. It regresses q1 synthesis by
about 23% at both order ranges, with all three process-pair ratios between
1.19× and 1.26×. q8 synthesis is within 2% at reduced order and about 8%
slower at full order; its three full-order ratios were 1.082×, 0.925× and
1.080×, so the q8 result is variable and does not support a gain. Per-process
runqueue wait was generally low for q8 (median below 0.5%); the highest
observed wait among these runs was 8.2% on a q1 reduced synthesis trial.

All 48 Rust/DUCC output comparisons were finite, repeat-consistent and
reset-consistent. Maximum relative L2 error was `3.64e-13`. The analysis
controls remained near baseline. The initial two-channel screen also showed
q1 synthesis slower (about 1.38× full and 1.52× reduced candidate/baseline),
but its q8 DUCC timings drifted sharply between source runs and are excluded
from q8 attribution.

## Generated-code and stage evidence

The two-channel V3 vectorizer is approximately 3.6 KiB and allocates a
392-byte stack frame; two YMM accumulators are loaded/stored at stack offsets
inside the recurrence loop. The one-channel V3 vectorizer is approximately
2.0 KiB and has no YMM stack references in its hot loop; it uses packed
`vfmadd*pd` operations for recurrence and accumulation, with scalar `sd`
instructions in the tail path. Thus the one-channel source change did remove
the measured spill mechanism, but its extra recurrence pass erased the
expected throughput benefit. This is a direct example where cleaner register
allocation alone did not improve end-to-end execution.

The one-channel phase screen at T720 CC showed the q1 recurrence/spectrum
phase rising from 95.3 to roughly 115–118 ms full and 45.8 to roughly 53.6–55.2
ms reduced. q8 full rose from 21.0 ms to roughly 22–24 ms; q8 reduced changed
from 17.5 ms to roughly 15.9–16.7 ms. These are instrumented Rust phases, not
cross-library stage ratios. Whole-transform timings above decide disposition.

## Reproduction and disposition

Pinned comparison: DUCC 0.42.0 commit
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2.0 with SHT sources
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the native driver uses
`-fno-fast-math`. Rust is 1.99.1 with `-C target-cpu=x86-64-v3`. The CPU was a
Ryzen 9 5950X; q1 used CPU 0 and q8 CPUs 0–7 within one L3 domain. Executable
hashes, batch samples, distributions and validation are preserved under
[`reports/data/plan16-parity-separated-t720-2026-10-22`](data/plan16-parity-separated-t720-2026-10-22/README.md).
The temporary implementations are preserved as a patch there and removed
from production `src/`.

The parity-separated hypothesis is closed without a production change. Plan
16 continues with Milestone C: measure and, if worthwhile, reuse the prepared
even/odd paired coefficient streams across ring groups. The code must first
confirm exact preparation reuse counts, cache memory, invalidation and
end-to-end costs; converted scalar-row reuse is already implemented and is
not the target of that experiment.
