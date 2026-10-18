# Paired process provenance

The five-pair f64 GL STANDARD scalar-analysis screen used one warm-up and five
timed calls per process at T511 and T1023. The q8 repeat added five pairs at
each size. Timing samples and summarized peak RSS remain in the adjacent
result tables.

- Baseline source commit: `387ef37`.
- Baseline binary SHA-256: `6b465745d6361ea975ba3b1ba9ce569ec5031aa16bd8330d500d5065975da4f6`.
- Candidate binary SHA-256: `34d0c36702eba92a7baf6a6c32aa1205b8ec55307d71f1d436ec1af6898e1fb8`.
- Workload: f64 scalar GL STANDARD analysis, full `mmax`, T511/T1023, q1/q8.

Raw process, affinity and host-load captures remain local and untracked.
