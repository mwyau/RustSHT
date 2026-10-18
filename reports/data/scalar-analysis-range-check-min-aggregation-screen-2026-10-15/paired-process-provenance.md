# Paired process provenance

The five-pair f64 GL STANDARD scalar-analysis screen used one warm-up and five
timed calls per process at T511 and T1023. The T1023 q8 repeat added five
pairs. Timing samples and summarized peak RSS remain in the adjacent result
tables.

- Baseline source commit: `26449b5`.
- Baseline binary SHA-256: `6b465745d6361ea975ba3b1ba9ce569ec5031aa16bd8330d500d5065975da4f6`.
- Candidate binary SHA-256: `044bbbf0a64079bd746dd12d278bbb06d0fd1f9b91044a7e06b18db2dc3224dd`.
- Workload: f64 scalar GL STANDARD analysis, full `mmax`, T511/T1023, q1/q8.

Raw process, affinity and host-load captures remain local and untracked.
