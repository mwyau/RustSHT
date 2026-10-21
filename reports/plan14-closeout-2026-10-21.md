# Plan 14 closeout — scalar GL baseline and remaining gaps

**Rust source:** `e487b85bc98273dd428a94529689124a5c9fa1e6`

**DUCC:** 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`

**Runtime:** Rust 1.99.1 / LLVM 23.1.1, Pulp 0.22.3 V3 AVX2/FMA;
DUCC GCC 15.2, SHT `-O3 -ffast-math -ffp-contract=fast
-march=x86-64-v3`, strict validation driver.

Plan 14 closes with the current-source full-transform sweep and the targeted
T720 synthesis validation/profile. The sweep covers 376 geometries, including
every `lmax=0…128` and 28 selected degrees through T3072, both scalar
operations, full/reduced `mmax`, q1/q8 and even-smooth plus selected
odd-minimum longitude shapes. All 1,504 operation/thread groups completed;
752 numerical validation configurations passed with no recorded failed or
unsupported cases. See the [full report](ducc-current-full-sweep-2026-10-21.md)
and [machine-readable results](data/ducc-current-full-sweep-2026-10-21/).

## Current T720 residual

The fresh sweep uses the same production scalar source as the earlier
`1951e833` sparse-tail stage profile; since that implementation, Rust source
changes were limited to `cfg(test)` workspace/fallback validation. The new
T720 q8 whole-call measurement independently reproduces the stage-profile
result:

| Case | Rust median | DUCC median | Rust/DUCC | Excess | Paired ratio p10–p90 | Relative L2 |
|---|---:|---:|---:|---:|---:|---:|
| Analysis, `mmax=180`, `nphi=361`, q8 | 1.164 ms | 0.948 ms | 1.228× | +0.216 ms | 1.195–1.282 | `4.89e-14` |
| Analysis, `mmax=180`, `nphi=378`, q8 | 0.932 ms | 0.923 ms | 1.010× | +0.009 ms | 0.970–1.034 | `4.91e-14` |
| Synthesis, `mmax=180`, `nphi=361`, q8 | 1.482 ms | 1.254 ms | 1.182× | +0.228 ms | 1.166–1.230 | `5.19e-14` |
| Synthesis, `mmax=180`, `nphi=378`, q8 | 1.256 ms | 1.220 ms | 1.030× | +0.037 ms | 1.017–1.053 | `4.71e-14` |
| Synthesis, `mmax=720`, `nphi=1441`, q8 | 3.915 ms | 3.874 ms | 1.010× | +0.041 ms | 0.986–1.044 | `4.45e-14` |
| Synthesis, `mmax=720`, `nphi=1458`, q8 | 2.835 ms | 2.700 ms | 1.050× | +0.136 ms | 1.029–1.072 | `4.41e-14` |

For reduced odd synthesis, the sweep measured `1.466/1.482/1.531 ms`
Rust p10/median/p90 and `1.246/1.254/1.262 ms` DUCC, using three
interleaved process pairs and 30 calibrated batches. This is consistent with
the previous matched `1.467/1.486/1.529` and `1.238/1.255/1.266 ms` stage
capture. The corresponding phase excess was split between recurrence plus
spectrum production (`+0.112 ms`) and inverse FFT plus writeback
(`+0.121 ms`). Sparse-tail initialization was only `1.62 µs`. The isolated
Rust real-C2R alternative was 8.8–10.0% slower than production, so no
replacement mechanism is supported. Detailed batch values and boundaries
remain in the [stage profile](data/plan14-sparse-spectrum-current-2026-10-21/stage_profile.json).

The sweep also records larger q1 reduced-order odd-shape excesses at T720:
`+1.452 ms` for analysis and `+2.080 ms` for synthesis. These are measured
whole-call residuals, not newly attributed stages. Earlier small-q1 phase
work found recurrence and FFT to dominate the tested T31/T63/T64 cases but
did not establish a general fix. The comprehensive sweep is a baseline, not
an optimization A/B; its ratios vary by degree, order range, longitude shape
and thread count. Large-band full-order cases are often near parity, while
the dense low-band q8 data include both wins and substantial regressions.

## Validation and disposition

Sparse-spectrum initialization passed repeated poisoned-workspace reuse,
odd/even latitude counts, partial polar limits, extreme finite coefficients,
subnormal decline after partial writes, clear-before-fallback, and output
reset checks. Maximum sweep relative L2 error was `2.536e-13`; roundtrip,
basis and adjoint maxima were `5.563e-13`, `5.394e-12` and `3.427e-12`.
The release workspace tests also passed:

- `cargo test --workspace --all-features --release`: 243 library tests,
  4 extreme-scaling tests, 11 f32 tests and 1 workspace compatibility test
  passed; 31 library and 1 f32 test ignored.
- `RUSTFLAGS='-Aunexpected_cfgs' cargo test --workspace --no-default-features --release`:
  same test counts passed.
- `cargo fmt --all -- --check` passed. Strict Clippy passed with the
  repository's `-Aunexpected_cfgs` allowance for its existing custom cfgs.
  The Python benchmark module compiled and its plotting smoke figures were
  parsed as valid SVG; all four principal sweep figures contain q1/q8,
  measured points and the parity line.

No production numerical edit is warranted by this closeout. Retain the
validated sparse-tail path and the existing robust fallback. The remaining
odd-q8 gap is measured and split across two stages; the tested inverse
alternative is slower and no recurrence change has a concrete source-grounded
mechanism. Preserve the q1 reduced odd residual as unresolved rather than
claiming scalar parity. Plan 14 is complete; the owner-authorized successor
is [Plan 15 — spin-1 GL/CC/F1](../plans/15-spin1-vector-gl-cc-f1-performance.md).
