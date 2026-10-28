# M6 — current-head DUCC parity triage

**Generic native-f32/f64 fused scalar-synthesis milestone: complete.** The shared typed recurrence and fused accumulation path, exponent-zero fast paths, and native-f32 SIMD leaves are implemented and validated. M6 performance parity remains open. Latest source is `ec5d56ca29ec6ad4e34fe80443d934677b0515f8`; the scaled-contribution packet leaf is based on exact parent `3816adce3226a92669a5807241b03f80cb6d8cfd`, with pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. GL T720 f32 scalar-synthesis candidate/DUCC is `36.37×`/`40.07×` full q1/q8 and `25.11×`/`45.32×` reduced q1/q8. Candidate/parent is `0.9783` full q1, `0.9754` full q8, `0.9685` reduced q1 and `0.9901` reduced q8; the reduced-q8 interval crosses parity. Three cells improved repeatably; reduced q8 is **INCONCLUSIVE**. All four DUCC cells remain **GAP**. This file is the single current M6 performance ledger; dated experiment details and source-specific historical rows below are evidence, not additional current ledgers.

**Previously accepted scheduling change — 32-ring f32 GL scalar synthesis:** against exact parent `9dc45160`, the q8 candidate/parent paired medians are `0.7808` full and `0.6420` reduced (15/15 faster pairs; bootstrap 95% intervals `[0.7331,0.8321]` and `[0.6216,0.6607]`). q1 medians are `0.9826`/`0.9840` from three pairs; q1 does not execute parallel tasks. All eight workers were already active at 64 rings; the split changes job count 12→23 and reduces max/min worker group-time skew 3.24→1.70 full and 6.56→3.49 reduced. Workspace payload grows 738 KB; q8 workspace construction adds about 3.4 ms full and 26 μs reduced, outside warm transform timing. Adjacent f32 and protected f64 controls show no meaningful regression. The [complete screen](data/ducc-parity-m6-2026-10-27/f32-32-ring-schedule-2026-10-27/README.md) contains exact revisions, paired timings, allocation, numerical checks, route counters, ring costs and worker diagnostics.

**Latest measured arithmetic change — scaled-contribution packets:** candidate `ec5d56ca` adds native-f32 SIMD arithmetic for fully checked packets while preserving the existing scalar helper for unsafe packets and tails. Against exact parent `3816adce`, 15 interleaved pairs showed candidate/parent medians `0.9783`, `0.9754`, `0.9685`, and `0.9901` for full/reduced q1/q8; the last interval crosses parity. All 60 candidate/parent primary outputs were bitwise equal. Adjacent f32 scalar analysis was neutral and protected f64 GL T1023 spin-1 synthesis had no meaningful regression. The [packet-leaf report](data/ducc-parity-m6-2026-10-27/f32-scaled-packet-leaf-2026-10-27/README.md) records source, binary and fixture hashes, pair timings, errors, controls and AVX2 code.

**Measured next bottleneck — scaled-prefix handoff:** an isolated q8 census at production source `ec5d56ca` accounts for exactly 45.164M full-mmax and 18.740M reduced-mmax fallback degree–latitude contributions, matching the earlier independent route counts. When every ring in an active 32-ring block reaches the existing strict f32 `SMALL..=BIG` state range after folding its exponent, a contiguous suffix through the final degree covers 58.04% of full and 66.35% of reduced fallback contributions; failed one-shot attempts would speculatively visit 3.54%/1.88% of fallback work before the first later decline. The finite-normal hardware range raises the successful suffix bound to 64.74%/72.34%, but is not needed for the first candidate. This corrected pass counts recurrence-state safety only; q1 route counts and accumulated-output safety are pending before candidate selection. It replaces an initial packet-counter pass discarded after finding a temporary counter-resizing bug. Final instrumented counts and compressed raw records are in the [transition-census report](data/ducc-parity-m6-2026-10-28/f32-scaled-transition-census/README.md). These are route-coverage bounds, not speedups. The bounded hypothesis to screen is a whole-block transition: fold the recurrence exponent once, attempt the shared fused f32/f64 kernel from that pair, stage sums transactionally, and leave the existing scaled path untouched if any later state declines. Do not use per-vector retries or mixed-lane masks. DUCC's scaled prefix and vectorized suffix remain the algorithmic reference; all four Rust/DUCC timing cells remain **GAP**.

