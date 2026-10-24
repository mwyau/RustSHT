# Plan 14: AVX2 compact-table scalar analysis at T127

**Implementation:** `0484a876b76aaa5927f6be3924e227452c2d3ed8`.
**Source baseline:** `1bfede726e824d4ded35201308de6b1da70814af`.
**DUCC:** 0.42.0, pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Change

The compact Legendre table is still used for T127 scalar analysis. On Pulp
V3, when `ntheta` is divisible by four, analysis now packs two reflected
complex latitude pairs into each YMM vector and accumulates adjacent even
and odd degrees together. The scalar path remains for other lane widths and
geometries. The analysis table selector, the T64 synthesis policy,
spin-1, f32, table construction, finite-value safeguards and fallback are
unchanged.

The implementation uses a separate f64 Pulp specialization; it is not an
intrinsic-only x86 path. The executing host reported runtime Pulp V3
(AVX2/FMA, four f64 lanes). The optimized Rust body contains packed
`vaddpd`, `vsubpd`, `vmulpd` and `vblendvpd`; reflected complex lanes are
reordered with vector shuffles. Products and additions remain separate
under Rust's strict floating-point settings. Horizontal reduction happens
after the ring-vector loop. The loop still has scalar iterator/bounds
control and some scalar iterator-state traffic; this change does not claim
that its assembly matches DUCC's simpler indexed loop. The local annotated
disassembly is not committed.

## Matched whole-transform results

The case is f64 scalar GL analysis, T127, full `mmax=127`, `ntheta=128`,
odd-minimum `nphi=255`, `phi0=0.23`. The baseline used clean commit
`1bfede72`. The candidate binary was built from that base with exactly the
source/test diff later committed as `0484a876`; its raw JSON records that
the worktree was dirty at capture time because the implementation had not
yet been committed. Rust was
1.99.1 / LLVM 23.1.1, release mode, `-C target-cpu=x86-64-v3`, with runtime
Pulp dispatch. DUCC used the pinned source, GCC 15.2 and x86-64-v3; DUCC SHT
sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, while
the benchmark driver used strict FP. The host was an AMD Ryzen 9 5950X.

Each Rust/DUCC pair reused plans and workspaces and validated outputs
outside the timed region. Trials alternated Rust/DUCC process order, with
three independent process pairs and ten calibrated 20–100 ms batches per
backend in each process (30 batches per backend/case). q1 used one physical
CPU; q8 used the same eight physical CPUs and reused worker pools. First
call and setup are recorded separately in the raw results.

| Threads | Rust baseline p10 / median / p90 (µs) | Rust candidate p10 / median / p90 (µs) | DUCC baseline → candidate median (µs) | Rust/DUCC baseline → candidate | Candidate excess (µs) | Rust change | Candidate relative L2 / max abs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| q1 | 396.597 / 398.496 / 404.001 | 373.555 / 376.600 / 394.807 | 291.294 → 294.042 | 1.368x → 1.281x | +82.558 | **-5.49%** | `8.52e-15` / `1.18e-15` |
| q8 | 218.029 / 221.254 / 248.754 | 218.301 / 223.743 / 233.333 | 80.150 → 80.872 | 2.760x → 2.767x | +142.871 | +1.12% | `8.52e-15` / `1.18e-15` |

The q1 whole-call reduction is useful and exceeds the observed within-run
spread, though the q1 host had nonzero runqueue wait (median 7–8%, p90
25–27%). DUCC's candidate-round median was 0.9% slower than its baseline
round, slightly understating the Rust gain. q8 is inconclusive: medians
overlap and host interference increased in the candidate round (runqueue
wait median 10.8%, involuntary switches median 2028 versus 757.5 at
baseline). There is no q8 improvement established by this A/B.

First-call Rust latency was 4.271 → 4.182 ms at q1 and 4.232 → 4.339 ms at
q8. These setup/cold values are not part of the warmed comparison. Baseline
and candidate q1 phase probes put the compact-table accumulation stage near
240 and 220 µs respectively, but only one candidate process was captured;
the full-call comparison above is the primary result.

## Dispatch and numerical checks

A separate T127 q1 production-route control measured three scalar-table
phase medians at 239.5–241.5 µs and forced recurrence at 243.3–245.5 µs.
That control does not support replacing the analysis table with recurrence.
The table remains selected; only its accumulation implementation changes.

The transform benchmark compared deterministic PCG64 inputs against DUCC
in all three process pairs. Relative L2 remained about `8.52e-15`, maximum
absolute error `1.18e-15`, and outputs were reset-consistent. The existing
zero, subnormal, extreme-finite, basis, roundtrip and adjoint validation
also passed. A production regression test compares table and recurrence
for all four scalar operators at T63, T126 and T127, including even and odd
latitude counts. Full release tests passed with and without parallel
features; strict Clippy, formatting and `git diff --check` passed.

## Disposition and next work

Keep the V3 table accumulator. It improves q1 T127 analysis by 21.9 µs but
still leaves an 82.6 µs whole-call gap to DUCC. It does not improve q8 in
this measurement. Preserve the production table selector and use the
scalar fallback on other vector widths or shapes.

Next, continue Plan 14 Phase A's fixed-cost attribution across T1–T64. Then
re-rank the largest q8 whole-transform excess using current-source matched
controls; do not infer a q8 win from this table-specific q1 result.

Raw native benchmark outputs, process distributions, checksums, host
samples and cold/warm plots are in
[`reports/data/plan14-t127-table-analysis-simd-2026-10-21/`](data/plan14-t127-table-analysis-simd-2026-10-21/).
The summary CSV is `comparison.csv` there. The disassembly and raw
instrumented stage logs remain local.
