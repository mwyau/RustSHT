# B1 lazy scaled-state setup

**Result:** retain a small f32-only B1 improvement. The candidate initializes
the scaled accumulator state only when a block first enters scaled accumulation.
This removes repeated zero-initialization for ordinary blocks. The change is
numerically transparent in the measured T720 cases and compiles away for f64,
but leaves the main f32 and f64 performance gaps open.

## Source and execution

- Parent: `0433c21587dbc20c0a1366ad38892c174769cb16`.
- Candidate production diff SHA-256:
  `068828b7e527edaeb6485b6482aca576c1bc9a359cd81fb8b2dad67223c3a5a7`.
- DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; rustc 1.99.1; `-C target-cpu=x86-64-v3`;
  Pulp V3 AVX2/FMA. q1 used one pinned physical core and q8 used eight;
  exact affinity is retained in the structured paired results.

**Execution crosswalk.** Rust's B1 route is `ShtPlan::analysis_into` and
`analyze_map_to_coefficients` in `src/transform/scalar.rs:1103,1271`, then
`analyze_order_jobs_dynamic` at line 1944 for q8 (q1 enters the order group
directly), and `analyze_symmetric_order_group` at line 2119. The Fourier
spectra and reflected ring pairs are prepared by
`prepare_symmetric_analysis_pair_block` in `src/transform/mod.rs:338`; the
shared precision kernel is
`fused_scalar_analysis_pair_block_preweighted_typed` in
`src/legendre/scalar_paired/analysis.rs:1165`, with AVX2 execution in
`FusedScalarAnalysisPairTyped::with_simd` at line 1401. The kernel advances
paired degrees using two FMAs, accumulates four reflected real/imaginary
channels, and reduces each channel over the latitude vectors. AVX2 uses eight
f32 lanes and four f64 lanes.

Pinned DUCC's scalar analysis enters `leg2alm_internal` in `sht.cc:777`, then
its normal-ring branch at lines 915–950 allocates `complex<double>` harmonic
scratch even for the `T=float` instantiation. `Ylmgen::prepare` is in
`sht_inner_loop.h:186`; `inner_loop_m2a` at line 1752 uses `fill_m2a` at 1660,
which widens the f32 Fourier phase into `complex<double>`. `calc_map2alm` at
line 636 calls `iter_to_ieee` (line 426), handles residual scaled values at
lines 653–673, then enters `map2alm_kernel` at line 592. That AVX2 kernel
advances two coefficient outputs in one ring-vector loop (`l += 4`, lines
596–616). DUCC defines `Tv=native_simd<double>` at line 252: its public f32
analysis therefore uses four double lanes for recurrence and accumulation,
while the input Fourier phase is f32. The Rust route stays native f32.

The whole-call screen uses deterministic nonzero GL T720 f32 fixtures with
matched geometry and input bits for Rust, pinned DUCC f32, and the widened
f64 diagnostic fixture. Test-only phase probes use the exact same f32 map and
its bit-exact f64 widening, with `phi0=0.23` and `nphi=1458`.
The pinned DUCC `map2alm_kernel` AVX2 excerpt is preserved in the earlier
[profile package](../m6-b1-profile-410c-2026-10-31/hot-loops.asm.txt); it shows
four-lane f64 FMA and the two-output recurrence loop.

## Profile and machine code

For full T720 q1, exact-parent Rust f32 Callgrind counted 794.30M instructions;
the order-group accounts for 94.88% inclusively and the V3 typed kernel for
83.23% self. Candidate counts are 780.75M total and 94.79% in the order-group.
The exact-source Rust f64 profile counted 320.34M instructions, with 83.80% in
the order-group. Its Callgrind profile records 163.46M data references, 2.02M
D1 misses, 45.30M conditional branches and 0.864M conditional mispredicts.
These are simulated counts, not cycles. `perf stat` rejects all hardware events
under `perf_event_paranoid=4`, and `perf record` is unavailable for the same
reason.

Matched-current-source phase probes (five warm samples per cell) show these
shares of whole-call time:

| Precision | q | Wall ms | Forward FFT ms | Legendre recurrence + accumulation ms | FFT share | Legendre share |
|---|---:|---:|---:|---:|---:|---:|
| f32 | 1 | 42.174 | 2.573 | 39.592 | 6.1% | 93.9% |
| f32 | 8 | 5.944 | 0.358 | 5.559 | 6.0% | 93.5% |
| f64 | 1 | 18.591 | 3.257 | 15.325 | 17.5% | 82.4% |
| f64 | 8 | 2.634 | 0.428 | 2.187 | 16.3% | 83.0% |

The q1/q8 calls use one exact f32 input fixture and its exact f64 widening.
Phase totals leave at most 0.02 ms unmeasured. Unfiltered command output stays
local under the repository's report-artifact policy; the structured phase
summaries and reproducibility patch are committed here.
The full fused-kernel interval is not split into recurrence advancement,
ordinary accumulation, scaled accumulation, normalization, and exceptional
fallback time yet. The test-only phase probes are reproducible from
[`phase-probe-tests.patch`](phase-probe-tests.patch), whose SHA-256 is recorded
in `profile-summary.json`.

The q8 f64 scheduler probe reuses the same widened fixture for 30 warm calls.
It dispatches 721 order jobs, with 88–91 jobs per worker. Median queue-lock
time is 33.0 microseconds against a 2.500 ms uninstrumented call (1.3%). The
per-worker summed job time is about 2.086 ms. Queue-lock time is measured
directly; the remaining scheduler/runtime interval is not a separately timed
phase. Counts and timing summaries are in `profile-summary.json`.

