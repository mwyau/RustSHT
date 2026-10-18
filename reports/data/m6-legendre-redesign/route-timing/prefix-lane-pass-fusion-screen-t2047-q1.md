# Prefix normalization and accumulation fusion T2047 q1 screen

## Result

The one-pair screen passed its fixed timing and numerical gates. The fused
process median was 1,235.878 ms versus 1,392.910 ms for the scaled-prefix
accumulator, an 11.27% reduction in full `synthesis_into` latency. Both
five-call series met the unchanged CV and first-three/final-two drift limits.
The output maps were bitwise identical. This result authorizes the separate
q1 validation matrix specified by
[protocol v1](prefix-lane-pass-fusion-screen-t2047-q1-protocol-v1.md); it does
not accept the kernel.

The comparison is between two isolated tile-128 candidates, not against the
clean production baseline. It did not time DUCC or SHTns. The accumulator
baseline's prior T2047 map comparison against DUCC 0.42.0 had relative-L2
error `3.015e-13` and maximum absolute error `1.110e-12`. The new map hash is
identical for the accumulator and fused processes in this screen. This is
consistent with the prior numerical result, but is not a new direct DUCC
timing or comparison.

## Frozen variants and workload

The accumulator source has `synthesis_fast.rs` SHA-256
`edc483d8670bfc6479f8dbacbd8aeb81e40a3636f5d556762b7905f495c9a9e0`. The
fused source applies patch SHA-256
`fe8c9cc2d049e2398cd731183f3b9a388f3db1061317f1766bbd3fbd30c4e0bb`; its
`synthesis_fast.rs` and `scalar.rs` SHA-256 values are
`eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407` and
`961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
Both binaries used the same harness, SHA-256
`4d76eeed1909adf665e8ec260544030183eec5bc42bf7cef53e836f40fdda364`, Rust
1.99.0 release settings, and the deterministic coefficient generator in the
protocol. The baseline and fused harness binary hashes are
`423e707c164f36bf80d53077e1af62623a1a503c5137e6f7afdbb394e3d03214` and
`40e63f7fd27998a8e50e399c03095bcee8109d6c0d705f9b50e74872c7001910`.

The workload was GL f64 scalar synthesis at T2047 on a `2048x4096` grid,
full `mmax=2047`, `phi0=0.37`, and q1. Each fresh process ran one warm-up and
five timed complete transformations. The plan, workspace, coefficients, and
output allocation were outside each timed interval. The 64 MiB output files
were kept locally; both had SHA-256
`03a5e1e7a3d0cd05663905bb962293561ac7fcb4a8cf716791efa768ab7fb1c8`.

## Timing and memory

| Variant | Samples (ms) | Median (ms) | CV | Drift | Peak RSS (KiB) |
|---|---|---:|---:|---:|---:|
| Scaled-prefix accumulator | 1392.910, 1383.390, 1375.874, 1399.263, 1395.448 | 1392.910 | 0.615% | 1.010% | 363,936 |
| Fused normalization and accumulation | 1234.862, 1237.215, 1236.864, 1235.878, 1234.118 | 1235.878 | 0.095% | -0.151% | 363,912 |

The process-level candidate / accumulator ratio was `0.887263`. The two
processes used the same single-logical-CPU affinity and one Rayon worker. The
fixed PSI gate passed without a warning-band reading. The cgroup recorded no
CPU throttling; pre/post checks found no competing CPU-intensive workload.
CPU-frequency variation was reviewed and the exact transient readings remain
local. Both within-process series remained stationary under the fixed timing
rules. Raw process, PSI, frequency, affinity, context-switch, GNU-time, and
host-state captures remain local and untracked.

## Mechanism and next step

The fused source normalizes a recurrence lane immediately before that lane's
scaled prefix contribution is accumulated. It uses the same lane normalizer
and keeps the accumulator transactional. No prefix contribution is skipped.
The T2047 route counter recorded 111,390,956 prefix lane-pairs versus
110,444,000 for the accumulator because some tiles remain in the scalar route
for one additional pair. The lower full-transform latency despite that added
work supports the two-pass reduction as a useful mechanism.

Focused fused-kernel and lane-normalization equivalence tests passed with all
features and without default features. The T511/T1023/T2047 analytic and
selected DUCC scalar controls across GL, CC, and F1 passed, as did the T1023
extreme-coefficient fallback control. The source remains experimental. The
next step is a matched six-pair q1 comparison against the clean baseline at
T1023 and T2047, followed by the T511 control. Direct DUCC accuracy and timing
must be recorded there. q8, spin-1 regression, and production routing remain
gated on those results.
