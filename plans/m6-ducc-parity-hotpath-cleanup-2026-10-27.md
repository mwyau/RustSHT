# M6 — close f64 B1/B3 analysis gaps against DUCC

**Owner direction (2026-11-01):** B1 f64 scalar analysis and B3 f64 spin-1 analysis are both active. Give B1 **at most two** bounded, measured attempts, then make **B3 f64 the primary target**. Do not park B3, resume f32 performance work, or pursue f32/f64 timing equality without a new owner instruction. B0 scalar synthesis and B2 spin-1 synthesis are protected controls. See [AGENTS.md](../AGENTS.md), the [per-cell ledger](../reports/DUCC_PARITY_M6.md) and [progress](../reports/PROGRESS.md).

## Source-specific starting evidence

Pinned native DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. GL T720 full M720/reduced M180, warm complete transform, q1/q8:

| Workload | Full q1 | Full q8 | Reduced q1 | Reduced q8 |
| --- | ---: | ---: | ---: | ---: |
| B1 f64 / DUCC f64 | 1.059× | 1.131× | 1.047× | 1.032× |
| B3 f64 / DUCC f64 | 1.213× | 1.346× | 1.236× | 1.295× |

B1's 15-pair full cells are GAP; reduced q1/q8 confidence intervals overlap parity and are INCONCLUSIVE. The B3 ratios are **older, source-specific** 15-pair GAP results at integrated `094d3162`, not a current-HEAD certification. See the [current B1 matrix](../reports/DUCC_PARITY_M6.md), [B1 zip/seed packages](../reports/archive/experiments/) and [B3 evidence](../reports/archive/experiments/m6-b3-scaled-recurrence-fusion-2026-10-30/README.md). Keep all source/fixture/ISA/affinity identities attached to results.

## 0. Rebaseline and control experimental conditions

1. Check worktrees, dirty state, remote `dev`, exact source blobs, commit and compiler; never discard or overwrite another worktree. Record CPU model, governor, affinity, load/runqueue, q1/q8 threads, Rust/LLVM/GCC, `-C target-cpu=x86-64-v3`, DUCC's actual `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` flags, binary and fixture hashes, and V3 AVX2/FMA route.
2. Reuse `scripts/benchmark_ducc_spin1_native.py` and `scripts/benchmark_ducc_full_transform_parity.py` or their archived exact-source runners. Verify matched Q/U data, geometry, normalization, `phi0`, packed orders, workspace reuse, warm-call boundary and full/reduced work.
3. Retain a short B1 f64 current-source control; refresh **B3 f64 current-source** full/reduced q1/q8 using three interleaved Rust/DUCC whole-call pairs. Record uncertainty and contention; no cross-source extrapolation. Protect f64 T1023 spin-1 synthesis and an out-of-GL control.

## 1. B1 f64 — finish quickly

Inspect disjoint wall-time stages and actual optimized f64 AVX2/FMA instructions. Test **no more than two** reversible hypotheses only if a meaningful remaining seed/recurrence/tail excess is measured. First isolate the helper, then run three exact-parent/candidate/DUCC f64 whole-transform pairs. Stop if no credible total-runtime gain. The accepted `7042ba9` seed-normalization removal and `8560bca` zipped traversal stay. Do not repeat the rejected f32 factor-reuse, range-fusion, lazy-clear, output-scale or normalizer paths unchanged.

## 2. B3 f64 — profile before changing code

Trace the **executed f64** spin-1 route in `src/transform/spin/analysis.rs`, `src/legendre/scalar_paired/analysis.rs` and `src/legendre/spin_fast.rs`. Distinguish reflected, fast, table, ordinary/scaled, SIMD/tail and fallback paths. The previously accepted fused recurrence is **f32-specific**; a f64 port is a hypothesis, not an instruction.

Profile disjoint, low-overhead B3 stages: Q/U FFT; ring scaling and reflected pair packing/compaction; product-safety scans; epsilon/factor/seed preparation; ordinary recurrence; checked/scaled recurrence and exponent normalization; two-channel complex accumulation; scalar-to-spin-1 E/B conversion; writes and scheduling. Record probe overhead and an uninstrumented whole-call control. Do not add inclusive Callgrind counts and call them time. Hardware `perf` may be blocked.

Compare pinned DUCC f64 at **equivalent useful work**: active ring×degree visits, scalar products per Q/U channel, ordinary/exceptional blocks, replay counts, vector and tail occupancy, bytes moved, and q1/q8 worker balance. Inspect optimized AVX2/FMA assembly for redundant bounds checks, slice reloads, scalarized operations, FMA chains, branches, register spills, frames, and factor reloads. Compare executed dominant variants, not source-level appearance. The f32 fusion produced substantially larger machine code; test f64 fusion only if disassembly and stage timings justify it.

