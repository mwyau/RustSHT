# Paired odd-length longitude FFT experiment

**Decision:** retain the guarded paired-row route on the isolated experiment branch. It improved the scalar f64 odd-longitude synthesis transform in all nine odd cases at q1 and q8; analysis improved in most cases, with smaller and less consistent q8 gains. The benchmarked source is committed locally on exp/paired-odd-longitude-fft only. It is not integrated into the active Plan 15 branch.

## Scope and source identity

The experiment answers whether two adjacent real longitude rows can share one length-N complex transform. It changes only scalar f64 odd-N analysis and synthesis. Even N, f32, spin-1 and other spin paths retain their existing route. A final unpaired latitude row uses the existing independent transform.

- RustSHT baseline source: ef238290f28d5192ca54de8e442c77640549828a.
- Initial implementation, benchmark harness and evidence commit: 4b74e3a50a6a683e1ead44650aa6e9b4dad81246.
- Follow-up numerical-test hardening commit: 8342f6631b9c5791f8e6ba4243e032dcb9d8eabd. It changes tests only; the production source used for the A/B is unchanged.
- Candidate source diff SHA256 used for the final whole-transform A/B: b5238c0acf3ef06a5edbe35141786f87c64c81aa6ee3a0220e1489b32c3e7b04.
- DUCC source: 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b, DUCC 0.42.0.
- Host: AMD Ryzen 9 5950X, performance governor; q1 CPU 8, q8 CPUs 8–15.
- Rust: 1.99.1 / LLVM 23.1.1, release with -C target-cpu=x86-64-v3. RustFFT 6.4.1 used runtime AVX2/FMA on this host.
- DUCC: GCC 15.2.0; transform kernel used -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver used strict floating point.
- Whole-transform binaries: Rust baseline SHA256 6b364a9c9672571b268a8fc8fc34e85f824cdc5079b7f47cb426ca1ed4ff5c51; candidate SHA256 6297e4cbbe5b581e59bf3d24ef090649b90d4a2a21d5679e067c73ea92f5baac; DUCC SHA256 3f3970c94e6046c6838619d8efac091b5ca425628e24b91e2b65afd5627641db.

