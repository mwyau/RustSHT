# M6 — current-head DUCC parity triage

**Source:** `26941cf45564a9637b72b0eca87ff880472c3505` on `dev`<br>
**DUCC:** 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`<br>
**Host:** AMD Ryzen 9 5950X, Linux x86-64; rustc/Cargo 1.99.1, LLVM 23.1.1; GCC 15.2.0.

This first M6 screen covers 14 selected warm whole-transform cells: GL T720 spin-1 analysis, protected T1023 synthesis controls, two reduced synthesis routes, and native-f32 GL T720 scalar/spin-1 analysis and synthesis. The rest of the agreed matrix remains unmeasured. No production algorithm changed during this screen.

## Method

Both Rust workers were built in release mode with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the f32 worker reported Pulp V3 and eight f32 lanes. DUCC numerical sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; both C++ drivers used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`. The pinned DUCC tree was clean at the exact source SHA. Binary and fixture SHA-256 values are recorded in the [per-cell CSV](data/ducc-parity-m6-2026-10-27.csv).

Each cell used three interleaved fresh-process pairs with shared deterministic fixtures, matching geometry, normalization, precision, and output dimensions. Timing covers warm calls using reusable Rust plans/workspaces and direct DUCC APIs; setup, fixture I/O, and output checks are outside the timed batches. The f64 worker targeted 25 ms and the f32 worker 10 ms per batch; each used ten batches for calls below 100 ms and five otherwise. q1 and q8 are thread counts. Workers pinned q1 to CPU 0 and q8 to eight distinct physical cores. f64 q8 used CPUs 0–7; f32 q8 used 0,2,4,6,8,10,12,14.

Three pairs give a triage range, not a confidence interval. The ledger retains each pair's Rust and DUCC medians, ratio, correctness data, fixture hashes, binary hashes, scheduler wait, and frequency samples. f64 comparisons passed finite, repeat, and reset checks. f32 comparisons passed finite, repeat, reset, and output-length checks; the f32 harness records DUCC-relative errors without enforcing a tolerance. Its broader independent-reference, adjoint, and exceptional-value gate remains open.

## f64 cells

| Workload | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Status | Max relative L2 |
|---|---:|---:|---:|---:|---|---:|
| GL T720 full spin-1 analysis q1 | 36.724 | 29.173 | 1.263× | 1.244–1.264× | **GAP** | 4.76e-14 |
| GL T720 full spin-1 analysis q8 | 5.285 | 4.244 | 1.236× | 1.232–1.249× | **GAP** | 4.76e-14 |
| GL T1023 full spin-1 synthesis q1 | 88.272 | 119.637 | 0.737× | 0.736–0.740× | **PASS** | 2.60e-13 |
| GL T1023 full spin-1 synthesis q8 | 14.072 | 19.737 | 0.719× | 0.704–0.720× | **PASS** | 2.60e-13 |
| CC T720 reduced m180 spin-1 synthesis q1 | 30.006 | 18.593 | 1.616× | 1.591–1.658× | **GAP** | 3.62e-13 |
| GL T720 reduced m180 over-resolved synthesis q8, ntheta=1441 | 4.255 | 4.647 | 0.916× | 0.913–0.920× | **PASS** | 2.27e-13 |

The T720 analysis gap is present at both thread counts. Rust q1 scheduler wait was 6.5–6.9% across its three analysis runs, while DUCC wait was low; all paired ratios exceeded 1.23. The CC reduced q1 Rust runs also had about 10% wait, and all three ratios remained above 1.59. The q8 analysis runs had low wait. The T1023 synthesis controls preserve the established speed advantage.

## Native-f32 cells

All cells use GL T720 full `mmax`, `ntheta=721`, `nphi=1458`.

