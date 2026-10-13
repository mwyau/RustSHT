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
part of M1. Root construction uses DUCC `RootCalc` octant reflection and double-precision
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
- `cargo test --manifest-path crates/rustsht-fft/Cargo.toml --all-targets` passed
  nine integration tests. Direct forward and independent inverse results
  match O(n^2) references for every length from 1 through 64 in both
  precisions. Tests also cover analytic impulses, constants, Fourier modes,
  deterministic random inputs, maximum-finite, subnormal, and mixed dynamic
  range normalized inverse inputs, concurrent workspaces for both precisions,
  and invalid lengths and buffers.
- `cargo clippy --manifest-path crates/rustsht-fft/Cargo.toml --all-targets -- -D
  warnings` passed. `cargo fmt --manifest-path crates/rustsht-fft/Cargo.toml --all
  -- --check` passed.
- The C2C dump comparison checked 107,172 f64 outputs for 33 lengths against
  pinned DUCC. It passed at `atol=rtol=2e-11` in both planner modes. The
  scalar DUCC run had maximum absolute and scaled errors of `1.023e-12` and
  `6.480e-13`; the vectorized-planner run had `1.819e-12` and `6.041e-13`.

## Initial C2C timing

`benches/results/m1-c2c.csv` records two process-level runs pinned to logical
one CPU core on an x86-64 CPU. A RustSHT benchmark process was active on 8 pinned CPU cores during these runs. Rust 1.99.0 used Cargo's optimized bench profile
without `target-cpu=native`; both implementations used f64 and reused plans
and scratch. Each process run reports the median of seven samples. Table values
are the mean of two process-level medians. The public operation measurement
includes inverse normalization and copying the transform output back to the
caller's buffer. It is comparable to another whole-call operation
measurement, not to an unnormalized kernel-only time. Each benchmark used one
thread. A separate peak-RSS sample of the already-built benchmark executable
at n=4,097 with a 32-transform C2C batch measured 4,624 KiB for RustSHT-FFT and
6,544 KiB for RustFFT. This process-level measurement includes the executable
and runtime as well as its plans and buffers. Load-average telemetry was retained only in local diagnostics. The general-prime
direct fallback dominates prime-heavy workloads, and the M1 implementation
is slower than RustFFT at every measured length. These timings establish a
baseline for the planned DUCC prime and Bluestein ports; they do not establish
performance parity.
