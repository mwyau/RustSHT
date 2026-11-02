# B3 scaled-prefix profile and split-channel candidate — 2026-11-02

This package profiles the exact `cc60dbdc386fb04e8571966c38a7e65db86605de` B3 f64 route and records one rejected candidate. It changes no production source. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Profile

Test-only timers distinguish ordinary dispatch, scaled dispatch, scaled prefix and the ordinary suffix after transition. Three uninstrumented whole-call samples per q1 cell showed 36.66–37.87 ms (full) and 15.00–15.09 ms (reduced); instrumentation measured 37.24 and 15.08 ms respectively. q8 route timings are worker-summed and must not be interpreted as wall contributions. The profile binary, CPU, flags and fixture hashes are in `scaled-route-profile.json` and `PROVENANCE.json`.

| Geometry | Scaled blocks | Ordinary blocks | Scaled-prefix steps | Scaled blocks reaching ordinary | Non-normal seed lanes |
|---|---:|---:|---:|---:|---:|
| Full | 599 | 2,980 | 27,332 | 577/599 | 9,442/26,204 (36.0%) |
| Reduced | 132 | 954 | 16,916 | 110/132 | 1,883/7,471 (25.2%) |

Scaled prefix accounts for 69.1% of scaled-dispatch q1 time at full order and 83.2% at reduced order. Scaled dispatch is 32.5% and 31.6% of ordinary-plus-scaled dispatch time respectively. These are instrumented q1 route timings, not inclusive stage totals. PMU events were unavailable; q8 values are per-worker sums.

The exact uninstrumented parent binary (`23bea81177ba96f2077293499514a15f6d8aaf6b3f517b948fd2f569333cdc2d`) emits the active `FusedCheckedScaledAnalysisPairChannels<true>` AVX2 leaf with a 2,728-byte `sub %rsp` allocation and 200 YMM stack references. The ordinary checked weighted leaf has a 632-byte allocation and 5 YMM stack references. Disassembly excerpts for both symbols are preserved as [`scaled-leaf-v3.asm.gz`](scaled-leaf-v3.asm.gz) and [`ordinary-leaf-v3.asm.gz`](ordinary-leaf-v3.asm.gz). This identifies substantial scaled-path register pressure, but does not prove spills are the only cause or estimate a whole-call saving.

## Rejected candidate

The isolated `rustsht_b3_scaled_channels_sequential` candidate ran two per-channel scaled recurrences sequentially on scaled blocks. It attempted to reduce live accumulator state but duplicated recurrence work. Three interleaved parent/candidate/DUCC pairs found a slowdown in every cell:

| Cell | Candidate / parent (95% CI) | Candidate / DUCC |
|---|---:|---:|
| Full q1 | 1.207 [1.181, 1.233] | 1.502 |
| Full q8 | 1.191 [1.156, 1.228] | 1.474 |
| Reduced q1 | 1.309 [1.301, 1.317] | 1.668 |
| Reduced q8 | 1.272 [1.208, 1.339] | 1.622 |

The candidate is rejected. Repeat/reset checks passed; candidate-to-parent relative L2 was `4.28e-17` full and `6.66e-17` reduced. Accuracy was unchanged within the recorded comparison. Do not promote this route.

## Reproduction and contents

- `route-profile-and-candidate.patch` contains the test-only route instrumentation and cfg-gated candidate.
- `candidate.patch` contains only the candidate source change.
- `scaled-route-profile.json` contains route counts, q1/q8 timing observations and lane census.
- `results.json` contains per-run times, paired ratios, fixture and binary hashes, checks, CPU placement and build flags.
- `PROVENANCE.json` binds the artifacts to source, fixture, patch, benchmark binary and disassembly hashes. The disassembly was extracted from the exact uninstrumented parent binary recorded in `results.json` using `nm -S -n -C` and `objdump -d -C`.

The bounded next diagnostic, if B3 work resumes, is a scaled-prefix-only accumulator microbenchmark that preserves one shared recurrence and changes only accumulator live state. The current data supports investigating that leaf; it does not justify another full kernel candidate without an isolated helper win. No production change is warranted by this package.
