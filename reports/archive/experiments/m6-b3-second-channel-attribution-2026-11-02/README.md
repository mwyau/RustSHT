# B3 second-channel accumulation attribution — 2026-11-02

## Provenance

- Rust production parent: `4a55859649d5f74603ad458890edc6a34adf3b6f`.
- DUCC reference revision: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Probe binary SHA-256: `fd1bb5718761324cffb155b64ab61bf5d3b6301a9333e58b9ddf134f38e9db84`.
- Test-only source patch SHA-256: `ba865bc59ab648f425a2a289f6cf10479cffe2afd2faebb4ecc64dbe24a14491`.
- Machine: AMD Ryzen 9 5950X; q1 pinned to CPU 0, q8 to CPUs 0–7.
- Rust toolchain: `rustc 1.99.1` / LLVM 23.1.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC uses g++ 15.2.0, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for SHT sources. The earlier exact-source baseline binary hashes are Rust `c33798bec2ed2df6385929da776a75045e3313ca76b328fda3507d5e551a1d97` and DUCC `b646183e9df8aac030f796e2357b8d92e3b4a194d21be98b460eae043ab63443`; they are distinct from the test-only probe binary.
- The original test-runner dump contains machine-local paths and is not included. Its numerical skip-probe measurements are preserved in [`skip-screen-summary.json`](raw/skip-screen-summary.json); the test-only patch and exact-current Callgrind profiles are included below.
- Source blobs: Rust `src/legendre/scalar_paired/analysis.rs` = `b0077a7236f7e2315fe0c6b07da74f3eb4e845ba`; DUCC `src/ducc0/sht/sht_inner_loop.h` = `f86d5a2ce61d7eea9b94253255fc3d3b9e30251f`.
- Fixture hashes (map0/map1): full `e3379278cb0dfc9a9f4d5f2c8ece40995f965bd727dcb4ce884b85fc62d7abba` / `dc3a132282b074d16cea41cf7305e64e796a48beac398221776ee8c7ad74d5cc`; reduced `33ed76ff3e7ebfbe866484fe2473634df9b5380cfe123936d263515be97b3eed` / `e92ffce48a2fa329a5bf113e0cdc6dd9b212bbea6c0f63568fb2e304e6bcf881`.

## Diagnostic

The ignored test-only profile ran ten calls per cell, alternating the normal route with a mode that skips only `AnalysisTwoBasisAccumulation` and its coefficient stores in ordinary fused leaves. The second-channel state remains present, recurrence-buffer rotation is unchanged, and scaled/fallback routes still execute. Skip-mode outputs are intentionally incomplete and are not correctness or performance candidates.

| GL T720 cell | Normal call (ms) | Skip second channel (ms) | Saved (ms) | Saved total | Saved recurrence+accum phase |
|---|---:|---:|---:|---:|---:|
| Full q1 | 34.684 | 27.952 | 6.733 | 19.4% | 35.3% |
| Full q8 | 5.362 | 4.310 | 1.052 | 19.2% | 35.3% |
| Reduced q1 | 14.062 | 10.735 | 3.327 | 23.6% | 35.3% |
| Reduced q8 | 1.969 | 1.509 | 0.459 | 23.2% | 36.6% |

The full q1 paired whole-call deltas were stable at 6.55–6.88 ms. q8 deltas ranged from 0.68–1.70 ms, so the absolute q8 values remain noisy. These are instrumented diagnostic calls, not production benchmark results. Their relative paired effect shows the second-channel accumulation is a large part of Rust's recurrence/accumulation stage.

The earlier per-operation timestamp split estimated about 41% of sampled fused-leaf time for the second channel, but timestamp overhead contaminates that percentage. The skip experiment is the stronger evidence: recurrence+accumulation elapsed time falls about 35% when the second channel's ordinary accumulation is omitted. The current phase profiler measures recurrence and both accumulations together at roughly 19 ms summed worker time in full q1.

## Source crosswalk

- Rust dispatch in `src/transform/spin/analysis.rs` reaches `FusedScalarAnalysisPair<true,false,false,true,true>` in `src/legendre/scalar_paired/analysis.rs` (the active four-lane AVX2/FMA leaf). `fused_analysis_two_pair_steps_with_simd` advances two degree pairs and accumulates the first channel into eight vector sums. It stores the recurrence states, then `AnalysisTwoBasisAccumulation` (around line 6699) reads two basis arrays and four same/opposite real/imag coefficient arrays, performs eight vector FMAs per packed-latitude visit, and reduces eight sums. Full GL ring blocks have no scalar SIMD tail. The normal path compiles out per-vector range checks for this specialization; it retains tail checks and the dispatcher’s exceptional fallback.
- Pinned DUCC `calc_map2alm` in `sht_inner_loop.h:636–684` transitions scaled states, then calls `map2alm_kernel` at `:592`. For each channel, the kernel advances two degree pairs, accumulates four packed ring/hemisphere real/imag arrays for both degrees (eight vector accumulators), then horizontally reduces with `vhsum_cmplx_special`. The spin-1 wrapper invokes the scalar analysis route for both spin-to-scalar channels. Both implementations use four f64 lanes under x86-64-v3.
- This comparison explains why skipping Rust’s second-channel accumulation is so expensive: it is required output work comparable to one DUCC scalar-channel accumulation. It does **not** show that these operations are redundant. Rust’s distinction is dataflow: its recurrence is shared across Q/U; the second channel consumes the saved basis states in a separate accumulator pass. Existing per-degree fusion was measured slower, so the skip result does not justify repeating that candidate.

