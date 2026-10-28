# Native-f32 GL T720 scaled-prefix to fused-suffix screen

**Decision: reject the isolated candidate.** It completed many safe SIMD suffixes, but all four matched whole-transform cells regressed. No production source changed.

## Source and run setup

- Exact Rust parent: 746c7e60c6d967f09ecbc7d87271a8938284d0e3 on dev.
- Candidate: isolated working-tree patch based on that parent; SHA-256 is in provenance.json. The patch includes the feature-gated census and candidate implementation; it was not applied to dev.
- DUCC: pinned 0.42.0 source 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b.
- Toolchain: rustc 1.99.1 / LLVM 23.1.1 and GCC 15.2.0; x86-64-v3, AVX2/FMA. Rust selected Pulp V3. q1 was pinned to one fixed CPU; q8 used the same fixed eight-CPU set for parent, candidate, and DUCC.
- Fixtures: GL T720, 721 rings, phi0=0.23; full mmax=720/nphi=1458 and reduced mmax=180/nphi=378. Input hashes and binary hashes are in provenance.json.
- Whole-transform data are three interleaved, warm parent/candidate/pinned-DUCC process pairs per cell. The stage screen used three paired feature-instrumented runs per cell and is diagnostic only.

## Whole-transform measurements

Medians are milliseconds. The Rust/DUCC ratio is the exact-parent baseline or the unpromoted candidate relative to pinned DUCC. Candidate/parent above 1 means slower.

| GL T720 f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent | Parent/DUCC | Candidate/DUCC | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Full q1 | 645.744 | 701.308 | 19.774 | 1.082 | 32.60x | 35.49x | GAP; reject |
| Full q8 | 96.136 | 108.665 | 2.662 | 1.155 | 36.12x | 40.83x | GAP; reject |
| Reduced q1 | 191.771 | 222.484 | 8.771 | 1.160 | 21.84x | 25.36x | GAP; reject |
| Reduced q8 | 50.782 | 64.536 | 1.205 | 1.270 | 42.15x | 53.55x | GAP; reject |

The candidate lost in all 12 initial pairs. No 15-pair extension was justified. Paired samples and output checks are retained in [paired-summary.json](paired-summary.json).

The feature-instrumented Legendre-stage medians also regressed in every cell: full q1 659.453 to 706.662 ms (1.071x), full q8 96.906 to 118.760 ms (1.253x), reduced q1 197.083 to 225.140 ms (1.142x), and reduced q8 51.267 to 65.227 ms (1.269x). q8 stage values are the maximum per-worker accumulated stage time, not a sum used as wall time. See [stage-summary.json](stage-summary.json).

## Transition and repeated-work census

The candidate ran the existing shared paired recurrence and f32 accumulator through a scaled prefix. At a block boundary it restored both recurrence states using their exponent fields and handed the remaining degrees to the shared fused kernel only if every restored state and staged output sum was normal or zero. The fused suffix kept conservative coefficient, product, sum, and recurrence guards. A decline discarded staged output and replayed the original block through the exact fallback.

The table reports completed useful suffix contributions separately from suffix lengths planned before guarded execution. Parent fallback contributions are from the current-source census. A declined or no-handoff block uses extra prefix/check work before its original fallback; partial tails bypass the transition.

| Cell | Parent fallback contributions | Complete blocks | Declined blocks | No handoff | Partial tail | Prefix terms | Completed useful suffix terms | Planned suffix terms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 46.406M | 3,114 | 445 | 98 | 98 | 8.134M | 33.598M | 37.715M |
| Full q8 | 45.164M | 6,041 | 940 | 131 | 98 | 7.328M | 33.048M | 37.279M |
| Reduced q1 | 19.769M | 720 | 159 | 93 | 98 | 3.282M | 13.695M | 15.931M |
| Reduced q8 | 18.740M | 1,453 | 303 | 93 | 98 | 2.395M | 13.736M | 15.789M |

