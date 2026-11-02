# B3 per-degree shared-channel fusion — rejected

## Decision and source

The isolated f64 candidate fused Q/U accumulation into a one-degree recurrence step. It avoided AVX2 YMM stack traffic, but the GL T720 screen was slower than its exact parent in all four cells by point estimate and slower than pinned DUCC in every cell. **Do not promote this candidate.** Production source is unchanged.

Rust source base: `4a55859649d5f74603ad458890edc6a34adf3b6f`. The candidate was built from this base in an isolated worktree with `rustsht_b3_per_degree_shared_channel`; the archived patch also contains the gated profiling and earlier two-degree experiment hooks. The local worktree path is omitted. Patch SHA-256: `ca3b11826d43514b84d5d3cfd95d6331b1b0fe2b682553ef006dce7c00097e35`.

Pinned DUCC source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0). Host: AMD Ryzen 9 5950X. Rust release flags: `-C target-cpu=x86-64-v3`; runtime path is Pulp V3 AVX2/FMA. DUCC uses `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with `-fno-fast-math` on the driver. `perf_event_paranoid=4` prevents PMU measurements.

Binary SHA-256: candidate `591c61f8dd236e6f1486dd7d4f022fab247fd47fe0d062883981e52630c8f99b`; parent `8c7d39c1547e7f056e0a760d41529da04c5db445996c7e1199c666c120a2ecd7`; DUCC `b646183e9df8aac030f796e2357b8d92e3b4a194d21be98b460eae043ab63443`.

## Screen

Three interleaved process triplets per cell; each worker used 10 warm batches targeting 25 ms per batch. q1 used one pinned worker and q8 used eight; exact logical CPU indices remain in local run data and are omitted here. All calls used GL T720, `phi0=0.23`, 721 rings, full M720 with 1458 longitudes or reduced M180 with 378 longitudes. Ratios are paired geometric means; intervals are 95% t intervals on log ratios with df=2, so treat them as a noisy rejection screen rather than a promotion campaign.

| Cell | Candidate ms | Parent ms | DUCC ms | Candidate/parent | Candidate/DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 37.824 | 32.972 | 29.599 | 1.119 [0.994, 1.259] | 1.246 [1.112, 1.395] GAP |
| Full q8 | 5.159 | 4.863 | 4.224 | 1.068 [1.045, 1.091] GAP | 1.247 [1.133, 1.372] GAP |
| Reduced q1 | 15.013 | 12.789 | 11.622 | 1.136 [0.955, 1.351] | 1.252 [1.079, 1.452] GAP |
| Reduced q8 | 2.131 | 1.917 | 1.730 | 1.084 [0.901, 1.303] | 1.238 [1.192, 1.286] GAP |

The candidate reproduced the parent output closely: E/B relative L2 was `4.71e-17`/`4.16e-17` full and `2.76e-17`/`3.04e-17` reduced. Candidate/DUCC relative L2 was `4.76e-14`/`4.76e-14` full and `5.19e-14`/`5.23e-14` reduced. These are regression comparisons, not independent numerical acceptance.

## Code-generation evidence and timed leaf comparison

The selected V3 candidate body is 2,869 bytes with 10 static YMM FMA instructions, 31 conditional branches, and no YMM stack references. The parent body is 6,129 bytes with 46 static YMM FMA instructions, 89 conditional branches, and no YMM stack references. Static counts do not measure dynamic cycles. The candidate steps one degree at a time and reduces/stores each degree; the parent advances two degrees in one recurrence traversal and accumulates the second channel separately. Repeated per-degree setup, range reduction, and output reduction are plausible costs, but this screen does not isolate them.

A test-only leaf probe then compared the candidate and production `FusedScalarAnalysisPair` on the same deterministic ordinary-range f64 vectors (m=16, lmax=720, 256 lanes, 9 interleaved q1 pairs of 1,000 calls). The candidate took 41.02 us/call versus 28.47 us/call for the exact parent: 1.425 [1.375, 1.477] candidate/parent. State reset copies were outside the timer; `Arch::dispatch` and the complete leaf were timed. This synthetic probe shows the per-degree helper itself is slower, so q8 scheduling cannot explain that candidate regression. It is not a cycle attribution for the production-vs-DUCC gap: the inputs are synthetic and PMU access remains blocked. Candidate and parent microbenchmark binary hashes are `2b41ed720a336882770c401a886d93252e7e447a570628fd472e29ff6b65574d` and `10f6bb2ad807287c7e14ee88e3a4699ba78402e9107dbd8c486f35ed0968c7c2`; the test-only patch SHA-256 is `dfa8d7bf1bbba8410619b4b5c2fe4aa8e90c17847d084f26690787dbc6d0afd5`. The paired measurements are in [`leaf-microbench-results.json`](leaf-microbench-results.json).

B3 has now reached the stop rule: the native-leaf route, duplicate scalar passes, and per-degree fusion each failed whole-call screens, with the last also losing at the leaf. Do not add more B3 production code in this bounded investigation. Return to the remaining bounded B1 f64 hypothesis; if B3 work resumes later, first measure actual production ordinary recurrence and channel accumulation disjointly on real fixture data. Existing failed route screens and the two-degree register-pressure attempt are recorded in the active M6 report.

Focused validation passed with the candidate cfg: `cargo fmt --all -- --check`; q1/q8 consistency; reflected extreme-input fallback and poisoned-scratch reset; fused-channel fallback; paired-analysis versus full-latitude recurrence; and poisoned-workspace extreme reuse. The per-pair times and output metrics are in [`paired-screen.json`](paired-screen.json); exact fixture hashes are in [`fixture-hashes.txt`](fixture-hashes.txt). The reproducible candidate/probe patch is [`candidate-and-probe.patch`](candidate-and-probe.patch).