The referenced [DUCC issue 86](https://github.com/mreineck/ducc/issues/86) describes the Rust port as an exercise and performance comparison; it does not prescribe a paired-row method.

## FFT planner and executed code

The existing even-N path remains a packed length-N/2 complex FFT. The existing odd-N real transform uses a length-N complex FFT. The optional N=361 inverse is a 19×19 MixedRadix plan and was measured both with and without that plan. GDB hit the actual timed N=257 paired forward call in RustFFT's AVX2 RadersAvx2 implementation. At N=509 the actual timed paired forward call used BluesteinsAvx; it did not select Rader. Disassembly of the executed AVX Bluestein kernel contains YMM-width operations and vfmaddsub231pd. These observations establish the exercised paths for N=257, N=361 inverse and N=509; they do not imply every odd length uses the same algorithm.

Hardware performance counters were unavailable because perf_event_paranoid was 4. GDB call stacks and objdump were used to identify the executed functions and instructions.

## Paired-row math and safeguards

For forward analysis, pack z[j] = x[j] + i y[j] and compute one unnormalized length-N complex DFT Z. For each requested nonnegative mode k, recover X[k] = (Z[k] + conj(Z[-k mod N])) / 2 and Y[k] = (Z[k] - conj(Z[-k mod N])) / (2i). The DC imaginary components are set to zero explicitly. Truncation only limits recombination and storage; the complex FFT still computes its full transform.

For inverse synthesis, pack the Hermitian spectra as Z[k] = X[k] + iY[k] and Z[N-k] = conj(X[k]) + i conj(Y[k]), with the DC cell handled explicitly. One unnormalized inverse produces the first and second rows in its real and imaginary components. No 1/N factor is added, matching RustSHT's existing inverse_series convention.

The methods reuse RealFftWorkspace transform and scratch buffers. The eligible fast path requires odd N and f64 scalar transforms. Inputs must be finite, normal (or zero), and within MAX/(8N) component headroom. Paired row maximum magnitudes must both be zero or differ by no more than 8:1. A produced subnormal rejects the paired result. Invalid or unsafe pairs fall back to two independent transforms; this includes nonfinite values, subnormals, extreme finite values and one-zero/one-nonzero pairs. Both-zero rows may pair. The 8:1 limit follows measured cross-contamination: a 10,000:1 N=509 forward pair leaked about 5.5e-12 relative error into the smaller row, and a 32:1 probe still exceeded the strict coefficient check. Pairs at or below 8:1 passed the strict checks used here.

The scalar analysis path partitions adjacent rows into contiguous worker groups and uses the existing row-parallel eligibility and capacity limits. Inverse synthesis groups rows as pairs. Odd latitude tails are independent. Existing checked fallbacks and full output initialization remain in place. Forward N=255 pairing is disabled: its isolated and whole-transform controls did not show a reliable benefit.

## Isolated row-stage measurements

The required odd lengths 129, 255, 257, 361, 497, 509, 511, 513 and 515 and even controls 256, 378, 500, 512 and 540 were checked. The validation driver completed 547 numerical checks, including direct-DFT comparisons, forward/inverse reconstruction, DC and Hermitian edges, equal/opposite rows, zero rows, amplitude imbalance, nonfinite and subnormal inputs, extreme finite values, and even controls. The full matrix includes full, reduced and sparse-polar spectra, both directions, q1/q8 and DUCC. The final guarded inverse synthesis sweep used 30 calibrated batches per process, three independent processes per route/case, and approximately 30 ms per batch.

Selected final-guard full-spectrum inverse row-stage medians are below. Each Rust value is the median across three process medians; DUCC is included as a reference, not as an attribution of the Rust paired speedup. Values are milliseconds for the complete longitude stage over all rows in the case.

| N (rows) | q | Rust real | Rust paired | Paired/real | DUCC real |
|---:|---:|---:|---:|---:|---:|
| 361 (721) | 1 | 1.9275 | 1.5584 | 0.802 | 0.9479 |
| 361 (721) | 8 | 0.4545 | 0.2153 | 0.481 | 0.1640 |
| 497 (249) | 1 | 1.0149 | 0.7081 | 0.698 | 1.0439 |
| 497 (249) | 8 | 0.1509 | 0.1154 | 0.760 | 0.1690 |
| 509 (255) | 1 | 1.0960 | 0.7103 | 0.650 | 1.9996 |
| 509 (255) | 8 | 0.1616 | 0.1114 | 0.692 | 0.2746 |
| 511 (256) | 1 | 0.8216 | 0.6150 | 0.755 | 1.1203 |
| 511 (256) | 8 | 0.1168 | 0.0914 | 0.777 | 0.1841 |
| 513 (257) | 1 | 0.6066 | 0.4994 | 0.844 | 0.3287 |
| 513 (257) | 8 | 0.0936 | 0.0790 | 0.843 | 0.0618 |

At N=361, the ordinary paired inverse measured 1.5584 ms q1 and 0.2153 ms q8; the existing mixed-radix paired inverse measured 1.4012 ms q1 and 0.3783 ms q8. q8 N=361 had wide process variation: Rust real process medians ranged 0.2732–0.5572 ms and mixed-radix pair 0.2053–0.3911 ms. The narrower ordinary-pair range was 0.2151–0.2189 ms. Treat the mixed-radix q8 comparison as inconclusive.

The N=255 forward row-stage A/B was negative in both repeated runs: q1 paired medians were 1.6–3.8% slower than Rust real; q8 paired medians were 12–43% slower. The earlier whole-transform A/B at T127 was 2.9% slower q1 and only 0.7% faster q8. This supports disabling forward pairing for N=255.

Across paired and independent Rust row routes, warmed allocations were zero. The f64 workspace size was identical between the two routes: 30,352 bytes/worker at N=361, 15,904 at N=497, 40,912 at N=509, 16,352 at N=511 and 16,416 at N=513. This is the FFT plan workspace measured by the harness; it excludes application-level map/spectrum arrays. Direct two-complex rows were slower than the paired route at these selected lengths. The complete route matrix and per-case process ranges are in [row-stage-summary.csv](data/paired-odd-longitude-fft-2026-10-21/row-stage-summary.csv).

Across the final row-stage matrix, host PSI-derived wait was 1.06% median, p90 9.12%, max 11.40%; q1 median/p90 was 8.87%/9.15%, and q8 was 0.80%/1.83%. The per-process host and scheduler captures remain local.

An earlier calibrated phase profile helps explain the row-stage gains but predates the final 8:1 scale guard, so it is mechanistic evidence only. For N=509 inverse, independent prep/FFT/extract medians were 0.452/7.912/0.416 μs per row pair; paired values were 1.130/3.899/0.720 μs. N=513 inverse was 0.457/3.947/0.419 μs independent versus 1.147/1.806/0.733 μs paired. Pairing roughly halves FFT time at those lengths while adding packing, split and guard work. Final guarded total timings above govern the decision.

## Whole-transform matched A/B

The final exact-source matrix covers T124, T127, T248, T254, T255, T256 and T257 GL cases; T720 reduced order with N=361; T720 full odd longitude; and T720 full even-smooth control. Each has analysis and synthesis at q1 and q8. Baseline, candidate and DUCC were interleaved in three process runs per case/operation/thread count. Each process collected 10 calibrated timed batches, about 25 ms per batch, for 30 batches per route/case; the batches repeated short warm calls. First calls and validation were outside timed batches. The raw batch totals had p10/median/p90 21.8/25.1/30.1 ms. Across 360 worker processes, median scheduler wait fraction was 1.29%, p90 23.34%, max 27.41%. CPU PSI snapshots were intermittently nonzero. This contention and process spread are retained in interpretation.

For the nine odd-longitude cases, synthesis improved in 9/9 cases at both q1 and q8. Median speedups were 1.161× q1 and 1.198× q8 (geometric means 1.137× and 1.185×); median absolute savings were 368 μs and 62.6 μs per transform. Analysis improved in 8/9 cases at q1 and 6/9 at q8; median speedups were 1.114× and 1.091×, with median savings 230 μs and 28.6 μs. T127 q8 analysis was 2.8% slower even though forward pairing is disabled there. T256 analysis was neutral at q1 and 3.2% slower at q8. Analysis improvements are therefore less uniform than synthesis.

At T720 full odd longitude, analysis improved 1.151× q1 and 1.091× q8 (2.935 ms and 0.253 ms saved); synthesis improved 1.161× and 1.271× (4.092 ms and 0.946 ms saved). At reduced-order N=361, synthesis improved 1.064× q1 and 1.284× q8. For T248–T256 synthesis, which corresponds to nphi 497, 509, 511 and 513, paired whole-call medians improved 1.055–1.186× q1 and 1.091–1.242× q8. The analysis q8 results at T255 and T256 regressed 1.8% and 3.2% respectively.

The isolated full-spectrum inverse row-stage median divided by the matched whole-synthesis median estimates the FFT-stage share. The two harnesses used the same candidate source and geometry; because these were separate runs, the ratios are stage-share estimates rather than an additive in-call trace.

| Case (nphi) | q | Row stage baseline → paired (ms) | Whole synthesis baseline → paired (ms) | Estimated stage share baseline → paired |
|---|---:|---:|---:|---:|
| T248 (497) | 1 | 1.0149 → 0.7081 | 2.3455 → 1.9779 | 43.3% → 35.8% |
| T248 (497) | 8 | 0.1509 → 0.1154 | 0.3783 → 0.3157 | 39.9% → 36.6% |
| T254 (509) | 1 | 1.0960 → 0.7103 | 2.4065 → 2.0352 | 45.5% → 34.9% |
| T254 (509) | 8 | 0.1616 → 0.1114 | 0.4079 → 0.3284 | 39.6% → 33.9% |
| T255 (511) | 1 | 0.8216 → 0.6150 | 2.3084 → 2.0411 | 35.6% → 30.1% |
| T255 (511) | 8 | 0.1168 → 0.0914 | 0.4619 → 0.4233 | 25.3% → 21.6% |
| T256 (513) | 1 | 0.6066 → 0.4994 | 2.0859 → 1.9764 | 29.1% → 25.3% |
| T256 (513) | 8 | 0.0936 → 0.0790 | 0.3527 → 0.3160 | 26.5% → 25.0% |

The T720 even-smooth synthesis control improved 1.044× q1 and 1.243× q8 although even-N bypasses pairing; even analysis q8 regressed 2.8%. Do not attribute the even synthesis movement to paired rows. It signals source-layout/host variation and limits causal interpretation of the smallest changes. Per-case Rust baseline/candidate/DUCC medians, absolute savings, process ratios and ranges are in [whole-transform-summary.csv](data/paired-odd-longitude-fft-2026-10-21/whole-transform-summary.csv).

The per-call p10/median/p90 distribution over the 30 calibrated batches for each baseline/candidate/DUCC group, plus process wait summaries, is in [whole-transform-batch-summary.csv](data/paired-odd-longitude-fft-2026-10-21/whole-transform-batch-summary.csv).

The complete matched median matrix is shown below in milliseconds. Rust cells show baseline → candidate and speedup; DUCC cells show the pinned reference median.

| Case / nphi | Op | q1 Rust baseline → paired (speedup) | q1 DUCC | q8 Rust baseline → paired (speedup) | q8 DUCC |
|---|---|---:|---:|---:|---:|
| T124 (GL, nphi 249) | analysis | 0.5195 → 0.4664 (1.114×) | 0.6922 | 0.0890 → 0.0831 (1.072×) | 0.1310 |
| T124 (GL, nphi 249) | synthesis | 0.5311 → 0.4439 (1.197×) | 0.7101 | 0.1097 → 0.0927 (1.183×) | 0.1259 |
| T127 (GL, nphi 255) | analysis | 0.2892 → 0.2879 (1.004×) | 0.3019 | 0.0764 → 0.0786 (0.972×) | 0.0803 |
| T127 (GL, nphi 255) | synthesis | 0.4241 → 0.3995 (1.062×) | 0.3317 | 0.1036 → 0.0994 (1.042×) | 0.0766 |
| T248 (GL, nphi 497) | analysis | 2.0214 → 1.7910 (1.129×) | 1.9627 | 0.3170 → 0.2883 (1.099×) | 0.3145 |
| T248 (GL, nphi 497) | synthesis | 2.3455 → 1.9779 (1.186×) | 2.1642 | 0.3783 → 0.3157 (1.198×) | 0.3310 |
| T254 (GL, nphi 509) | analysis | 2.1643 → 1.8443 (1.174×) | 2.9883 | 0.3264 → 0.2931 (1.114×) | 0.4583 |
| T254 (GL, nphi 509) | synthesis | 2.4065 → 2.0352 (1.182×) | 3.2120 | 0.4079 → 0.3284 (1.242×) | 0.4670 |
| T255 (GL, nphi 511) | analysis | 1.8894 → 1.7584 (1.074×) | 2.0798 | 0.3007 → 0.3061 (0.982×) | 0.3377 |
| T255 (GL, nphi 511) | synthesis | 2.3084 → 2.0411 (1.131×) | 2.3321 | 0.4619 → 0.4233 (1.091×) | 0.3524 |
| T256 (GL, nphi 513) | analysis | 1.7229 → 1.7244 (0.999×) | 1.4005 | 0.2819 → 0.2914 (0.968×) | 0.2374 |
| T256 (GL, nphi 513) | synthesis | 2.0859 → 1.9764 (1.055×) | 1.5719 | 0.3527 → 0.3160 (1.116×) | 0.2463 |
| T257 (GL, nphi 515) | analysis | 2.4214 → 2.0280 (1.194×) | 2.6418 | 0.3617 → 0.3233 (1.119×) | 0.4365 |
| T257 (GL, nphi 515) | synthesis | 2.7731 → 2.2921 (1.210×) | 2.8615 | 0.4537 → 0.3579 (1.268×) | 0.4180 |
| T720 (GL, nphi 361, mmax 180) | analysis | 7.9003 → 7.3345 (1.077×) | 6.4521 | 1.1665 → 1.0471 (1.114×) | 1.0466 |
| T720 (GL, nphi 361, mmax 180) | synthesis | 10.8020 → 10.1553 (1.064×) | 9.5328 | 1.7910 → 1.3949 (1.284×) | 1.2702 |
| T720 (GL, nphi 1441) | analysis | 22.4391 → 19.5038 (1.151×) | 23.2186 | 3.0332 → 2.7805 (1.091×) | 3.1434 |
| T720 (GL, nphi 1441) | synthesis | 29.4931 → 25.4007 (1.161×) | 29.8348 | 4.4420 → 3.4962 (1.271×) | 3.8488 |
| T720 (GL, nphi 1458, even smooth) | analysis | 16.1883 → 15.9109 (1.017×) | 14.7990 | 2.2082 → 2.2724 (0.972×) | 2.1127 |
| T720 (GL, nphi 1458, even smooth) | synthesis | 21.2124 → 20.3095 (1.044×) | 20.1426 | 3.4632 → 2.7863 (1.243×) | 2.6799 |

## Numerical validation and checks

All 40 whole-transform validation groups passed finite-output and reset-consistency checks. Maximum Rust adjoint relative residual was 1.4263e-13; maximum Rust roundtrip relative L2 error was 6.506e-14. Candidate versus baseline maximum relative L2 difference was 2.3113e-15; candidate versus pinned DUCC maximum relative L2 difference was 5.195e-14. These include analysis and synthesis over the listed geometries and q1/q8.

The implementation's unit tests exercise paired versus independent transforms at N=129, 257, 361, 509 and 513, accepted 8:1 inputs, equal/opposite and zero rows, unsafe scale ratios, nonfinite/subnormal/extreme inputs, and N=361 mixed-radix inverse. A poisoned transform and scratch workspace is reused for three paired forward/inverse cycles at N=129, 361 and 509. Transform tests exercise even and odd latitude counts, q8, complete spectrum initialization, forced fallback, and an independent-row inverse comparison. Separate forward and inverse tests put a nonfinite value in either row, take the independent fallback, and verify the neighboring row remains finite and matches its independent result. Existing analytic, adjoint, f32, GL, CC, F1 and other-spin regression coverage also passed through the repository suites.

Verification completed on the final source:

- cargo fmt --all
- Strict release Clippy with parallel feature: passed.
- Release tests with parallel feature: 254 unit tests passed, 34 ignored; all integration suites passed (extreme scaling 4, f32 11 passed/1 ignored, workspace compatibility 1).
- Release tests with no default features: 206 unit tests passed, 17 ignored; all integration suites passed (extreme scaling 4, f32 11 passed/1 ignored, workspace compatibility 1).
- Python syntax compilation for the benchmark runners: passed.

## Conclusion and limits

The paired route is accepted as an isolated experimental implementation because row-stage and whole-transform synthesis improvements agree across the tested odd lengths, and all numerical/fallback checks pass. Forward pairing remains disabled at N=255. Analysis is promising but less robust under q8; changes of only a few percent should not be treated as conclusive on the observed host. The even control confirms that not every whole-transform difference is caused by the new route.

The candidate changes no public API and adds no dependency. Production integration still conflicts with the active owner direction to defer odd-length FFT work, so it requires a later explicit scope change and a clean matched rerun on that source. Nothing was pushed; the experimental worktree is left available for review. Raw disassembly, per-batch logs and host/process/PSI snapshots remain under the local temporary data paths. Benchmark build outputs are untracked and excluded from the commit.
