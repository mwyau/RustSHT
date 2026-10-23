# Spin-1 checked pair preparation reuse

**Date:** 2026-10-23
**Disposition:** retain the bounded reuse in production.

## Production change

For spin-1 synthesis, paired coefficients depend on Fourier order and converted
E/B rows, not latitude. When two or more adjacent 64-latitude blocks pass the
existing sine, polar-order, and scratch checks, synthesis now prepares the
checked two-channel coefficient pair once and reuses it for that contiguous
run. A one-block run keeps the original preparation path. Unsafe, excluded,
nonfinite, and table-backed work keeps its existing checked path and fallback.

This avoids repeating the same degree-pair preparation across safe blocks
without adding a transform-wide cache or workspace.

## Matched whole-transform comparison

The comparison used exact CC T720 (`ntheta=722`), full and quarter order
(`mmax=720/180`, `nphi=1458/378`), q1/q8, and analysis/synthesis. Baseline
source was `8e3f8a3b`; the candidate adds only per-order checked pair reuse.
Rust used 1.99.1 with `-C target-cpu=x86-64-v3`; DUCC 0.42.0 was pinned to
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Each backend collected 30 timed
batches per case across three interleaved process pairs on a Ryzen 9 5950X.

| `mmax` | Threads | Operation | Baseline Rust (ms) | Candidate Rust (ms) | DUCC (ms) | Candidate / baseline by pair |
|---:|---:|---|---:|---:|---:|---|
| 180 | q1 | Synthesis | 43.255 | 42.434 | 18.489 | 0.992, 0.980, 0.972 |
| 180 | q8 | Synthesis | 7.033 | 6.900 | 2.651 | 0.990, 0.986, 0.965 |
| 720 | q1 | Synthesis | 93.212 | 91.566 | 43.623 | 0.986, 0.978, 0.987 |
| 720 | q8 | Synthesis | 15.316 | 14.938 | 6.123 | 0.978, 0.976, 0.971 |

All four synthesis rows improved in each process pair. The primary full-order
q8 case improved 2.5% (`0.378 ms`); it remains `2.44x` DUCC. Analysis was an
unchanged control; its process-pair variation does not support attributing a
change to this code.

Every candidate output matched the parent exactly. All finite, repeat, and
reset checks passed; maximum candidate-to-DUCC relative L2 error was
`3.64e-13` (quarter-order synthesis). The full-order q8 synthesis error was
`2.29e-13`.

## Validation and provenance

- `cargo fmt --check`
- `cargo test --release --lib -- --test-threads=1`: 278 passed, 36 ignored.
- `cargo test --release --no-default-features --lib -- --test-threads=1`:
  226 passed, 17 ignored.
- `cargo clippy --all-targets --all-features -- -D warnings`
- Existing T720 changed-coefficient and T1023 cache reuse tests passed.

Baseline, candidate, and DUCC executable SHA-256 hashes, along with raw
timings and host samples, remain in the local untracked capture.

An adjacent F1 reflected-dispatch trial was rejected and removed: full-order
T720 q8 synthesis measured `14.367 ms` on the parent and `51.569 ms` on the
candidate. The current change does not enable that route.
