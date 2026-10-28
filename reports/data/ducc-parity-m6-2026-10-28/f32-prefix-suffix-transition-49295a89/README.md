# Native-f32 T720 scaled-prefix to fused-suffix screen

**Decision: do not promote.** The bounded candidate successfully moved about 33–38% of the active degree–latitude terms into the existing native-f32 fused kernel. It reduced whole-transform time 4.3–5.7% in three cells, but the reduced-mmax q8 cell regressed 1.4% in 12/15 paired runs. The suffix still rejects 39–63 blocks per cell at later recurrence-range checks. This is useful negative evidence, not a material algorithmic step toward parity.

## Exact revisions and setup

- Parent source: RustSHT `49295a89f2d13d8b02c2cbfa8566a8c5c21c6a80` (`dev`), identical to `ef11ce0a` in all `src/` paths. No production code was changed on `dev`.
- Candidate: isolated patch from the exact parent. Patch SHA-256 is recorded in `candidate.patch.sha256`; the complete patch, including feature-gated counters and scratch tests, is stored as [`candidate.patch.gz`](candidate.patch.gz). Decompress it with `gzip -dc candidate.patch.gz > candidate.patch`; the uncompressed SHA-256 is in `provenance.json`, and the compressed file hash is in [`candidate.patch.gz.sha256`](candidate.patch.gz.sha256).
- Pinned DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X. Rust 1.99.1 / LLVM 23.1.1 and DUCC GCC 15.2.0; both worker binaries used x86-64-v3 AVX2/FMA. q1 used CPU 3; q8 used CPUs 8–15.
- Fixtures: GL T720, `ntheta=721`, `phi0=0.23`; full `mmax=720, nphi=1458`; reduced `mmax=180, nphi=378`. The deterministic fixtures match the current-source family baseline. Per-pair timing observations, fixture hashes and the candidate patch are retained alongside the results; machine-specific binary paths and raw disassembly captures remain local.

The primary timing screen used 15 interleaved fresh-process parent/candidate/DUCC pairs per cell, warm reusable workspaces, matched affinity and ISA. All 180 workers passed finite, repeat-consistency and workspace-reset checks. Full per-pair values and error metrics are in [`paired-results.json`](paired-results.json).

## Candidate route and transition evidence

The candidate reused `PairedScalarRecurrencePrecision` for its scaled prefix and `FusedSynthesisBlock<f32>` for the suffix. It started the suffix only after `normalize_pair_state` reported that every lane's two recurrence states were in native-f32 normal range with exponent zero. This does **not** apply a nonzero residual scale correction like DUCC. The fused block retained per-pair contribution checks and recurrence-range checks. It staged sums on the stack; on decline, the original recurrence and sums remained intact and the existing scalar callback resumed at the same degree. Polar/mixed blocks and partial SIMD tails stayed on their existing routes.

| Cell | Active degree–latitude terms | Initial fused blocks | Fallback blocks | Suffix attempts / successes / declines | Useful suffix terms | Suffix share | Fused pair-lane updates / recurrence state updates | Discarded replay pair-lane / recurrence updates |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 161.20M | 1,744 | 3,657 | 3,193 / 3,154 / 39 | 61.91M | 38.41% | 31.01M / 30.81M | 193,680 / 192,112 |
| Full q8 | 161.20M | 4,337 | 7,112 | 6,262 / 6,199 / 63 | 61.47M | 38.13% | 30.78M / 30.58M | 95,616 / 94,752 |
| Reduced q1 | 81.21M | 922 | 972 | 821 / 785 / 36 | 27.07M | 33.33% | 13.55M / 13.50M | 193,656 / 192,088 |
| Reduced q8 | 81.21M | 2,036 | 1,849 | 1,663 / 1,608 / 55 | 27.10M | 33.37% | 13.56M / 13.51M | 92,456 / 91,712 |

All declines came from recurrence-range checks after the prefix; there were no coefficient, contribution, or scalar-tail declines. The suffix-fraction histograms show where transition points occurred across attempted blocks. Failed speculative work is small—0.12–0.48% of the parent pair-lane contribution counts—so retry work is not the dominant remaining expense. Candidate route counters and the eight suffix-length bins are in [`route-counts.json`](route-counts.json); these counters were compiled only into an isolated profiling build and are absent from timed binaries.

The current-source work census counts 161.20M / 81.21M total full/reduced degree–latitude terms and 80.70M / 40.64M paired recurrence slots in Rust. Its equivalent DUCC count is 80.73M / 40.66M computed northern-hemisphere terms, with reflected outputs reconstructed. Rust therefore performs about twice the useful recurrence work. DUCC's useful and padded update definitions are preserved in the referenced work-count data in `route-counts.json`.

For q8, all eight workers handled 23 ring-group jobs. The recorded job counts were `[1,7,2,1,2,4,4,2]` full and `[1,5,5,3,1,3,1,4]` reduced. Per-worker contribution totals ranged from 3.12M to 56.47M full and 3.09M to 18.21M reduced. The per-worker nanoseconds include first-call counters and are diagnostic only, not production scheduling timings. The scheduler was unchanged. The candidate added no workspace allocation; its trial sums occupied 512 bytes of stack storage per attempted block.

