# Shared paired synthesis two-latitude-vector tile rejected

## Provenance and method

- Exact source parent: b3b93f17fe4d404fe4c033ffc7420eb190dc00b0 (origin/dev at the run).
- Candidate patch SHA-256: d1eab23e7acbb902d4d0b2cff3677d3176496aa16ef8b3cb08feb6d5ae0e12fd; it changes only src/legendre/scalar_paired/synthesis.rs, processing two adjacent SIMD latitude vectors per paired-degree recurrence. Existing shared recurrence, checked transition, scalar tail and exceptional fallback remain in place.
- DUCC native-f32 reference: 0.42.0, exact source commit 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b; f64 protected control used the same DUCC commit.
- Host: AMD Ryzen 9 5950X, AVX2/FMA. Rust 1.99.1 / LLVM 23.1.1, --locked --release --features parallel, CARGO_PROFILE_RELEASE_DEBUG=1, RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'. DUCC used GCC 15.2.0, CMake Release with -g; SHT target flags -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3, driver overrides fast math with -fno-fast-math.
- T720 public f32 operation: GL, ntheta=721, full mmax=720,nphi=1458, reduced mmax=180,nphi=378, phi0=0.23, spin-1 synthesis. Parent, candidate and native-f32 DUCC ran in three interleaved comparisons per cell, 25 ms calibrated warm batches; q1 pinned to one physical core and q8 to eight distinct physical cores. The adapter creates the f64 plan/workspace before timing and includes Complex32-to-Complex64 promotion, the public f64 transform/FFT, and map narrowing/writeback. Reusable conversion-buffer capacity was 25,148,480 bytes full and 8,015,360 bytes reduced; warm setup is amortized.
- T1023 protected f64 GL spin-1 synthesis: full mmax 1023, grid 1024x2048, three interleaved comparisons at q1/q8, 25 ms batches. This legacy worker records repeat/reset but omits its finite_outputs field; candidate/parent output comparison was bitwise exact and candidate/DUCC comparison metrics were finite.

## Complete f32 public-call screen

Times are medians in ms; ratios are paired candidate/parent (C/P) and candidate/native-f32-DUCC (C/D). Three-pair intervals are observed ranges, not confidence intervals.

| GL T720 spin-1 synthesis | Parent / candidate / DUCC ms | C/P median [range] | C/D median [range] | Max relative-L2 vs DUCC | Decision |
|---|---:|---:|---:|---:|---|
| Full q1 | 38.532 / 38.447 / 40.521 | 0.998 [0.996, 1.007] | 0.952 [0.949, 0.952] | 3.91e-8 | Neutral vs parent |
| Reduced q1 | 28.150 / 33.045 / 17.630 | 1.181 [1.169, 1.242] | 1.883 [1.863, 1.984] | 3.89e-8 | Regression; reject |
| Full q8 | 8.917 / 8.614 / 5.926 | 0.974 [0.944, 0.985] | 1.453 [1.370, 1.472] | 3.91e-8 | Small gain; still GAP |
| Reduced q8 | 3.391 / 3.626 / 2.597 | 1.050 [0.994, 1.111] | 1.305 [1.275, 1.396] | 3.89e-8 | No stable gain; GAP |

All 36 workers passed finite-output, repeat and poisoned-output reset checks. Candidate and parent outputs were bitwise equal in every cell. Maximum candidate/DUCC absolute error was 1.20e-7. These three-pair screens are sufficient to reject the repeatable reduced-q1 regression; no cell was extended to 15 pairs.

## Protected f64 result and mechanism

The T1023 f64 candidate/parent paired medians were 0.9974 q1 (95% bootstrap interval [0.9948,1.0007]) and 0.9802 q8 ([0.7816,1.0028]), with candidate/parent outputs bitwise equal and all workers repeat/reset consistent. The q8 comparison was highly variable; this does not establish a small f64 gain or regression.

Executed AVX2 V3 code for FusedPreparedSynthesisPairBlock confirms the intended coefficient broadcasts are reused across two vector lanes, but the candidate body spills multiple YMM recurrence/sum vectors to the stack inside the pair loop. Its stack frame is 0x228 bytes versus 0x98 for the parent body. In the candidate body around 0x1f6e00–0x1f6f5f, repeated vmovupd loads/stores use stack slots including 0x90, 0xb0, 0x120, 0x140, 0x1a0–0x200(%rsp). This register pressure explains why the reduced q1 whole call regressed despite fewer broadcasts. Do not increase this tile or retry the same loop interchange unchanged.

Focused candidate validation passed cargo fmt --check and cargo test --locked --release --features parallel --lib spin1 -- --test-threads=1 (49 passed, 4 ignored), covering spin conversion identities, hemisphere synthesis/reference comparisons, T720 adjoints, poles, extremes, cache/workspace reset and q1/q8 agreement. All benchmark candidate/parent outputs were exact. This rejected screen did not claim broad production acceptance.

## Reproduction evidence

f32-results.json retains the four full pair records, fixture/output hashes, timings, ratios, numerical error and worker checks. f64-protected-results.json retains the direct-f64 controls and hashes. The exact patch, benchmark workers, protected test log and selected parent/candidate executed assembly are saved beside this report; SHA256SUMS covers all package files. Binary SHA-256 values: Rust f32 parent 8f4998ea9f0c751ba7edba52a8c2aefc2b4b093f445afc97c523c5d07ececd4c, Rust f32 candidate ea34975c61b4bda8a414246c35ae3b8f09675e438e1d66da916a896b4ba17bf2, DUCC f32 5edd5f6eaa72cec2f010fe322bc5cc59da2a6c5b2673e89845e5aa372de0341b; Rust f64 parent c15c0f5ef97c72f82bbaed862b6cb04cd8c19a2a534bf23409bcad776f5b8bab, Rust f64 candidate e105b5475aaa506720df357cedff1181c27fed724b924c1f3b7f0919b1b578e3, DUCC f64 273fd7745f012b7291030030bf86ac7a0d5a9e5b37701869a074651df7b9c659.
