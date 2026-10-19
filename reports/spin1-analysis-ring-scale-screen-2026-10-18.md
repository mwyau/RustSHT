# Spin-1 GL analysis ring-scale screen

**Date:** 2026-10-18  
**Baseline:** `9e9c56752162bc237e727e8cf1218e871de51a47`  
**Candidate:** `89d3ff49ca237ad7e450f7e89d2e82f03cab5422`

The candidate computes `ring_weight / sin(theta)` and optional Fourier
normalization once per latitude ring per transform. Paired spin-1 analysis
reuses that scale for every order and for both Q and U. Complex phase and
real-scale multiplication keep their previous order. The workspace adds one
reusable `ntheta`-element real buffer. The path applies to f64 GL spin-1
analysis; other grids, spins, precisions and adjoints keep their existing
routes.

The comparison used `m1b_bench`, Rust release builds from the baseline and
candidate commits, Rust 1.99.0 / LLVM 23.1.1, Pulp 0.22.3, Rayon 1.12.0 and
RustFFT 6.4.1. Plans and workspaces were reused. Each process did one warmup
followed by five timed full `analysis_spin_into` calls; values are medians of
the five calls. Full-call timings include two forward real FFTs and Legendre
analysis. They do not isolate Legendre time and do not compare against DUCC.

| Grid and mmax | Threads | Baseline ms | Candidate ms | Candidate / baseline |
|---|---:|---:|---:|---:|
| T511, 512x1025, full | 1 | 29.65 | 28.89 | 0.974 |
| T511, 512x1025, full | 8 | 5.13 | 5.13 | 1.000 |
| T720, 721x1443, full | 1 | 80.33 | 77.84 | 0.969 |
| T720, 721x1443, full | 8 | 13.69 | 13.52 | 0.987 |
| T300, 301x603, mmax=150 | 1 | 7.55 | 7.49 | 0.992 |
| T300, 301x603, mmax=150 | 8 | 1.41 | 1.43 | 1.014 |

The q1 values are medians of three paired process rounds. q8 values are medians
of six paired process rounds, with both execution orders represented. q1 fell
2.6% at T511 and 3.1% at T720; reduced T300 was nearly unchanged. q8 was within
host variation, including occasional long samples at T511 and T720, with no
consistent regression. These results support retaining the small q1
improvement; they do not establish a q8 gain or DUCC parity.

The host was an AMD Ryzen 9 5950X (x86_64, 16 cores / 32 threads, 96 GiB RAM)
running Ubuntu 26.04.1 on Linux 7.0.14-20-pve. AVX2 and FMA are available;
Pulp runtime dispatch was used without global native-CPU build flags.

Validation passed with `cargo fmt --all -- --check`, workspace Clippy,
`cargo test --release --workspace --all-features`, and
`cargo test --release -p rustsht --no-default-features`. The release tests
include pinned DUCC GL spin-1 comparisons, reflected-recurrence checks,
exceptional-input fallback checks, cross-grid analysis/adjoint identities and
q1/q8 compacted execution checks.
