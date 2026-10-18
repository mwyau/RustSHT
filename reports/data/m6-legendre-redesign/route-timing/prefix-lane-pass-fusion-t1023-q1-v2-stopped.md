# Prefix lane-pass fusion T1023 q1 v2 stopped

## Decision

The six-pair T1023 cell stopped at the baseline process in pair 6. Its
SHTns-adapter timing series failed the fixed CV limit, so pair 6 has no valid
baseline/candidate ratio. The candidate process that ran first in pair 6
passed. Protocol v2 requires stopping at the first invalid process; no retry,
replacement, T2047, T511, q8, or spin-1 measurement was run. The candidate
remains experimental and unaccepted.

## Frozen configuration and source

- GL f64 scalar `STANDARD` synthesis, `1024x2048`, full `mmax=1023`,
  `phi0=0.37`, q1, caller-provided output, one process per variant.
- Both variants used five untimed transformations per backend, followed by
  five measured calls. Each pair followed the v2 alternating process order.
- Baseline extension SHA-256:
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Candidate source SHA-256 values:
  `synthesis_fast.rs` `eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407`;
  `scalar.rs` `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
  Candidate extension SHA-256:
  `70f99288f4b6cf64a462b285b6ebfac149d1ab1514871c1c28d8db2a77029081`.
- Patched harness SHA-256:
  `61b640657990bebf80a1d19a1cd4cd0e99e2adcb7234161ea9b984b6971669ab`.
  Reference revisions and library identities are frozen in
  [protocol v1](prefix-lane-pass-fusion-q1-matrix-t2047-t1023-t511-protocol-v1.md).
- Every process passed the preflight and postflight CPU-pressure rejection
  gates. Warning-band readings occurred; exact system captures remain local.
  Monitoring found no clear external CPU-intensive workload or cgroup
  throttling. Worker placement stayed within the fixed affinity mask.

## Paired RustSHT synthesis medians

| Pair | Order | Baseline median (ms) | Candidate median (ms) | Candidate/baseline | Status |
|---:|---|---:|---:|---:|---|
| 1 | Baseline, candidate | 293.166 | 158.515 | 0.540700 | Complete, valid |
| 2 | Candidate, baseline | 293.393 | 158.600 | 0.540572 | Complete, valid |
| 3 | Baseline, candidate | 293.988 | 160.189 | 0.544883 | Complete, valid |
| 4 | Candidate, baseline | 294.936 | 158.619 | 0.537808 | Complete, valid |
| 5 | Baseline, candidate | 308.315 | 160.128 | 0.519365 | Complete, valid |
| 6 | Candidate, baseline | 294.932 | 160.834 | Not computed | Baseline reference failed timing gate |

The first five complete ratios have a descriptive median of `0.540572`, a
45.94% reduction in T1023 q1 RustSHT synthesis time. They do not satisfy the
frozen six-pair acceptance rule, so no bootstrap interval or candidate
decision is reported. The valid candidate processes had a median per-process
RustSHT/DUCC ratio of `3.3104`; parity has not been reached. The process-level
backend timing samples, CV/drift values, errors, peak RSS, and pressure-gate
outcomes are in the [structured evidence](prefix-lane-pass-fusion-t1023-q1-v2-timing-samples.jsonl).

## Pair 6 failure and correctness

The failing baseline SHTns-adapter samples were `35.967, 35.565, 42.194,
35.415, 34.772` ms. Its median was `35.565` ms and CV was `7.4299%`, above
the 5% limit; its drift was `2.4561%`. RustSHT and DUCC timing series passed
in that process. RustSHT versus DUCC relative-L2 error was `1.182e-13` and
maximum absolute error `1.556e-13`. SHTns versus DUCC relative-L2 error was
`8.764e-14` and maximum absolute error `1.161e-13`. Thus the stop is a
reference-timing validity failure, not a numerical failure.

Across all twelve matrix processes, maximum peak RSS was 280,492 KiB
(273.9 MiB). Active-call frequency and thread-placement monitoring showed no
clear external interference. The isolated SHTns timing outlier cannot be
attributed to a system event from the retained evidence. Raw timing stdout,
process and thread captures, GNU-time output, exact PSI, frequency, affinity,
and context-switch data remain local and untracked.

## Follow-up

Preserve this incomplete matrix as diagnostic evidence. Do not restart it under
v2, change its thresholds, or combine its five valid pairs with future
measurements. Resume source-grounded Legendre work and route analysis; any new
acceptance screen requires a prospectively frozen protocol and complete
baseline controls.
