# M2 real transforms

Date: 2026-10-12. M2 adds reusable f32 and f64 R2C/C2R plans with an ordinary
`Complex<T>` half-spectrum. Forward transforms are unnormalized; inverse
transforms include `1/n`. Plans hold their radix factorization and roots, and
caller-owned workspaces hold recursive half-spectrum scratch. Repeated
execution allocates no memory.

## Implementation

The factor order follows DUCC's real factorizer: factors of four, a leading
factor of two, and ascending odd factors. The recursive decimation-in-time
passes transform the strided subsequences `x[j + r*t]`, where `N = r*M`. If
`X_j[q]` is the length-`M` transform of subsequence `j`, the forward pass
forms `X[k] = sum_j X_j[k mod M] exp(-2*pi*i*j*k/N)` for
`0 <= k <= floor(N/2)`. A wrapped child frequency above its Nyquist bin is
read by conjugating the corresponding positive bin. C2R builds each child
spectrum from Hermitian partners, ignores imaginary DC and even-length
Nyquist values, and distributes normalization over the radix stages. Its
correction sum preserves subnormal contributions when radix scaling would
round them away.

The Rust implementation works directly with the public half-spectrum; it does
not use DUCC's packed real array internally. The C++ reference harness converts
DUCC's FFTPACK layout for output comparisons. Dedicated fixed butterflies for
radices 2, 3, 4, and 5 use the same radix decompositions as DUCC's `rfftp2`,
`rfftp3`, `rfftp4`, and `rfftp5`. They express those passes in the ordinary
half-spectrum layout. Larger factors still use a generic scalar combine. The
implementation has not yet ported `rfftpg`, `rfftpblue`, or DUCC's conditional
`rfftp_complexify` planner. A prime length therefore takes O(p^2) work, which
accounts for most of the M2 performance gap. The general-prime and Bluestein
passes will replace that path.

## Validation

- `cargo test --manifest-path crates/rustsht-fft/Cargo.toml --all-targets` passed
  16 integration tests. The 9 complex tests and 7 real tests include
  independent direct DFTs in f32 and f64 for lengths 1 through 64, analytic
  impulses and constants, odd/even endpoints, invalid buffers, concurrent
  workspaces, and mixed maximum-finite/subnormal inverse data.
- Clippy passed with `--all-targets -- -D warnings`; `cargo fmt --all -- --check`
  passed.
- The comparison harness checked 187,574 output records for C2C, R2C, and C2R
  against the pinned DUCC revision in scalar and vectorized planner modes. It
  also compared RustSHT-FFT directly with RustFFT. All comparisons passed with
  `atol=rtol=2e-11`. RustSHT-FFT's maximum absolute error against DUCC was
  `1.023e-12` in scalar planner mode and `1.819e-12` in vectorized mode. The
  maximum absolute error against RustFFT was `3.070e-12`.

## Initial real timing

`benches/results/m2-real.csv` records two process-level runs. The 5950X host
has 16 cores and 32 logical CPUs. Rust 1.99.0 used Cargo's optimized bench
profile without `target-cpu=native`; the C++ reference used g++ 15.2.0 with
`-std=c++20 -O3 -DNDEBUG -march=native -pthread`, one thread, and
`vectorize=false`. Each process used one executing thread and was pinned to
logical CPU 30. Both runs began with `/proc/loadavg` at `4.22 3.73 3.56`; a
compute-heavy RustSHT `m1b_bench` process was active on CPU 5 at the pre-run
host check. That check does not establish whether it remained active through
both runs, so the timing results are provisional.
Each process reports medians of seven samples. RustSHT-FFT and RustFFT were f64;
the table gives the mean per-transform whole-call time at batch count one.
RustSHT-FFT's C2R includes normalization but uses the public half-spectrum
directly. The C++ and RustFFT whole-call times include conversion to or from
their internal packed/adapter layouts.

| Length | DUCC R2C / C2R (µs) | RustFFT R2C / C2R (µs) | RustSHT-FFT R2C / C2R (µs) |
| ---: | ---: | ---: | ---: |
| 257 | 2.941 / 3.552 | 1.222 / 1.222 | 88.978 / 307.930 |
| 1,025 | 4.063 / 3.421 | 6.067 / 6.107 | 79.234 / 221.317 |
| 2,049 | 38.422 / 37.892 | 34.725 / 34.975 | 1,906.841 / 6,771.228 |
| 4,097 | 69.505 / 69.285 | 60.820 / 57.363 | 1,452.616 / 4,820.620 |

Batch counts were 1 and 32 at lengths 257 and 1,025, 1 and 4 at length 2,049,
and 1 at length 4,097. The batch was reduced at larger sizes to bound the
generic large-radix benchmark time. The checksum is calculated from the output
buffer produced by the final timed sample, with that buffer observed outside
the timing interval. A separate `time -v` sample of the already-built
benchmark executable at length 4,097 and batch count one measured peak RSS of
2,492 KiB for RustSHT-FFT and 4,256 KiB for RustFFT; it includes the process
runtime, plans, and buffers.

RustSHT-FFT is slower than both references at every measured length. The generic
large-radix combine is especially costly for C2R at lengths 2,049 and 4,097.
These results establish the M2 baseline; they do not indicate performance
parity.

The follow-up native radix-11 pass, DUCC comparison, and paired timing results
are recorded in [M2 native real radix-11](m2-real-radix11.md).
