# Native-f32 exact-zero suffix handoff — rejected on T720 numerical error

**Decision: reject the candidate.** Requiring a zero product to have an exact-zero operand did not change the transition routes on these fixtures and did not fix the handoff error. One-call q1 comparisons against exact parent `6db3f65a` produced relative L2 errors of `0.430165` full and `0.376613` reduced, with maximum absolute errors `0.716657` and `0.611738`. The suffix tests passed, but they do not cover the nonzero-exponent transition that fails here. No timing campaign was run.

## Provenance and method

- Exact RustSHT parent: `6db3f65aa139aeef7ba9a74b00ec01d3d5826fb4`; production transform source matches `54b166f2c0698f1c70e88f8679d12fa4376704fe`.
- Candidate: the rejected shared packet-suffix implementation plus a SIMD predicate accepting a zero product only when one operand is bitwise zero. The instrumented patch, including its temporary output and route counters, is preserved in [`exact-zero-operand-candidate-instrumented.patch.gz`](exact-zero-operand-candidate-instrumented.patch.gz). The raw patch SHA-256 is `5db7b02b9da5fc435da44d533afc91814ce6c1c7de360b08cb32023b3e5f83d8`; the compressed patch SHA-256 is `7da2d7d5f521becbf15710af5594842dd83563ba1f609dd6b4d667f4bf45c5bb`.
- Pinned DUCC is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. It was not built or timed in this diagnostic because the q1 numerical gate failed.
- Release build used `-C target-cpu=x86-64-v3`; the separate route-only binary additionally used `--cfg m6_profile_suffix`. Rustc was `1.99.1`. Route census ran one first transform per process on GL T720 f32 scalar synthesis (`phi0=0.23`, `ntheta=721`, full `mmax=720/nphi=1458`, reduced `mmax=180/nphi=378`). q1 used one fixed pinned core; q8 used the same set of eight physical cores across cells. Parent and candidate output captures used the same q1 affinity. The exact fixture hashes and per-worker route totals are in [`results.json`](results.json).

## Transition and suffix work

`pair_start` is where the candidate first reported that the existing suffix completed. These counts are reported route work from an instrumented binary, not evidence that the corrected state is numerically safe. The real-state replay below shows that some of these reported suffixes began with a nonzero recurrence value that became zero when its binary exponent was applied.

| Cell | Completed packets | Packets with nonzero exponent | Nonzero exponent lanes | First ready pair, median / p90 / max | Prefix visits before success | Reported prefix slots | Reported suffix slots | Reported suffix / active slots | Declines |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 29,188 | 8,833 | 30.26% | 2 / 26 / 340 | 231,982 | 3,711,712 | 88,765,136 | 55.07% | 0 |
| Full q8 | 28,135 | 11,058 | 39.30% | 2 / 26 / 340 | 234,336 | 3,749,376 | 86,183,800 | 53.46% | 0 |
| Reduced q1 | 7,950 | 1,942 | 24.43% | 3 / 32 / 340 | 85,630 | 1,370,080 | 37,948,296 | 46.73% | 0 |
| Reduced q8 | 7,570 | 2,350 | 31.04% | 4 / 33 / 340 | 85,630 | 1,370,080 | 35,891,784 | 44.20% | 0 |

The exact-zero and earlier normal-or-zero-result predicates completed the same number of packets with no declines. This rules out zero-product admission as the cause of the large error. The specific real-state failure is identified below.

## Real T720 state replay and corrected route count

At the first captured q1 full-mmax transition (`m=40`, `ring_start=0`, `pair_start=0`, 681 degree terms left), the eight recurrence exponents were `[-320, -320, -240, -240, -240, -240, -240, -160]`. Applying them to the nonzero current-state mantissas underflowed every lane to zero. The candidate fused suffix then accumulated eight zeros. Replaying the same suffix from the original mantissas and exponents through `multiply_complex_scaled_f32` produced nonzero values, up to `1.64e-8` real and `8.65e-9` imaginary; packet relative L2 error was `1.0`. The prefix sums were zero in this packet. This proves the observed failure: a corrected recurrence state can underflow even though multiplication by later coefficients recovers representable contributions.