## DUCC algorithm and machine code

Pinned DUCC `init_lambda` initializes scaled recurrence state; `iter_to_ieee` advances the scaled prefix; `calc_alm2map` continues scaled accumulation through the normal-range transition, applies the remaining correction once, and hands the state to `alm2map_kernel`. DUCC's f32 interface uses f64 for Legendre work. Its reflected GL pairing computes one hemisphere and reconstructs the other. The hot AVX2 kernel unrolls four recurrence pairs (eight degrees), carries recurrence and parity accumulators in registers, and has no calls in its loop.

The candidate V3 specialization emits packed YMM `vfmadd*ps` recurrence and accumulation operations. Its loop also performs contribution-normality and recurrence-range checks at every pair. DUCC's V3-equivalent kernel uses packed YMM `vfmadd*pd` and enters the unrolled suffix after a one-time transition. This structural difference, plus Rust's roughly 2× recurrence work, remains after removing scalar contribution callbacks from the safe suffix. Inspection of the emitted candidate and DUCC AVX2 kernels is summarized in [`assembly-summary.md`](assembly-summary.md). Full disassembly captures remain local because they are binary-specific.

A matched parent/DUCC Callgrind screen at `ef11ce0a` is code-identical to this exact parent in `src/`: 6,610,413,955 Rust Ir versus 260,801,202 DUCC Ir on full q1. Normalized to the 161,199,393 required output degree–latitude slots, that is 41.01 versus 1.62 Ir per required slot. Ir are instruction references, not cycles. This screen did not collect candidate Ir. Hardware `perf` counters were unavailable on the host (`perf_event_paranoid=4`). The current candidate's route work, stage timings and emitted assembly are recorded instead of inferring candidate cycles from static instructions.

## Whole-transform results

Medians are from the 15 paired runs; candidate/parent and Rust/DUCC columns are medians of paired ratios.

| GL T720 f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent | Parent/DUCC | Candidate/DUCC | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Full q1 | 677.754 | 649.485 | 19.907 | 0.9570 | 33.97× | 32.64× | GAP |
| Full q8 | 100.800 | 96.577 | 2.795 | 0.9576 | 35.79× | 34.58× | GAP |
| Reduced q1 | 201.600 | 189.859 | 8.714 | 0.9426 | 23.09× | 21.72× | GAP |
| Reduced q8 | 53.024 | 53.688 | 1.273 | 1.0142 | 41.60× | 42.09× | GAP |

Reduced q8 was slower in 12/15 pairs; a 200,000-resample bootstrap of paired candidate/parent ratios gives a 95% interval of `[1.0043, 1.0187]`. The three improvements are real in this screen but too small to justify the extra candidate path, especially with a repeatable target-cell regression. The f32 Rust/DUCC gap remains 21.72–42.09×.

An eight-sample phase probe independently timed the Legendre stage and inverse FFT. Legendre medians (parent → candidate) were 700.606 → 646.016 ms full q1, 208.553 → 191.357 ms reduced q1, 103.746 → 94.202 ms full q8, and 55.410 → 53.813 ms reduced q8. FFT medians were about 1.9–2.3 ms full / 0.4–0.5 ms reduced q1 and 0.30 ms full / 0.10–0.15 ms reduced q8. These samples used a local Rayon pool and are diagnostic rather than the whole-transform acceptance gate; raw samples are in [`stage-results.json`](stage-results.json).

## Numerical and build checks

Candidate/parent max relative-L2 error was `1.57e-7`–`1.78e-7`; max absolute difference was `1.79e-7`–`2.38e-7`. Candidate/DUCC relative-L2 error remained `1.72e-4` full and `2.73e-4` reduced, with max absolute errors `9.64e-4` and `1.55e-3`.

The isolated candidate passed six paired-synthesis unit tests, including transactional decline for subnormal products, overflow, nonfinite values and exact zero; 12 f32 integration tests passed (one ignored), including T720 analytic modes, scalar adjoint, polar behavior and workspace reset; four extreme FFT/range tests passed. Formatting and warning-denied Clippy passed. The latest public Rust CI run for exact parent `49295a89` also succeeded: [run 40672993320](https://github.com/mwyau/RustSHT/actions/runs/40672993320). Adjacent f32 analysis/spin and protected f64 T1023 controls were not run because the candidate was not promoted.

## Decision and next measured hypothesis

Do not carry this candidate forward unchanged. It only transitions after the exponents are already zero, still checks every fused pair, and covers roughly one third of reduced work; failed speculative work is negligible. The larger measured opportunity is reflected-ring reuse: current Rust updates about twice as many recurrence states as DUCC. Continue with a synthesis-only reflected route reusing the shared recurrence and native f32 arithmetic, keeping scalar analysis and f64 dispatch unchanged. The previously rejected broad reflected f32 candidate regressed reduced scalar-analysis q8 by 4.3%; do not restore it. M6 remains open.
