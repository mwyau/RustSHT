# B3 normalizer-mask reduction screen — 2026-11-02

## Decision

**Rejected; no production source change.** Combining the finite and in-range lane predicates reduced helper time on captured stable vectors, but the three-pair whole-transform screen did not resolve a gain in any cell. Candidate/parent intervals cross 1.0 in all four cells, while all candidate/DUCC f64 cells remain GAP. The captured helper improvement therefore does not support another B3 kernel change under the active stop rule.

## Helper results

The test-only variants were measured over nine interleaved trials per category on the four-lane AVX2 f64 path. Values are ns/vector in the order baseline, combined finite/in-range, combined finite/in-range/zero-exponent:

| Captured group | Full q1 | Reduced q1 |
|---|---:|---:|
| Zero exponent | 3.095 / 2.699 / 2.388 | 2.656 / 2.202 / 1.902 |
| Stable, nonzero exponent | 2.731 / 2.310 / 3.219 | 2.606 / 2.188 / 3.136 |
| Exponent changing | 17.492 / 17.737 / 17.794 | 18.490 / 18.637 / 18.709 |

The whole-call candidate implements only the finite/in-range combination. The captured-state frequency gives an estimated ceiling below 0.34% full and 0.76% reduced call time; this is a frequency-weighted estimate, not an observed call-level saving. The all-predicate helper is diagnostic only and regresses stable nonzero-exponent groups. All helper variants produced bitwise-identical outputs on stable, scaled and exponent-changing captured vectors.

## Whole-transform results

Three interleaved parent/candidate/DUCC pairs were run for each GL T720 full/reduced q1/q8 cell. Ratios are paired geometric means with 95% t intervals on log ratios. A crossed interval is INCONCLUSIVE; it is not a PASS.

| Cell | Candidate / parent | Candidate / DUCC f64 | Result |
|---|---:|---:|---|
| Reduced q1 | 0.996 [0.989, 1.004] | 1.263 [1.261, 1.266] | Parent comparison INCONCLUSIVE; DUCC GAP |
| Reduced q8 | 0.984 [0.940, 1.030] | 1.279 [1.217, 1.344] | Parent comparison INCONCLUSIVE; DUCC GAP |
| Full q1 | 0.985 [0.961, 1.010] | 1.234 [1.203, 1.267] | Parent comparison INCONCLUSIVE; DUCC GAP |
| Full q8 | 1.023 [0.908, 1.152] | 1.273 [1.145, 1.416] | Parent comparison INCONCLUSIVE; DUCC GAP; noisy |

Candidate and parent outputs were bitwise identical in all 12 process pairs; repeat and workspace-reset checks passed. Focused tests also passed: `normalize_pair_lanes_preserves_ordinary_lanes_in_mixed_blocks`, `spin1_reflected_analysis_subranges_fall_back_for_extreme_inputs_on_all_grids`, and `spin1_reflected_analysis_subranges_reset_poisoned_scratch_on_gl_cc`. Relative L2 error versus DUCC was unchanged from parent: `5.2127e-14` reduced and `4.7606e-14` full. No accuracy regression was observed.

## Provenance and reproduction

- Repository `dev` HEAD at packaging: `ba9e54097092d32a8c1a9b9e0ae6d677e1a72eaf`.
- Candidate build base: `be8f72d7fc55c420da5cf12b843b9353d2316eec`; the relevant production source blobs match `dev` at packaging. The run used parent and candidate binaries built from this base, with only the candidate `src/legendre/scalar.rs` patch applied.
- DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Parent/candidate/DUCC binary SHA-256: `d401668c6aa462fe2e81725672cdd5e097d30bdfd6f8271268398af79005f25d`, `da012dfd269fdbaec6f0c478cdcefc07bf3247535a96fe2e331b4a14637f3f82`, `5dacb5987ee621a5115fed405775dc85231361ab703b3eae1694d086a5a7bdf8`.
- Rust 1.99.1 / LLVM 23.1.1; Rust `-C target-cpu=x86-64-v3`; DUCC SHT `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, driver `-fno-fast-math`.
- AMD Ryzen 9 5950X; Pulp V3 AVX2/FMA. q1 pinned to CPU 0; q8 to CPUs 0–7. GL T720, `ntheta=721`, `phi0=0.23`; full `mmax=720,nphi=1458`, reduced `mmax=180,nphi=378`.
- Machine-readable paired records, fixture hashes, worker batches, output metrics and environment are in [`data/threeway-screen.json`](data/threeway-screen.json). Helper timings are in [`data/helper-summary.json`](data/helper-summary.json).
- The compact runner is [`reproduce-screen.py`](reproduce-screen.py); invoke with explicit `--repo`, `--parent`, `--candidate`, `--ducc` and `--results` arguments.
- Production source SHA-256: base `9ae6f11231973077c90478da4258d0115254180ab6334208220c6b62ae98551a`; candidate `6bcb70b45266d03095a8b598b87bf762c082a2cb05d93cb3d09b23b1fa055f0a`. Git blob IDs: `e0d789e4e5c56cd31dade204fc240b7ae4bc4d52` and `2a58405fbfe21aec451b4e72fa6274892d2a6fda`.
- Test source SHA-256: captured replay base `e22e21ccecc549ef7a64e5d939042c1b87d19410ad97d31f2404f1a9a4d9e017`; test-only candidate `fcedf5ced2f7ab9175d85c24dd6d9b1276eea6aee06953965bc31fb00bdb487d`. Test binary SHA-256: `5fe6f94aa66def2f68ef621f6feb8a1c5a3215a438b3565817fc37ba8349be62`.
- Apply the prior captured-state patch from the [recurrence-normalizer replay package](../m6-b3-recurrence-normalizer-replay-2026-11-02/README.md), then `patches/test-only-helper-variants.patch.gz` for helper trials. `patches/production-mask-candidate.patch.gz` records the rejected production candidate separately.
- The earlier recurrence/normalizer replay package includes the sanitized baseline AVX2 disassembly. Raw console logs and host telemetry were not included in this committed package; q8 candidate/parent uncertainty is visible in the paired interval.

## NEXT

B3 mask reduction is closed without a production change. Follow the active plan: at most one remaining bounded B1 f64 attempt, or choose a distinct B3 disjoint measurement only if it isolates an unmeasured cost. Do not continue B3 mask or normalizer microkernel variants without new whole-call evidence.
