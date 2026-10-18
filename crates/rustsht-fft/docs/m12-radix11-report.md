# AVX2 radix-11 pass for the N=682 convolution

Date: 2026-10-15. Baseline: M11 commit `fcce532`.

## Result

The f64 N=682 complex plan now vectorizes its radix-11 pass when its layout is
`l1=2`, `ido=31` and runtime AVX2 is available. Each vector processes two
adjacent inner indices. The kernel evaluates the fixed radix-11 equations,
applies the stage twiddles, and leaves inner index 30 to the scalar caller. The inverse kernel is selected only when the existing deferred-normalization
range check passes. Other precisions, layouts, and unsafe inverse inputs retain
the prior path. Six alternating process pairs against M11 reduced N=2049/1,024-row operation
latency by 21.7% for R2C and 20.8% for C2R. Both paired 95% intervals were
above parity. N=682 C2C forward and inverse controls also improved at one, 22,
and 44 rows. The kernel declares two 11-element AVX2 local arrays (704 bytes
combined) and adds no plan or workspace allocation. N=2049 retains 190,721
plan bytes and 159,856 workspace bytes; the N=682 complex plan retains 28,961
plan bytes and 11,904 workspace bytes.

## Numerical checks and dispatch

The pass-level direct-DFT test checks forward and inverse transforms, stage
twiddles, both outer groups, output guards, and the untouched scalar tail. The route test confirms selection on ordinary AVX2 inputs and confirms that
high-amplitude inverse inputs use the existing safety path. The N=682
integration test compares forward and inverse outputs with RustFFT 6.4.1. Route evidence for the standalone N=682 plan and the N=2049 Rader-683 path is
in [the route diagnostics](../benches/results/m12-radix11-ido31-route-diagnostics.txt).

## Paired latency against M11

Each process reported the median of seven warmed samples. Six alternating
process pairs were pinned to logical one CPU core. Ratios are `M11 time / M12 time`,
so values above one favor M12. Intervals are percentile-bootstrap 95% intervals
over the six paired ratios. Plan and workspace construction were measured
separately from public operation time.

| N | Rows | R2C speedup (95% interval) | C2R speedup (95% interval) |
|---:|---:|---:|---:|
| 2049 | 1 | 1.283 (1.278–1.298) | 1.215 (1.204–1.324) |
| 2049 | 32 | 1.281 (1.274–1.297) | 1.263 (1.219–1.279) |
| 2049 | 1024 | 1.277 (1.272–1.300) | 1.262 (1.222–1.277) |

At 1,024 rows, median public-operation times were 36.284 µs for M12 R2C and
39.611 µs for M12 C2R, compared with 46.378 µs and 49.985 µs for M11.

| N | Rows | C2C forward speedup (95% interval) | C2C inverse speedup (95% interval) |
|---:|---:|---:|---:|
| 682 | 1 | 1.432 (1.382–1.543) | 1.319 (1.294–1.326) |
| 682 | 22 | 1.417 (1.378–1.425) | 1.317 (1.293–1.330) |
| 682 | 44 | 1.408 (1.375–1.421) | 1.315 (1.294–1.324) |

The control workloads do not use this N=682-only route. N=257 C2R was 1.1%
slower by the paired median and N=723 C2R was 0.4% slower; both intervals were
below parity. N=241 C2R and R2C intervals crossed parity. N=257 R2C also
crossed parity, while N=723 R2C improved 1.6% with its interval above parity. These observations are retained as control results and do not establish a
cause for the changes. Structured timing samples, benchmark configurations, and the per-cell summary are in
the [N=2049 acceptance capture](../benches/results/m12-radix11-ido31-target-acceptance-six-pairs.jsonl),
the [N=682 C2C capture](../benches/results/m12-radix11-ido31-n682-control-six-pairs.jsonl),
the control captures, and the
[acceptance summary](../benches/results/m12-radix11-ido31-acceptance-summary.csv).

## Instruction profile

N=2049 C2R operations, compared with 100,510,977 for M11, a 27.9% reduction. The exclusive scalar `radix11` count fell from 14,818,682 to 478,022
references. The new AVX2 pass recorded 6,172,351 exclusive references and its
scalar-tail dispatcher recorded 628,698. The radix-31 kernel retained its
22,232,829-reference count, which represents 30.67% of the lower M12 total. Instruction references are counts, not elapsed time or cycle measurements.

contain the compared runs.

## Build and validation

Measurements used an x86-64 CPU under Linux x86-64, Rust 1.99.0 stable,
the release bench profile, a portable compiler target, runtime AVX2, and one
process pinned to one CPU core. Load-average telemetry was retained only in local diagnostics. The M11 baseline executable SHA-256 is
`54c97e86891dfb6f76ba7f1e3483ad35edb9dbe9ce700f0780373c1998e29926`; the M12
candidate executable SHA-256 is
`69ce17d85fc9762da1513749f6d7d03e0a456e453fe344b5a94d7c2912cd4eff`. The following checks passed in `crates/rustsht-fft/`:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features` (101 tests passed)
- `cargo test --release --lib f64_complex_radix11_ido31_matches_direct_pass_and_preserves_scalar_tail`
- `cargo test --release --lib m12_c2c_682_selects_avx2_radix31_and_radix11_and_guards_unsafe_inverse_inputs`
- `cargo test --release --test complex f64_682_radix31_transform_matches_rustfft_forward_and_inverse`
- `cargo bench --bench rustfft --no-run`

No new cross-library performance campaign was run. RustFFT 6.4.1 was used for
the N=682 numerical integration check.
