# B1 scaled-output profile and rejected f32 scale fast path — 2026-11-02

## Scope and provenance

This B1-only diagnostic starts from production source `8560bcae34e0532a230b9a3e0d05cf10380cfccf` and pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The host was an AMD Ryzen 9 5950X. Rust used release mode and `-C target-cpu=x86-64-v3`; the selected Pulp backend was V3 AVX2/FMA. q1 was pinned to CPU 2 and q8 to CPUs 0–7. `perf stat` and `perf record` could not access PMU events (`perf_event_paranoid=4`).

The phase/profile test and the candidate-only production helper patch were isolated in a temporary worktree outside the production checkout; no candidate code was applied to the repository. The test-only patch's uncompressed SHA-256 is `6717042a76e059df7c9c17edcb9a1dd41b1bec5d81868f4dfe9fe33c5f3093db`. `lane-profile.json` records source, binary and fixture hashes. The whole-transform screen used three interleaved process pairs per cell and exact parent source `8560bcae`; it is a screen, not a promotion-quality timing claim.

## Profile distribution

Test-only stage probes on nonzero GL T720 fixtures show that Legendre and accumulation dominate the B1 call, particularly for f32. Values below are diagnostic medians in milliseconds; instrumented timings are not acceptance timings.

| Precision, truncation, q | Whole call | Legendre + accumulation | FFT | Legendre share |
|---|---:|---:|---:|---:|
| f32 M720 q1 | 41.458 | 38.613 | 2.859 | 93.1% |
| f64 M720 q1 | 16.283 | 13.016 | 3.222 | 79.9% |
| f32 M180 q1 | 14.423 | 13.651 | 0.691 | 94.7% |
| f64 M180 q1 | 6.958 | 6.011 | 0.911 | 86.4% |
| f32 M720 q8 | 6.471 | 5.913 | 0.542 | 91.4% |
| f64 M720 q8 | 2.832 | 2.161 | 0.621 | 76.3% |
| f32 M180 q8 | 2.117 | 1.965 | 0.144 | 92.8% |
| f64 M180 q8 | 1.284 | 1.045 | 0.209 | 81.4% |

These stage costs are disjoint within the test probe. They do not split recurrence, ordinary/scaled accumulation, exponent normalization or fallback into separate elapsed-time shares. Do not combine them with inclusive Callgrind region percentages.

For f32 M720, route counters recorded 266,685 typed scaled pair calls, 159,441 scaled-state initializations, 1,066,740 scaled output values, and 816,698 outputs eligible for the normal exponent-safe scale operation. The exponent state samples were nonzero in 81.1% of full-order samples and 78.2% of reduced-order samples. Hard recovery was uncommon by count (5,601 scalar lanes and 13,643 vectors at M720; 5,601 and 11,899 at M180). q1 and q8 route counts matched. For f64, no typed scaled pair calls were recorded; the SIMD seed attempt declined 474 full-order and 132 reduced-order blocks into its fallback route. These counts suggest scaled work is a useful f32 target and that exceptional-lane recovery is not the dominant f32 work count, but counts alone do not establish cycle shares.

The current shared profile gives a distinct hypothesis for each precision: f32 spends most of its Legendre time in scaled recurrence and accumulation, while f64 is already near DUCC and still has measurable seed-decline/fallback plus ordinary-kernel overhead. The exact next attribution needs isolated stage or kernel benchmarks; no PMU cycle attribution is available on this host.

## Tested candidate: exact f32 output scaling

The isolated candidate replaces `numeric::scale_binary_f32` with an exponent-field shortcut only when the input and result are normal finite values; all exceptional and out-of-range values retain the existing exact path. A test checked 4,096 bit patterns across 17 exponents, including `i64` exponent extremes, and matched the baseline exactly.

The pinned helper microbenchmark used 8,192 values, 75% eligible, 1,024 repetitions and 15 alternating samples. The baseline was 2.585 ns/value; the candidate was 2.246 ns/value, with paired geometric ratio `0.881 [0.855, 0.909]` (95% t interval on log ratios). This isolated helper gain did not transfer to the whole transform.

Three interleaved whole-call pairs compared exact parent, candidate and matching DUCC on all full/reduced q1/q8 T720 fixtures. Parent and candidate output buffers were byte-identical. Candidate/parent geometric ratios were:

| Cell | f32 candidate/parent (95% interval) | f64 candidate/parent (95% interval) | Candidate/Rust-matching DUCC point ratio, f32 / f64 |
|---|---:|---:|---:|
| Full q1 | 1.053 [0.986, 1.126] | 1.011 [0.967, 1.057] | 2.847 / 1.069 |
| Full q8 | 1.010 [0.913, 1.117] | 0.979 [0.860, 1.115] | 2.722 / 1.100 |
| Reduced q1 | 1.004 [0.928, 1.086] | 1.006 [0.988, 1.023] | 2.245 / 1.019 |
| Reduced q8 | 1.034 [0.987, 1.082] | 1.016 [0.996, 1.037] | 2.141 / 1.079 |

All candidate/parent confidence intervals cross one. The f32 point estimate is slower in three cells, and no whole-call benefit is established. The candidate is rejected and must not be promoted. The three-pair ratios are a screen; the existing 15-pair ledger remains the current parity record.

## Disassembly and explanation

The monomorphized f32 V3 function grew from 22,307 to 23,610 bytes (+5.8%) and from 5,023 to 5,346 decoded instructions (+6.4%). The candidate adds 38 comparisons and 52 branches; both have 204 FMA instructions, the same 0x17e8-byte frame, and the same 86 YMM stack-memory references across the large function. The ordinary two-pair loop keeps the accepted zipped-loop FMA sequence. The scale shortcut is inlined into the same large function. This code growth is consistent with the inconclusive/slower transform screen but does not prove a branch or instruction-cache penalty.

The profile and code together indicate that accelerating the final scale helper alone is too narrow: the helper microbenchmark improves, but output scaling is only one operation within the much larger Legendre path. The measured root-cause hypothesis is that native f32's scaled recurrence/accumulation and state normalization dominate the remaining gap; f64's fallback from failed typed seed setup merits its own isolated measurement. This remains a hypothesis about elapsed cost, not a proven cycle decomposition. The candidate's exactness check is not a substitute for the existing full numerical suite.

## Artifacts and next

- `lane-profile.json`: phase measurements, route counts, test/production binary hashes and fixture hashes.
- `whole-transform-screen-3pair.json`: per-pair screen, confidence intervals, dispatch and artifact provenance.
- `scale-microbench-summary.json`: path-sanitized helper benchmark parameters and paired samples. The untracked raw console capture is kept locally.
- `disassembly-summary.txt`: compiler, symbol sizes and static instruction summary.
- `candidate-and-probe.patch.gz`: isolated candidate and test-only instrumentation patch; uncompressed patch SHA-256 is recorded above.
- `route-q1.log.gz`, `route-q8.log.gz`: bounded route traces.
- `SHA256SUMS`: checksums for all package files other than itself.

**NEXT:** measure the scaled pair step and exponent normalization separately for a representative nonzero f32 order, and isolate the f64 typed-seed decline/fallback route with a focused microbenchmark. Use non-overlapping regions and current `8560bcae` production code, then test a candidate only if a material avoidable cost is identified. Keep B0, B2 and B3 unchanged.
