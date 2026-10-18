# Exponent-epoch prefix component profile, results

The single T2047 q1 profile passed its route test and produced a map bitwise
equal to the unprofiled warm-up map. The profile is a route diagnostic; it has
one timed transform and no timing-stability interval.

The exact source reconstruction, test-only patch, binary, workload and
execution rules are in the [v1 protocol](exponent-epoch-prefix-component-profile-v1-protocol.md).
Raw process, frequency, affinity and GNU-time output remains local.

## Route timing and work

| Measurement | Value | Scope |
|---|---:|---|
| Full `synthesis_into` call | 1470.183 ms | One profiled call |
| Fast transition, including SIMD suffix | 1311.730 ms | Summed across completed tiles |
| Ordinary SIMD suffix | 547.002 ms | Nested in the fast transition |
| Pre-suffix transition estimate | 764.727 ms | Transition minus suffix |
| Scaled fallback | 12.484 ms | Summed across fallback calls |
| Coefficient-safety scans | 78.118 ms | Test timer total |
| Peak RSS | 299,968 KiB | Process high-water mark |

The route counters recorded 23,201 completed tiles, no declined or empty
tiles, 110,608,856 prefix pair-lanes, 1,622,437,750 suffix pair-lanes and
1,331,458 fallback pair-lanes. The fallback lanes were polar. Timer scopes
overlap: the suffix is inside the transition, while fallback and coefficient
scans are inside tile work. Do not add those values to the full-call time.
The pre-suffix estimate was 52.0% of the profiled call. The test binary
includes route counters and the component timers, so its full-call time is
not a candidate-versus-baseline measurement.

## Sampled prefix components

The profile timed one degree pair in each 64-pair interval, with the sample
offset varied by harmonic order. It recorded 16,313 sampled pairs covering
1,728,964 lane visits and 10,948 exponent changes.

| Sampled operation | Time (ms) |
|---|---:|
| Factor checks, products and scaled complex accumulation | 7.038 |
| Recurrence updates | 0.726 |
| Range checks and recurrence normalization | 3.247 |
| Exponent-change scan and accumulator rebase | 1.310 |
| Transition eligibility checks | 0.378 |
| Prefix materialization at transition | 0.097 |

The sampled totals cover selected iterations only. Accumulation took the
largest measured sample total, followed by normalization. Variable prefix
lengths and sampling overhead prevent extrapolation to the whole transition.
The profile supports measuring a distinct mechanism that reduces the scalar
factor and complex-accumulation work while preserving the existing scaled
range checks and fallback. It does not validate such a change.

## Host and numerical checks

CPU PSI `some avg10` was `0.00%` before and `0.32%` after the process. The
single worker remained on its requested logical CPU. Manual review found no
competing CPU-intensive build or benchmark and no cgroup throttling. The
process recorded 414 involuntary context switches; frequency and thread
captures were reviewed locally.

The ignored route test passed and asserted bitwise equality with its
unprofiled warm-up map. The complete workload was f64 GL scalar synthesis at
T2047, full `mmax`, `2048×4096`, `phi0=0.37`, and q1. The candidate remains
isolated. The next performance experiment requires a concrete factor or
accumulator mechanism and a source-matched whole-transform screen under the
fixed q1 protocol.