**Earlier rejected screens:** the global task-grain candidate hurt protected f64 T1023 spin-1 synthesis; the transactional normal-suffix handoff hurt protected f64 T720 spin-1 analysis; the mixed-lane and per-vector retries did not deliver a stable whole-transform gain. Their source-specific evidence remains linked in the historical sections below.



**Most recent broad current-head baseline:** RustSHT `98adf1e6b5334021bfe6ae9abb618c6be167a822` on `dev`; pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Host: AMD Ryzen 9 5950X; rustc/Cargo 1.99.1, LLVM 23.1.1; GCC 15.2.0. The detailed [22-cell report](data/ducc-parity-m6-2026-10-27/current-head-rebaseline-98adf1e6/README.md), [machine-readable results](data/ducc-parity-m6-2026-10-27/current-head-rebaseline-98adf1e6/current-head-rebaseline.json), and [per-cell ledger](data/ducc-parity-m6-2026-10-27.csv) record that source's timings, pair ratios, hashes, errors, and worker diagnostics.

**Latest accepted source slice:** the shared f32/f64 recurrence at `a30537bdcfa0fc5273985abfd592998325a608de` plus the f32-only exponent-zero caller fast path screened below. The shared recurrence's exact parent is `972bacdaef57388509738d1c2cdebbb7973b89be`. Its original focused GL T720 scalar-synthesis screen measured q1 candidate/DUCC at 47.66×/34.36× and q8 at 140.90×/117.24× across full/reduced mmax; the [fused-synthesis report](data/ducc-parity-m6-2026-10-27/f32-fused-synthesis-2026-10-27/README.md) preserves that evidence. This is not a current full-matrix baseline.

## Native-f32 exponent-zero caller fast path — 2026-10-27

The paired scalar-synthesis caller now handles exponent-zero lanes with the same two native-f32 multiplies used inside `multiply_complex_scaled_f32`, avoiding the helper call for the common case. Against exact parent `a30537bd`, three alternating GL T720 full-mmax scalar-synthesis triplets per q gave paired median candidate/parent ratios `0.974301` q1 and `0.986955` q8; all six candidate runs were faster and Rust outputs were bitwise identical. Candidate/DUCC was `45.854×` q1 and `137.242×` q8, so the remaining f32 gap is substantial. Scalar tests passed (168 passed, 16 ignored), f32 integration passed (11 passed, 1 ignored), and all worker finite/repeat/reset checks passed. See the [screen report and preserved patch](data/ducc-parity-m6-2026-10-27/f32-expzero-caller-fastpath-2026-10-27/README.md).

## GL T720 f32 scalar-synthesis q8 Callgrind attribution — 2026-10-27

One warmed transform was profiled at current source `a30537bd` against exact parent `972bacda` and pinned DUCC `42ce8ab5`. The current whole-call profile collected **8,465,515,501 Ir**, versus **9,609,850,325** for the parent and **252,805,383** for DUCC. This is an 11.91% instruction-count reduction from the parent, not a cycle or elapsed-time measurement. The separate whole-transform timing screen recorded candidate/parent `0.9846` and candidate/DUCC `140.90×` for this q8 cell; parity remains far off. Output stayed finite; candidate/parent relative L2 was `3.9400e-7`, and candidate/DUCC was `1.72343e-4`. The [evidence folder](data/ducc-parity-m6-2026-10-27/f32-callgrind-q8-2026-10-27/README.md) preserves compressed valid profiles, temporary instrumentation patch, disassembly, source/fixture/output hashes, and Valgrind warning details.

`multiply_complex_scaled_f32` accounts for 39.15% inclusive Ir, while the paired fallback traversal is about 40.78% inclusive, including nested calls; do not add these nested counts. The all-normal fused SIMD block executes but admits only fully normal blocks. A separate trace at parent `972bacda` saw 1,744/5,499 blocks eligible. The mixed-lane mask follow-up was screened and rejected; the result is recorded below.

