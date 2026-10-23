# Spin-1 paired coefficient preparation

**Date:** 2026-10-23

**Disposition:** retain the fused preparation loop.

## Change

Spin-1 synthesis prepares even/odd recurrence coefficients for two converted
scalar rows. The f64 path now computes both rows in one degree-pair loop and
loads each pair's shared recurrence factors once. The coefficient arithmetic
and output values are unchanged. Both the order-major route and the existing
hemisphere route use the helper. A failed or nonfinite row still declines the
paired path before either channel is written to the output spectra.

## Validation

- `cargo test --release --lib -- --test-threads=1`: 278 passed, 36 ignored.
- `cargo test --release --lib --no-default-features -- --test-threads=1`:
  226 passed, 17 ignored.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- The new tests compare both fused rows exactly with independent preparation
  across order/degree boundaries and reject a nonfinite second row.

## Focused whole-transform comparison

Exact CC T720, full order (`mmax=720`, `ntheta=722`, `nphi=1458`), spin-1
STANDARD synthesis at q8. The baseline is source commit `2461866f`; the
candidate adds only this production change. Both Rust release builds used
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. The DUCC 0.42.0 executable uses
pinned source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` and its matched
x86-64-v3 release build.

Three interleaved process groups compared baseline Rust, candidate Rust and
DUCC. Each backend contributed 30 timed batches (10 per independent process);
calibration batches targeted 25 ms, and the actual median batch durations were
31–32 ms for Rust and 25 ms for DUCC. The table reports per-transform latency
medians across all samples. The paired ratios use each process group's
candidate and baseline medians.

| Backend | Median (ms) | p10 (ms) | p90 (ms) | Timed batches |
|---|---:|---:|---:|---:|
| Parent Rust | 16.015 | 15.859 | 16.672 | 30 |
| Candidate Rust | 15.449 | 15.239 | 16.574 | 30 |
| Pinned DUCC | 6.289 | 6.159 | 6.367 | 30 |

Candidate/parent ratios were `0.962`, `0.959` and `0.968` across the three
process groups. All three process medians improved. The overall candidate and
parent batch distributions overlap, and q8 host-frequency samples varied, so
the result is evidence for this workload rather than a broad spin-1 claim.
Candidate/DUCC median ratio was `2.45x`; a large performance gap remains.

The candidate matched parent output exactly. Candidate versus DUCC maximum
relative L2 was `2.29e-13`. Repeat and reset checks passed for every process.
Raw samples, telemetry and baseline/candidate binary hashes remain in a local
untracked JSON capture.
