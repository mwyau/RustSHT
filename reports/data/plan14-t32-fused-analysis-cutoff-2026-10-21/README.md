# T31/T32 fused-analysis cutoff A/B data

Six q1 cases are retained under `baseline-*` and `candidate-*`; each JSON
contains its full 30-batch process data, numerical checks, fixture hashes,
dispatch and host samples. `comparison.csv` joins the cases, and
`phase-summary.csv` contains the T32 full-order Rust phase probes. DUCC is
pinned to 0.42.0 commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

The baseline code is commit `c2f89215b24668503b3e1f4abf0e36ab26c3ac96`.
The candidate adds exactly the f64 floor and test changes later committed
as `de4c6569`. Both raw benchmark metadata records a dirty worktree: the
baseline included a report-only wording edit; the candidate included that
same edit plus the threshold/test diff. No other production-source changes
were present. See the parent report for flags and timing protocol.
