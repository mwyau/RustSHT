# Current-source B1/B3 profile and candidate evidence

Parent source: `afd3df285dcb601371bb5b7ba1ec4fdc56ab1645`; pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Callgrind instruction references are diagnostics, not cycles.

## Findings

- The original q1 profiles put 95.9%/96.4% of B1/B3 f32 instruction references in Legendre blocks; FFT accounts for 1.8%/3.6%. Original B1/B3 f32 counts were 2.290B/2.258B Ir.
- The latitude-invariant coefficient-scale cache was rejected: B1/B3 counts rose to 4.190B/3.645B and the B1 whole-call screen regressed 1.56–1.76×.
- The compact hard-scale vector path is integrated in `47906d09`.
- The sparse fallback lane index is integrated in `a3cd8964`. See the [exact-source results](candidate-sparse-fallback-index-15pair/README.md). It improves B1/B3 19–28% against the parent across eight 15-pair cells, while absolute f32 parity remains a GAP.

## Current residual work

Exact parent/candidate q1 Callgrind counts for the sparse-index source are B1 1.465B→1.244B Ir and B3 1.992B→1.636B Ir. Next, profile the residual fused recurrence work and identify a materially different reduction in recurrence or exceptional-lane cost. Do not repeat the coefficient-cache or lane-scan transformations.

Historical full Callgrind and disassembly captures remain in local untracked experiment scratch; committed summaries retain source, binary and capture hashes where applicable.
