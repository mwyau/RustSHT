# Native-f32 reflected spin-1 accumulation leaf — rejected

This isolated candidate tested whether GL T720 spin-1 synthesis could reuse the existing hemisphere scheduler while evaluating both scalar channels with the shared f32 paired recurrence.

## Provenance and method

- Exact source parent: `5d6e5e34f92ae321d5754d120258e2fa63c79cb3`.
- Candidate source patch: `candidate.patch`, SHA256 `21bacf263d1cb93c5fcc40bb4dfcc425fb19947a3e252740bcc886c4e6a4e953`; `git apply --check` passed against the exact parent. The candidate is not promoted.
- Pinned reference: DUCC 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host/compiler: Ryzen 9 5950X, x86_64; rustc 1.99.1 / LLVM 23.1.1; Rust release `parallel`, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; selected Pulp V3 with eight f32 lanes. DUCC used g++ 15.2.0, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its driver used `-fno-fast-math`.
- q1 used one pinned logical CPU; q8 used eight distinct physical cores. Parent, candidate, and DUCC shared each cell's affinity. Three interleaved pairs were run for an intentionally decisive screen. Plans/workspaces were set up before timed warm calls; the timed public f32 calls include the whole transform. Inputs, executable hashes, and per-pair records are in `fixture-hashes.json`, `binary-hashes.txt`, and `results-f32.csv`.

## Whole-call result

Times are medians in milliseconds. Candidate/parent and candidate/DUCC are medians of paired ratios; bracketed ranges are the observed three-pair minimum and maximum.

| GL T720 spin-1 synthesis | Parent / candidate / DUCC ms | Candidate/parent paired median [range] | Candidate/DUCC paired median [range] | Candidate vs parent / DUCC relative-L2 | Status |
|---|---:|---:|---:|---:|---|
| Full M720 q1 | 1496.584 / 1577.282 / 40.712 | 1.057 [1.032, 1.059] | 38.743 [37.931, 38.987] | 1.660e-4 / 2.136e-4 | GAP; q1 regressed |
| Reduced M180 q1 | 602.805 / 592.610 / 17.553 | 0.985 [0.967, 0.985] | 33.844 [32.250, 33.844] | 2.344e-4 / 1.949e-4 | GAP; no material gain |
| Full M720 q8 | 270.681 / 596.808 / 8.051 | 2.196 [2.164, 2.219] | 74.735 [69.852, 88.326] | 1.664e-4 / 2.140e-4 | GAP; rejected |
| Reduced M180 q8 | 108.633 / 340.934 / 2.715 | 3.145 [2.801, 3.210] | 125.054 [119.888, 131.682] | 2.350e-4 / 1.958e-4 | GAP; rejected |

All workers were finite, repeat-consistent, and consistent after output poisoning/reset. The candidate's DUCC relative-L2 and maximum absolute errors were `2.136e-4` / `1.5664e-3` full and `1.949e-4` / `1.3469e-3` reduced. Parent relative-L2 error was lower in every cell (`1.426e-4` full, `1.415e-4` reduced). This performance failure and accuracy regression ended the candidate; no broader analytic/adjoint/extreme matrix was run for it.

## Route and cause

The focused release route-counter test passed. Every attempted block used the new leaf: full q1/q8 `2841/2841` and `8050/8050`; reduced q1/q8 `905/905` and `2685/2685`; all decline counts were zero. Thus the q8 regression is not a dormant-route or fallback-activation issue.

Matched q1 full Callgrind measured `22,509,083,997` parent instruction references and `15,097,441,690` candidate references (32.9% fewer Ir). Ir is not cycles: despite this reduction, candidate wall time was 5.7% slower. Exclusive candidate cost was dominated by `multiply_component_scaled_with_normal_factor_f32` (5.853B Ir, 38.77%) and `multiply_complex_scaled_f32` (2.294B, 15.20%). The shared paired visitor's packet accumulator was 2.067B (13.69%), while the successful reflected leaf itself was about 0.069B (0.46%). The normal-range bulk path emits packed AVX2/FMA, but scaled packets still process individual coefficient/ring lanes through scalar scaling helpers. The current leaf therefore spends most of its work in scaled accumulation and gets especially poor q8 scaling.

Sanitized exclusive profiles and executed candidate assembly are preserved as `callgrind-*-self.txt.gz`, `callgrind-*-q1-full.callgrind.gz`, and `candidate-paired-recurrence.asm.gz`. Assembly has packed and scalar FMA variants, with scalar multiply/scale fallback. Do not interpret the Ir decrease as a timing win.

## Protected f64 control and checks

Three interleaved GL T1023 M1023 f64 spin-1 synthesis pairs produced q1 parent/candidate medians of `89.877/88.095 ms` (paired ratio `0.980`, observed range `0.927–1.011`) and q8 medians `21.121/19.182 ms` (paired ratio `1.002`, range `0.784–1.120`). Parent and candidate output hashes were bitwise equal; timing is too variable to claim a meaningful improvement or regression. f64 and f32 worker hashes are in `binary-hashes.txt`.

`cargo fmt --check`, release `cargo check --features parallel --lib`, and the focused route test passed. The test also checked repeated calls and poisoned-output reset. `candidate.patch` is the complete recoverable source change; no production code was changed by this experiment.

## Decision and next experiment

Reject this reflected f32 spin-1 leaf. Do not broaden its route or rerun it unchanged. Callgrind identifies the next bounded hypothesis: preserve exact scaled recurrence state, but use the existing checked `try_accumulate_f32_scaled_synthesis_packet` helper to accumulate complete safe packets across the even/odd coefficient rows; retain scalar exact handling for tails or unsafe packets and use transactional decline if a full packet cannot be proven safe. This targets the measured dominant scaled-helper work while keeping the shared recurrence and protected f64 routes intact. First require unit/range checks and then repeat the whole T720 parent/candidate/DUCC screen with route counters.

## Reproduction inputs

`harness/` preserves the matched benchmark runner, both DUCC C++ worker sources and CMake files, the temporary Callgrind instrumentation patch, and the deterministic f64 T1023 fixture generator. SHA256 values are recorded in `harness/SHA256SUMS`. The runner uses CPU `0` for q1 and CPUs `0,2,4,6,8,10,12,14` for q8 on this host; q8 CPUs are distinct physical cores. Rust worker compilation uses the exact source parent plus `candidate.patch`; DUCC is built from the pinned revision. Fixture hashes and final binary hashes remain in the files above.
