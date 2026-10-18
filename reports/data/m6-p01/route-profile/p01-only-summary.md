# P01-only T1023 scalar synthesis route profile

The q1 T1023 profile used the retained P01 candidate source with P02
excluded. It measured one warmup, one uninstrumented synthesis, then one
instrumented synthesis. The instrumented map matched the uninstrumented map
bitwise. The committed M1 baseline profile provides the comparison; these
route timers are sums of worker CPU time rather than transform wall time.

| Route | M1 baseline CPU timer sum | P01-only CPU timer sum | Change |
|---|---:|---:|---:|
| Direct fused | 55.352 ms | 54.404 ms | −1.71% |
| General visitor | 191.910 ms | 69.752 ms | −63.65% |
| Shared transition | 13.420 ms | 39.293 ms | +192.79% |
| **Three-route total** | **260.682 ms** | **163.448 ms** | **−37.30%** |

The profile counted the same 225,005,258 committed route units. Visitor lanes
fell from 23,055,672 to 9,062,802; the scalar transition handled more of the
remaining work, with 1,912 completed transitions and no declines. The direct
fused count was unchanged. Counter enablement changed wall time by +0.60% in
the baseline run and +0.19% in the candidate run. This result supports the M1 interpretation that P01 reduces expensive general
visitor work by routing an ordinary scalar suffix through a scalar-only
transition. It agrees with the separate six-pair T1023 scalar q1 synthesis
ratio of 0.6647. It does not measure spin-1 behavior, replace the complete
control matrix, or accept P01. The T511 spin-1 default-link control remains
unresolved. The route counts, timer results, source patch and hashes are retained in this directory; raw test output remains local and untracked.
