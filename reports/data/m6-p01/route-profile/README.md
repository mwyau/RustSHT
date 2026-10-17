# P01/P02 candidate T1023 route profile

This opt-in test profiles the current uncommitted P01/P02 candidate at T1023,
q1. It is a route-accounting diagnostic, not an acceptance benchmark. The
test asserts that enabling counters preserves the complete output bitwise and
prints the per-route counts and CPU timer sums. The exact source diff against
the branch head, the test-only route-profile module, the test log and binary
hash are retained here. This first profile is a combined P01/P02 source snapshot; it does not isolate
P02. Its counters are summarized in
[`combined-p01-p02-summary.md`](combined-p01-p02-summary.md). The separate
six-pair P01-to-P01+P02 comparison remains the evidence that P02 coefficient
reuse has no material gain. A second profile from the retained P01-only source
snapshot, with P02 excluded, is summarized in
[`p01-only-summary.md`](p01-only-summary.md), with raw output and source hashes
in `p01-only-test.log` and the related `p01-only-*` files. Its exact changed
source files are retained under `p01-only-source/`. Compare either
profile with the committed baseline T1023 q1 row in
[`m6-current-baseline/route-profile/summary.csv`](../../m6-current-baseline/route-profile/summary.csv). Per-worker route timers are CPU-time sums and are not wall time. Two additional P01-only profiles cover q8 at T1023 and T2047. Their worker
timers, load balance, instrumentation overhead, executable hash, and raw
evidence are summarized in [`p01-only-q8-summary.md`](p01-only-q8-summary.md). These single-run diagnostics do not satisfy the P01 acceptance matrix. The profile's measured visitor decrease and transition increase are consistent
with P01 moving scalar work from the general paired visitor into its scalar
transition. They support the scalar synthesis target but do not resolve the
T511 spin-1 default-link regression and do not accept P01.
