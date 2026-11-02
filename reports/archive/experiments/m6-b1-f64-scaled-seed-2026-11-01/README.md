# B1 f64 scaled-seed continuation — rejected

This isolated candidate attempted to initialize a SIMD-scaled sectoral seed and continue directly into the typed native-f64 scalar-analysis recurrence. It was measured against the exact `31c7cd5046d2389d5f0de230577dc147ff37f873` parent and pinned DUCC 0.42.0. The candidate was not promoted.

## Result

Three interleaved whole-transform pairs show a large slowdown in every tested GL T720 cell. Medians below are milliseconds; `candidate / parent` is the geometric mean of paired ratios.

| Cell | Parent | Candidate | DUCC f64 | Candidate / parent |
|---|---:|---:|---:|---:|
| Reduced q1 | 6.084 | 16.731 | 5.947 | 2.717× |
| Reduced q8 | 0.928 | 2.346 | 0.944 | 2.520× |
| Full q1 | 15.219 | 34.459 | 14.730 | 2.266× |
| Full q8 | 2.278 | 4.832 | 2.139 | 2.128× |

The path adds scaled-seed conversion and range handling before the existing ordinary recurrence. It is materially worse, so no larger benchmark campaign was run. Existing numerical checks in the screen were retained; this is a performance rejection, not a correctness claim.

## Provenance

- Parent source: `31c7cd5046d2389d5f0de230577dc147ff37f873`.
- DUCC source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; Rust 1.99.1 / LLVM 23.1.1; `-C target-cpu=x86-64-v3`.
- Affinity: q1 pinned to one physical core; q8 pinned to eight physical cores.
- Fixture hashes, executable hashes, three-pair timings, and validation summaries: [`screen-results.json`](screen-results.json). Host-frequency traces and absolute executable paths were omitted from the committed summary.
- Reproduction patch: [`patches/rejected-scaled-seed.patch.gz`](patches/rejected-scaled-seed.patch.gz); decompress with `gzip -dc` before applying.

The unredacted measurement capture remains local and untracked because it contains local executable paths and per-core host details.

No production source was modified by this candidate.