Current f32 route counters show 1,067,695 ordinary and 266,685 scaled pair
accumulation invocations for full T720 (20.0% scaled), and 581,344 ordinary
and 89,429 scaled for reduced T720 (13.3%). The probe uses zero Fourier
spectra to isolate recurrence routing; these are invocation counts, not a
time share or lane-update count. All 6,437 full and 2,119 reduced f32 typed
blocks succeed, with no seed or recurrence declines.

The AVX2 f32 ordinary loop uses four YMM FMA accumulators for same/opposite
real and imaginary channels, followed by `vextractf128`/`vaddps` reductions.
Scaled work loads lane factors and exponent state; exceptional handling
remains in the same large 19,820-byte V3 body. The ordinary FMA loop has no
per-iteration stack-vector spills in the inspected assembly. The f64 V3 body
is 7,236 bytes, uses four f64 lanes and the two-pair unroll with
`vfmadd...pd`; its ordinary vector loop likewise has no visible repeated
YMM spill traffic. DUCC's kernel body is 722 bytes and also uses four double
lanes. Kernel sizes do not prove instruction-cache cost.

The f32 current-source Callgrind profile counts 794.30M instructions for the
parent and 780.75M for the candidate (−1.71%). The order group is 94.88%
inclusive; the typed V3 vector body is 83.23% by its Callgrind self count.
Other visible rows include forward FFT 40.63M Ir (5.12%), recurrence-lane
initialization 33.75M Ir (4.25%), paired spectrum preparation 21.20M Ir
(2.67%), and `memset` 34.68M parent versus 31.22M candidate. These function
rows overlap where shown in the call tree and must not be added. The f64
profile counts 320.34M Ir, with 83.80% in the order group. Callgrind read/write
and simulated-cache counts are retained in `profile-summary.json`; none are
hardware cycles. `perf stat` and `perf record` are unavailable because
`perf_event_paranoid=4` and the process lacks perf capabilities.

The f32/f64 and DUCC Callgrind totals and hashes are in
[`profile-summary.json`](profile-summary.json). Raw command captures remain
local and untracked; the profile summary and representative instruction
excerpt in [`assembly-excerpts.txt`](assembly-excerpts.txt) are the committed
evidence.

The direct setup microbenchmark measures 66.10 ns for eager 32-lane
`F32ScaledAnalysisState` initialization versus 0.21 ns for a lazy `None` block.
The full-q1 paired whole-transform median saving is 0.416 ms. This is a
verified but small avoidable cost, not the dominant remaining bottleneck. The
largest established category is Legendre recurrence plus accumulation. The
candidate's time reduction and unchanged output are measured; the exact cycle
cost of recurrence, normal/scaled accumulation, normalization, and exceptional
handling remains unproven. The f64 gap also lies mostly in its Legendre phase,
but this profile has not isolated a larger f64 avoidable operation.

## Fifteen-pair GL T720 screen

Median milliseconds; ratios use paired runs and 95% t intervals on log ratios.
Candidate/parent is an optimization check. `INCONCLUSIVE` means the interval
crosses 1.0.

| Cell | Parent f32 | Candidate f32 | DUCC f32 | Candidate/parent | Candidate/DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 42.9485 | 42.5329 | 14.1514 | 0.997 [0.981, 1.013] INCONCLUSIVE | 3.030 GAP |
| Full q8 | 5.7980 | 5.7726 | 2.0527 | 0.995 [0.987, 1.004] INCONCLUSIVE | 2.811 GAP |
| Reduced q1 | 15.5145 | 15.1324 | 5.8164 | 0.979 [0.961, 0.998] PASS | 2.612 GAP |
| Reduced q8 | 2.1593 | 2.1316 | 0.8948 | 0.987 [0.975, 0.999] PASS | 2.368 GAP |

Candidate outputs are bitwise equal to the exact parent across all 60 pairs;
repeat/reset checks pass. The f64 matching-DUCC baselines are unchanged at
1.310, 1.273, 1.278 and 1.226 for full q1, full q8, reduced q1 and reduced q8.
The candidate f64 `.text` is identical to parent. There is no f32/Rust-f64
acceptance target.

## Correctness and protected control

`cargo fmt --all -- --check`, strict all-target/all-feature Clippy,
`cargo test --release --features parallel` (329 unit, 4 extreme-range, 14 f32,
and 1 workspace-compatibility tests), `uv run pytest -q python/tests` (20
tests), and `uv run --project python/pyspharm-rs --group dev pytest
python/pyspharm-rs/tests` (7 passed, 1 skipped) pass. All 47 scalar-paired
analysis unit tests pass, including mixed
scale, subnormal, tail, recurrence-decline and range-boundary checks. The T720
analytic, adjoint and workspace-reset integration test passes.

The protected GL T1023 f64 spin-1 synthesis control used three interleaved
parent/candidate pairs per q. Outputs were bitwise equal. Candidate/parent
medians were 1.005 q1 (90.34/90.92 ms) and 1.014 q8 (14.33/14.42 ms); this is a
short control, not a performance claim. Per-pair data and fixture hashes are
in [`t1023-control.json`](t1023-control.json). The source edit is in B1 f32
analysis only.

## Next

Keep this small setup saving. The current profile localizes the dominant stage
but does not split its normal/scaled accumulation, recurrence, normalization,
and exceptional work into non-overlapping current-source regions. Add bounded
Callgrind gates or typed-block microbenchmarks for those regions on the exact
f32/f64 fixtures, then target the largest measured region. Do not infer cycles
from Callgrind or repeat rejected tile, threshold, factor-cache, lane-scan or
recurrence-fusion variants.
