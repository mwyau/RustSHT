# Integrated m=0 skip-and-restore diagnostic — 2026-10-29

**Decision: reject.** This variant skips the generic m=0 recurrence and
restores the Fourier column from a compensated f32 dot product after the other
orders complete. It fixes the narrow zonal probe, but still prepares the full
m=0 basis using an f64 plan-time recurrence. That exceeds the allowed small
constant preparation. The added path also misses native-f32 DUCC timing in all
four cells. No production dispatch changed.

## Provenance

- Base source: `b6a96aa7161a64f678b614d73478496f50f0551e`.
- Comparison parent: uniform-scale source patch
  `57e58a06c46b3af90209c239c2fda419ee8edf7c19b0d7b2a8d3c9742a70bbd3`.
- Candidate source patch SHA-256:
  `76402d2a71b5beb2a621ed583aab91fa4851bc4a0de1829ad804121270d9eccc`;
  compressed patch SHA-256:
  `6b06ad30abaaeb561d3c937c41800563dfadef2e10c86283f8db3c5831095f0b`.
- Candidate binary SHA-256:
  `b1fe67ebb0078155ccc5a3f864ecf75ac013547d994bbf2a826d6f92fe41a215`.
- Comparison parent binary SHA-256:
  `d9158760d31937718f43e2ad09542a60ca2ca90e0018164238b2445345d3d932`.
- Pinned DUCC 0.42.0 binary SHA-256:
  `be18c89ea434258e4a07c28ccd8fd31d6262ac3503dcaf81d8aa5176415f1484`;
  source commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Build flags: `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; AMD Ryzen 9
  5950X; q1 affinity `0`; q8 affinity `0,2,4,6,8,10,12,14`.
- Three balanced interleaved process pairs per cell. Fixture hashes and pair
  timings are in [`results-3.json`](results-3.json).

## Screen

| GL T720 scalar synthesis | Uniform-scale parent ms | Candidate ms | DUCC ms | Candidate/DUCC median |
|---|---:|---:|---:|---:|
| Full q1 | 23.483 | 24.197 | 18.942 | 1.276 |
| Full q8 | 4.590 | 5.779 | 3.914 | 1.265 |
| Reduced q1 | 8.294 | 8.940 | 8.669 | 1.036 |
| Reduced q8 | 1.324 | 1.640 | 1.279 | 1.284 |

The q8 timings varied substantially within pairs; this is a diagnostic screen,
not an acceptance interval. The zonal correction reduced the all-ring
`l=720,m=0` maximum absolute error to `1.483e-8`, but random-fixture relative
L2 error versus DUCC remained `1.864e-4` full and `1.885e-4` reduced. The
candidate remained slower than both DUCC in all cells and its comparison
parent in the reduced cells.

`cargo check --features parallel` and the focused zonal test passed. The
focused test reported the same corrected zonal error as the prepared-basis
diagnostic. **The full library suite was not run on this exact variant.** The
complete source patch, result JSON, and exact binary/fixture provenance are
retained here; raw fixtures remain in experiment scratch.

## Next

Do not retain the full basis table or post-transform dot pass. Implement the
native-f32 compensated recurrence leaf with plan-time high/low geometry and
small recurrence constants, combining runtime f32 coefficients with f32
compensated operations inside the shared order-major route.