Transition degree ranges were 5/399/720 (min/median/max) full q1, 5/392/720 full q8, 5/130/715 reduced q1, and 5/132/715 reduced q8. Recurrence-state guards did not cause suffix declines. Every decline came from coefficient/product or accumulated-output safety checks. Thus the corrected recurrence state generally stayed safe once handed off; repeated coefficient/sum checking and fallback replay were the blockers.

Exact block totals and completed-versus-planned suffix counts are in [route-summary.csv](route-summary.csv). Planned terms on declined blocks are not counted as completed useful suffix work.

## DUCC path and emitted code

Pinned DUCC uses double recurrence state and one scale value per lane. In sht_inner_loop.h, init_lambda seeds the recurrence and normalizes it with that scale. iter_to_ieee advances two paired recurrence steps at a time while the lane group remains in the scaled prefix, rescaling the states and updating scale as needed. calc_alm2map keeps a residual scaled prefix only while its per-lane correction is needed; once every lane can use normal IEEE values, it applies the correction to both recurrence states once, clears the residual scale, and enters alm2map_kernel.

On AVX2, alm2map_kernel processes four degree values, or two recurrence pairs, per loop over four double-precision latitude lanes. It keeps multiple output accumulators live and combines recurrence and coefficient accumulation in one loop. The pinned binary's emitted kernel uses YMM double FMA instructions and has no calls in that kernel body. The captured executable code is in [ducc-alm2map-kernel.disasm.txt](ducc-alm2map-kernel.disasm.txt).

Rust already shares the paired recurrence and native-f32 fused accumulation for blocks that enter normal range immediately. Other blocks use F32SynthesisAccumulator and the checked scaled packet leaf. This candidate added a block-wide transition, but scanned all staged sums at each possible degree, checked the suffix coefficient/product/output range per degree, and replayed the full block after a decline. The selected Rust V3 normal fused leaf emits YMM f32 FMA; its static disassembly is retained in [rust-f32-fused-v3-normal-leaf.disasm.txt](rust-f32-fused-v3-normal-leaf.disasm.txt). The suffix specialization was inlined and is not represented by that separately named normal-leaf symbol.

Static instruction counts and code size are not dynamic work or elapsed time. A matched candidate-specific dynamic instruction profile was not obtained: perf hardware events are denied on this host (perf_event_paranoid=4), and a one-transform Callgrind attempt on the parent exceeded 77 seconds before producing a complete profile, so it was terminated. No equivalent-work-normalized candidate Ir claim is made. The whole-transform and instrumented stage timings determine the rejection.

The route scan plus per-degree checks explain the result: although 13.7M–33.6M safe suffix terms completed per cell, every cell became slower, and 252–1,071 blocks per cell either declined or found no handoff and repeated their original fallback. Do not repeat this guarded block-wide route unchanged.

## Numerical checks and next step

The isolated candidate passed release f32 integration tests (13 passed, 1 ignored), scalar recurrence tests (94 passed, 8 ignored), and paired synthesis tests (6 passed). The suites include independent T720 analytic modes, scalar adjoints, polar and extreme-value paths, subnormal/overflow behavior, partial vectors, repeated calls, and workspace reset. All timed workers reported finite, repeat-consistent, reset-consistent output. Parent/candidate relative L2 was 1.57e-7–1.78e-7 with max absolute difference 1.79e-7–2.38e-7. Candidate/DUCC relative L2 remained 1.7234e-4 full and 2.7255e-4 reduced, matching the parent. No tolerance was weakened.

Adjacent timing extensions and a candidate f64 T1023 spin-1 control were not run after the conclusive four-cell regression; no candidate source was promoted. The production parent 746c7e60 Rust CI workflow completed successfully: [Rust CI run 40726268996](https://github.com/mwyau/RustSHT/actions/runs/40726268996).

Keep all four current synthesis cells marked GAP. The next measured f32 work is spin-1 synthesis and analysis on current source, as recorded in the single [M6 performance ledger](../../../DUCC_PARITY_M6.md). M6 remains open.
