# B1 f64 pair-advance branch hoist — rejected screen

## Question and source

On exact parent 4a55859649d5f74603ad458890edc6a34adf3b6f (Rust analysis.rs blob b0077a7236f7e2315fe0c6b07da74f3eb4e845ba), does specializing the optional second recurrence step remove a per-vector decision and pointer spill from B1's ordinary SIMD loop quickly enough to improve a complete transform?

The isolated candidate changes only accumulate_two_normal_scalar_analysis_pairs in src/legendre/scalar_paired/analysis.rs:1349. It dispatches once per two-degree block to const variants (ADVANCE_AFTER_SECOND_PAIR=true/false) instead of testing the runtime flag inside the SIMD loop. Recurrence and accumulation order are unchanged. The candidate source blob is 8e9839b4b3321dcf2d5e425082edb8e039b1bb2d; its patch SHA-256 is cd0c060e8ebec7c5ad2a0792be81ae43b55248fe0e77aab69b26004dd68d4325.

The current parent routes through analysis_into → analyze_order_jobs_dynamic → analyze_order_group → analyze_symmetric_order_group → fused_scalar_analysis_pair_block_preweighted_typed → FusedScalarAnalysisPairTyped<f64> / accumulate_two_normal_scalar_analysis_pairs. Runtime dispatch selects the Pulp V3 AVX2/FMA leaf on this Ryzen 9 5950X (four f64 lanes). The ordinary recurrence is next = fma(fma(a, cos², b), current, previous); the same SIMD visit performs same/opposite real/imag accumulation for two degree pairs.

The exact pinned DUCC comparison target remains 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b. This screen stopped at the Rust parent/candidate gate because no material whole-call improvement appeared; it contains no new candidate/DUCC measurement. Existing DUCC ratios remain source-specific and are not combined with this synthetic-fixture screen.

## Current-source elapsed phases

Five warm calls per process, using the test-only phase observer at the exact parent source. GL T720, ntheta=721, nphi=1458, phi0=0.37; deterministic nonzero map
sin(i*0.000173) + 0.2*cos(i*0.000071). q1 was pinned to CPU 0; q8 to CPUs 0–7.

| Cell | Wall ms | Legendre recurrence + accumulation ms | Forward FFT ms | Legendre / wall |
|---|---:|---:|---:|---:|
| Full q1 | 15.505 | 12.337 | 3.150 | 79.6% |
| Full q8 | 2.481 | 2.010 | 0.457 | 81.0% |
| Reduced q1 | 7.647 | 5.538 | 2.098 | 72.4% |
| Reduced q8 | 1.263 | 0.855 | 0.327 | 67.7% |

Setup and unmeasured phases were below 0.02 ms. These phase shares localize the remaining elapsed time but do not split recurrence from accumulation or assign cycles. perf stat failed because perf_event_paranoid=4; hardware cycles, instructions and cache counters are unavailable.

## AVX2/FMA inspection

Parent Pulp V3 FusedScalarAnalysisPairTyped<f64> is 6,818 bytes with a 472-byte stack frame. Its ordinary two-pair loop uses YMM FMA for four complex components at both degrees and recurrence updates. In that loop cmp pair_index_plus_two, pair_count guards the second recurrence on each four-lane iteration; the same loop reloads one spectrum base pointer from a stack slot.

The candidate is 7,956 bytes (+16.7%) with the same 472-byte frame. The common specialized loop removes the per-vector pair-count branch and the associated base-pointer reload; the function as a whole has seven YMM stack references versus four in the parent, so specialization moves/increases stack traffic in other generated paths. These disassembly observations motivated the screen; they do not establish cycle savings.

The rejected candidate and test-only phase-probe patches are preserved under `raw/`. Raw pair logs, test output and disassembly captures remain local and untracked; the summary metrics below are the published record for this screen.

## Three-pair whole-call screen

The test-only phase observer ran five warm complete analysis_into calls per process. For each cell, three parent/candidate process pairs alternated order P→C, C→P, P→C. The table reports each paired ratio as candidate median / parent median; ratios below 1 favor the candidate.

| Cell | Parent median ms | Candidate median ms | Paired ratios | Median ratio |
|---|---:|---:|---|---:|
| Full q1 | 15.615 | 15.747 | 1.003, 1.016, 1.002 | 1.003 |
| Full q8 | 2.299 | 2.287 | 0.987, 0.991, 1.018 | 0.991 |
| Reduced q1 | 7.817 | 7.733 | 0.969, 0.987, 0.996 | 0.987 |
| Reduced q8 | 1.191 | 1.184 | 0.963, 1.070, 0.994 | 0.994 |

This is a screening sample, not a confidence claim. Full q1 is slightly slower; full q8 and reduced q8 are unresolved; reduced q1 has a small favorable point estimate but the three paired ratios vary from 0.969 to 0.996. Reject the candidate: there is no credible material whole-call gain to justify the 16.7% larger leaf. Production source and dispatch remain unchanged.

Focused release tests passed: 38 scalar-paired analysis tests, 9 manual benchmarks ignored. Formatting and git diff --check passed. No 15-pair campaign, broad suite, protected T1023 control or CI run was warranted for a rejected screen. No code was promoted or pushed.

## Reproduction and provenance

- Parent commit/source: 4a55859649d5f74603ad458890edc6a34adf3b6f, tracked analysis blob b0077a7236f7e2315fe0c6b07da74f3eb4e845ba.
- Candidate code patch: raw/candidate.patch, SHA-256 cd0c060e8ebec7c5ad2a0792be81ae43b55248fe0e77aab69b26004dd68d4325.
- Test-only phase-probe patch: raw/test-probe.patch, SHA-256 a48829a43badf0a5461ffa530da18f78edf7f7864af129ed57ab027e2e33045e.
- Parent test binary SHA-256 1d854e01a791fabbd712fc1669f04f8a9c926cfc724c253f7ffdc069b53cfb35; candidate test binary SHA-256 b8d1dd4e741387044ecdfebddbf4f2ebe87b1c827100be1d0f3fdd646a880971.
- Isolated candidate worktree ref: `m6-b1-hoist-inner-branch-20261102`; parent probe worktree ref: `RustSHT-b1-batch-parent-4a558`. Neither changes the `dev` checkout.
- Toolchain: rustc 1.99.1, LLVM 23.1.1, Cargo release profile; CPU AMD Ryzen 9 5950X. perf_event_paranoid=4 prevented PMU access.
- The exact runner, affinity snapshots, phase logs and per-process paired logs remain local scratch and are not part of this compact archive. The workload, cell order, aggregate measurements and validation result above are the durable screen record.

## Decision

Do not promote this branch-hoist candidate. B1's measured Legendre region remains a source-specific elapsed-time finding, but the tested per-vector branch/spill mechanism is not a demonstrated transform bottleneck. The B1/B3 investigation is paused; no automatic follow-up experiment is scheduled.
