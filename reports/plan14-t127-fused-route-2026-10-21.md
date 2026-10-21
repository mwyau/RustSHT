# Plan 14 T127 reduced-order fused analysis route

**Implementation:** [`50c9866b`](https://github.com/astro-informatics/RustSHT/commit/50c9866b196d7979eb2f1f301fc9b3a5d07b6adf)
**Source baseline:** `23755ba2d684d337191b1d05a1d50f083d671079`.
**Pinned DUCC:** 0.42.0,
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Result

The earlier T127 analysis profile reported a nearly 2x recurrence-stage gap,
but it did not time the fused production implementation. At that source,
`FUSED_SCALAR_ANALYSIS_MIN_LMAX` was 255; the T127 reduced-`mmax` plan used
`for_each_y_lm_paired_status_dispatched` and invoked a scalar coefficient
accumulation callback at each degree. The compiled AVX2/FMA fused symbol
inspected in the earlier report was not the executed T127 route.

The f64 scalar analysis cutoff is now 127. Below T127, behavior is unchanged;
f32 remains at 255. This only changes eligibility for the existing checked
fused route. Scaled recurrence, coefficient/input preflights, safe cache
proof, late range validation, and callback fallback remain intact. A
production-path test verifies 0 fused blocks at T126 and 32 at T127 for
`mmax=31`; f32 retains its 255 cutoff.

### Calibrated active-ring kernel

Geometry was T127 GL, `mmax=31`, `ntheta=128`, odd-minimum `nphi=63`,
`phi0=0.23`, order 31. One active block contains 64 reflected pairs, 16
four-lane YMM vectors, 49 paired coefficient slots and 97 degree outputs.
The same deterministic FFT spectra, weights, phase and ring data were used
for each route. Both reset coefficient outputs per operation. Rust callback
timing includes its paired recurrence, callback accumulation and coefficient
finalization. The fused route uses preweighted opposite spectra and the
checked fused kernel. DUCC timing calls pinned `init_lambda` and
`calc_map2alm` (including `iter_to_ieee`, scaled-prefix advancement,
`map2alm_kernel` and DUCC's coefficient adjustment).

| Rust route | Rust p10 / median / p90 (µs) | DUCC p10 / median / p90 (µs) | Rust/DUCC | Rust reset-inclusive median (µs) | DUCC reset-inclusive median (µs) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Existing callback | 3.675 / 3.700 / 3.859 | 0.699 / 0.705 / 0.723 | 5.24x | 3.711 | 0.716 |
| Checked fused candidate | 1.489 / 1.499 / 1.541 | 0.698 / 0.707 / 0.716 | 2.12x | 1.510 | 0.717 |

Each distribution contains 90 calibrated batches: 30 in each of three
independent process runs. Batch durations were 22–39 ms for the callback
comparison and 25–52 ms for the fused comparison. The trials were
interleaved `Rust, DUCC, DUCC, Rust, Rust, DUCC`, pinned to physical CPU 0;
CPU 1 monitored host activity. CPU PSI stayed at zero. There were isolated
batch outliers (maximum callback Rust 5.18 µs, fused Rust 2.17 µs, DUCC about
1.10 µs); the table uses all samples and robust percentiles. The f64 AVX2/FMA
V3 dispatch was verified at runtime, with four lanes. DUCC used GCC 15.2,
kernel flags `-O3 -DNDEBUG -ffast-math -ffp-contract=fast
-march=x86-64-v3`, and a strict validation driver. Rust 1.99.1 / LLVM 23.1.1
used release mode, strict FP and `-C target-cpu=x86-64-v3`.

The fused path reduced Rust kernel median by 59.5%. Its advantage is not
attributed to a changed mathematical recurrence: the main structural change
is that coefficient accumulation is fused into the vector traversal instead
of calling the accumulation operation once for every degree. The fused
candidate remains 2.12x slower than DUCC at this isolated boundary, so this
is not kernel parity.

Coefficient maximum scaled difference versus DUCC was `2.73e-16`.
Relative-L2 output errors were `2.87e-15` for the callback and `2.74e-15`
for the fused route. Full-transform validation and independent tests are
listed below.

### Full T127 transform

The production-dispatched full analysis used the same map/coefficient
fixture, plan/workspace reuse, geometry, normalization, phase and native
boundaries. It ran 30 calibrated 20–100 ms batches in each of three
interleaved process pairs on one physical core. Rust and DUCC were reset and
validated outside timed regions. Cold first-call and setup remain separate.

| Source | Rust warmed median (p10 / p90, ms) | DUCC warmed median (p10 / p90, ms) | Rust/DUCC | Excess (ms) | relative L2 / max abs |
| --- | ---: | ---: | ---: | ---: | ---: |
| Prior callback route (`dc63adba`) | 0.188940 | 0.103935 | 1.818x | +0.085005 | `9.15e-15` / `2.11e-15` |
| Fused route (`50c9866b`) | 0.100249 (0.099310 / 0.102510) | 0.103999 (0.103156 / 0.108691) | 0.964x | -0.003749 | `8.77e-15` / `1.55e-15` |

The Rust reduction from the prior callback route is 46.9%. Paired candidate
Rust/DUCC ratios had p10/p90 `0.934/0.993`. Candidate first-call times were
0.140 ms Rust and 0.293 ms DUCC; plan/workspace setup was 0.149 and 0.069
ms respectively. These are first-call/setup diagnostics, not the warmed
comparison. Candidate batches were stable for Rust. Host runqueue-wait
fraction had median 10.4% and p90 29.3%, so the slight 3.7 µs whole-call
advantage over DUCC should be read as approximate parity, not a robust claim
of a DUCC speedup. It is nevertheless a large, repeatable reduction of the
previous 85 µs excess. The direct-kernel gap remains measurable.

## Validation and reproduction

The cutoff test confirms f64 T126/T127 dispatch and f32's unchanged 255
threshold. The test-only kernel comparison asserts that the candidate route
was exercised, resets output for every operation, and checks output
stability. Existing safety tests cover zero, subnormal, extreme finite,
scaled recurrence, polar lanes, SIMD tails, cache-scoped proof, adjoints and
fallbacks. `cargo test --workspace --all-features --release` passed
(233 library tests, 4 extreme-scaling tests, 11 f32 tests, 1 workspace test;
29 ignored). `cargo test -p rustsht --no-default-features --release` passed
(195 library tests, 4 extreme-scaling tests, 11 f32 tests, 1 workspace test;
16 ignored). Formatting, Python byte-compilation for the benchmark driver,
and strict Clippy with the repository's custom-cfg allowance passed.

Reproduce the direct boundary with
`scripts/benchmark_plan14_active_ring.py` (three interleaved process pairs,
30 calibrated batches per process and route). The script captures its
fixture through the ignored test
`transform::tests::capture_plan14_t127_active_ring_kernel_input`, runs the
Rust ignored test `legendre::scalar_paired::kernel_bench::benchmark_plan14_t127_active_ring_route`
for `callback` and `fused`, and builds the native DUCC wrapper from the
modified `reports/data/scalar-analysis-pair-grouping-2026-10-19/ducc-kernel-fast.cpp`.
For the end-to-end comparison, rerun
`scripts/benchmark_ducc_full_transform_parity.py` with
`--lmax 127 --mmax 31 --fixed-nphi 63 --fixed-ntheta 128
--operation analysis --threads 1` and the pinned DUCC revision. Raw batch,
Raw host and process logs remain local and untracked; only the summarized
numerical evidence is committed.

The initially drafted DUCC microbenchmark wrapper omitted the insignificant
prefix advancement and final coefficient adjustment. Its timings were
discarded. The reported measurements use the corrected `calc_map2alm`
wrapper and include those operations.

## Next work

This closes the reduced-order T127 q1 analysis lead; it does not close all
small-q1 behavior. Continue the Plan 14 small serial fixed-cost attribution
at the remaining T1–T64 cases. Separately retain the already measured T720
q8 synthesis odd-length inverse FFT as a diagnostic lead; the available
standalone real inverse was slower and does not justify a production FFT
replacement. Re-rank whole-call excess before choosing any additional
change.
