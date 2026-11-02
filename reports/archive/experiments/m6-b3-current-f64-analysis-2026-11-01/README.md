# B3 current-source f64 analysis profile and rejected candidates

This package rebaselines spin-1 analysis at `31c7cd5046d2389d5f0de230577dc147ff37f873`, profiles disjoint Rust phases, records the executed AVX2 kernels, and screens two candidates. Neither candidate is promoted. The production source remains unchanged.

## Exact-parent baseline

The uninstrumented whole-call measurements use 15 interleaved Rust-parent / candidate / DUCC pairs, GL T720, deterministic nonzero PCG64 inputs, Rust 1.99.1 / LLVM 23.1.1, `-C target-cpu=x86-64-v3`, AMD Ryzen 9 5950X. q1 is pinned to one physical core; q8 to eight physical cores. DUCC is pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, compiled with the parity runner's `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3 -fno-fast-math` flags. Paired ratios use the geometric mean and 95% t intervals on log ratios.

| Cell | Rust parent ms | DUCC f64 ms | Rust / DUCC (95% CI) | Status |
|---|---:|---:|---:|---|
| Full q1 | 37.067 | 29.660 | 1.242 [1.224, 1.260] | GAP |
| Full q8 | 5.374 | 4.272 | 1.258 [1.250, 1.266] | GAP |
| Reduced q1 | 15.000 | 11.698 | 1.279 [1.262, 1.296] | GAP |
| Reduced q8 | 2.222 | 1.747 | 1.285 [1.262, 1.309] | GAP |

Sanitized per-pair timings, fixture hashes, executable hashes, and output checks are in [`recurrence-fusion-results.json`](recurrence-fusion-results.json); parent rows are the exact-source baseline. The summary also records parent/candidate bitwise equality and repeat/reset checks.

## Rust phase and kernel profile

Test-only phase probes were compared to uninstrumented whole-call medians. Instrumentation changed totals by at most about 3% in the sampled comparisons, so phase percentages are useful at this resolution but are not PMU cycle attribution. `perf` hardware events are unavailable on this host (`perf_event_paranoid=4`). The committed, path-free medians and work counts are in [`profile-summary.json`](profile-summary.json); full console captures remain local and untracked.

| Cell | Whole-call profile ms | FFT ms | Legendre recurrence + accumulation ms | Other setup/cleanup ms |
|---|---:|---:|---:|---:|
| Full q1 | 37.18 | 6.30 | 30.54 | 0.34 |
| Full q8 | 5.36 | 0.92 | 4.12 | 0.32 |
| Reduced q1 | 15.04 | 1.50 | 13.50 | 0.05 |
| Reduced q8 | 2.16 | 0.23 | 1.85 | 0.08 |

The q1 per-order profile places about 26.7 ms (full) and 11.9 ms (reduced) in the block loop. Sampled block instrumentation, scaled by the sampling interval, estimates fused recurrence/accumulation at 22.3 ms full and 10.0 ms reduced. Gather, raw safety, reflected compaction and pair safety account for most of the remainder. These are sampled test-only elapsed-stage estimates and should not be treated as disjoint hardware counters.

The route processes 3,579 eligible full-order blocks and 1,086 reduced-order blocks. The full case has 194,197 active pair lanes, 48,135 AVX2 f64 chunks and 1,657 scalar-tail lanes; reduced has 64,364 active lanes, 16,015 AVX2 chunks and 304 scalar-tail lanes. The route reports no scaled/mixed blocks for these random benchmark fixtures. The q8 worker traces show broadly balanced leaf work, which does not support claiming load imbalance as the main cause.

The Rust AVX2 V3 leaf is `NativeFusedAnalysisPairChannels<f64, false>`: symbol size `0xc98` (3,224 bytes), 44 static `vfmadd*`, one `vmulpd`, eight `vaddpd`, and a 280-byte stack frame. The extracted leaf has no YMM stack references. DUCC's actual AVX2 `map2alm_kernel` leaf is 722 bytes with 18 static `vfmadd*`, six `vaddpd`, and no stack references. Both execute four-lane f64 vector code. These counts describe compiled leaf bodies, not dynamic instruction totals or a direct work-normalized comparison; DUCC uses a different loop boundary and invokes the scalar-channel analysis twice. A sanitized instruction excerpt is in [`assembly-summary.md`](assembly-summary.md); full disassemblies remain local and untracked.

## Source crosswalk

Pinned DUCC source is the checkout at the revision above.

