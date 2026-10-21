# T63/T64 fused-analysis cutoff A/B data

Baseline and candidate native benchmark captures are split by longitude
shape under `baseline-*` and `candidate-*`. Each JSON contains all timed
batches, numerical comparisons, fixture hashes, dispatch and host samples.
`comparison.csv` joins the four q1 cases. `phase-summary.csv` records the
test-only calibrated Rust stage profiles. DUCC is pinned to 0.42.0 commit
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

The baseline is clean `f63c83d5e5a7fe9e155685378cf946a351b06179`. Candidate
native transforms were measured from that source plus exactly the two-file
cutoff/test diff later committed as
`7e51ae43b5cca74fd716511493df3b20262dad13`; candidate raw JSON therefore
records a dirty worktree. No other source changes were present at capture.
The report contains flags, interleaving, affinity and interpretation.
