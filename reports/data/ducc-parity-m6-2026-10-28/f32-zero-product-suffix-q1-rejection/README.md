# Native-f32 zero-result suffix candidate — q1 rejected on numerical error

**Decision:** reject the candidate. The `normal-or-zero` product-result guard lets all state-ready suffix packets complete and improves the measured q1 timings, but its outputs differ substantially from the exact parent and DUCC. Relative L2 error is `0.4302` full and `0.3766` reduced; maximum absolute errors are `0.7167` and `0.6118`. Finiteness, repeat and workspace-reset checks passed, but this does not compensate for the numerical failure. No q8 or protected-cell timing was run.

## Provenance

- Exact parent: RustSHT `55f22fc7c1c882ac1e5daafee426e5aaf65ffc20` (production source `54b166f2c0698f1c70e88f8679d12fa4376704fe`).
- Candidate: the prior rejected suffix implementation plus a product-result check allowing normal or zero values; source diff SHA-256 `266e14080352473610b24fd0b9841bb68891d6e803633f3b05cd41344e880ddd`.
- Candidate patch: [`zero-result-candidate.patch.gz`](zero-result-candidate.patch.gz), SHA-256 `0bc5241d78726b3817a2aac5a9a425ac2c3d2884d03fbbd6932769a8031a557b`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Binary SHA-256: parent `566861000a5c3e978a288154e8cc65d79cac0da5768095bc961f3e13b480bd04`; candidate `0aef076e8204291cc14875167e2effd0a0cbea30ce5502c7da6eed69821bfba1`; DUCC `6df137897bcf9e7bba544ace6c7ca76ab1cbd3fd0af65ee992b338e569cddcba`.
- Rust release binaries used `-C target-cpu=x86-64-v3`; DUCC used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with the driver built using `-fno-fast-math -ffp-contract=fast`. Host was AMD Ryzen 9 5950X, rustc 1.99.1 / LLVM 23.1.1, GCC 15.2.0.
- GL T720 f32 scalar synthesis, `phi0=0.23`, `ntheta=721`; full M720 / `nphi=1458`, reduced M180 / `nphi=378`. Three interleaved fresh-process parent/candidate/DUCC q1 triplets used identical deterministic fixtures, reusable plan/workspace and the same warm-call protocol. Sanitized timing samples, correctness checks and fixture hashes are retained in [`q1-results.json`](q1-results.json), SHA-256 `ba90e7d69e88b108a8c5e19c8a3a6783e337e8928fa695070e2c6a3e9241f9be`.

## Timings and errors

| GL T720 f32 scalar synthesis q1 | Parent ms | Candidate ms | DUCC ms | Candidate/parent paired median | Candidate/DUCC paired median | Candidate/parent relative L2 | Candidate/parent max abs |
|---|---:|---:|---:|---:|---:|---:|---:|
| Full mmax | 695.805 | 461.653 | 18.827 | 0.6638 | 24.676× | 0.4302 | 0.7167 |
| Reduced mmax | 207.761 | 125.700 | 8.255 | 0.6050 | 15.232× | 0.3766 | 0.6117 |

The candidate is faster in these q1 measurements, but they are invalid for performance acceptance because numerical behavior failed. Each worker returned finite output and passed repeat/reset checks. Full process-pair data, including candidate-versus-DUCC errors, is in the JSON. No stage timer or Callgrind profile was collected.

## What the census did and did not show

The prior route census showed that allowing a zero-valued product **result** admitted all previously declined packets. It did not prove the product was mathematically zero: a floating-point multiply can round a nonzero product to zero. The candidate's large output error is consistent with such an underflowed product reaching the fused multiply-add path, but no lane-by-lane operand classification was collected, so that cause is not yet proven. The strict candidate had small parent-relative error; the change in the result predicate is the only arithmetic policy difference in this screen.

Do not reuse the `normal-or-zero-result` predicate. The next bounded route test must accept a zero result only when at least one operand is exactly zero, while continuing to reject subnormal and nonfinite products; then verify route coverage and numerical output before any timing campaign.