| Role | DUCC 0.42.0 | Rust at parent `31c7cd50` |
|---|---|---|
| High-bandlimit spin-1 route and two scalar channels | `src/ducc0/sht/sht.cc:851–910`: fast-ring selection, `YlmBase(lmax+spin,mmax,0)`, two `inner_loop_m2a` calls, normalization and `spin0to1` | `src/transform/spin/analysis.rs:596–675`: reflected order/block route and paired Q/U gather; shared block dispatch in `src/legendre/scalar_paired/analysis.rs:5929–6085` |
| Prepared scalar recurrence factors | `sht_inner_loop.h:186–205`, `Ylmgen::prepare`, producing `eps`, `alpha`, `coef.a`, `coef.b` | precision-specific factors prepared through `T::fill_epsilon` and `T::fill_paired_scalar_factors` in `transform/spin/analysis.rs:644–652` |
| Sectoral seed / scaling | `sht_inner_loop.h:413–424`, `init_lambda`; `:426–456`, `iter_to_ieee` | native pair-state initialization and ordinary-seed decision in `scalar_paired/analysis.rs:5996–6046`; scaled path dispatch at `:6014–6038` |
| Degree recurrence and accumulation | `sht_inner_loop.h:592–634`, `map2alm_kernel`; `:636–684`, `calc_map2alm` including scaled-prefix handling. Ordinary recurrence is `(a*csq+b)*current+previous`, with four channel accumulators and two-degree advancement. | `scalar_paired/analysis.rs:5508–5545` and the continuation of `NativeFusedAnalysisPairChannels::with_simd`; common native dispatch at `:5929–6085`. The fused leaf advances recurrence and accumulates both reflected channels. |
| Ring preparation / scheduling | `sht_inner_loop.h:1660–1698`, `fill_m2a`; `:1752–1795`, `inner_loop_m2a` | `transform/spin/analysis.rs:655–675` builds reflected ring pairs; block loop continues through the shared fused path. Dynamic order jobs begin at `:1704`. |
| Spin conversion and final coefficient accumulation | `sht.cc:382–410`, `spin0to1`; called at `:902` after scalar normalization | `transform/spin/analysis.rs:943–960`, `scalar_to_spin1_with_epsilon` and output adds; implementation in `legendre/spin_fast.rs:167–220` |

At T720 GL, all rings are expected to exceed DUCC's spin-1 fast-route cutoff `abs(sin(theta)) >= 1e-4`; this is a geometry inference from the GL nodes, not a separately instrumented per-ring report. The whole-call profile confirms Rust phase boundaries but does not yet reveal DUCC's time split between its two scalar-channel loops and `spin0to1`.

## Candidate 1 — recurrence fusion, rejected

This candidate fused the ordinary f64 recurrence/accumulation path, adapting the accepted f32 fusion. Focused correctness tests passed and parent/candidate outputs were bit-identical. The 15-pair candidate/parent intervals crossed 1.0 in every cell:

| Cell | Candidate / parent (95% CI) |
|---|---:|
| Full q1 | 0.990 [0.978, 1.002] |
| Full q8 | 1.006 [0.986, 1.027] |
| Reduced q1 | 0.993 [0.983, 1.003] |
| Reduced q8 | 0.993 [0.975, 1.011] |

The possible small gains are unresolved and do not justify promotion. The [reproduction patch](patches/rejected-recurrence-fusion.patch.gz) and [sanitized pair data](recurrence-fusion-results.json) preserve the experiment; decompress the patch with `gzip -dc` before applying.

## Candidate 2 — fuse postprocessing, rejected

This candidate removed the paired parity-normalization write/read and scalar-to-spin staging, directly accumulating E/B outputs from the reflected values. Focused spin-analysis, adjoint, poisoned-workspace, fallback, and q1/q8 scheduling tests passed. Outputs were bitwise equal to parent; relative L2 against DUCC stayed `4.76059e-14` full and `5.21269e-14` reduced.

Three-pair candidate/parent results were: full q1 `1.006 [0.975, 1.038]`, full q8 `1.031 [1.024, 1.038]`, reduced q1 `1.002 [0.974, 1.031]`, reduced q8 `0.985 [0.951, 1.021]`. Full q8 is a resolved slowdown; the other cells are inconclusive. Reject this candidate without a 15-pair campaign. Details are in [`postprocess-results.json`](postprocess-results.json) and the [candidate/test-probe patch](patches/rejected-postprocess-and-probes.patch.gz).

## Diagnosis and next discriminating experiment

Both attempts exhausted the bounded candidate budget without reducing the gap. The measured Rust cost is dominated by the large fused order/ring recurrence-and-accumulation loop, not by FFT, coefficient preparation, or an observed q8 load imbalance. Candidate 2's postprocessing removal was too small to offset this dominant loop. The Rust leaf is larger than DUCC's leaf, but static code size alone does not identify the cycle cause.

Next, instrument a separate pinned DUCC checkout at `inner_loop_m2a`/`calc_map2alm` and `spin0to1`, retaining its binary semantics, and time the two scalar-channel calls separately under the same fixture and affinity. Count recurrence updates and active ring lanes per stage, and compare these against Rust's fused leaf. This will distinguish a work-count difference from per-update instruction/dataflow cost before another kernel candidate is selected. Do not infer DUCC's stage split from the whole-call ratio.

## Raw evidence and checksums

The committed package contains sanitized paired results, stage summaries, assembly facts and exact candidate patches. The full unredacted phase logs, executable disassemblies, runner scripts and raw checksum lists remain local and untracked because they contain local paths or raw process output. The committed JSON retains source/binary/fixture hashes and the measurements needed to audit the conclusions.
