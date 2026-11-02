# B3 sampled scaled-leaf profile — 2026-11-02

This test-only profile samples the active f64 spin-1 analysis scaled prefix in
GL T720 full/reduced q1/q8 runs. It separates masked accumulation,
exception-lane recovery, tail work, and recurrence/normalization. Production
routing and binaries were not changed. The profiler did not justify a broad
kernel rewrite; it identified partial exceptional vectors as the next bounded
candidate.

## Findings

The active scaled prefix has about 427 sampled recurrence steps per full-order
call and 264 per reduced-order call at a 1-in-64 sampling rate. q1 expansion
below multiplies sampled service time by 64; it estimates the prefix's work and
is not an independent whole-transform phase timer.

| Cell | Instrumented whole-call median | Scaled prefix, ×64 | Masked accumulation | Exceptional recovery | Tail | Recurrence/normalization |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 38.0 ms | 12.54 ms | 1.89 ms | 3.10 ms | 0.85 ms | 2.24 ms |
| Full q8 | 5.61 ms | 13.76 ms worker service | 1.99 ms | 3.38 ms | 0.94 ms | 2.42 ms |
| Reduced q1 | 15.57 ms | 7.35 ms | 1.27 ms | 1.56 ms | 0.45 ms | 1.36 ms |
| Reduced q8 | 2.14 ms | 7.69 ms worker service | 1.28 ms | 1.62 ms | 0.49 ms | 1.41 ms |

The four disjoint subphases sum to 8.08 ms (full) and 4.64 ms (reduced) of the
expanded q1 scaled-prefix estimate. The remaining 4.46/2.71 ms covers leaf
setup and work outside those timer regions. q8 stage totals are aggregate
worker service and must not be compared directly with parallel wall time.

The exception phase is largest in both q1 cells. Lane counters show a
specific mixed-vector opportunity:

| Cell | Exceptional vector lanes | Scalar fallback lanes | Scalar share | Full-exception vectors |
|---|---:|---:|---:|---:|
| Full q1 | 2,405 | 607 | 25.2% | 453 |
| Full q8 | 2,436 | 596 | 24.5% | 457 |
| Reduced q1 | 1,009 | 395 | 39.1% | 153 |
| Reduced q8 | 952 | 388 | 40.8% | 142 |

Counts are medians per profile call, across 15 sampled calls per cell. The
remaining exception lanes in full four-lane groups use the existing checked
vector helper. Partial groups call the scalar scaled accumulator once for each
exception lane. Tail exceptions are rare in the full fixture (median 38) and
absent in the reduced fixture; tail work is not the priority.

The test-only profiler has negligible measured q1 whole-call perturbation: its
three-pair median ratios (profile/plain) are 1.004 full and 1.008 reduced.
q8 overhead is unresolved because process-pair ratios are noisy: 0.995–1.093
full and 0.970–0.978 reduced. The profile path outlines only sampled steps;
timer attribution is therefore diagnostic, not a cycle-accurate estimate of
the inlined production leaf. `perf stat` cannot access PMU events in this
container (`perf_event_paranoid=4`).

## Assembly evidence

The no-profile Pulp V3 AVX2 specialization of
`FusedCheckedScaledAnalysisPairChannels<false>` in
`src/legendre/scalar_paired/analysis.rs` is 24,903 bytes. Its prologue reserves
2,696 bytes of stack; the body contains 244 static `vfmadd*` instructions, 108
calls, and 186 YMM stack loads/stores. This confirms heavy stack traffic in
the large inlined route but does not prove that spills dominate runtime. The
complete bounded disassembly is compressed under `data/`.

## Next experiment

Try one isolated B3 f64 candidate: for partial four-lane exception groups,
mask ordinary-lane coefficients to zero and pass the group through the
existing checked scaled-vector helper. The helper already preserves exact
zero products while scaling nonzero normal products; it falls back to the
existing scalar path if a product or exponent is unsafe. This targets the
measured 607/395 scalar fallback lanes per sampled call without removing
exceptional-value checks or duplicating recurrence. First test output errors
and a helper microbenchmark, then run three whole-call pairs for all four
cells. Reject it if the saved scalar work does not improve whole-call time.

## Provenance and reproduction

- RustSHT base commit: `f956320cfd26bd9959dee7bfa42770ae510a0d43`.
- Base `analysis.rs` blob: `baa5cee6c64ac43aa54828ab4c15055e8d23532e`.
- Base `transform/spin/analysis.rs` blob: `88a30c434410d8e44f4b690321380fa3671110ed`.
- DUCC reference: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Compiler: Rust 1.99.1 / LLVM 23.1.1; release, `-C target-cpu=x86-64-v3`,
  `parallel`; Pulp dispatched V3 AVX2/FMA on an AMD Ryzen 9 5950X.
- GL geometry: `ntheta=721`, `phi0=0.23`; full `mmax=720,nphi=1458`, reduced
  `mmax=180,nphi=378`; spin 1, two deterministic map components.
- q1 used one pinned physical core; q8 used eight physical cores. Exact CPU
  indices and raw console output remain in local experiment scratch.
- Fixture hashes: full map0/map1 `e3379278cb0dfc9a9f4d5f2c8ece40995f965bd727dcb4ce884b85fc62d7abba` /
  `dc3a132282b074d16cea41cf7305e64e796a48beac398221776ee8c7ad74d5cc`;
  reduced `33ed76ff3e7ebfbe866484fe2473634df9b5380cfe123936d263515be97b3eed` /
  `e92ffce48a2fa329a5bf113e0cdc6dd9b212bbea6c0f63568fb2e304e6bcf881`.
- Plain/profile test-binary SHA-256:
  `8e624759902d5a6dabc0e22c5ad37cf9b765658f26cca635840e0efb87ecd995` /
  `87543513baf025054920d261a6f7ca2d58c30f296af1b1d950bad8702e398e83`.
- Test-only patch: [`patches/test-only-sampled-profile.patch.gz`](patches/test-only-sampled-profile.patch.gz),
  SHA-256 `aa63b73d0d67ac37e3f5384a5875c601b48cd51036b6934449226b76ceaf5abf`.
- The machine-readable stage samples, stage medians, and paired overhead
  observations are in [`data/`](data/).

Apply the test-only patch at the base commit, build one plain test binary and
one with `--cfg rustsht_b3_sampled_profile`, then run the ignored
`m6_b3_sampled_scaled_leaf_timing` test with `RUSTSHT_B3_Q`,
`RUSTSHT_B3_REDUCED`, `RUSTSHT_B3_MAP0`, and `RUSTSHT_B3_MAP1` set to the
matching fixture files. The test checks bitwise repeatability; it is not an
independent numerical acceptance test.
