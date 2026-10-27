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

The two-vector SIMD recurrence interleave candidate has now been screened at GL T720 spin-1 analysis q1/q8 against the exact parent and pinned DUCC. Across three triplets per q, parent/candidate outputs were bitwise identical and finite/repeat/reset checks passed, but paired candidate/parent medians were 1.105232 at q1 and 1.077169 at q8; all six pairs favored the parent. q1 wait was balanced, and one q8 candidate had 1.42% wait. The candidate was removed. See the [screen report](data/ducc-parity-m6-2026-10-27/two-vector-pipeline-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md). The f64 M6 gap remains open. A focused CC T720 reduced-synthesis q1 profile attributes the large phase to recurrence and spectral accumulation, not resampling or inverse FFTs. See the [route profile](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-route-profile-2026-10-27.md); any next source candidate needs a distinct recurrence/dataflow mechanism.
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
| 2 | Checked-scaled `FusedCheckedScaledAnalysisPairChannels` prefix (`0x1d4ba0` family), 599 calls across false/true specializations; false specialization has 32.68M exclusive source-attributed Ir. | Exclusive source attribution is 5.1% of whole-call Ir; inclusive path cost is larger but overlaps children. The route is polar/subnormal safety work, so practical removable fraction is lower; medium-low confidence. | **Screened and removed:** prefix/update fusion was slower in all three q1 and q8 triplets (paired median C/P 1.046406/1.130237); retain checked fallback and range behavior. |
| 3 | Reflected-order wrapper, preparation and safety scans. Four per-block safety passes total 14,316 scans / 813,340 lane inputs; Rust FFT and spin conversion are already below DUCC's corresponding Ir totals. | The safety scans are about 1–2% of the whole call; low expected return. | Defer; avoid broad scan fusion and preserve transactional fallback. |

This is a bounded causal profile, not a parity claim: the earlier measured q1/q8 Rust/DUCC runtime ratios were 1.263×/1.236× in the triage ledger. The indexed-loop trial below does not establish a performance improvement or parity.

### q1 source-line attribution follow-up — 2026-10-27

A fresh Rust-only Callgrind run used the exact current production source from base commit `1b875a6bb95d5c0a4ca3cc23d82185d9c1d3990b`; the relevant production files match the profiled-source commit `26941cf45564a9637b72b0eca87ff880472c3505`. The saved zero-context instrumentation patch is `ef6991cc0f736971642f0c7f34cdf28a13287a813bdf0a0d199cc8ee826d3c92`. GL T720/full-mmax spin-1 analysis ran at q1 with `ntheta=721`, `nphi=1458`, `phi0=0.23`, rustc/Cargo 1.99.1, LLVM 23.1.1, release debug symbols, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, Pulp V3, and Callgrind 3.26.0. Deterministic Python `random.Random(1416973757)` inputs are hashed in the linked CSV. The profiled call produced bitwise-identical output to the initial call and two warm calls. Callgrind collected `639,748,794` Ir, within 0.002% of the prior q1 profile. It emitted the known `brk segment overflow` warning but exited successfully. This is source attribution, not elapsed-time evidence or a DUCC comparison.

The `analyze_spin_order_group` call accounts for `534,888,388` inclusive Ir (83.61% of the whole call). The call site for `try_fused_scalar_analysis_pair_shared_recurrence` has `408,167,451` inclusive Ir across 3,579 calls (63.80%); the hot `FusedScalarAnalysisPair<true,false,false,true,true>` Pulp vectorizer has `106,607,678` directly attributed Ir (16.66%). The `store_analysis_pair` source assignments account for `19,383,051` Ir total (3.03%), and sectoral initialization accounts for `10,210,528` inclusive Ir (1.60%). These rows overlap and must not be summed. The source attribution does not show a larger low-risk setup target after the three no-gain screens. No candidate was selected from this profile; the M6 gap remains open. Next work should target a concrete reduction in the core fused recurrence/accumulation instruction count, with exact-parent q1/q8 measurements.