An isolated guard that declines whenever a nonzero state scales to zero was then used only for a route recount. Across the four q/mmax cells, all suffixes that completed under this guard had zero exponents; no nonzero-exponent packet completed. Counts below are paired-accumulation lane slots and recurrence lane updates, not individual degree terms. They account for `46.62%`, `41.99%`, `41.10%`, and `37.09%` of the full/reduced active paired slots, respectively. No latency or whole-transform correctness run was made for this guarded diagnostic.

| Cell | Completed packets | Fused suffix paired-accumulation lane slots | Fused recurrence lane updates | Nonzero-exponent lanes | Share of active paired slots |
|---|---:|---:|---:|---:|---:|
| Full q1 | 28,269 | 37,624,520 | 37,398,368 | 0 | 46.62% |
| Full q8 | 26,647 | 33,884,672 | 33,671,496 | 0 | 41.99% |
| Reduced q1 | 7,589 | 16,700,328 | 16,639,616 | 0 | 41.10% |
| Reduced q8 | 7,164 | 15,074,152 | 15,016,840 | 0 | 37.09% |

The earlier table's nonzero-exponent completions were not safe transitions; its zero-as-safe state predicate admitted nonzero-to-zero underflow. The instrumented source diff is preserved in [`real-state-transition-candidate.patch.gz`](real-state-transition-candidate.patch.gz), and the compact route and replay evidence is in [`real-state-transition-results.json`](real-state-transition-results.json). The prior q1 full/reduced whole-transform comparison still establishes that the unguarded candidate is unacceptable; no timing campaign was run.

## Numerical result and decision

One first-transform q1 output per parent and candidate used the same deterministic fixtures, CPU affinity, compiler and ISA. Outputs were finite but not bitwise equal. Full relative L2/max absolute error was `0.430165` / `0.716657`; reduced was `0.376613` / `0.611738`. A longitude DFT showed the error is concentrated in the middle orders: full median per-mode relative error was near `1e-7` for `m=1..40`, then `0.36–0.58` for bins from `m=41..320`; reduced errors were similar across `m=41..180`. High polar orders `m=601..720` remained near `1e-6`. The real-state replay below identifies the specific nonzero-to-zero underflow failure.

The two original focused suffix unit tests passed; they exercise an exponent-zero handoff/decline and workspace reuse. A follow-up synthetic transition test with nonzero exponents `-80` and `+80`, a one-pair scaled prefix, and suffix start at pair 1 also passed against a scalar recurrence from the same rescaled states. That synthetic case did not cover the real `-160..-320` underflow captured above.

As a separate q1 isolation control, the suffix was disabled only for packets with any nonzero lane exponent. Full/reduced parent-relative relative L2 fell to `1.9147e-7` / `1.7901e-7`, with max absolute error `2.3842e-7` / `1.7881e-7`. Outputs were finite but not bitwise identical. This agrees with the real-state replay: nonzero-exponent handoffs are the unsafe route. The gate was not timed and is not a production candidate. Full hashes and values are in [`state-equivalence-results.json`](state-equivalence-results.json), and the reproducible temporary source is in [`state-equivalence-diagnostic-candidate.patch.gz`](state-equivalence-diagnostic-candidate.patch.gz).

The direct T720 checks used one transform and did not exercise repeated-call reset or adjoints. These results fail acceptance, so q8 timings, DUCC candidate timings and protected-cell timings were correctly not run.

**Next:** run the isolated f32-I/O/f64-internal T720 Legendre and whole-transform control at the current production source, because native-f32 exponent folding loses nonzero recurrence states on the actual handoff route. Keep production source unchanged until a new safe algorithm demonstrates whole-transform value.