## Interpretation and next discriminating step

The Rust path advances the recurrence once, accumulates the first channel during the two-degree step, then runs `AnalysisTwoBasisAccumulation` over the resulting basis vectors for the second channel. Omitting that second accumulation removes about one fifth to one quarter of the instrumented whole call. The necessary second output exists in DUCC as well, so this does not by itself show that Rust performs duplicate mathematical work; it identifies the cost that must be made cheaper to close the remaining B3 gap.

The accumulation-only leaf comparison was the next proposed diagnostic at the time of the initial skip experiment. The exact-current profile below supersedes that proposal: instruction/code-size excess is verified, but no cycle cost or removable operation is established. Do not start another kernel/layout candidate from these counts alone. A follow-up needs cycle/front-end/backend attribution on a host with usable hardware PMU counters, or another measurement that separates recurrence dependencies from accumulation throughput without changing the production route.

## Exact-current source control and Callgrind profile — `4a558596`

The GL T720 `even-smooth` control uses three interleaved process pairs and the current whole-transform binaries listed above. The intervals below are 95% Student t intervals on the logs of the three paired process ratios (df=2); they are screens, not 15-pair certification.

| Cell | Rust f64 ms | DUCC f64 ms | Rust / DUCC (95% interval) | Status |
|---|---:|---:|---:|---|
| Full q1 | 33.106 | 30.587 | 1.077 [1.065, 1.090] | GAP |
| Full q8 | 4.881 | 4.256 | 1.144 [1.070, 1.224] | GAP; wide interval |
| Reduced q1 | 12.783 | 11.652 | 1.098 [1.075, 1.122] | GAP |
| Reduced q8 | 1.933 | 1.738 | 1.110 [1.077, 1.143] | GAP |

The current-source process-pair ratios, source/binary/fixture metadata and interval calculation are in [`current-source-callgrind-summary.json`](raw/current-source-callgrind-summary.json); complete Rust Callgrind profiles are compressed in [`full q1`](raw/current-source-rust-full-q1.callgrind.out.gz) and [`reduced q1`](raw/current-source-rust-reduced-q1.callgrind.out.gz). Rust source blobs at `4a558596` are `src/transform/spin/analysis.rs` `88a30c434410d8e44f4b690321380fa3671110ed` and `src/legendre/scalar_paired/analysis.rs` `b0077a7236f7e2315fe0c6b07da74f3eb4e845ba`. The profiling harness binary SHA-256 is `dc2a5f16e5bff0c20289b9ac122aa9697e3632cad4a8f3881e14b50549fa4181`; it profiles one warm whole transform after warmup and workspace-reset checks. Its harness source and C Callgrind shim are preserved in the [ordinary-leaf package](../m6-b3-ordinary-leaf-profile-2026-11-02/repro/).

The production-path Rust Callgrind run reproduces the prior exact-code profile: 577.66M full-q1 and 235.07M reduced-q1 instructions per transform. The ordinary fused leaf accounts for 262.90M / 135.18M instructions (45.5% / 57.5%); against DUCC's two-channel `map2alm_kernel`, its instruction count is 1.178× / 1.207×. The full-transform Rust/DUCC instruction-count ratio is 1.334× / 1.345×. These are Callgrind counts, not cycles. Valgrind reported a `brk segment overflow` warning during profiling; it nevertheless completed and produced finite profiles matching the earlier exact-code counts.

The executed leaf is four-lane AVX2/FMA. Its body is 11,160 bytes with 252 static `vfmadd*`, 44 `vaddpd`, and no YMM stack references; DUCC's `map2alm_kernel` body is 722 bytes with 18 static FMA and six `vaddpd`, also without YMM stack references. Hardware PMU access remains blocked (`perf_event_paranoid=4`). The measured instruction/code-size excess does not establish cycle impact or identify a removable operation. The earlier 15-pair seed-handoff matrix remains the stronger whole-call reference; this three-pair control is not a promotion benchmark.

No production kernel was changed or accepted. The second-channel skip remains an attribution probe, not a valid optimization. B1's screened suffix-handoff candidate remains rejected because repeated-output reset validation failed; its timings are invalid for promotion. Checksums for all package evidence are in [`SHA256SUMS`](SHA256SUMS).
