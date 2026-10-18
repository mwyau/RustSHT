# Prefix lane-pass fusion T1023 baseline stop

## Decision

The frozen q1 matrix stopped at its first T1023 baseline process because the
DUCC timing series failed the predeclared drift limit. No candidate process
ran, no pair was formed, and no retry or replacement sample was collected.
The fused candidate remains experimental and unaccepted. Results from this
process do not authorize the T2047, T511, q8, or spin-1 cells.

## Frozen configuration and provenance

- Workload: GL f64 scalar standard synthesis, `1024x2048`, full `mmax=1023`,
  `phi0=0.37`, one thread, caller-provided output (`scope=into`). Each
  backend used one warm-up and five timed calls.
- Harness: `python_perf_baseline.py`, SHA-256
  `1f1bdfa8f2cf886db3874b2ae9160d9c68f89f0a4026a19bc9ada47948606fb8`.
- Clean RustSHT extension SHA-256:
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- DUCC 0.42.0 source revision:
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`; extension SHA-256
  `1218502fa03116cd14578fe73f716f5a1d88db74d296359cc9e8c27e04047d1e`.
- SHTns 3.7.5 source revision:
  `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`; library SHA-256
  `641195ac574ba02078f3238b357caecf0e268b3f4ef7eba1d87898197fcc888f`.
- Candidate source was frozen at `synthesis_fast.rs` SHA-256
  `eca2c4ce4dec0e1c02e948ae46ce0a29b4d47b03537a5d5b57b6ccea462aa407`
  and `scalar.rs` SHA-256
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
  Its extension was not run in this matrix process.

## Measurements and validity

| Backend | Timed samples (ms) | Median (ms) | CV | First-three/final-two drift | Gate |
|---|---|---:|---:|---:|---|
| RustSHT baseline | 298.315, 335.990, 324.779, 308.112, 318.840 | 318.840 | 4.1162% | 3.5450% | Pass |
| DUCC | 47.724, 47.200, 50.188, 49.014, 52.988 | 49.014 | 4.1752% | 6.6858% | Fail: drift > 5% |
| SHTns | 34.450, 34.609, 34.727, 34.630, 34.733 | 34.630 | 0.2969% | 0.2094% | Pass |

Numerical checks passed: RustSHT versus DUCC relative-L2 error was
`1.182e-13`, maximum absolute error `1.556e-13`; SHTns versus DUCC relative-L2
error was `8.764e-14`, maximum absolute error `1.161e-13`. Peak process RSS was
279,556 KiB. The preflight and postflight pressure gates passed. Other
validity records did not establish clear external interference, but the DUCC
drift failure alone invalidates the process under protocol v1.

## Follow-up

The matrix stops here. Preserve the fixed protocol and this baseline as
historical diagnostic evidence. Do not compare it with another source revision
or use it to estimate candidate speedup. Any future matrix needs a separately
versioned protocol validated prospectively on baseline controls before
candidate acquisition.
