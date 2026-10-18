# Prefix lane-pass fusion v2 baseline controls at T1023

## Decision

Both prospective baseline-only controls passed protocol v2. Each process
used the same clean baseline extension and passed the numerical checks and
all three backend timing limits. CPU pressure gates passed; postflight entered
the allowed warning band in both processes. Placement and system checks did
not show clear workload interference or cgroup throttling. The frozen q1
candidate matrix may proceed at T1023 under v2. These controls are not
candidate pairs and do not accept the candidate.

## Frozen configuration

- GL f64 scalar `STANDARD` synthesis, `1024x2048`, full `mmax=1023`,
  `phi0=0.37`, q1, caller-provided output, RustSHT/DUCC/SHTns.
- Each backend received one correctness call, three fixed extra warm-up calls,
  the harness's existing warm-up call, and five measured calls. All samples
  were retained.
- Clean baseline extension SHA-256:
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Harness base SHA-256:
  `1f1bdfa8f2cf886db3874b2ae9160d9c68f89f0a4026a19bc9ada47948606fb8`;
  warm-up patch SHA-256:
  `dba74d632d2abdcc20c73e90a8b2538efbd0f298d797488dde6d8bf427e6dc45`;
  patched harness SHA-256:
  `61b640657990bebf80a1d19a1cd4cd0e99e2adcb7234161ea9b984b6971669ab`.
- DUCC and SHTns revisions and binary identities are recorded in
  [protocol v1](prefix-lane-pass-fusion-q1-matrix-t2047-t1023-t511-protocol-v1.md).

## Timing samples and gates

| Process | Backend | Samples (ms) | Median (ms) | CV | Drift | Gate |
|---|---|---|---:|---:|---:|---|
| Control 1 | RustSHT | 292.426, 292.224, 292.744, 294.236, 293.959 | 292.744 | 0.2803% | 0.5710% | Pass |
| Control 1 | DUCC | 49.874, 48.764, 47.196, 47.012, 49.655 | 48.764 | 2.4753% | 0.8828% | Pass |
| Control 1 | SHTns | 35.161, 35.010, 34.980, 34.737, 34.763 | 34.980 | 0.4570% | 0.7433% | Pass |
| Control 2 | RustSHT | 295.397, 295.712, 294.837, 294.776, 292.789 | 294.837 | 0.3456% | 0.5476% | Pass |
| Control 2 | DUCC | 51.054, 50.775, 49.439, 48.483, 48.385 | 49.439 | 2.2512% | 4.7351% | Pass |
| Control 2 | SHTns | 34.241, 34.028, 34.148, 34.383, 34.733 | 34.241 | 0.7077% | 1.1974% | Pass |

RustSHT versus DUCC relative-L2 error was `1.182e-13` in both processes;
maximum absolute error was `1.556e-13`. SHTns versus DUCC relative-L2 error
was `8.764e-14`; maximum absolute error was `1.162e-13` and `1.161e-13`.
Peak RSS was 275,932 KiB and 275,624 KiB (269.5 MiB maximum).

The within-process RustSHT/DUCC median ratios were 6.0033 and 5.9637. These
are descriptive clean-baseline controls at T1023 q1, not candidate results or
six-pair acceptance estimates. The fixed worker placement remained within
its configured single-CPU mask. Frequency monitoring during active calls was
stable enough to show no clear frequency-related interference; exact system
captures remain local. No competing CPU-intensive benchmark or build was
observed. Raw process, thread, pressure, frequency, context-switch, and
GNU-time captures remain outside the repository.

## Follow-up

Proceed with the frozen six-pair baseline/candidate T1023 q1 cell using the
same warm-up protocol and timing gates. Stop at the first invalid process.
Do not combine these baseline-only controls with candidate-pair statistics.
