# Mixed scalar-analysis recurrence screen

The candidate fuses one mixed-scale coefficient accumulation with the
recurrence advance for the next degree pair. The checked lane normalization
and ordinary SIMD transition remain in place.

The Rust-only baseline was commit `8f2c597a`; the candidate is the source tree
containing this record. Both used Rust 1.99.0, release optimization,
`-C target-cpu=x86-64-v3`, and an AMD Ryzen 9 5950X. Each comparison used one
warmup and five timed calls. No DUCC build was rerun; use
[`scalar-analysis-stage-profile-2026-10-18.md`](../../scalar-analysis-stage-profile-2026-10-18.md)
for pinned DUCC context, whose phase boundaries differ.

## T720 phase screen

The test-only phase profile used GL, 721 latitude rings, 1443 odd longitude
samples, full `mmax=720`, and `phi0=0.37`. The existing ignored
`m5_analysis_phase_profile` test was temporarily narrowed to q1/q8 and given
an unmeasured warmup. Its sanitized source delta is
[`m5-phase-screen.patch`](m5-phase-screen.patch). The measured medians were:

| q | Legendre recurrence and accumulation, baseline → candidate | Full call, baseline → candidate |
|---:|---:|---:|
| 1 | 23.508 → 23.023 ms (-2.1%) | 35.233 → 34.250 ms (-2.8%) |
| 8 | 3.463 → 3.306 ms (-4.5%) | 6.191 → 5.902 ms (-4.7%) |

The profile times the complete Legendre stage, not the mixed blocks separately.
It supports a modest stage improvement but does not isolate the mixed route's
contribution.

## Full-call control screens

The reusable-plan `m1b_bench` screen used even-longitude T720 GL (721×1442,
full `mmax`) and reduced T300 GL (301×603, `mmax=150`). At T720 the q1 medians
were 31.116/31.040 ms and q8 medians were 5.566/5.628 ms for baseline/candidate.
The reduced T300 q1 comparison changed direction between paired runs. A q8
repeat measured 0.672/0.664 ms; the first pair had greater sample variation.
These end-to-end comparisons are host-sensitive and show no consistent
additional change across both workloads.

The screen does not establish parity with DUCC. The next performance check
should isolate the mixed-scale phase before further scalar-loop changes.