## Mixed-lane f32 scalar-synthesis fusion screen — rejected

Three alternating fresh-process parent/candidate/DUCC triplets per q ran for GL T720 full-mmax scalar synthesis. The q1 paired median candidate/parent ratio was `0.993786` (range `0.980393–1.003015`), a mixed near-parity result. At q8 it was `1.019974` (`1.016261–1.025878`), with all three candidate pairs 1.63–2.59% slower; candidate/DUCC was `46.878×` q1 and `142.984×` q8. The candidate matched parent within relative L2 `1.240014e-7`, and all workers passed finite, repeat, and reset checks. The focused mixed-lane test, scalar suite (169 passed, 16 ignored), f32 integration suite (11 passed, 1 ignored), formatting, and diff checks passed. The extra branch and fused pass were not retained. See the [screen evidence](data/ducc-parity-m6-2026-10-27/mixed-lane-fused-synthesis-screen-2026-10-27/README.md).

## Native-f32 vector-group fusion screen — rejected, 2026-10-27

At exact current parent `3189fe8e`, this candidate retried the existing fused synthesis kernel one native f32 vector at a time and kept exceptional groups on the scaled callback. Three alternating parent/candidate/DUCC triplets per GL T720 spin-0 synthesis cell covered full/reduced mmax at q1/q8. Median candidate/parent ratios were `0.99044` full q1, `0.99864` full q8, `0.98358` reduced q1, and `1.00002` reduced q8. The narrow q1 gains (0.1–1.6%) did not carry to q8; the path added a substantial f32-specific branch, so it was removed. No 15-pair screen was warranted. All 36 workers passed finite, repeat, and reset checks; candidate/parent relative L2 was at most `1.18e-7`, and candidate/DUCC error matched the parent to the shown precision. Focused mixed-scale/scalar/f32 tests, format, diff checks, and warning-denied library Clippy passed. See the [screen report and exact rejected patch](data/ducc-parity-m6-2026-10-27/f32-vector-fusion-screen-2026-10-27/README.md), [triplet data](data/ducc-parity-m6-2026-10-27/f32-vector-fusion-screen-2026-10-27/triplets.json), and [provenance](data/ducc-parity-m6-2026-10-27/f32-vector-fusion-screen-2026-10-27/provenance.json).

## Native-f32 analysis current-source rebaseline — 2026-10-27

Eight GL T720 scalar and spin-1 analysis cells were remeasured against exact parent `8c4dba98`, using production source `3189fe8e` and pinned DUCC 0.42.0. Three parent/current/DUCC triplets per cell showed bitwise-equal current/parent output in all 24 comparisons. The scalar-analysis route was 9.6–15.1% faster; spin-1 analysis was near parity, with median changes from 0.4–1.5% slower. All 72 workers passed finite, repeat, and reset checks. Current/DUCC remains a 27.38×–38.50× gap for scalar analysis and 28.81×–36.57× for spin-1 analysis. See the [cell report and results](data/ducc-parity-m6-2026-10-27/f32-analysis-rebaseline-2026-10-27/README.md) and [provenance](data/ducc-parity-m6-2026-10-27/f32-analysis-rebaseline-2026-10-27/provenance.json).

This fresh baseline covers six f64 spin-1 controls and all 16 native-f32 GL T720 scalar/spin-1 analysis/synthesis full/reduced q1/q8 cells. Three interleaved fresh-process pairs ran per cell; the ranges below are descriptive triage ranges, not confidence intervals. All 22 cells passed finite, repeat, and reset checks. The existing f32 integration suite passed: 11 passed, 1 ignored, 0 failed. No production code changed in this baseline.

## Selected source-dated f64 cells

These representative f64 rows belong to their recorded earlier source screens;
they are not a full remeasurement of source `9dc45160`. The scoped f32
candidate's f64 scalar T720 and protected spin-1 T1023 synthesis controls are
the current affected-path measurements, linked in the experiment report above.