See the [source-line attribution report](data/ducc-parity-m6-2026-10-27/source-line-attribution-q1-2026-10-27.md) and [machine-readable counts](data/ducc-parity-m6-2026-10-27/source-line-attribution-q1-2026-10-27.csv).

## Phase C indexed-loop screen — candidate removed

The experiment changed only the `ROTATE_SEQUENTIAL_STATES` SIMD-vector loop in `fused_analysis_two_pair_steps_with_simd`: it replaced the nested zipped iterators with an integer index and `get_unchecked(_mut)` after the existing entry checks establish equal vector-slice lengths. The recurrence expressions and update order stayed unchanged; the non-rotating loop was untouched. Candidate patch SHA-256 is `1c5a867b641dec243b3754d03a69d8964123d675e8ce3f1ad43609e7f588cce0`; exact parent source was `1af82adb5190264e89586219c50d61e30b223822`. The patch was reverted after measurement and is preserved with the runner and full results in the [experiment evidence folder](data/ducc-parity-m6-2026-10-27/README.md).

Parent, candidate, and pinned DUCC were built in separate scratch targets. Rust used Cargo/rustc 1.99.1, LLVM 23.1.1, release mode, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the measured worker uses runtime Pulp dispatch, which it does not print. DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2, numerical-source `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and a driver with `-fno-fast-math -ffp-contract=fast`. The case was GL, spin 1 analysis, T720/full-mmax, `ntheta=721`, `nphi=1458`, `phi0=0.23`, seeded finite inputs (seed `1416973757`; map hashes match the Phase B worker). Each process warmed and checked repeat/reset, then recorded ten warm batches targeting 25 ms. Fifteen balanced interleaved parent/candidate/DUCC triplets ran at each of q1 and q8 on the same physical-core affinity used by the checked benchmark harness.

| q | Parent / candidate / DUCC median of process medians (ms) | Paired C/P ratio, median [bootstrap 95% CI] | Faster C/P pairs | Parent/DUCC; candidate/DUCC paired-ratio medians |
|---:|---:|---:|---:|---:|
| 1 | 36.153 / 35.892 / 29.304 | 0.993778 [0.991391, 1.001167] | 11/15 | 1.241058; 1.225888 |
| 8 | 5.293 / 5.307 / 4.224 | 1.001789 [0.999099, 1.011079] | 6/15 | 1.254247; 1.255433 |

Candidate and parent outputs were bitwise identical in all 30 triplets; all three backends were finite and passed repeat/reset checks. Parent and candidate both had maximum DUCC relative-L2 error `4.760590e-14` and maximum absolute error `1.295838e-15`. Timing uncertainty includes substantial q1 runqueue wait (parent/candidate medians about 6.7%/6.8%) and q8 frequency samples reaching 1.746 GHz; no sample was dropped. The q1 interval includes 1.0, q8 is neutral-to-slightly-slower, and the q8 pair spread is wide. The 5.6% zip source attribution from Phase B was only an upper bound; no candidate-specific whole-call Ir reduction was measured. **Decision:** remove the unsafe indexed variant; the screen does not support a material or repeatable f64 gain, while retaining unchecked accesses would add maintenance and proof burden. This is not a parity result.

Candidate validation passed `cargo test --locked --all-features --lib legendre::scalar_paired::analysis` (34 passed, 8 ignored), `cargo test --locked --all-features --lib spin1` (49 passed, 4 ignored), and `cargo fmt --all -- --check`. These focused cases include paired recurrence references, GL/CC/F1 fallback paths, poisoned workspace reuse, q1/q8 recurrence checks, and spin-1 adjoint identities. The full workspace suite and warning-denied Clippy were not run for this rejected variant. Per-triplet process-median timings, summarized wait/frequency data, binary/input/output hashes, exact runner provenance and recoverable source diff are linked from the [evidence folder](data/ducc-parity-m6-2026-10-27/README.md).


## Phase C follow-up — shared-cosine safety-scan candidate removed

The candidate patch combines Q/U input and post-compaction safety scans while reusing cosine validation; checked-scaled recurrence, range caching, and fallback routing are unchanged. It was applied in an isolated worktree based on d3aeb208222d4142217f4c59b83f712b0945d4a6, whose production source matches 469f175a905df34b4514a9f21db391a424adbe15. The patch SHA-256 is c71b5e36e5c953dbfbfcafdc389226ab045d242292cd17dafb76318093757071. The candidate was not applied to production.

Three balanced parent/candidate/DUCC triplets ran at each thread count on GL T720 full-mmax spin-1 analysis. The case used ntheta=721, nphi=1458, phi0=0.23, seed 1416973757, pinned DUCC 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b, rustc 1.99.1/LLVM 23.1.1, and GCC 15.2.0. Each worker warmed and recorded ten 25 ms target batches. Paired median C/P ratios were 0.999166 q1 and 0.998194 q8 (2/3 candidate-faster pairs for each). Median parent/candidate/DUCC process times were 36.411/36.380/29.463 ms q1 and 5.340/5.230/4.235 ms q8. Parent/candidate outputs were bitwise equal in all six triplets; outputs were finite, and repeat/reset checks passed. Both Rust versions had max DUCC relative-L2 error 4.760590e-14 and max absolute error 1.295838e-15.

The screen does not support a repeatable gain. q1 parent/candidate wait was 6.5–6.9%; at q8, one parent sample had 2.46% wait versus 0.55% for its candidate, while the other two pairs were near parity. Candidate/DUCC paired-ratio medians were 1.239101 q1 and 1.234792 q8. The production M6 gap remains open. Candidate focused tests passed before timing (paired analysis 35 passed/8 ignored; spin-1 49 passed/4 ignored; format and diff checks); no expanded suite or warning-denied Clippy was run for this rejected variant.

**Decision:** keep the checked path and do not extend this candidate. Per-pair batch samples, diagnostics, hashes, corrected runner provenance, and the recoverable patch are linked in the [evidence index](data/ducc-parity-m6-2026-10-27/README.md). The [screen report](data/ducc-parity-m6-2026-10-27/pair-safety-screen-2026-10-27.md) is the concise result record.

## Phase C follow-up — reflected active-lane mask screened and removed

The candidate records active reflected-latitude lanes during the staging pass and reuses a 64-bit mask during block compaction, removing the second pass's north/south ring-limit reloads and predicate checks. The exact parent and candidate base are `914e68fbdf4385b3a7e74304e2bcdc77cfd6e4c1`; candidate patch SHA-256 is `a2a399159a825272db5b04cad6d3e41fdd0fe93f7a478cf13edd4dceb47ffa04`. It changes no recurrence arithmetic, checks, or fallback contract and was not applied to production.

Three interleaved parent/candidate/DUCC triplets ran at each q on GL T720 full-mmax spin-1 analysis with the M6 fixture and pinned DUCC 0.42.0. Paired median C/P was `0.992072` q1 (2/3 candidate-faster pairs) and `1.008278` q8 (0/3 candidate-faster pairs). Median process times were 36.438/36.418/29.075 ms q1 and 5.236/5.293/4.201 ms q8. Parent/candidate outputs were bitwise identical in all six triplets; all finite, repeat/reset checks passed, and maximum DUCC relative-L2 error was `4.760590e-14` for both. q1 had 6.6–6.9% runqueue wait; q8 frequency samples included 1.746 GHz readings. The small q1 change did not repeat at q8, so the candidate was removed without extending the screen. Focused validation passed (49 spin tests, 4 ignored; format and diff checks). The [screen report](data/ducc-parity-m6-2026-10-27/active-mask-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md) contain the full results and exact patch. M6 parity remains open; next, inspect a distinct profiled cost.

## Phase C follow-up — paired preweight setup fusion screened and removed

The candidate replaced four stack-array copies, separate Q and U opposite-channel preweight/safety traversals, and the `cos²` traversal with one per-lane setup loop in `try_fused_scalar_analysis_pair_shared_recurrence`. Weighted values and recurrence coefficients remain local; unsafe products still decline before staged output commit. Exact parent/candidate base was `85fd4998827619887c6d1642330ceb614b014935`; measured patch SHA-256 was `a232afd292df1b9acc822fc93f301f42e34afcaa2e4e83b05cc88def12a9097c`.

Three interleaved parent/candidate/DUCC triplets ran at q1/q8 on GL T720 full-mmax spin-1 analysis. Paired median C/P was `1.001462` q1 (1/3 candidate-faster) and `0.998948` q8 (2/3 candidate-faster). Median process times were 36.177/36.068/29.063 ms q1 and 5.248/5.217/4.230 ms q8. Outputs were bitwise identical and finite; repeat/reset passed; maximum DUCC relative-L2 error was `4.760590e-14` for both Rust versions. q1 had 6.0–6.9% wait, q8 had 0.6–1.2% wait and frequency readings at 1.746 GHz. This near-neutral split does not support a gain or material regression, so the candidate was removed without extending the screen. Focused paired-analysis and spin suites passed (34 + 49 tests; 12 ignored total), along with format/diff checks. The [screen report](data/ducc-parity-m6-2026-10-27/preweight-fusion-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md) contain the full results. M6 parity remains open.

## Phase C follow-up — checked-scaled prefix/update fusion screened and removed

The candidate fused the checked-scaled pair-channel vector accumulation with recurrence advancement in `FusedCheckedMixedAnalysisPairChannelsStep`. Scaled exception handling, scalar-tail work, and decline/fallback behavior remained in place. Exact parent was `67eb85151d0e91cca0fcc3bc6b89372f67e64dcd`; the candidate patch SHA-256 was `aa01403775a9113be444d693c659d825f673875f53e7a87b275c36734b95bed7`.

Three interleaved parent/candidate/DUCC triplets ran per q on GL T720 full-mmax f64 spin-1 analysis. Median process times (parent/candidate/DUCC) were 36.164/37.916/29.139 ms q1 and 5.391/5.963/4.288 ms q8. Median paired candidate/parent ratios were `1.046406` q1 (3/3 candidate-slower) and `1.130237` q8 (3/3 candidate-slower). q1 scheduler wait was balanced between parent and candidate; one q8 candidate process had 3.50% runqueue wait. Treat the three-pair q8 direction cautiously. The candidate remained 1.294× and 1.372× DUCC by median paired ratio, so this screen does not close the M6 gap.

Candidate and parent outputs were bitwise identical in all six triplets, finite, and repeat/reset consistent. Candidate versus DUCC relative-L2 error was `4.760590e-14` and maximum absolute error `1.295838e-15`. Focused candidate validation passed (paired-analysis 34 passed/8 ignored; spin-1 49 passed/4 ignored; format check). The candidate was not applied to production and was not extended. Exact timings, host diagnostics, hashes, and the recoverable patch are in the [screen report](data/ducc-parity-m6-2026-10-27/checked-prefix-fusion-screen-2026-10-27.md) and [evidence folder](data/ducc-parity-m6-2026-10-27/README.md). The subsequent two-vector pipeline screen is recorded below; neither candidate was applied to production.

## Phase C follow-up — two-vector recurrence pipeline screened and removed

The candidate interleaved two independent SIMD-latitude vectors through the same two-degree recurrence. It preserved each output accumulator's vector order and the exact per-lane arithmetic; only the hot paired-state path was specialized. Exact parent was `30a17ee82154d761ccbbb40a256e27a97d3c2946`; patch SHA-256 was `f786de45bb2e09854146ad1259324e89619b4278b0fdb423351802f8e69f18a6`.

Three interleaved parent/candidate/DUCC triplets ran per q on GL T720 full-mmax f64 spin-1 analysis. Median process times (parent/candidate/DUCC) were 36.299/39.869/29.307 ms q1 and 5.266/5.679/4.216 ms q8. Median paired candidate/parent ratios were `1.105232` q1 (3/3 candidate-slower) and `1.077169` q8 (3/3 candidate-slower). q1 parent/candidate scheduler wait was similar at 6.4–6.9%; q8 wait was low in five workers, with one candidate at 1.42%. The candidate remained 1.356× and 1.342× DUCC by median paired ratio; this did not improve the measured cell.

Parent/candidate outputs were bitwise identical in all six triplets, finite, and repeat/reset consistent. Candidate versus DUCC relative-L2 error was `4.760590e-14` and maximum absolute error `1.295838e-15`. Focused paired-analysis tests (34 passed/8 ignored), spin-1 tests (49 passed/4 ignored), and format check passed. The candidate was not applied to production and was not extended. Exact timings, summary diagnostics, hashes, and recoverable source patch are in the [screen report](data/ducc-parity-m6-2026-10-27/two-vector-pipeline-screen-2026-10-27.md) and [evidence folder](data/ducc-parity-m6-2026-10-27/README.md).

## CC T720 reduced synthesis q1 route profile — 2026-10-27

The current q1 M6 cell is CC `lmax=720`, `mmax=180`, `ntheta=722`, `nphi=378`, spin 1: Rust 30.006 ms versus pinned DUCC 18.593 ms (1.616×; three-pair range 1.591–1.658×). The Rust runs had about 10% scheduler wait, so this remains a gap rather than a quiet-host performance estimate.

The existing ignored `profile_spin1_standard_synthesis_phases` helper was run at exact source `f2f813fd51c58b52a5991e666c92d83b3286bf7f` with release `x86-64-v3`, one Rayon thread, `phi0=0.23`, three warmups and three profiled calls. It generated deterministic coefficients internally; this is route evidence, not a matched benchmark or an independent numerical check. The targeted helper passed. The test binary SHA-256 is `6bde446901d25c045c09f3e050e68b565144436e0487d7b2bb2f05b358143ba7`; `src/transform/spin/synthesis.rs` SHA-256 is `8ef50c59d134f8c816793d8567198ff4120919503f9c243a8984ce6d13fef4e1`.

| Repetition | Total ms | Latitude/setup ms | Order recurrence and spectral accumulation ms | Two inverse FFTs and map writeback ms |
|---:|---:|---:|---:|---:|
| 0 | 28.191 | 0.068 | 27.048 | 1.055 |
| 1 | 29.344 | 0.066 | 28.218 | 1.029 |
| 2 | 28.039 | 0.074 | 26.967 | 0.968 |

The internal worker counters report 1,784 pair-eligible blocks and 1,736 ordinary fused-kernel successes per call. There are 305 fallback entries: 191 transition successes and 114 per-degree blocks; those 114 also report successful polar splits. Worker-sum timers put the fused recurrence/accumulation at 11.60–12.07 ms and scaled/per-degree fallback at 10.50–11.19 ms. They are nested diagnostics and should not be added to the phase timings. Order-factor fill, paired coefficient preparation, and finite scans are each smaller in these counters. No scalar coefficient cache or order-major scratch is allocated on this serial route.

The order recurrence and accumulation phase dominates this profile; inverse longitude FFTs and writeback are about 1 ms together. This does not support CC resampling or FFT tuning as the next cause. It points to a new synthesis recurrence/dataflow hypothesis. Do not retry the reduced reflected scalar component route unchanged: its earlier reduced-order T720 version failed an existing test, and its full-order CC q8 screen had no repeatable gain. The compact per-call counters, command, toolchain and limits are in the [route-profile report](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-route-profile-2026-10-27.md) and [CSV](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-route-profile.csv).
