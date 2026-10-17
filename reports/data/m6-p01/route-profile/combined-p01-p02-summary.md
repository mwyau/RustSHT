# Combined P01/P02 T1023 scalar synthesis route profile

The current uncommitted P01/P02 source snapshot produced this opt-in q1
profile. The test compares an uninstrumented map with a counter-enabled map
bitwise. Route timers are worker CPU-time sums, not transform wall time.

| Route | CPU timer sum | Count |
|---|---:|---:|
| Direct fused | 54.367 ms | 199,808,786 fused pairs |
| General visitor | 71.201 ms | 7,481,326 normal + 1,581,476 scaled lanes |
| Shared transition | 39.486 ms | 3,847,504 prefix lanes + 12,286,166 fused pairs |
| **Three-route total** | **165.054 ms** | **225,005,258 committed route units** |

The profile reports 1,926 transition calls, of which 1,912 completed and none
declined. This is supplementary to the P01-only profile. It does not isolate
P02 and is not an acceptance benchmark. The P01-to-P01+P02 six-pair screen is
the evidence for the P02 decision.