| Workload | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Status | Max relative L2 |
|---|---:|---:|---:|---:|---|---:|
| GL T720 full spin-1 analysis q1 | 36.569 | 29.234 | 1.249× | 1.234–1.271× | **GAP** | 4.76e-14 |
| GL T720 full spin-1 analysis q8 | 5.372 | 4.260 | 1.298× | 1.220–1.307× | **GAP** | 4.76e-14 |
| GL T1023 full spin-1 synthesis q1 | 87.990 | 121.330 | 0.734× | 0.721–0.745× | **PASS** | 2.60e-13 |
| GL T1023 full spin-1 synthesis q8 | 14.080 | 19.775 | 0.715× | 0.696–0.716× | **PASS** | 2.60e-13 |
| CC T720 reduced m180 spin-1 synthesis q1 | 30.010 | 18.644 | 1.608× | 1.600–1.704× | **GAP** | 3.62e-13 |
| GL T720 reduced m180 over-resolved synthesis q8 | 4.255 | 4.677 | 0.911× | 0.902–0.912× | **PASS** | 2.27e-13 |

The T720 analysis target remains open: all three q1 and q8 pair ratios were above parity. Rust q1 scheduler wait was 6.4–6.7% (DUCC below 0.1%); q8 Rust wait was 0.4–1.1%. T1023 synthesis retains its speed advantage. CC reduced synthesis is a separate gap with elevated q1 Rust wait around 9.8–10.3%.

## Latest native-f32 result

The current-source GL T720 f32 scalar-synthesis cells are `ec5d56ca` in the machine-readable [per-cell ledger](data/ducc-parity-m6-2026-10-27.csv): candidate/DUCC is `36.37×`/`40.07×` at full `mmax=720` q1/q8 and `25.11×`/`45.32×` at reduced `mmax=180` q1/q8. Candidate/parent is `0.9783`/`0.9754` full and `0.9685`/`0.9901` reduced q1/q8; reduced q8 remains inconclusive. The [packet-leaf report](data/ducc-parity-m6-2026-10-27/f32-scaled-packet-leaf-2026-10-27/README.md) records matched timings, correctness, route bounds, assembly and protected controls; the [32-ring report](data/ducc-parity-m6-2026-10-27/f32-32-ring-schedule-2026-10-27/README.md) records worker scheduling, ring costs, task overhead and workspace allocation. All four candidate/DUCC cells remain **GAP**.

The scoped candidate also screens adjacent full-mmax f32 scalar analysis and spin-1 analysis/synthesis at q1/q8, plus f64 T720 scalar and protected T1023 spin-1 synthesis. All parent/candidate outputs were bitwise equal; those controls are in the same report. The broader 16-cell f32 matrix still contains source-dated gaps from the `98adf1e6` baseline; no full/reduced spin-1 synthesis parity is claimed. T720 error measurements are recorded, but no new tolerance was set.

