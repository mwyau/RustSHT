# Prefix lane-pass fusion q1 protocol v2

## Scope and version change

Protocol v2 starts with a baseline-only T1023 q1 stability diagnostic. It
does not reuse or reinterpret the invalid T1023 process under v1. The old
sample remains historical evidence. V2 keeps the v1 workload, source and
binary identities, five measured calls per backend, numerical limits, CPU
pressure gates, CV and drift limits, six-pair decision rule, and stop-on-first
invalid process rule. It changes only the fixed number of calls made before
each measured series and adds two identical-baseline control processes
before candidate acquisition.

Inspection of the frozen v1 harness found that `backend=all` invokes each
backend once for its numerical comparison before `report()` calls
`time_runs()`. `time_runs()` then invokes it once more before collecting the
five timing samples. Thus each backend already received two untimed calls
before measurement, although v1 recorded only the latter as its warm-up. V2
makes the sequence explicit: one correctness call, three additional fixed
warm-up calls, the existing one warm-up call, then five measured calls per
backend. This gives five untimed transformation calls per backend. No sample
is selected or removed based on observed timing.

The warm-up count is fixed for this entire protocol. It is motivated by the
prior q8 baseline diagnostic, where one untimed transform did not prevent two
early slow calls in one process; it is a prospective diagnostic choice, not a
claim that five calls guarantee stability. The unchanged timing gates decide
whether acquisition may continue.

## Baseline-only diagnostic gate

Run exactly two fresh processes using the same clean baseline extension,
DUCC, SHTns, harness patch, and T1023 GL f64 scalar `STANDARD` synthesis
configuration. Use q1, full `mmax=1023`, `phi0=0.37`, a `1024x2048` grid,
`scope=into`, `runs=1`, and `backend=all`. Do not load or invoke the candidate
extension during this diagnostic. Each process runs the prescribed five
untimed calls followed by five timed calls for each backend.

Review each complete process before starting the next. Both processes must
pass every backend's population CV limit of 5%, first-three/final-two median
drift limit of 5%, numerical checks, preflight and postflight CPU pressure
gates, and interference review. Retain every sample. Do not retry, replace,
or omit a process. If either control fails, stop this experiment and report
the failure; do not run the candidate matrix under v2.

These two controls are diagnostics only. They are not candidate pairs and
are excluded from the six-pair acceptance calculations.

## Candidate matrix after both controls pass

Only after both baseline controls pass may the frozen candidate matrix begin.
Use six fresh-process baseline/candidate pairs at T1023, alternating order
baseline/candidate then candidate/baseline. Apply the same five untimed and
five measured calls per backend to every process. Stop at the first invalid
process. Continue to T2047 only if every T1023 process passes; run the T511
scalar control only if both high-resolution cells pass. q8 and spin-1
regression measurements remain unauthorized until the q1 decision and T511
control pass.

The acceptance thresholds and six-pair bootstrap rule are unchanged from
[protocol v1](prefix-lane-pass-fusion-q1-matrix-t2047-t1023-t511-protocol-v1.md).
In particular, retain population CV at most 5%, first-three/final-two median
drift at most 5%, the 2.00% preflight/postflight PSI rejection limit, the
0.50–2.00% warning band, and all original numerical limits. Clear evidence
of interference still invalidates a process below the PSI limit. Exact PSI,
frequency, placement, context-switch, and process captures remain local and
untracked; committed reports record gate outcomes and scientific measurements.

## Frozen harness and binaries

- Baseline extension SHA-256:
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Candidate extension SHA-256:
  `70f99288f4b6cf64a462b285b6ebfac149d1ab1514871c1c28d8db2a77029081`.
- Harness base SHA-256:
  `1f1bdfa8f2cf886db3874b2ae9160d9c68f89f0a4026a19bc9ada47948606fb8`.
- Apply [`prefix-fusion-explicit-warmup-v2.patch`](prefix-fusion-explicit-warmup-v2.patch)
  to the frozen harness. Set `RUSTSHT_BENCH_EXTRA_WARMUPS=3` for every
  measured backend series. Keep all binaries built before acquisition and
  record the patched harness and resulting script hashes in the result
  report.
- DUCC and SHTns revisions, library identities, release settings, affinity
  policy, and workload definition remain those in protocol v1.

Raw stdout, process and thread-placement captures, affinity, frequency,
context-switch, PSI, cgroup, and GNU-time details remain outside the
repository. Commit structured numerical and timing evidence, aggregate peak
RSS, gate outcomes, and conclusions only.