| Transform | q | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Max relative L2 | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Scalar analysis | 1 | 591.691 | 13.922 | 42.515× | 42.398–42.606× | 1.83e-5 | **GAP** |
| Scalar analysis | 8 | 77.904 | 3.434 | 22.139× | 17.287–27.916× | 1.83e-5 | **GAP** |
| Scalar synthesis | 1 | 873.293 | 19.420 | 44.881× | 44.424–45.501× | 1.72e-4 | **GAP** |
| Scalar synthesis | 8 | 339.733 | 3.748 | 91.134× | 65.084–118.232× | 1.72e-4 | **GAP** |
| Spin-1 analysis | 1 | 1084.978 | 28.811 | 37.530× | 37.402–37.740× | 1.57e-5 | **GAP** |
| Spin-1 analysis | 8 | 141.391 | 4.420 | 32.144× | 29.616–32.463× | 1.57e-5 | **GAP** |
| Spin-1 synthesis | 1 | 1535.415 | 40.581 | 37.715× | 36.713–38.063× | 1.43e-4 | **GAP** |
| Spin-1 synthesis | 8 | 279.188 | 6.063 | 46.628× | 41.908–46.691× | 1.43e-4 | **GAP** |

The q1 f32 runs had low scheduler wait. q8 scalar analysis/synthesis saw median wait of about 14.6%/10.5%; spin-1 q8 wait was lower but variable. Those conditions widen the q8 ratios, while every observed f32 ratio remains at least 17× DUCC. The timing classification is a clear **GAP**; the dedicated f32 numerical acceptance gate remains pending.

## Next

The ranked indexed-loop candidate has now been screened at GL T720 spin-1 analysis q1/q8 against the exact parent and pinned DUCC. Outputs matched bitwise, but the 15-pair timing intervals include parity at both thread counts; the unsafe indexed variant was removed. The next investigation is the checked-scaled path from rank 2 below. The selected-cell ledger and bounded harness patch are in [the evidence directory](data/ducc-parity-m6-2026-10-27/README.md). This triage does not certify the broader f64/f32 matrix.

## Phase B profile — current source `feb46e5f`

The release worker uses the same GL T720/full-mmax geometry and seeded finite maps as the M6 harness. Source files in the profile worktree match production source at `26941cf4`; only test-gated work counters were added for the separate work-count helper. Rust used rustc/Cargo 1.99.1, LLVM 23.1.1, release debug symbols, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, and Pulp V3. Pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2 with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its driver added `-fno-fast-math -ffp-contract=fast`. Valgrind/Callgrind was 3.26.0. Each worker warmed three times and then measured one transform between Callgrind START/STOP controls; setup and output I/O were outside the measured region. Callgrind reported a `brk segment overflow` warning but completed all four profiles. `perf` was unavailable at `perf_event_paranoid=4`; system settings were left unchanged.

| q | Rust whole-call Ir | DUCC whole-call Ir | Rust/DUCC Ir | Rust fused pair kernel inclusive Ir | DUCC `calc_map2alm` inclusive Ir |
|---:|---:|---:|---:|---:|---:|
| 1 | 639,742,202 | 429,390,692 | 1.4899× | 260,221,946 | 227,226,660 |
| 8 | 643,881,097 | 429,534,467 | 1.4990× | 260,221,946 | 227,226,660 |

These are instruction counts, not cycles. Inclusive rows overlap their callees and must not be summed. Rust's `analyze_spin_scalar_reflected_order` stage is 534,633,890 Ir at q1 and 534,626,783 at q8 (about 83% of each whole call); it calls the fused pair-block route 3,579 times. The hot `FusedScalarAnalysisPair<true,false,false,true,true>` specialization is called 2,971 times. Its inclusive Pulp vectorization cost is 260.22M Ir, while direct source attribution is 106.61M Ir. The separate checked-scaled path runs 577 `<false>` calls and 22 `<true>` calls; its `<false>` vectorization source attribution is 32,682,944 Ir (5.1% of whole-call Ir). Do not add these nested inclusive and source-attributed values.

The executed AVX2 recurrence body at Rust PC `0x1d3212` ran 3,996,655 vector iterations at q1 and q8. Each iteration has 12 `vfmadd*` operations, six `vmovupd` loads, two vector stores and loop control; no vector spills appear in that loop. The separate U-channel accumulation body at `0x1d3ab5` ran 1,849,862 iterations, with eight `vfmadd*` operations and about ten vector loads; it also has no loop-body spills. DUCC `map2alm_kernel` at `0x67cab0` ran 9,934,482 vector iterations in each profile. Its paired-degree body has 12 FMA instructions, memory operands in the FMAs, recurrence stores, and loop control. The different batch/lane layouts make iteration totals a trace of executed work, not a standalone speed comparison.