**Current measurement (2026-11-02):** the pinned DUCC stage split attributes 62–73% of B3 fast-branch wall to `calc_map2alm` recurrence/accumulation and 80–82% to its two scalar-channel calls. It performs 40.54M full-order and 20.39M reduced recurrence pair updates per channel. Rust uses the same active ring/order geometry and advances the fused recurrence once for both Q/U channels, ruling out excess recurrence updates. Exact-fixture GDB traces establish that all four T720 q1/q8 cells execute the weighted `FusedScalarAnalysisPair` V3 leaf; the smaller `NativeFusedAnalysisPairChannels` leaf is not selected. Opposite-channel cosine preweighting is already present. Two isolated route changes were screened: selecting the native leaf (1.94–2.11× parent) and running two scalar passes (1.20–1.23× parent); both are resolved slowdowns. A test-only accumulator-leaf discriminator then measured the fused two-channel helper at 454.09 ns versus 506.39 ns for sequential channels (nine interleaved samples, 100,000 calls each, bitwise-equal outputs). The sequential helper repeats basis/exponent/mask work and is 11.5% slower; this does not establish whether the active leaf's 2,728-byte frame and 200 YMM stack references cause material spill cost. The B3 stop condition remains: no further B3 kernel candidate without a new, disjoint profile of the active leaf. Full source, binary, fixture, route-hit and paired-result provenance: [DUCC B3 stage split](../reports/archive/experiments/m6-b3-ducc-stage-split-2026-11-01/README.md); the helper comparison is in the [accumulator-leaf package](../reports/archive/experiments/m6-b3-accum-leaf-microbench-2026-11-02/README.md).

**Completed profile (2026-11-02):** sampled timing in the active scaled leaf estimates full/reduced q1 prefix work at 12.54/7.35 ms. The disjoint subphases rank: exceptional recovery 3.10/1.56 ms; recurrence/normalization 2.24/1.36 ms; masked accumulation 1.89/1.27 ms. Values are 1-in-64 sample expansions; q8 sums are worker service, not wall shares. Probe/plain whole-call ratios were 1.004/1.008 q1 full/reduced; q8 was noisy. Full q1 has 607 scalar fallback lanes among 2,405 exceptional lanes per sampled call; reduced has 395/1,009. See the [sampled-leaf profile package](../reports/archive/experiments/m6-b3-sampled-scaled-leaf-2026-11-02/README.md) for source, per-call records, overhead, and AVX2 disassembly.

The B3 stop condition was reached after two whole-call route candidates were resolved slowdowns (compact native leaf 1.94–2.11× parent; two scalar passes 1.20–1.23×), and partial-exception SIMD measured 2.835 [2.763, 2.908]× scalar over two pinned 9-pair helper screens. The follow-up test-only replay captured 427 full and 264 reduced q1 recurrence steps. Stable normalization cost 2.73–2.98 ns/vector versus 1.42–1.46 ns/vector for recurrence replay; exponent-changing groups cost 17.3–18.4 ns/vector but were only 2.40%/0.76% of captured groups. Disassembly shows multiple horizontal reductions in the steady-state check, making repeated mask validation the next testable mechanism. These are helper-throughput results, not cycles or whole-call savings. Capture changed sampled-step timing; q8 captures were disabled, and q8 stage timings are worker service. No production change was made. See the [recurrence/normalizer replay package](../reports/archive/experiments/m6-b3-recurrence-normalizer-replay-2026-11-02/README.md), the [partial-exception screen](../reports/archive/experiments/m6-b3-partial-exception-vector-screen-2026-11-02/README.md), and the [sampled-leaf profile](../reports/archive/experiments/m6-b3-sampled-scaled-leaf-2026-11-02/README.md).

**NEXT:** in test-only captured-state microbenchmarks, remove or combine one steady-state mask reduction at a time while retaining the existing finite/range/overflow fallback. First quantify the common stable-vector path, then run a three-pair whole-call screen only if the helper shows a material gain. No production kernel change is justified yet. B1 remains bounded to at most one further f64 attempt; f32 performance remains deferred.

## 3. Evidence thresholds, correctness and cleanup

**Screen:** narrow f64 helper/call-site microbenchmark and focused tests; then three interleaved whole-transform parent/candidate/DUCC pairs for affected T720 full/reduced q1/q8. Reject small helper wins with no stable whole-call benefit. **Promote:** 15+ exact-source paired whole-call observations with uncertainty for materially affected cells, accurate DUCC comparison, relevant f32 shared-code guards, and T1023 f64 controls. Keep PASS/GAP/INCONCLUSIVE separate; a ratio near 1 with an interval crossing 1 is not established parity.

Preserve GL/CC/F1, full/reduced mmax, even/odd nphi/Nyquist, spin/sign/adjoint correctness, S/A/S*/A*, poles/equator, tiny/subnormal/overflow/nonfinite, SIMD width/tails, and poisoned/reused workspace behavior. Do not weaken tolerances. Investigate the intermittent **f32 nonfinite SIMD-tail** failure as a small, separate correctness commit, not a reason to resume f32 performance work. Run full Rust/Python, Clippy, format and CI only for a candidate ready for promotion or a finished cleanup, not every speculation.

After measurement, simplify only demonstrably redundant production code (dead branches, scratch, extra allocations, duplicated factor work); retain DUCC-derived attribution and a single shared generic mathematical algorithm with narrow precision-specific leaves. Separate behavior-preserving cleanup commits from performance changes. Preserve experiment patches/data/hashes in the archive instead of deleting evidence.

## 4. Broaden f64 only after B3

Once B3 is stable, inventory remaining **current-source** f64 GL/CC/F1 scalar/spin-1 analysis, synthesis and adjoint cases (q1/q2/q4/q8, full/reduced, T511/T720/T1023, exact/oversampled). Rank by absolute recoverable milliseconds and regression risk; historical synthesis PASS/GAP does not certify current HEAD. Never average away per-cell regressions.

## Stop and report

If two B1 experiments fail, move to B3. If B3 cannot be attributed or two high-value candidate mechanisms fail whole-call screens, **stop adding speculative code** and report the ranked evidence and remaining limitations. Update the live [ledger](../reports/DUCC_PARITY_M6.md), [five-checkpoint progress](../reports/PROGRESS.md) and [status page](../PERFORMANCE_STATUS.md). Keep archived narratives historical, not new execution overrides.