The existing Phase B GL T720 analysis profile remains applicable: the diff from source `26941cf4` to `98adf1e6` contains no `src/`, `Cargo.toml`, or `Cargo.lock` changes. Its q1/q8 Callgrind totals were 1.4899×/1.4990× DUCC Ir; the reflected fused recurrence and accumulation dominate. This is instruction attribution, not cycles or a timing claim. See the [profile](#phase-b-profile--current-source-feb46e5f).

## Current analysis-path candidate screen — 2026-10-27

The cached-proof post-recurrence scan candidate (ledger B44) was not promoted. Candidate and parent outputs matched byte-for-byte; the focused spin-1 test suite passed (49 passed, 4 ignored). GL T720 full spin-1 analysis paired C/P was `0.98331 [0.97886,0.99329]` q1 and `1.01836 [0.97894,1.09460]` q8 on the alternate core set. On the first set the medians were `0.99202` q1 and `0.94441` q8, with wide uncertainty and elevated q8 scheduler wait. The direction changed across core sets, so no stable q1/q8 gain is established. The current 1.249×/1.298× DUCC analysis gaps remain open; production source is unchanged. See the [screen report](data/ducc-parity-m6-2026-10-27/analysis-range-proof-scan-2026-10-27/README.md), [machine-readable results](data/ducc-parity-m6-2026-10-27/analysis-range-proof-scan-2026-10-27/screen-results.json), and [candidate patch](data/ducc-parity-m6-2026-10-27/analysis-range-proof-scan-2026-10-27/candidate.patch.gz).

## Historical first screen at source `26941cf4`

The section below preserves the initial M6 baseline and the subsequent experiment history. Its performance rows are historical and do not replace the source-specific per-cell entries in the current M6 ledger above.

**Source:** `26941cf45564a9637b72b0eca87ff880472c3505` on `dev`<br>
**DUCC:** 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`<br>
**Host:** AMD Ryzen 9 5950X, Linux x86-64; rustc/Cargo 1.99.1, LLVM 23.1.1; GCC 15.2.0.

This first M6 screen covers 14 selected warm whole-transform cells: GL T720 spin-1 analysis, protected T1023 synthesis controls, two reduced synthesis routes, and native-f32 GL T720 scalar/spin-1 analysis and synthesis. The rest of the agreed matrix remains unmeasured. No production algorithm changed during this screen.

### Method

Both Rust workers were built in release mode with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the f32 worker reported Pulp V3 and eight f32 lanes. DUCC numerical sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; both C++ drivers used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`. The pinned DUCC tree was clean at the exact source SHA. Binary and fixture SHA-256 values are recorded in the [per-cell CSV](data/ducc-parity-m6-2026-10-27.csv).

Each cell used three interleaved fresh-process pairs with shared deterministic fixtures, matching geometry, normalization, precision, and output dimensions. Timing covers warm calls using reusable Rust plans/workspaces and direct DUCC APIs; setup, fixture I/O, and output checks are outside the timed batches. The f64 worker targeted 25 ms and the f32 worker 10 ms per batch; each used ten batches for calls below 100 ms and five otherwise. q1 and q8 are thread counts. Workers pinned q1 to CPU 0 and q8 to eight distinct physical cores. f64 q8 used CPUs 0–7; f32 q8 used 0,2,4,6,8,10,12,14.

Three pairs give a triage range, not a confidence interval. The ledger retains each pair's Rust and DUCC medians, ratio, correctness data, fixture hashes, binary hashes, scheduler wait, and frequency samples. f64 comparisons passed finite, repeat, and reset checks. f32 comparisons passed finite, repeat, reset, and output-length checks; the f32 harness records DUCC-relative errors without enforcing a tolerance. Its broader independent-reference, adjoint, and exceptional-value gate remains open.

### Historical f64 cells

| Workload | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Status | Max relative L2 |
|---|---:|---:|---:|---:|---|---:|
| GL T720 full spin-1 analysis q1 | 36.724 | 29.173 | 1.263× | 1.244–1.264× | **GAP** | 4.76e-14 |
| GL T720 full spin-1 analysis q8 | 5.285 | 4.244 | 1.236× | 1.232–1.249× | **GAP** | 4.76e-14 |
| GL T1023 full spin-1 synthesis q1 | 88.272 | 119.637 | 0.737× | 0.736–0.740× | **PASS** | 2.60e-13 |
| GL T1023 full spin-1 synthesis q8 | 14.072 | 19.737 | 0.719× | 0.704–0.720× | **PASS** | 2.60e-13 |
| CC T720 reduced m180 spin-1 synthesis q1 | 30.006 | 18.593 | 1.616× | 1.591–1.658× | **GAP** | 3.62e-13 |
| GL T720 reduced m180 over-resolved synthesis q8, ntheta=1441 | 4.255 | 4.647 | 0.916× | 0.913–0.920× | **PASS** | 2.27e-13 |

The T720 analysis gap is present at both thread counts. Rust q1 scheduler wait was 6.5–6.9% across its three analysis runs, while DUCC wait was low; all paired ratios exceeded 1.23. The CC reduced q1 Rust runs also had about 10% wait, and all three ratios remained above 1.59. The q8 analysis runs had low wait. The T1023 synthesis controls preserve the established speed advantage.

### Historical native-f32 cells

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

## Phase C follow-up — forced native-spin analysis route screened and removed

The candidate disabled the GL scalar fast-range selection so GL T720 full-mmax spin-1 analysis used the existing direct native-spin recurrence. The exact parent and candidate base were `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; candidate patch SHA-256 is `8e0d496aac44c14b368d239586d75486c0728a7ed2e00f4adc47ad1385b5d072`. The candidate changed no production files and kept every numerical, workspace, reset, and extreme-input assertion; the route-specific cache expectation was adjusted only in the isolated test worktree.

Three interleaved parent/candidate/DUCC triplets ran per q. Median parent/candidate/DUCC times were **35.946/927.170/29.122 ms q1** and **5.285/121.786/4.227 ms q8**. The direct route was about 25.8×/23.0× slower than the current route. Candidate/DUCC relative-L2 error was `5.9393e-14`, maximum absolute error `1.7168e-15`; finite and repeat/reset checks passed. Focused spin-1 validation passed (49 passed, 4 ignored), plus formatting and diff checks. The benchmark harness's inherited final bitwise-parent condition caused its exit code 2 because this route is not bitwise-identical; all 18 backend workers completed and their numeric checks are recorded. The direct route is rejected for this cell. Exact timings, output hashes, and the recoverable patch are in the [screen report](data/ducc-parity-m6-2026-10-27/native-spin-route-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md). Cleanup removed only its exact owned scratch and two worktrees; available space increased by 941,285,376 bytes and free inodes by 10,408.

## Phase C follow-up — shared phase-scale and FMA rotation screened and removed

The candidate combines the per-ring scale with the shared phase factor for Q and U, then applies explicit FMA rotation in `analyze_spin_scalar_reflected_order`. Exact parent was `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; patch SHA-256 is `062113589c586100665fd4b60776fe1de4609ece9f29e29638a4743a9aaba139`. It affects only spectrum phase/scale arithmetic; recurrence, input safety checks, transactional fallback, and coefficient staging are unchanged. Candidate validation passed 49 spin-1 tests (4 ignored), format check, and diff check.

Fifteen balanced parent/candidate/DUCC triplets per q ran on GL T720 full-mmax spin-1 analysis. Median parent/candidate/DUCC process times were **36.325/36.109/29.309 ms q1** and **5.342/5.330/4.259 ms q8**. Paired candidate/parent medians were **0.995288 [0.984092, 1.007003] q1** and **0.998563 [0.976639, 1.024761] q8**; the intervals include parity and only 11/15 q1 and 8/15 q8 pairs favored the candidate. Candidate-parent relative-L2 error was `1.07e-15`, max absolute error `4.17e-17`; candidate/DUCC relative-L2 remained `4.76e-14`, max absolute `1.30e-15`. q1 wait was elevated but balanced; q8 variation was wide. **Decision:** reject as inconclusive and do not promote the extra f64 arithmetic path. Rust remains about 1.23× DUCC on this cell. Full per-triplet diagnostics and patch are in the [screen report](data/ducc-parity-m6-2026-10-27/phase-scale-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).

The next measured gap remains CC T720 reduced-mmax spin-1 synthesis q1. Its route profile shows recurrence and accumulation dominate, not the inverse FFTs/writeback; next source work should target a distinct synthesis dataflow mechanism and preserve the measured analysis controls. M6 remains active.


## Phase C follow-up — fuse inverse-sine scaling into paired synthesis SIMD; rejected

The isolated candidate wrote per-lane inverse-sine values into the prepared kernel's no-longer-used `previous` scratch, scaled all four Q/U recurrence sums in Pulp SIMD, and removed the fused branch's later per-ring spectrum-scale loop. The recurrence and FMA order were unchanged. Exact parent was `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; candidate patch SHA-256 is `e0725761add848c53cf4032d0b68f3865aa6141703b9758cda5e338d78e4c6fe`. It was never applied to production.

Three balanced parent/candidate/pinned-DUCC triplets per q ran on exact CC T720 reduced-mmax spin-1 synthesis (`mmax=180`, `ntheta=722`, `nphi=378`, `phi0=0.23`). Median parent/candidate/DUCC process times were 30.623/31.429/20.664 ms q1 and 5.704/5.479/3.016 ms q8. Paired C/P medians were `1.026317` q1 (all three slower, range 1.024277–1.034464) and `0.977610` q8 (2/3 faster, range 0.877852–1.002711). q1 parent/candidate median wait was 9.75%/9.84%; q8 was noisy, with a candidate-first outlier. The screen does not establish a new DUCC ratio.

Parent and candidate outputs were bitwise equal in all six triplets; all 18 workers were finite and passed repeat/reset. Candidate-parent relative-L2 and max absolute error were zero; candidate-DUCC max relative-L2 was `3.619e-13` and max absolute error `3.025e-9`. Focused candidate validation passed the spin-1 suite (49 passed, 4 ignored), format and diff checks. **Decision:** reject the SIMD post-scale; the q1 slowdown is repeated and q8 does not establish a compensating gain. Keep the current source. Full measurements, diagnostics, hashes, and cleanup are in the [screen report](data/ducc-parity-m6-2026-10-27/simd-scale-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).

M6 remains active. Next: test whether the prepared pair kernel can write directly to its cleared per-order Q/U slots, removing the current scratch-to-spectrum read/add step without changing recurrence arithmetic. Preserve the CC q1 fallback and GL f64 synthesis controls.


## Phase C follow-up — direct writes to cleared paired-synthesis spectra; rejected

The isolated candidate replaced the prepared pair kernel's Q/U `Complex64 +=` with direct stores of newly computed sums. Its caller clears spectrum storage before each transform and owns each order/ring slot once. The store keeps `0.0 + sum` to preserve signed-zero behavior. Exact parent was `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; patch SHA-256 is `de5863798bf82495d2a235c77ea4d9e7e75682019e26c57b8e5383d2d888a037`. It was not applied to production.

Fifteen balanced parent/candidate/DUCC triplets per q ran on CC T720 reduced-mmax spin-1 synthesis (`mmax=180`, `ntheta=722`, `nphi=378`, `phi0=0.23`). Paired C/P medians were `0.997470` q1 (95% bootstrap interval `[0.992019,1.002035]`, 9/15 faster) and `1.008259` q8 (`[0.982173,1.010737]`, 5/15 faster). Median parent/candidate/DUCC process times were 27.823/27.796/18.559 ms q1 and 4.426/4.436/2.657 ms q8. Parent and candidate were bitwise equal in all 30 triplets; all 90 workers passed finite, repeat, and reset checks. Candidate-parent error was zero; candidate/DUCC maximum relative-L2 was `3.619e-13`, maximum absolute error `3.025e-9`. The small q1 lead is within the paired interval and does not repeat at q8. **Decision:** reject; no whole-transform gain is established. The exact data, runner, patch, and cleanup record are in the [screen report](data/ducc-parity-m6-2026-10-27/direct-store-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).

M6 remains open. The prepared-kernel indexed-read candidate has been screened and rejected after a 0.12% q1 whole-call Ir reduction and q8 slowdowns in both controls. Next inspect the checked transition/fallback path using the [CC source attribution](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-source-attribution-2026-10-27.md), while retaining the CC q1/q8 and GL T1023 f64 synthesis controls.


## CC T720 reduced synthesis q1 source attribution — 2026-10-27

The exact parent `f70d9845edf6bba617b9d034b1f5e1d628c429e0` was profiled with Callgrind around one warmed CC T720 reduced-mmax spin-1 synthesis call. The source file `src/transform/spin/synthesis.rs` matches the SHA in the existing route profile. The run collected 613,272,869 Ir; the prepared fused pair path was 47.26% inclusive, with 10.06% directly attributed to the nested iterator zip implementation in its Pulp V3 vectorizer. Checked transition, polar-safe-lane, and per-degree fallback routes also have material inclusive counts. Inclusive counts overlap and are not elapsed-time estimates. No candidate or production change was made. The next hypothesis is indexed pair reads in `FusedPreparedSynthesisPairBlock::with_simd`, screened at CC q1/q8 and the GL T1023 f64 synthesis control. Full method, source hashes, raw profile and cleanup are in the [focused report](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-source-attribution-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).


## Prepared paired-synthesis indexed traversal — rejected

The candidate changed only `FusedPreparedSynthesisPairBlock::with_simd`, replacing nested zip/enumerate traversal with ordered index reads for both SIMD vectors and the scalar tail. Parent `f70d9845` and candidate outputs were bitwise equal. Focused tests passed (49, 4 ignored); all 12 CC T720 reduced / GL T1023 full-mmax q1/q8 triplets passed finite, repeat, and reset checks. Candidate/DUCC maximum relative-L2 was `3.619e-13` on CC and `2.602e-13` on GL. Matched CC q1 Callgrind fell from 613,507,756 to 612,782,575 Ir, only 0.1182%; the candidate moved much of the zip source count into bounds/comparison code. q8 paired median C/P was `1.014293` on CC (0/3 faster) and `1.011036` on GL (1/3 faster). GL q1 had a small unconfirmed lead; all timing screens had n=3 and no confidence intervals. **Decision:** reject; no supported whole-transform gain, and the q8 results favor the parent. Full triplets, profiles, patch, hashes, and scratch cleanup are in the [screen report](data/ducc-parity-m6-2026-10-27/indexed-pair-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).


## Prepared synthesis safety-check cache — screened and rejected

The isolated candidate memoized the prepared-pair coefficient range check across adjacent latitude blocks in one prepared run. It computes the result lazily only after the ordinary fused route declines, leaving successful fused blocks unchanged; the low-level transition API retains its direct check. Exact parent was `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; candidate patch SHA-256 was `36553db57b80244ac39325f8049097fa5d9c405b8a9449412c8c3b6380f3c341`. It was never applied to production.

Three interleaved parent/candidate/DUCC triplets per q ran for CC T720 reduced-mmax synthesis and GL T1023 full-mmax synthesis. Parent/candidate outputs were bitwise identical across all 12 triplets; all 36 workers passed finite, repeat, and reset checks. Focused transition tests and the release all-features spin-1 suite passed (49 passed, 4 ignored). Candidate/DUCC maximum relative-L2 and maximum absolute error were `3.619e-13` / `3.025e-9` for CC and `2.602e-13` / `5.410e-9` for GL.

| Case | q | Median C/P | Median parent / candidate / DUCC time (ms) | Parent/candidate wait |
|---|---:|---:|---:|---:|
| CC T720 reduced synthesis | 1 | 1.001844 | 27.833 / 27.884 / 18.621 | 10.35% / 10.30% |
| CC T720 reduced synthesis | 8 | 1.000910 | 4.406 / 4.414 / 2.648 | 1.25% / 0.47% |
| GL T1023 full synthesis | 1 | 0.991833 | 88.352 / 87.818 / 119.909 | 2.09% / 2.07% |
| GL T1023 full synthesis | 8 | 1.000487 | 14.164 / 14.291 / 19.579 | 0.72% / 0.72% |

**Decision:** reject the extra cache plumbing. CC results were effectively neutral, and the small GL q1 lead did not repeat at q8; the q1 CC workers also had about 10% scheduler wait. These three-triplet screens have no confidence intervals. Full provenance, source hashes, triplet details, and cleanup are in the [screen report](data/ducc-parity-m6-2026-10-27/prepared-safety-cache-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).

M6 remains open. The shared generic fused scalar-synthesis milestone and the 32-ring scheduling experiment are complete. Current source `ec5d56ca` adds a measured native-f32 scaled-contribution packet leaf; the f32 GL T720 scalar-synthesis cells remain 36.37×/40.07× DUCC full q1/q8 and 25.11×/45.32× reduced q1/q8. An isolated predicate count estimates that 5.195M/1.653M nonzero-exponent contributions per full/reduced q8 transform remain on the scalar path; the timed binary has no counters. Next measure normal-range transition degrees and suffix coverage per active packet before considering a bounded synthesis-only recurrence handoff. Then extend profiling to f32 scalar analysis and spin-1 synthesis/analysis, followed by the remaining f64 GL analysis and CC reduced-synthesis gaps. Keep per-cell statuses and exact source revisions in this ledger and the [CSV](data/ducc-parity-m6-2026-10-27.csv); neither the fused milestone, scheduling gain nor packet leaf establishes DUCC parity.