Thread-safe test-only geometry counters gave the same q1/q8 totals: 387,673 active ring/order pairs; 81,083,602 reflected pair-lane degree states; 162,167,204 Q/U component-channel accumulations; 3,579 fused blocks; zero generic fallback or decline; and 14,316 safety scans over 813,340 lane inputs. The separately derived DUCC recurrence work is 9,934,482 vector iterations × 4 AVX2 lanes × 2 degree-state updates = 79,475,856 executed ringdata degree-state updates. Rust's pair-lane state formula and DUCC's ringdata update formula are close in cardinality, but their accumulation definitions differ; they are reported separately rather than treated as identical arithmetic. The geometry helper used deterministic random finite maps, while the Callgrind worker used seed `1416973757`; route totals matched, but the helper is not timing evidence.

The Rust forward FFT is 34,545,994 Ir and spin conversion is 11,455,960 Ir. DUCC's real FFT is 60,264,064 Ir, `spin0to1` is 16,407,759 Ir, and `fill_m2a` is 13,912,862 Ir. These stages are not the leading Rust/DUCC instruction-count excess. Rust/DUCC worker binaries were SHA-256 `59ed4481bec7fbed4e339f0562f457e9b9965d802c35df26d6ec735856136f48` / `f48064f89dae19e932f22ae92f880f8edc399d14b56ce6af17e4d977fd7c2d7f`; the profiled map inputs were `e3379278cb0dfc9a9f4d5f2c8ece40995f965bd727dcb4ce884b85fc62d7abba` / `dc3a132282b074d16cea41cf7305e64e796a48beac398221776ee8c7ad74d5cc`. Callgrind profile SHA-256 values, in q1 Rust/DUCC then q8 Rust/DUCC order, are `092a91da4a4a8bad9e8d17f0b79a27d975083566f1988230f299379921e220e`, `4110614f0e0602ac8b80ec8c23cfcddff8ba016c840be2b1e63f72abd09e86f2`, `052e0fafaec957a96ab03404349ee4eab60a6eee7a9b92dae045004d74f26fec`, and `933285ef080ef42e1a46ae845ee186ce64d498c936fd8c111d9c1425e4c4708d`. Compact values and definitions are in the [profile CSV](data/ducc-parity-m6-2026-10-27-profile.csv).

### Ranked hypotheses and selected experiment

| Rank | Exact route and evidence | Removable fraction and confidence | Decision |
|---:|---|---|---|
| 1 | Rust fused pair vector loop (`scalar_paired/analysis.rs`, hot PC `0x1d3212`) vs DUCC `map2alm_kernel` (`sht_inner_loop.h`, `0x67cab0`). Rust spends 36,024,856 Ir in `core::iter::zip` attribution inside the fused specialization (5.6% of whole-call Ir); the executed AVX2 loop has six explicit vector loads and two stores around the 12 FMAs, while DUCC uses memory operands in its FMA sequence. Their source-defined pair-lane work counts are similar. | At most 5.6% whole-call Ir is attributed to zip, and only part may be loop machinery; likely 0–5.6% Ir, low-to-medium confidence for elapsed time. | **Screened and removed:** the 15-pair indexed-loop attempt below had no supported q1/q8 speedup; keep the checked iterator unless new codegen evidence changes the trade-off. |
| 2 | Checked-scaled `FusedCheckedScaledAnalysisPairChannels` prefix (`0x1d4ba0` family), 599 calls across false/true specializations; false specialization has 32.68M exclusive source-attributed Ir. | Exclusive source attribution is 5.1% of whole-call Ir; inclusive path cost is larger but overlaps children. The route is polar/subnormal safety work, so practical removable fraction is lower; medium-low confidence. | Defer until the indexed-loop experiment closes; do not weaken fallback or range behavior. |
| 3 | Reflected-order wrapper, preparation and safety scans. Four per-block safety passes total 14,316 scans / 813,340 lane inputs; Rust FFT and spin conversion are already below DUCC's corresponding Ir totals. | The safety scans are about 1–2% of the whole call; low expected return. | Defer; avoid broad scan fusion and preserve transactional fallback. |

