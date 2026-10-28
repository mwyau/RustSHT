# Native-f32 exact-zero suffix handoff — rejected on T720 numerical error

**Decision: reject the candidate.** Requiring a zero product to have an exact-zero operand did not change the transition routes on these fixtures and did not fix the handoff error. One-call q1 comparisons against exact parent `6db3f65a` produced relative L2 errors of `0.430165` full and `0.376613` reduced, with maximum absolute errors `0.716657` and `0.611738`. The suffix tests passed, but they do not cover the nonzero-exponent transition that fails here. No timing campaign was run.

## Provenance and method

- Exact RustSHT parent: `6db3f65aa139aeef7ba9a74b00ec01d3d5826fb4`; production transform source matches `54b166f2c0698f1c70e88f8679d12fa4376704fe`.
- Candidate: the rejected shared packet-suffix implementation plus a SIMD predicate accepting a zero product only when one operand is bitwise zero. The instrumented patch, including its temporary output and route counters, is preserved in [`exact-zero-operand-candidate-instrumented.patch.gz`](exact-zero-operand-candidate-instrumented.patch.gz). The raw patch SHA-256 is `5db7b02b9da5fc435da44d533afc91814ce6c1c7de360b08cb32023b3e5f83d8`; the compressed patch SHA-256 is `7da2d7d5f521becbf15710af5594842dd83563ba1f609dd6b4d667f4bf45c5bb`.
- Pinned DUCC is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. It was not built or timed in this diagnostic because the q1 numerical gate failed.
- Release build used `-C target-cpu=x86-64-v3`; the separate route-only binary additionally used `--cfg m6_profile_suffix`. Rustc was `1.99.1`. Route census ran one first transform per process on GL T720 f32 scalar synthesis (`phi0=0.23`, `ntheta=721`, full `mmax=720/nphi=1458`, reduced `mmax=180/nphi=378`). q1 used one fixed pinned core; q8 used the same set of eight physical cores across cells. Parent and candidate output captures used the same q1 affinity. The exact fixture hashes and per-worker route totals are in [`results.json`](results.json).

## Transition and suffix work

`pair_start` is the first paired degree where both recurrence states could be rescaled into the checked native-f32 range and the existing suffix completed. Prefix and suffix counts below are useful degree–latitude contributions; a final unpaired degree is counted once. `prefix_not_ready_visits_before_first_success` counts earlier recurrence visits before each completed packet's first ready transition. These are work counters from an instrumented binary, not elapsed time.

| Cell | Completed packets | Packets with nonzero exponent | Nonzero exponent lanes | First ready pair, median / p90 / max | Prefix visits before success | Executed prefix terms | Fused suffix terms | Suffix / active terms | Declines |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 29,188 | 8,833 | 30.26% | 2 / 26 / 340 | 231,982 | 3,711,712 | 88,765,136 | 55.07% | 0 |
| Full q8 | 28,135 | 11,058 | 39.30% | 2 / 26 / 340 | 234,336 | 3,749,376 | 86,183,800 | 53.46% | 0 |
| Reduced q1 | 7,950 | 1,942 | 24.43% | 3 / 32 / 340 | 85,630 | 1,370,080 | 37,948,296 | 46.73% | 0 |
| Reduced q8 | 7,570 | 2,350 | 31.04% | 4 / 33 / 340 | 85,630 | 1,370,080 | 35,891,784 | 44.20% | 0 |

The exact-zero and earlier normal-or-zero-result predicates completed the same number of packets with no declines. Thus no attempted suffix on these fixtures depended on accepting a zero product produced from two nonzero operands. This rules out that predicate as the explanation for the large error; it does **not** identify which part of the scaled-state handoff is wrong.

## Numerical result and decision

One first-transform q1 output per parent and candidate used the same deterministic fixtures, CPU affinity, compiler and ISA. Outputs were finite but not bitwise equal. Full relative L2/max absolute error was `0.430165` / `0.716657`; reduced was `0.376613` / `0.611738`. A longitude DFT showed the error is concentrated in the middle orders: full median per-mode relative error was near `1e-7` for `m=1..40`, then `0.36–0.58` for bins from `m=41..320`; reduced errors were similar across `m=41..180`. High polar orders `m=601..720` remained near `1e-6`. This points toward the scaled-prefix transition route, but the specific defect is not proven.

The two focused suffix unit tests passed. They exercise an exponent-zero handoff/decline and workspace reuse; they do not establish the scaled, nonzero-exponent state correction used by the T720 candidate. The direct T720 check used one transform and did not exercise repeated-call reset or adjoints. These results fail acceptance, so q8 timings, DUCC candidate timings and protected-cell timings were correctly not run.

**Next:** add a direct nonzero-exponent transition equivalence check that starts from the shared recurrence state and compares the corrected suffix with the existing scaled contribution path before attempting another whole-transform candidate. Keep the current production source unchanged until that check passes.
