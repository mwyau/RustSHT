# Plan 15 spin-1 synthesis dataflow closeout

**Status: complete, 2026-10-21.** The GL, CC and F1 implementation sequence is
finished. The remaining CC/F1 synthesis gap is isolated to the order/recurrence
and spectrum stage; the two final dataflow experiments below did not justify
another production kernel. The current Rust implementation remains in place.

## Matched experiment

Both A/B screens used the production source at
`ae78e18044fea2d65316d230dd434e4b4f8222be` as baseline, pinned DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, Rust 1.99.1 / LLVM 23.1.1 with
`-C target-cpu=x86-64-v3`, and DUCC GCC 15.2 with
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` (strict driver).
The host was a Ryzen 9 5950X. q1 was pinned to CPU 0 and q8 to CPUs 0–7.
Three interleaved process triples collected ten calibrated ~25 ms batches for
short cases; longer calls used the harness's individually timed long-call
protocol. Both exact-CC workloads used `ntheta=722`, nonzero `phi0`, even
smooth `nphi=378` at `mmax=180` and `nphi=1458` at `mmax=720`.

### Active-ring compaction candidate

The candidate packed only rings whose polar order limit included the current
order, then sent them through the ordinary two-channel recurrence. It reduced
profiled fallback blocks from 1,589 to 1,204 (24%) and increased successful
paired blocks from 5,028 to 5,413. However, the q8 whole-transform changes
were only 0.2–0.9% and the paired runs crossed 1.0. Full/reduced synthesis
medians were 23.908/17.507 ms before and 23.702/17.429 ms after. The phase
profile remained about 21 ms, within run variation. Reduced fallback worker
time fell, but recurrence and coefficient-preparation worker sums rose; those
overlapping worker sums do not represent wall time. The extra route was
discarded. Candidate outputs were bitwise equal to baseline; all 24 numerical
and reset checks passed, with maximum DUCC relative L2 error `3.63e-13`.

### Degree-pair-outer candidate

The production AVX2 kernel traverses vector blocks first, keeping recurrence
state and four output sums in YMM registers across degree pairs. DUCC's
`alm2map_spin_kernel` broadcasts pair coefficients before its vector-block
loop and updates per-vector state in workspace. The temporary Rust candidate
reversed the two loops so scalar coefficient broadcasts were shared across
the block's vector lanes; it stored recurrence state and four sums to lane
scratch between pairs.

| CC T720 synthesis | Production Rust | Pair-outer Rust | DUCC | Candidate / production |
|---|---:|---:|---:|---:|
| q1, `mmax=180` | 43.780 ms | 55.375 ms | 18.457 ms | 1.265× |
| q1, `mmax=720` | 94.437 ms | 116.612 ms | 43.998 ms | 1.235× |
| q8, `mmax=180` | 17.348 ms | 17.273 ms | 2.646 ms | 1.000× paired median |
| q8, `mmax=720` | 23.678 ms | 24.899 ms | 6.134 ms | 1.047×; all three pairs regressed |

The q1 synthesis regressions were consistent across the three process pairs.
q8 reduced synthesis was neutral. q8 analysis controls changed by at most
about 1%. The candidate was removed. All 24 candidate numerical/reset checks
passed; output matched production exactly and the maximum relative L2 error
versus DUCC was `3.63e-13`.

Generated optimized code supports the tradeoff explanation. The executed Rust
V3 specialization uses four f64 lanes per YMM vector. Its lane-major hot loop
keeps the current/previous recurrence and four accumulators in registers,
forms four cosine-weighted coefficient vectors, and stores the four sums once
per vector block. The pair-outer experiment removes repeated broadcasts but
the generated loop loads and stores recurrence state and each sum for every
degree pair and vector block. Its measured q1 regression and q8 full-order
regression show that the scratch traffic costs more here than the broadcasts
save. DUCC's spin recurrence is a different mathematical dataflow and performs
additional independent recurrence streams, so static instruction counts
alone do not establish an equivalent-work speed advantage or explain the
whole gap.

## Remaining measured gap and decision

After the accepted CC surplus-group queue, exact T720 q8 CC synthesis is
23.596 ms versus DUCC 6.087 ms full order (3.87×), and 17.179 versus 2.647 ms
at `mmax=180` (6.48×). The matched Rust phase probe measured 18.994 ms and
15.641 ms in order scheduling, recurrence and spectrum accumulation; the two
inverse rows plus writeback were 1.205 and 0.181 ms. Current-source F1 q8
synthesis measured 22.382/5.884 ms full order and 14.338/2.545 ms reduced.
The residual is therefore not in CC/F1 output FFTs or the active-lane
fallback count. It is in the common spin-1 coefficient-conversion recurrence
and spectrum path. The source comparison identifies coefficient-broadcast
reuse versus lane-state traffic as the next structural design constraint,
but the direct loop reversal did not improve whole-transform execution.

No further Plan 15 kernel change is supported by these measurements. The
validated GL, CC and F1 changes remain; rejected candidates were removed. Plan
15 is closed with the recurrence/spectrum residual and the tested failed
mechanism documented rather than claiming DUCC parity.

## Reproduction and artifacts

The three-way timing summaries, process distributions, fixture hashes,
numerical checks and binary hashes are in
[`reports/data/plan15-spin1-synthesis-dataflow-closeout-2026-10-21/`](data/plan15-spin1-synthesis-dataflow-closeout-2026-10-21/README.md).
The degree-pair-outer source patch is retained there for review but is not part
of production. Rust baseline/candidate/DUCC executable hashes are recorded in
the JSON summaries. The native driver and timing command are the same as
those in the [CC scheduling report](plan15-spin1-cc-group-scheduling-2026-10-21.md).