This is a bounded causal profile, not a parity claim: the earlier measured q1/q8 Rust/DUCC runtime ratios were 1.263×/1.236× in the triage ledger. The indexed-loop trial below does not establish a performance improvement or parity.

## Phase C indexed-loop screen — candidate removed

The experiment changed only the `ROTATE_SEQUENTIAL_STATES` SIMD-vector loop in `fused_analysis_two_pair_steps_with_simd`: it replaced the nested zipped iterators with an integer index and `get_unchecked(_mut)` after the existing entry checks establish equal vector-slice lengths. The recurrence expressions and update order stayed unchanged; the non-rotating loop was untouched. Candidate patch SHA-256 is `1c5a867b641dec243b3754d03a69d8964123d675e8ce3f1ad43609e7f588cce0`; exact parent source was `1af82adb5190264e89586219c50d61e30b223822`. The patch was reverted after measurement and is preserved with the runner and full results in the [experiment evidence folder](data/ducc-parity-m6-2026-10-27/README.md).

Parent, candidate, and pinned DUCC were built in separate scratch targets. Rust used Cargo/rustc 1.99.1, LLVM 23.1.1, release mode, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the measured worker uses runtime Pulp dispatch, which it does not print. DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2, numerical-source `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and a driver with `-fno-fast-math -ffp-contract=fast`. The case was GL, spin 1 analysis, T720/full-mmax, `ntheta=721`, `nphi=1458`, `phi0=0.23`, seeded finite inputs (seed `1416973757`; map hashes match the Phase B worker). Each process warmed and checked repeat/reset, then recorded ten warm batches targeting 25 ms. Fifteen balanced interleaved parent/candidate/DUCC triplets ran at each of q1 and q8 on the same physical-core affinity used by the checked benchmark harness.

| q | Parent / candidate / DUCC median of process medians (ms) | Paired C/P ratio, median [bootstrap 95% CI] | Faster C/P pairs | Parent/DUCC; candidate/DUCC paired-ratio medians |
|---:|---:|---:|---:|---:|
| 1 | 36.153 / 35.892 / 29.304 | 0.993778 [0.991391, 1.001167] | 11/15 | 1.241058; 1.225888 |
| 8 | 5.293 / 5.307 / 4.224 | 1.001789 [0.999099, 1.011079] | 6/15 | 1.254247; 1.255433 |

Candidate and parent outputs were bitwise identical in all 30 triplets; all three backends were finite and passed repeat/reset checks. Parent and candidate both had maximum DUCC relative-L2 error `4.760590e-14` and maximum absolute error `1.295838e-15`. Timing uncertainty includes substantial q1 runqueue wait (parent/candidate medians about 6.7%/6.8%) and q8 frequency samples reaching 1.746 GHz; no sample was dropped. The q1 interval includes 1.0, q8 is neutral-to-slightly-slower, and the q8 pair spread is wide. The 5.6% zip source attribution from Phase B was only an upper bound; no candidate-specific whole-call Ir reduction was measured. **Decision:** remove the unsafe indexed variant; the screen does not support a material or repeatable f64 gain, while retaining unchecked accesses would add maintenance and proof burden. This is not a parity result.

Candidate validation passed `cargo test --locked --all-features --lib legendre::scalar_paired::analysis` (34 passed, 8 ignored), `cargo test --locked --all-features --lib spin1` (49 passed, 4 ignored), and `cargo fmt --all -- --check`. These focused cases include paired recurrence references, GL/CC/F1 fallback paths, poisoned workspace reuse, q1/q8 recurrence checks, and spin-1 adjoint identities. The full workspace suite and warning-denied Clippy were not run for this rejected variant. Per-triplet process-median timings, summarized wait/frequency data, binary/input/output hashes, exact runner provenance and recoverable source diff are linked from the [evidence folder](data/ducc-parity-m6-2026-10-27/README.md).
