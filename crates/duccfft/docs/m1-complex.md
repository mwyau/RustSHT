# M1 complex transforms

Date: 2026-10-12. M1 adds reusable native f32 and f64 C2C plans with
unnormalized forward transforms and normalized inverse transforms. Plans own
factorization and a full root table. Callers allocate one workspace per
simultaneous execution; execution performs no heap allocation.

## Numerical implementation

The planner follows DUCC's factor order. The recursive Rust schedule uses
fixed butterflies for radices 2, 3, 4, 5, 7, 8, and 11. It uses a direct
O(p^2) kernel for other radix leaves. DUCC's `cfftpg` small-prime algorithm,
`cfftpblue` Bluestein algorithm, vector passes, and nested threading are not
part of M1.

Root construction uses DUCC `RootCalc` octant reflection and double-precision
trigonometry for f32/f64. The plan stores every root rather than reconstructing
the full table from DUCC's two smaller tables. Inverse normalization is
distributed across radix stages. Stages pre-scale inputs when their magnitude
approaches an overflow bound and post-scale ordinary inputs after the
butterfly. For pre-scaled inputs, a correction transform recovers values lost
to rounding before applying `1/radix`; this preserves representable
subnormal terms in groups that also contain large values. This keeps the
public `1/n` inverse convention.

## Validation

- Pinned DUCC revision: `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.
- `cargo test --manifest-path crates/duccfft/Cargo.toml --all-targets` passed
  nine integration tests. Direct forward and independent inverse results
  match O(n^2) references for every length from 1 through 64 in both
  precisions. Tests also cover analytic impulses, constants, Fourier modes,
  deterministic random inputs, maximum-finite, subnormal, and mixed dynamic
  range normalized inverse inputs, concurrent workspaces for both precisions,
  and invalid lengths and buffers.
- `cargo clippy --manifest-path crates/duccfft/Cargo.toml --all-targets -- -D
  warnings` passed. `cargo fmt --manifest-path crates/duccfft/Cargo.toml --all
  -- --check` passed.
- The C2C dump comparison checked 107,172 f64 outputs for 33 lengths against
  pinned DUCC. It passed at `atol=rtol=2e-11` in both planner modes. The
  scalar DUCC run had maximum absolute and scaled errors of `1.023e-12` and
  `6.480e-13`; the vectorized-planner run had `1.819e-12` and `6.041e-13`.

## Initial C2C timing

`benches/results/m1-c2c.csv` records two process-level runs pinned to logical
CPU 30 on an AMD Ryzen 9 5950X. A RustSHT benchmark process was active on CPUs
0–7 during these runs. Rust 1.99.0 used Cargo's optimized bench profile
without `target-cpu=native`; both implementations used f64 and reused plans
and scratch. Each process run reports the median of seven samples. Table values
are the mean of two process-level medians. The public operation measurement
includes inverse normalization and copying the transform output back to the
caller's buffer. It is comparable to another whole-call operation
measurement, not to an unnormalized kernel-only time. Each benchmark used one
thread. A separate peak-RSS sample of the already-built benchmark executable
at n=4,097 with a 32-transform C2C batch measured 4,624 KiB for DUCCFFT and
6,544 KiB for RustFFT. This process-level measurement includes the executable
and runtime as well as its plans and buffers. The memory processes were pinned
to CPU 30; host load averages immediately beforehand were 3.44, 3.82, and 3.53.

| Length | DUCCFFT forward / inverse (µs) | RustFFT forward / inverse (µs) |
| ---: | ---: | ---: |
| 64 | 0.636 / 0.766 | 0.070 / 0.090 |
| 512 | 6.668 / 8.256 | 0.641 / 0.761 |
| 1,024 | 19.647 / 23.970 | 1.552 / 1.778 |
| 2,048 | 37.641 / 45.715 | 3.692 / 4.197 |
| 257 | 176.574 / 177.591 | 1.142 / 1.202 |
| 1,025 | 125.724 / 129.030 | 5.696 / 5.962 |
| 2,049 | 3,799.794 / 3,807.092 | 34.148 / 34.654 |
| 4,097 | 2,866.712 / 2,866.667 | 55.649 / 56.725 |

These results are provisional, from two process-level runs. The general-prime
direct fallback dominates prime-heavy workloads, and the M1 implementation
is slower than RustFFT at every measured length. These timings establish a
baseline for the planned DUCC prime and Bluestein ports; they do not establish
performance parity.
