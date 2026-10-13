# T72–T720 FFT workload performance and algorithm assessment

Date: 2026-10-13. Branch: `feat/rustsht_fft`. The closeout source baseline before this Admiral follow-up was `fd7c1bedb3fbb8c5e3b75d64c92d199d6673f8d2`; the report follow-up is committed separately on this branch.

## Scope and summary

This report evaluates the RustSHT-FFT paths used by RustSHT atmospheric SHTs at T72, T127, T255, T360, T511 and T720. It covers longitude R2C/C2R, meridional C2C row batches, alternate valid longitude sample counts, the f64 real general-prime AVX2 change, and a bench-only Rader experiment inspired by Admiral.

The current RustSHT plan makes one reusable real FFT plan per longitude size. Scalar analysis and synthesis execute one longitude transform per latitude ring; spin-1 executes two. The meridional transform loops run over latitude/order profiles and also reuse FFT plans and workspaces. The row batches here reproduce those transform sizes, row counts, ordering, input layout, plan reuse and scratch reuse. They are not full RustSHT SHT wall times: Legendre recurrences, harmonic packing, grid weighting and the complete application pipeline were not changed or timed. See [the inspected call-site/workload map](rustsht-workloads.md).

The main production change is an f64 AVX2 kernel for real general-prime passes with `ido == 1`. Across the relevant scalar-row cases, paired process measurements show about 1.4–2.0× execution gains at N=513, 1025 and 1443, and also improve several shorter general-prime lengths. Runtime AVX2 dispatch selects the kernel when supported; other CPUs use the scalar fallback. The kernel does not change Bluestein selection, so N=257 and N=723 show about 1.0× in paired comparisons.

A follow-up C2C optimization caches phase indices for general-prime radix passes from 13 through 109, eliminating repeated phase modulo work inside each butterfly while keeping DUCC's factorization, roots, normalization and scalar algorithm. On N=722 and N=1442, three alternating baseline/candidate process pairs show repeated-row median gains of 1.20–1.24×. This campaign ran during heavy RustSHT and JAX compilation; its absolute times and ratios are provisional. The plan stores 342 bytes of phase indices at N=722 and 5,253 bytes at N=1442; reusable workspace size is unchanged. N=1442 plan construction rose by about 15–16 µs in the same contested campaign, a setup cost amortized across the repeatedly reused SHT plans.

The Rader prototype is bench-only. Against the current Bluestein route it gives about 4.1× on repeated SHT-shaped N=257 rows (single-transform gains are 3.3× for C2R and 4.0× for R2C) and about 2.3× at N=723. Direct-DFT, RustFFT and pinned DUCC comparisons pass. Production integration requires planner, real-pass scratch ownership and numerical fallback changes beyond this bounded experiment.

The largest bottleneck depends on the workload. N=257 and N=723 need a prime algorithm change. Before the C2C phase-index follow-up, T720 C2C at N=1442 was about 9.6× slower than RustFFT and 6.9× slower than native DUCC. The follow-up's source-relative gain is 1.23–1.24× on repeated rows, but no new cross-library campaign was run. The real general-prime kernel closes gaps at N=513 and N=1025. At T720's current odd longitude N=1443, RustSHT-FFT is 1.3–1.6× slower than RustFFT and 3.3–4.2× slower than native DUCC. The benchmark host ran the Legendre agent, so absolute times are provisional despite CPU affinity and repeated process samples.

### Branch and source commits

- Production real-prime kernel: [`1124c6eb7e79fedd0937a1856ef7efce74e77e81`](https://github.com/mwyau/RustSHT/commit/1124c6eb7e79fedd0937a1856ef7efce74e77e81).
- Workload and Rader comparison harness: [`3a737b04a2b8c60f316ca27cdd563890dbadded9`](https://github.com/mwyau/RustSHT/commit/3a737b04a2b8c60f316ca27cdd563890dbadded9).
- Compact M5 result tables: [`cfdf9ae570e2463ad57cde482e985bd86253b96a`](https://github.com/mwyau/RustSHT/commit/cfdf9ae570e2463ad57cde482e985bd86253b96a).
- Paired performance baseline for the f64 kernel: `a1240461ee7174404f76710e2bb1d7eb0d2371d8`.
- The T72–T720 benchmark report was committed after the source, harness and result commits. The M6 standalone-readiness audit began from clean branch tip `b92727ef16aef366a951347be1a9ac8646d03305`; its additions are recorded in later commits on `feat/rustsht_fft`.
- Pinned DUCC source: `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.
- Inspected Admiral checkout: `ce67bc00ff9e6de96bf5b75f7a789581105ddb58`.

## Workload dimensions and sampling constraints

For inclusive Fourier order L, `N=2L+1` and `N=2L+2` both preserve all orders `-L..L`. For the even case, the Nyquist order is `L+1` and is not used. The existing convention is `N=2L+3`; it keeps one additional margin bin. The FFT-only size comparison does not change RustSHT's public sampling or claim that a different grid is interchangeable for every complete SHT operation.

| T | minimum odd `2L+1` | minimum even `2L+2` | existing `2L+3` | measured friendly even | scalar rows | spin-1 rows |
|---:|---:|---:|---:|---:|---:|---:|
| 72 | 145 | 146 | 147 | 150 | 73 | 146 |
| 127 | 255 | 256 | 257 | 256 | 128 | 256 |
| 255 | 511 | 512 | 513 | 512 | 256 | 512 |
| 360 | 721 | 722 | 723 | 726 | 361 | 722 |
| 511 | 1023 | 1024 | 1025 | 1024 | 512 | 1024 |
| 720 | 1441 | 1442 | 1443 | 1452 | 721 | 1442 |

RustSHT's measured C2C meridional lengths include the original, prepared and full profile sizes. For the sampled workloads they include N=146/150/300 at T72; N=256 at T127; N=512 at T255; N=722/726/1452 at T360; N=1024 at T511; and N=1442/1452/2904 at T720. Repeated-row timings below execute rows serially and report per-row latency; a row batch is not a parallel or across-row SIMD transform.

## Measurement method and reproducibility

### Environment

- CPU: AMD Ryzen 9 5950X (Zen 3), 16 cores / 32 threads, AVX2. The measured process was pinned to logical CPU 30 with `taskset -c 30`; one transform thread was used.
- Rust: rustc 1.99.0 (`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`), LLVM 23.1.1; Cargo 1.99.0. Cargo `bench` profile (`opt-level=3`), no LTO, no global `target-cpu`, and no custom `RUSTFLAGS` for the portable runtime-dispatch path.
- RustFFT: 6.4.1, default planner/runtime CPU dispatch. The real adapter uses a half-length complex transform for even N and a full-length complex transform with Hermitian reconstruction for odd N.
- DUCC: direct C++ 1D plans at `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, g++ 15.2.0. The portable build uses `-std=c++20 -O3 -DNDEBUG -pthread` with DUCC vectorization disabled. The separately labeled native build uses `-std=c++20 -O3 -DNDEBUG -march=native -pthread` with DUCC's vectorized planner enabled. ISA differences are explicit; the native build is not presented as a portable comparison.
- RustSHT-FFT: portable Rust target with function-level AVX2 kernels selected by runtime CPU detection. A separate `-C target-cpu=native` Rust build is summarized below and was not used for the portable result columns.
- Frequency scaling was enabled. The host also ran the Legendre workload; process pinning prevents migration but does not prevent shared-cache, memory-bandwidth or package-power interference. Treat microsecond differences and absolute time as provisional.

The representative release benchmark binary fingerprints are Rust `rustfft` SHA-256 `66274ac64cbf755b5a59095d0bda42e0f4938891c4d4af9973f908aaade1201a`, current Rader validation/dump executable `caa6d12721afb8ce9f05b15bad8ce9068622cd314a3fe902f826338d00389ad8`, DUCC portable `fe241e7efa57953567675cc67d73d519488923887f9c32d0456e1f5eb1f7e86d`, and DUCC native `09fe220341903e95788d96983aeddb7652da5a1947ee01ae439b0322fdfce178`. Release binaries are not committed.

### Sampling and timings

Plans and workspaces were created before execution timings. All repeated row runs reused one plan, input/output storage and caller-owned scratch. Each operation used the same deterministic trigonometric inputs and spectrum formulas across libraries. The benchmark measured single transforms, scalar row counts `L+1`, and spin-1 row counts `2(L+1)` for f64. The f32 cross-library pass measured single transforms and 32 sequential rows. C2C measured single rows and RustSHT-shaped meridional row counts. Whole-call timing includes each library's public real packing, normalization and output conversion.

All reported throughput measurements use one FFT worker. An eight-worker throughput matrix was not collected. The inspected RustSHT longitude loops and RustSHT-FFT batch methods execute rows serially, so the reported row counts reproduce the current FFT call pattern; shared-plan concurrency is covered by correctness tests but its throughput under an external worker pool is unmeasured. These tables must not be interpreted as eight-thread results. Measure worker scaling with the actual SHT scheduler during integration, where task partitioning and workspace ownership can be included in the whole-operation timing.

The f64 real-prime candidate was compared with the source baseline in six alternating process pairs for each length, row class and direction. The reported candidate gain is the median within-pair baseline/candidate ratio; ranges show the minimum and maximum paired ratios. The reference timing columns come from six process-level cross-library samples over the same deterministic workload. f32 real comparisons used three process rounds. C2C cross-library row batches used six process samples; C2C single transforms used three. The factor-order screen used three process pairs, and the Rader comparison used three alternating baseline/prototype process pairs. Individual process samples use warmups and repeated timed calls; setup measurements use repeated plan/workspace constructions. Summary CSVs preserve the values used here.

The f64 real public-operation table uses the baseline cross-library measurements with the RustSHT-FFT candidate timing updated from the six paired source comparisons. RustFFT and DUCC runs were collected in separate process sequences using the same input distribution and binary configurations. This is a multi-process cross-library comparison, not a simultaneous run in one process. The short campaigns do not eliminate host contention.

### Commands

The following commands reproduce the principal benchmark entry points after the pinned DUCC source is checked out and its reference binaries are built with the flags above. The multi-process collector alternated implementation order and recorded process IDs, row count, setup, scratch, timings and checksums.

```sh
# Build the portable and native DUCC reference tools.
DUCC_SRC=/tmp/ducc-pinned CXXFLAGS='-O3 -DNDEBUG' \
  RUSTSHT_FFT_BUILD_DIR=/tmp/ducc-portable crates/rustsht-fft/benches/reference/build.sh
DUCC_SRC=/tmp/ducc-pinned CXXFLAGS='-O3 -DNDEBUG -march=native' \
  RUSTSHT_FFT_BUILD_DIR=/tmp/ducc-native crates/rustsht-fft/benches/reference/build.sh

# f64 real single transform and repeated scalar/spin-shaped rows.
taskset -c 30 cargo bench --manifest-path crates/rustsht-fft/Cargo.toml \
  --bench rustfft -- bench-native-real 1025 1 512 1024

# Compare the public R2C operation with the RustFFT adapter and DUCC's C++ tool.
taskset -c 30 cargo bench --manifest-path crates/rustsht-fft/Cargo.toml \
  --bench rustfft -- bench-one r2c 1025 512
RUSTSHT_FFT_DUCC_VECTORIZE=0 taskset -c 30 /tmp/ducc-portable/rustsht-fft-reference \
  bench-one r2c 1025 512
RUSTSHT_FFT_DUCC_VECTORIZE=1 taskset -c 30 /tmp/ducc-native/rustsht-fft-reference \
  bench-one r2c 1025 512

# f64 C2C single transform and a T720 meridional row batch.
taskset -c 30 cargo bench --manifest-path crates/rustsht-fft/Cargo.toml \
  --bench rustfft -- bench-native-c2c 1442 1 721

# f32 target-size cross-library screening (single and batch of 32 rows).
taskset -c 30 cargo bench --manifest-path crates/rustsht-fft/Cargo.toml \
  --bench f32_real_simd -- 145 146 147 150 255 256 257 511 512 513 \
  721 722 723 726 1023 1024 1025 1441 1442 1443 1452
DUCC_SRC=/tmp/ducc-pinned CXXFLAGS='-O3 -DNDEBUG' \
  RUSTSHT_FFT_BUILD_DIR=/tmp/ducc-f32-portable \
  crates/rustsht-fft/benches/reference/build-f32.sh
DUCC_SRC=/tmp/ducc-pinned CXXFLAGS='-O3 -DNDEBUG -march=native' \
  RUSTSHT_FFT_BUILD_DIR=/tmp/ducc-f32-native \
  crates/rustsht-fft/benches/reference/build-f32.sh
RUSTSHT_FFT_DUCC_VECTORIZE=0 taskset -c 30 \
  /tmp/ducc-f32-portable/rustsht-fft-reference-f32 bench-one r2c 1025 32
RUSTSHT_FFT_DUCC_VECTORIZE=1 taskset -c 30 \
  /tmp/ducc-f32-native/rustsht-fft-reference-f32 bench-one r2c 1025 32

# Isolated real Rader prototype: validation and a single timed invocation.
taskset -c 30 cargo bench --manifest-path crates/rustsht-fft/Cargo.toml \
  --bench rader_real_prototype -- validate
taskset -c 30 cargo bench --manifest-path crates/rustsht-fft/Cargo.toml \
  --bench rader_real_prototype -- bench rader 257 128 r2c
```

The C++ reference tool also accepts `dump MODE N` for per-output comparisons. The exact compact process summaries are [`m5-sht-real-f64-summary.csv`](../benches/results/m5-sht-real-f64-summary.csv), [`m5-sht-real-f32-summary.csv`](../benches/results/m5-sht-real-f32-summary.csv), [`m5-sht-c2c-summary.csv`](../benches/results/m5-sht-c2c-summary.csv), [`m5-phase-breakdown-screen.csv`](../benches/results/m5-phase-breakdown-screen.csv), [`m5-rader-prototype-summary.csv`](../benches/results/m5-rader-prototype-summary.csv), [`m5-factor-order-screen.csv`](../benches/results/m5-factor-order-screen.csv), [`m5-rust-native-target-summary.csv`](../benches/results/m5-rust-native-target-summary.csv), and [`m5-f32-accuracy.csv`](../benches/results/m5-f32-accuracy.csv). Raw M5 process logs and binaries are uncommitted files under `/tmp`.

## f64 longitude R2C/C2R results

Each timing cell is `RustSHT-FFT / RustFFT / DUCC portable / DUCC native`, in microseconds per transform. Ratios are `RustSHT-FFT / RustFFT`, `RustSHT-FFT / DUCC portable`, and `RustSHT-FFT / DUCC native`; below 1 means RustSHT-FFT is faster. `single`, `scalar` and `spin1` rows distinguish one transform, `L+1` serial transforms and `2(L+1)` serial transforms. The paired-gain cell gives the f64 real-prime kernel candidate gain over source `a124` and its observed pair range; values near 1 mean no measured change.

In compact table headers, `DFFT` means RustSHT-FFT.

| T | nφ role | N | row pattern | rows | R2C times, µs (DFFT / RustFFT / DUCC portable / DUCC native) | R2C ratios (DFFT / RustFFT / DUCC p / DUCC n) | paired gain vs a124 | C2R times, µs (DFFT / RustFFT / DUCC p / DUCC n) | C2R ratios | paired gain vs a124 |
|---:|---|---:|---|---:|---|---|---:|---|---|---:|
| 72 | min-odd | 145 | single | 1 | 0.756 / 0.922 / 0.762 / 0.847 | 0.82 / 0.99 / 0.89 | 1.73× (1.32–1.78×) | 0.791 / 0.932 / 0.767 / 0.691 | 0.85 / 1.03 / 1.15 | 1.65× (1.64–1.67×) |
| 72 | min-odd | 145 | scalar | 73 | 0.711 / 0.906 / 0.741 / 0.686 | 0.79 / 0.96 / 1.04 | 1.77× (1.70–2.09×) | 0.769 / 0.921 / 0.724 / 0.660 | 0.83 / 1.06 / 1.16 | 1.68× (1.63–1.68×) |
| 72 | min-odd | 145 | spin1 | 146 | 0.616 / 0.920 / 0.760 / 0.695 | 0.67 / 0.81 / 0.89 | 2.08× (2.05–2.16×) | 0.770 / 0.932 / 0.746 / 0.677 | 0.83 / 1.03 / 1.14 | 1.67× (1.66–1.68×) |
| 72 | min-even | 146 | single | 1 | 1.558 / 0.641 / 2.400 / 2.249 | 2.43 / 0.65 / 0.69 | 1.80× (1.44–2.47×) | 1.482 / 0.511 / 2.474 / 2.309 | 2.90 / 0.60 / 0.64 | 1.81× (1.81–1.82×) |
| 72 | min-even | 146 | scalar | 73 | 1.310 / 0.559 / 1.912 / 2.242 | 2.34 / 0.69 / 0.58 | 2.10× (1.86–2.29×) | 1.462 / 0.487 / 1.986 / 2.264 | 3.00 / 0.74 / 0.65 | 1.83× (1.82–1.84×) |
| 72 | min-even | 146 | spin1 | 146 | 1.199 / 0.558 / 1.921 / 2.244 | 2.15 / 0.62 / 0.53 | 2.28× (2.28–2.31×) | 1.460 / 0.486 / 2.004 / 2.283 | 3.00 / 0.73 / 0.64 | 1.83× (1.83–1.84×) |
| 72 | current | 147 | single | 1 | 1.167 / 0.431 / 0.521 / 0.496 | 2.71 / 2.24 / 2.35 | 1.01× (0.96–1.04×) | 1.342 / 0.381 / 0.601 / 0.476 | 3.52 / 2.23 / 2.82 | 0.99× (0.93–1.01×) |
| 72 | current | 147 | scalar | 73 | 1.139 / 0.350 / 0.521 / 0.491 | 3.25 / 2.18 / 2.32 | 1.01× (0.97–1.02×) | 1.294 / 0.376 / 0.570 / 0.446 | 3.44 / 2.27 / 2.90 | 1.00× (0.99–1.02×) |
| 72 | current | 147 | spin1 | 146 | 1.143 / 0.351 / 0.502 / 0.480 | 3.26 / 2.28 / 2.38 | 1.00× (0.96–1.02×) | 1.299 / 0.369 / 0.563 / 0.449 | 3.52 / 2.31 / 2.89 | 1.00× (0.99–1.02×) |
| 72 | friendly-even | 150 | single | 1 | 0.481 / 0.401 / 0.361 / 0.345 | 1.20 / 1.33 / 1.39 | 1.25× (0.83–1.27×) | 0.461 / 0.331 / 0.351 / 0.296 | 1.40 / 1.31 / 1.56 | 1.00× (0.92–1.04×) |
| 72 | friendly-even | 150 | scalar | 73 | 0.457 / 0.385 / 0.355 / 0.333 | 1.19 / 1.29 / 1.37 | 1.05× (1.03–1.07×) | 0.430 / 0.310 / 0.329 / 0.245 | 1.39 / 1.31 / 1.75 | 0.98× (0.92–1.00×) |
| 72 | friendly-even | 150 | spin1 | 146 | 0.451 / 0.388 / 0.347 / 0.333 | 1.16 / 1.30 / 1.35 | 1.04× (1.02–1.05×) | 0.433 / 0.315 / 0.339 / 0.254 | 1.37 / 1.27 / 1.70 | 0.99× (0.93–1.02×) |
| 127 | min-odd | 255 | single | 1 | 1.037 / 0.962 / 1.077 / 0.751 | 1.08 / 0.96 / 1.38 | 1.54× (1.23–1.91×) | 1.122 / 1.032 / 0.912 / 0.711 | 1.09 / 1.23 / 1.58 | 1.51× (1.49–1.54×) |
| 127 | min-odd | 255 | scalar | 128 | 0.946 / 0.872 / 0.852 / 0.761 | 1.08 / 1.11 / 1.24 | 1.67× (1.66–1.69×) | 1.113 / 0.908 / 0.864 / 0.691 | 1.23 / 1.29 / 1.61 | 1.51× (1.50–1.52×) |
| 127 | min-odd | 255 | spin1 | 256 | 0.948 / 0.871 / 0.868 / 0.777 | 1.09 / 1.09 / 1.22 | 1.68× (1.66–1.69×) | 1.115 / 0.908 / 0.864 / 0.692 | 1.23 / 1.29 / 1.61 | 1.50× (1.50–1.51×) |
| 127 | min-even;friendly-even | 256 | single | 1 | 0.707 / 0.501 / 0.495 / 0.556 | 1.41 / 1.43 / 1.27 | 1.00× (0.98–1.26×) | 0.571 / 0.391 / 0.546 / 0.486 | 1.46 / 1.05 / 1.18 | 1.01× (1.00–1.03×) |
| 127 | min-even;friendly-even | 256 | scalar | 128 | 0.677 / 0.499 / 0.484 / 0.509 | 1.36 / 1.40 / 1.33 | 1.00× (0.98–1.01×) | 0.560 / 0.386 / 0.520 / 0.493 | 1.45 / 1.08 / 1.14 | 1.01× (0.98–1.02×) |
| 127 | min-even;friendly-even | 256 | spin1 | 256 | 0.682 / 0.499 / 0.500 / 0.518 | 1.36 / 1.36 / 1.32 | 0.99× (0.97–1.01×) | 0.564 / 0.385 / 0.521 / 0.486 | 1.47 / 1.08 / 1.16 | 1.00× (0.98–1.02×) |
| 127 | current | 257 | single | 1 | 10.419 / 1.232 / 4.839 / 2.930 | 8.46 / 2.15 / 3.56 | 1.00× (0.99–1.24×) | 10.550 / 1.278 / 4.954 / 3.041 | 8.26 / 2.13 / 3.47 | 1.00× (0.99–1.01×) |
| 127 | current | 257 | scalar | 128 | 10.378 / 1.226 / 4.816 / 2.838 | 8.46 / 2.15 / 3.66 | 1.00× (0.98–1.01×) | 10.554 / 1.250 / 4.845 / 2.864 | 8.44 / 2.18 / 3.69 | 1.00× (0.99–1.01×) |
| 127 | current | 257 | spin1 | 256 | 10.426 / 1.227 / 4.860 / 2.920 | 8.50 / 2.15 / 3.57 | 1.00× (0.98–1.01×) | 10.533 / 1.261 / 4.883 / 2.915 | 8.36 / 2.16 / 3.61 | 1.00× (0.99–1.01×) |
| 255 | min-odd | 511 | single | 1 | 7.043 / 2.885 / 4.543 / 4.358 | 2.44 / 1.55 / 1.62 | 1.65× (1.43–2.06×) | 7.609 / 2.960 / 4.859 / 5.000 | 2.57 / 1.57 / 1.52 | 1.54× (1.53–1.59×) |
| 255 | min-odd | 511 | scalar | 256 | 6.142 / 2.892 / 4.281 / 4.516 | 2.12 / 1.44 / 1.36 | 1.89× (1.86–1.91×) | 7.639 / 2.949 / 4.391 / 4.499 | 2.59 / 1.74 / 1.70 | 1.55× (1.54–1.60×) |
| 255 | min-odd | 511 | spin1 | 512 | 6.146 / 2.900 / 4.321 / 4.533 | 2.12 / 1.42 / 1.36 | 1.89× (1.86–1.91×) | 7.642 / 2.951 / 4.407 / 4.493 | 2.59 / 1.73 / 1.70 | 1.55× (1.54–1.59×) |
| 255 | min-even;friendly-even | 512 | single | 1 | 1.649 / 1.007 / 1.167 / 1.147 | 1.64 / 1.41 / 1.44 | 0.98× (0.81–1.00×) | 1.192 / 0.782 / 1.177 / 1.016 | 1.52 / 1.01 / 1.17 | 1.00× (1.00–1.05×) |
| 255 | min-even;friendly-even | 512 | scalar | 256 | 1.416 / 1.028 / 1.147 / 1.201 | 1.38 / 1.23 / 1.18 | 1.00× (0.99–1.01×) | 1.193 / 0.796 / 1.168 / 1.018 | 1.50 / 1.02 / 1.17 | 1.00× (1.00–1.04×) |
| 255 | min-even;friendly-even | 512 | spin1 | 512 | 1.421 / 1.032 / 1.145 / 1.196 | 1.38 / 1.24 / 1.19 | 1.00× (0.98–1.01×) | 1.201 / 0.800 / 1.175 / 1.027 | 1.50 / 1.02 / 1.17 | 1.00× (0.99–1.04×) |
| 255 | current | 513 | single | 1 | 2.475 / 2.570 / 1.808 / 1.613 | 0.96 / 1.37 / 1.53 | 1.47× (1.40–1.78×) | 2.584 / 2.424 / 2.013 / 1.493 | 1.07 / 1.28 / 1.73 | 1.46× (1.40–1.50×) |
| 255 | current | 513 | scalar | 256 | 2.306 / 2.332 / 1.694 / 1.612 | 0.99 / 1.36 / 1.43 | 1.57× (1.56–1.59×) | 2.604 / 2.397 / 1.818 / 1.349 | 1.09 / 1.43 / 1.93 | 1.44× (1.42–1.48×) |
| 255 | current | 513 | spin1 | 512 | 2.306 / 2.342 / 1.689 / 1.607 | 0.98 / 1.36 / 1.44 | 1.57× (1.57–1.59×) | 2.604 / 2.400 / 1.831 / 1.362 | 1.08 / 1.42 / 1.91 | 1.44× (1.42–1.47×) |
| 360 | min-odd | 721 | single | 1 | 13.796 / 6.267 / 8.371 / 8.165 | 2.20 / 1.65 / 1.69 | 1.53× (1.43–1.75×) | 13.099 / 6.342 / 8.852 / 8.706 | 2.06 / 1.48 / 1.50 | 1.63× (1.62–1.65×) |
| 360 | min-odd | 721 | scalar | 361 | 11.138 / 6.309 / 7.982 / 8.420 | 1.76 / 1.40 / 1.32 | 1.92× (1.87–1.97×) | 13.135 / 6.359 / 8.087 / 8.421 | 2.06 / 1.62 / 1.56 | 1.63× (1.62–1.64×) |
| 360 | min-odd | 721 | spin1 | 722 | 11.086 / 6.332 / 7.974 / 8.459 | 1.75 / 1.39 / 1.31 | 1.93× (1.88–1.97×) | 13.108 / 6.406 / 8.139 / 8.493 | 2.05 / 1.61 / 1.54 | 1.64× (1.62–1.65×) |
| 360 | min-even | 722 | single | 1 | 10.835 / 4.328 / 3.126 / 2.380 | 2.50 / 3.47 / 4.55 | 1.15× (0.94–1.27×) | 12.298 / 4.138 / 3.496 / 2.700 | 2.97 / 3.52 / 4.55 | 1.12× (1.10–1.13×) |
| 360 | min-even | 722 | scalar | 361 | 10.588 / 4.323 / 2.804 / 2.389 | 2.45 / 3.77 / 4.43 | 1.17× (1.17–1.18×) | 12.445 / 4.031 / 3.003 / 2.156 | 3.09 / 4.14 / 5.77 | 1.12× (1.10–1.12×) |
| 360 | min-even | 722 | spin1 | 722 | 10.584 / 4.378 / 2.817 / 2.403 | 2.42 / 3.76 / 4.41 | 1.17× (1.16–1.18×) | 12.446 / 4.038 / 3.024 / 2.177 | 3.08 / 4.12 / 5.72 | 1.12× (1.10–1.13×) |
| 360 | current | 723 | single | 1 | 32.986 / 4.704 / 16.732 / 11.837 | 7.01 / 1.97 / 2.79 | 1.01× (0.98–1.02×) | 33.092 / 4.874 / 16.906 / 11.973 | 6.79 / 1.96 / 2.76 | 1.01× (0.98–1.02×) |
| 360 | current | 723 | scalar | 361 | 32.942 / 4.700 / 16.680 / 11.692 | 7.01 / 1.98 / 2.82 | 1.01× (0.98–1.03×) | 33.426 / 4.773 / 16.839 / 11.585 | 7.00 / 1.99 / 2.88 | 1.00× (0.99–1.02×) |
| 360 | current | 723 | spin1 | 722 | 32.990 / 4.739 / 16.756 / 11.693 | 6.96 / 1.97 / 2.82 | 1.01× (0.99–1.03×) | 33.505 / 4.804 / 17.034 / 11.763 | 6.97 / 1.97 / 2.85 | 1.00× (0.99–1.02×) |
| 360 | friendly-even | 726 | single | 1 | 8.211 / 1.839 / 2.560 / 2.214 | 4.47 / 3.21 / 3.71 | 1.00× (0.81–1.01×) | 9.223 / 1.498 / 3.071 / 2.168 | 6.16 / 3.00 / 4.25 | 1.00× (0.99–1.01×) |
| 360 | friendly-even | 726 | scalar | 361 | 8.241 / 1.833 / 2.395 / 2.227 | 4.50 / 3.44 / 3.70 | 1.00× (0.99–1.01×) | 9.338 / 1.513 / 2.666 / 1.903 | 6.17 / 3.50 / 4.91 | 1.00× (0.99–1.01×) |
| 360 | friendly-even | 726 | spin1 | 722 | 8.259 / 1.840 / 2.390 / 2.242 | 4.49 / 3.46 / 3.68 | 1.00× (0.99–1.01×) | 9.332 / 1.515 / 2.680 / 1.914 | 6.16 / 3.48 / 4.88 | 1.00× (0.99–1.01×) |
| 511 | min-odd | 1023 | single | 1 | 12.037 / 7.644 / 4.534 / 3.517 | 1.57 / 2.65 / 3.42 | 1.33× (1.26–1.40×) | 13.701 / 7.864 / 5.135 / 3.426 | 1.74 / 2.67 / 4.00 | 1.27× (1.25–1.29×) |
| 511 | min-odd | 1023 | scalar | 512 | 11.402 / 7.567 / 4.284 / 3.529 | 1.51 / 2.66 / 3.23 | 1.41× (1.39–1.42×) | 13.820 / 7.670 / 4.614 / 3.208 | 1.80 / 3.00 / 4.31 | 1.27× (1.25–1.30×) |
| 511 | min-odd | 1023 | spin1 | 1024 | 11.504 / 8.048 / 4.433 / 3.666 | 1.43 / 2.60 / 3.14 | 1.41× (1.39–1.43×) | 13.866 / 8.133 / 4.890 / 3.624 | 1.71 / 2.83 / 3.83 | 1.26× (1.25–1.29×) |
| 511 | min-even;friendly-even | 1024 | single | 1 | 3.076 / 2.184 / 2.334 / 2.414 | 1.41 / 1.32 / 1.27 | 0.99× (0.83–1.03×) | 2.465 / 1.728 / 2.689 / 2.109 | 1.43 / 0.92 / 1.17 | 1.01× (1.00–1.02×) |
| 511 | min-even;friendly-even | 1024 | scalar | 512 | 3.003 / 2.233 / 2.290 / 2.496 | 1.34 / 1.31 / 1.20 | 0.99× (0.98–1.00×) | 2.492 / 1.725 / 2.355 / 2.164 | 1.45 / 1.06 / 1.15 | 1.00× (0.99–1.02×) |
| 511 | min-even;friendly-even | 1024 | spin1 | 1024 | 3.017 / 2.337 / 2.418 / 2.660 | 1.29 / 1.25 / 1.13 | 1.00× (0.97–1.00×) | 2.506 / 1.843 / 2.610 / 2.452 | 1.36 / 0.96 / 1.02 | 1.00× (1.00–1.00×) |
| 511 | current | 1025 | single | 1 | 7.113 / 6.096 / 5.340 / 3.672 | 1.17 / 1.33 / 1.94 | 1.72× (1.69–1.94×) | 7.264 / 6.332 / 5.064 / 3.987 | 1.15 / 1.43 / 1.82 | 1.69× (1.68–1.70×) |
| 511 | current | 1025 | scalar | 512 | 6.036 / 6.171 / 4.901 / 3.684 | 0.98 / 1.23 / 1.64 | 2.04× (2.02–2.05×) | 7.281 / 6.290 / 4.627 / 3.333 | 1.16 / 1.57 / 2.19 | 1.70× (1.69–1.72×) |
| 511 | current | 1025 | spin1 | 1024 | 6.077 / 6.679 / 5.014 / 3.818 | 0.91 / 1.21 / 1.59 | 2.04× (2.02–2.07×) | 7.293 / 6.760 / 4.897 / 3.575 | 1.08 / 1.49 / 2.04 | 1.70× (1.68–1.73×) |
| 720 | min-odd | 1441 | single | 1 | 29.511 / 12.508 / 18.073 / 16.240 | 2.36 / 1.63 / 1.82 | 1.85× (1.56–1.90×) | 34.574 / 12.854 / 19.085 / 16.300 | 2.69 / 1.81 / 2.12 | 1.60× (1.60–1.62×) |
| 720 | min-odd | 1441 | scalar | 721 | 30.201 / 13.527 / 17.776 / 16.697 | 2.23 / 1.70 / 1.81 | 1.84× (1.83–1.86×) | 34.991 / 13.464 / 18.210 / 16.769 | 2.60 / 1.92 / 2.09 | 1.60× (1.56–1.61×) |
| 720 | min-odd | 1441 | spin1 | 1442 | 30.254 / 13.959 / 18.356 / 17.100 | 2.17 / 1.65 / 1.77 | 1.84× (1.83–1.89×) | 34.991 / 13.955 / 18.896 / 17.268 | 2.51 / 1.85 / 2.03 | 1.60× (1.59–1.62×) |
| 720 | min-even | 1442 | single | 1 | 62.622 / 8.130 / 15.464 / 12.418 | 7.70 / 4.05 / 5.04 | 0.96× (0.96–0.97×) | 64.070 / 7.469 / 15.759 / 13.044 | 8.58 / 4.07 / 4.91 | 1.00× (1.00–1.00×) |
| 720 | min-even | 1442 | scalar | 721 | 63.255 / 8.449 / 14.610 / 11.850 | 7.49 / 4.33 / 5.34 | 0.96× (0.95–0.97×) | 64.793 / 7.801 / 14.767 / 12.164 | 8.30 / 4.39 / 5.33 | 1.00× (1.00–1.01×) |
| 720 | min-even | 1442 | spin1 | 1442 | 63.259 / 8.671 / 15.188 / 12.384 | 7.29 / 4.17 / 5.11 | 0.96× (0.96–0.97×) | 64.760 / 8.173 / 15.356 / 12.284 | 7.92 / 4.22 / 5.27 | 1.00× (1.00–1.01×) |
| 720 | current | 1443 | single | 1 | 19.180 / 13.190 / 7.038 / 5.400 | 1.45 / 2.73 / 3.55 | 1.35× (1.32–1.45×) | 21.780 / 13.159 / 7.835 / 5.169 | 1.66 / 2.78 / 4.21 | 1.29× (1.28–1.31×) |
| 720 | current | 1443 | scalar | 721 | 18.180 / 14.205 / 6.595 / 5.529 | 1.28 / 2.76 / 3.29 | 1.45× (1.44–1.50×) | 22.174 / 14.294 / 7.243 / 5.319 | 1.55 / 3.06 / 4.17 | 1.28× (1.27–1.29×) |
| 720 | current | 1443 | spin1 | 1442 | 18.495 / 14.826 / 7.380 / 6.333 | 1.25 / 2.51 / 2.92 | 1.43× (1.42–1.46×) | 22.346 / 14.813 / 8.029 / 5.926 | 1.51 / 2.78 / 3.77 | 1.27× (1.26–1.28×) |
| 720 | friendly-even | 1452 | single | 1 | 8.591 / 3.817 / 5.039 / 3.777 | 2.25 / 1.71 / 2.27 | 1.01× (0.97–1.06×) | 10.069 / 3.236 / 5.425 / 4.018 | 3.11 / 1.86 / 2.51 | 0.99× (0.98–1.00×) |
| 720 | friendly-even | 1452 | scalar | 721 | 8.629 / 3.939 / 5.469 / 4.132 | 2.19 / 1.58 / 2.09 | 1.02× (1.01–1.03×) | 10.190 / 3.326 / 5.994 / 4.472 | 3.06 / 1.70 / 2.28 | 1.00× (0.99–1.01×) |
| 720 | friendly-even | 1452 | spin1 | 1442 | 8.896 / 4.287 / 6.102 / 4.883 | 2.08 / 1.46 / 1.82 | 1.02× (1.00–1.02×) | 10.442 / 3.772 / 6.694 / 5.108 | 2.77 / 1.56 / 2.04 | 1.00× (0.98–1.01×) |

### Paired f64 real general-prime kernel change

The new kernel hoists phase lookup into plan-time cosine/sine tables and evaluates multiple branch frequencies with AVX2 for eligible f64 passes. The planner only installs it when `ido == 1`, the prime factor is in the current non-Bluestein general-prime range, and runtime AVX2 detection succeeds. Scalar fallback and existing inverse-safety routing are unchanged. No production planner threshold changed. These six-pair results are per scalar-row workload; paired ranges and control lengths are included.

| N | R2C candidate gain vs a124 (range) | C2R candidate gain vs a124 (range) |
|---:|---:|---:|
| 145 | 1.77× (1.70–2.09×) | 1.68× (1.63–1.68×) |
| 146 | 2.10× (1.86–2.29×) | 1.83× (1.82–1.84×) |
| 147 | 1.01× (0.97–1.02×) | 1.00× (0.99–1.02×) |
| 150 | 1.05× (1.03–1.07×) | 0.98× (0.92–1.00×) |
| 255 | 1.67× (1.66–1.69×) | 1.51× (1.50–1.52×) |
| 256 | 1.00× (0.98–1.01×) | 1.01× (0.98–1.02×) |
| 257 | 1.00× (0.98–1.01×) | 1.00× (0.99–1.01×) |
| 511 | 1.89× (1.86–1.91×) | 1.55× (1.54–1.60×) |
| 512 | 1.00× (0.99–1.01×) | 1.00× (1.00–1.04×) |
| 513 | 1.57× (1.56–1.59×) | 1.44× (1.42–1.48×) |
| 721 | 1.92× (1.87–1.97×) | 1.63× (1.62–1.64×) |
| 722 | 1.17× (1.17–1.18×) | 1.12× (1.10–1.12×) |
| 723 | 1.01× (0.98–1.03×) | 1.00× (0.99–1.02×) |
| 726 | 1.00× (0.99–1.01×) | 1.00× (0.99–1.01×) |
| 1023 | 1.41× (1.39–1.42×) | 1.27× (1.25–1.30×) |
| 1024 | 0.99× (0.98–1.00×) | 1.00× (0.99–1.02×) |
| 1025 | 2.04× (2.02–2.05×) | 1.70× (1.69–1.72×) |
| 1441 | 1.84× (1.83–1.86×) | 1.60× (1.56–1.61×) |
| 1442 | 0.96× (0.95–0.97×) | 1.00× (1.00–1.01×) |
| 1443 | 1.45× (1.44–1.50×) | 1.28× (1.27–1.29×) |
| 1452 | 1.02× (1.01–1.03×) | 1.00× (0.99–1.01×) |

The strongest relevant effects are approximately 1.57× R2C and 1.44× C2R at N=513, 2.04× and 1.70× at N=1025, and 1.45× and 1.28× at N=1443. N=257 and N=723 show about 1.0× because their current real planner routes the large prime factor through Bluestein. The candidate plan's additional f64 phase tables are about 20.7 KiB for radix 73, 41.6 KiB for radix 103 and 67.6 KiB for radix 131; the workspace byte count does not change. `general_prime_ido1` occupies 1,801 bytes in the inspected optimized benchmark binary. The overall `rustfft` benchmark binary text grew by about 6.6 KiB versus the recorded pre-change build; this includes test/benchmark linkage and is not a pure kernel attribution.

### Setup and reusable scratch

The following table reports f64 setup for each unique tested longitude size. Plan construction and workspace allocation are microseconds. Scratch bytes count reusable work buffers and exclude caller input/output buffers and allocator metadata. The competitor setup numbers are the six-process cross-library sample medians; only RustSHT-FFT plan/workspace values include the new candidate source.

| N | DFFT plan / workspace, µs | DFFT scratch B | RustFFT plan, µs / scratch B | DUCC portable plan, µs / scratch B | DUCC native plan, µs / scratch B |
|---:|---:|---:|---:|---:|---:|
| 145 | 4.591 / 0.871 | 5376 | 5.455 / 4640 | 0.456 / 1160 | 0.492 / 1160 |
| 146 | 8.826 / 0.812 | 8192 | 9.424 / 2336 | 0.598 / 1168 | 0.490 / 1168 |
| 147 | 3.712 / 0.662 | 4448 | 6.866 / 4704 | 0.536 / 1176 | 0.523 / 1176 |
| 150 | 3.749 / 0.695 | 4800 | 5.333 / 2400 | 0.565 / 1200 | 0.539 / 1200 |
| 255 | 5.882 / 0.654 | 7952 | 8.945 / 8160 | 0.853 / 2040 | 0.757 / 2040 |
| 256 | 5.434 / 0.780 | 7232 | 6.165 / 4096 | 0.707 / 2048 | 0.762 / 2048 |
| 257 | 22.053 / 1.752 | 37584 | 18.635 / 8224 | 5.104 / 27080 | 4.221 / 27080 |
| 511 | 15.337 / 1.151 | 16992 | 19.042 / 16352 | 0.933 / 4088 | 0.884 / 4088 |
| 512 | 11.155 / 1.065 | 15456 | 11.191 / 8192 | 1.063 / 4096 | 1.036 / 4096 |
| 513 | 10.231 / 1.073 | 15424 | 15.857 / 16416 | 1.088 / 4104 | 1.032 / 4104 |
| 721 | 25.242 / 1.205 | 23952 | 25.483 / 23072 | 1.133 / 5768 | 1.138 / 5768 |
| 722 | 14.827 / 1.222 | 21600 | 49.657 / 30352 | 1.140 / 5776 | 1.112 / 5776 |
| 723 | 31.017 / 2.283 | 48640 | 30.610 / 23136 | 5.823 / 28984 | 5.217 / 28984 |
| 726 | 13.792 / 1.375 | 22128 | 15.287 / 11616 | 1.253 / 5808 | 1.228 / 5808 |
| 1023 | 19.799 / 1.386 | 29376 | 27.016 / 32736 | 1.467 / 8184 | 1.369 / 8184 |
| 1024 | 19.968 / 1.504 | 27776 | 21.298 / 16384 | 1.482 / 8192 | 1.405 / 8192 |
| 1025 | 20.293 / 1.337 | 28944 | 28.896 / 32800 | 1.482 / 8200 | 1.359 / 8200 |
| 1441 | 43.269 / 2.216 | 43056 | 43.047 / 46112 | 1.748 / 11528 | 1.618 / 11528 |
| 1442 | 33.734 / 1.607 | 37952 | 32.768 / 23072 | 2.342 / 11536 | 2.102 / 11536 |
| 1443 | 27.309 / 1.840 | 40976 | 79.455 / 77280 | 1.694 / 11544 | 1.586 / 11544 |
| 1452 | 36.044 / 1.703 | 35248 | 26.427 / 23232 | 2.156 / 11616 | 2.053 / 11616 |

### Packing, computation and whole-call cost

`m5-phase-breakdown-screen.csv` separates kernel, adapter conversion and whole public-call timing for the six existing `2L+3` longitude sizes. These phase values are from the pre-`1124c6e` cross-library phase campaign; the RustSHT-FFT AVX2 change does not modify its public pack/unpack wrapper. The table retains this source note so the old phase data are not mistaken for candidate measurements. RustSHT-FFT's public real API combines packing, the multipass/general-prime kernels, normalization and output conversion; that harness does not split the internal phase without an invasive benchmark hook. RustFFT and DUCC expose adapter phases in their harnesses. Their phase tuple is kernel / adapter pack-and-copy / whole public operation, in microseconds.

| T | N | direction | RustSHT-FFT whole public, µs | RustFFT kernel / adapter / whole, µs | DUCC portable kernel / adapter / whole, µs | DUCC native kernel / adapter / whole, µs |
|---:|---:|---|---:|---|---|---|
| 72 | 147 | R2C | 1.192 | 0.381 / 0.075 / 0.431 | 0.476 / 0.070 / 0.521 | 0.466 / 0.060 / 0.496 |
| 72 | 147 | C2R | 1.373 | 0.321 / 0.090 / 0.381 | 0.521 / 0.090 / 0.601 | 0.431 / 0.090 / 0.476 |
| 127 | 257 | R2C | 10.399 | 1.162 / 0.091 / 1.232 | 4.764 / 0.091 / 4.839 | 2.860 / 0.075 / 2.930 |
| 127 | 257 | C2R | 10.520 | 1.167 / 0.150 / 1.278 | 5.360 / 0.135 / 4.954 | 2.941 / 0.130 / 3.041 |
| 255 | 513 | R2C | 3.617 | 2.414 / 0.190 / 2.570 | 1.643 / 0.166 / 1.808 | 1.498 / 0.141 / 1.613 |
| 255 | 513 | C2R | 3.757 | 2.139 / 0.290 / 2.424 | 1.754 / 0.215 / 2.013 | 1.452 / 0.245 / 1.493 |
| 360 | 723 | R2C | 33.252 | 4.473 / 0.226 / 4.704 | 16.531 / 0.200 / 16.732 | 11.726 / 0.130 / 11.837 |
| 360 | 723 | C2R | 33.568 | 4.489 / 0.386 / 4.874 | 16.606 / 0.281 / 16.906 | 11.621 / 0.280 / 11.973 |
| 511 | 1025 | R2C | 12.313 | 5.746 / 0.320 / 6.096 | 4.920 / 0.316 / 5.340 | 3.471 / 0.185 / 3.672 |
| 511 | 1025 | C2R | 12.233 | 5.746 / 0.551 / 6.332 | 4.599 / 0.406 / 5.064 | 3.522 / 0.456 / 3.987 |
| 720 | 1443 | R2C | 25.799 | 12.689 / 0.536 / 13.190 | 6.773 / 0.370 / 7.038 | 5.100 / 0.226 / 5.400 |
| 720 | 1443 | C2R | 28.012 | 12.573 / 0.651 / 13.159 | 7.123 / 0.530 / 7.835 | 4.563 / 0.521 / 5.169 |

Plan construction, workspace allocation and scratch capacity are separate from the execution call in the f64 and f32 summaries. This phase table shows that RustFFT's even-length half-complex adapter conversion can be a meaningful part of a short FFT call (for example, N=1024), while the DUCC adapter conversion is smaller in these samples. It does not identify internal RustSHT-FFT packing as the source of its remaining T720 prime-stage cost. Previous real pack and terminal-pass experiments are documented in [the M4 real multipass report](m4-real-multipass.md) and [the inverse spectrum-pack report](m4-real-inverse-pack.md).

## f32 real-transform results and accuracy

The f32 cross-library harness uses the same target sizes and distinguishes single transforms from a reusable-plan batch of 32 sequential rows. The adapter converts real values to a half-length complex transform for even N and performs a full complex transform for odd N. Times and ratios use the same column conventions as the f64 table.

| T | nφ role | N | call pattern | rows | R2C times, µs (DFFT / RustFFT / DUCC portable / DUCC native) | R2C ratios | C2R times, µs | C2R ratios |
|---:|---|---:|---|---:|---|---|---|---|
| 72 | min-odd | 145 | single | 1 | 1.229 / 0.632 / 0.684 / 0.656 | 1.94 / 1.80 / 1.87 | 1.283 / 0.724 / 0.671 / 0.688 | 1.77 / 1.91 / 1.86 |
| 72 | min-odd | 145 | batch32 | 32 | 1.232 / 0.627 / 0.680 / 0.663 | 1.96 / 1.81 / 1.86 | 1.286 / 0.729 / 0.666 / 0.686 | 1.76 / 1.93 / 1.87 |
| 72 | min-even | 146 | single | 1 | 2.715 / 0.391 / 2.481 / 1.933 | 6.94 / 1.09 / 1.40 | 2.698 / 0.304 / 2.474 / 2.006 | 8.89 / 1.09 / 1.34 |
| 72 | min-even | 146 | batch32 | 32 | 2.729 / 0.392 / 2.483 / 1.935 | 6.96 / 1.10 / 1.41 | 2.703 / 0.303 / 2.472 / 2.006 | 8.91 / 1.09 / 1.35 |
| 72 | current | 147 | single | 1 | 1.111 / 0.267 / 0.475 / 0.410 | 4.17 / 2.34 / 2.71 | 1.316 / 0.372 / 0.447 / 0.416 | 3.54 / 2.94 / 3.17 |
| 72 | current | 147 | batch32 | 32 | 1.115 / 0.266 / 0.475 / 0.408 | 4.19 / 2.35 / 2.73 | 1.337 / 0.374 / 0.452 / 0.418 | 3.58 / 2.96 / 3.19 |
| 72 | friendly-even | 150 | single | 1 | 0.502 / 0.348 / 0.358 / 0.264 | 1.44 / 1.40 / 1.90 | 0.447 / 0.259 / 0.322 / 0.268 | 1.72 / 1.39 / 1.67 |
| 72 | friendly-even | 150 | batch32 | 32 | 0.491 / 0.352 / 0.360 / 0.270 | 1.39 / 1.36 / 1.82 | 0.452 / 0.263 / 0.322 / 0.268 | 1.72 / 1.40 / 1.69 |
| 127 | min-odd | 255 | single | 1 | 1.510 / 0.631 / 0.773 / 0.697 | 2.40 / 1.95 / 2.17 | 1.652 / 0.794 / 0.727 / 0.647 | 2.08 / 2.27 / 2.55 |
| 127 | min-odd | 255 | batch32 | 32 | 1.524 / 0.636 / 0.747 / 0.694 | 2.40 / 2.04 / 2.20 | 1.659 / 0.801 / 0.728 / 0.655 | 2.07 / 2.28 / 2.53 |
| 127 | min-even | 256 | single | 1 | 0.648 / 0.437 / 0.453 / 0.402 | 1.48 / 1.43 / 1.61 | 0.546 / 0.293 / 0.457 / 0.389 | 1.86 / 1.19 / 1.40 |
| 127 | min-even | 256 | batch32 | 32 | 0.644 / 0.445 / 0.456 / 0.414 | 1.45 / 1.41 / 1.55 | 0.547 / 0.301 / 0.453 / 0.385 | 1.82 / 1.21 / 1.42 |
| 127 | current | 257 | single | 1 | 10.421 / 0.656 / 4.101 / 2.938 | 15.88 / 2.54 / 3.55 | 10.679 / 0.835 / 4.160 / 2.998 | 12.80 / 2.57 / 3.56 |
| 127 | current | 257 | batch32 | 32 | 10.367 / 0.647 / 4.119 / 2.937 | 16.01 / 2.52 / 3.53 | 10.613 / 0.834 / 4.181 / 3.022 | 12.73 / 2.54 / 3.51 |
| 127 | friendly-even | 256 | single | 1 | 0.645 / 0.442 / 0.455 / 0.406 | 1.46 / 1.42 / 1.59 | 0.545 / 0.298 / 0.454 / 0.383 | 1.83 / 1.20 / 1.42 |
| 127 | friendly-even | 256 | batch32 | 32 | 0.652 / 0.447 / 0.459 / 0.407 | 1.46 / 1.42 / 1.60 | 0.549 / 0.302 / 0.453 / 0.389 | 1.82 / 1.21 / 1.41 |
| 255 | min-odd | 511 | single | 1 | 11.107 / 1.409 / 4.439 / 4.460 | 7.88 / 2.50 / 2.49 | 11.807 / 1.753 / 4.428 / 4.515 | 6.73 / 2.67 / 2.62 |
| 255 | min-odd | 511 | batch32 | 32 | 11.136 / 1.417 / 4.463 / 4.487 | 7.86 / 2.50 / 2.48 | 11.908 / 1.754 / 4.467 / 4.512 | 6.79 / 2.67 / 2.64 |
| 255 | min-even | 512 | single | 1 | 1.365 / 0.904 / 1.005 / 0.876 | 1.51 / 1.36 / 1.56 | 1.165 / 0.606 / 0.976 / 0.773 | 1.92 / 1.19 / 1.51 |
| 255 | min-even | 512 | batch32 | 32 | 1.313 / 0.886 / 1.018 / 0.896 | 1.48 / 1.29 / 1.47 | 1.127 / 0.600 / 0.977 / 0.780 | 1.88 / 1.15 / 1.45 |
| 255 | current | 513 | single | 1 | 3.398 / 1.453 / 1.276 / 1.081 | 2.34 / 2.66 / 3.15 | 3.698 / 1.842 / 1.427 / 0.987 | 2.01 / 2.59 / 3.75 |
| 255 | current | 513 | batch32 | 32 | 3.397 / 1.458 / 1.289 / 1.087 | 2.33 / 2.63 / 3.12 | 3.715 / 1.785 / 1.425 / 1.004 | 2.08 / 2.61 / 3.70 |
| 255 | friendly-even | 512 | single | 1 | 1.313 / 0.893 / 1.003 / 0.884 | 1.47 / 1.31 / 1.49 | 1.115 / 0.606 / 0.973 / 0.774 | 1.84 / 1.15 / 1.44 |
| 255 | friendly-even | 512 | batch32 | 32 | 1.315 / 0.890 / 1.017 / 0.893 | 1.48 / 1.29 / 1.47 | 1.122 / 0.600 / 0.971 / 0.783 | 1.87 / 1.16 / 1.43 |
| 360 | min-odd | 721 | single | 1 | 20.533 / 4.291 / 8.545 / 8.562 | 4.79 / 2.40 / 2.40 | 21.543 / 4.747 / 8.497 / 8.558 | 4.54 / 2.54 / 2.52 |
| 360 | min-odd | 721 | batch32 | 32 | 20.664 / 4.308 / 8.434 / 8.522 | 4.80 / 2.45 / 2.42 | 21.665 / 4.751 / 8.458 / 8.551 | 4.56 / 2.56 / 2.53 |
| 360 | min-even | 722 | single | 1 | 11.863 / 2.432 / 2.047 / 1.750 | 4.88 / 5.80 / 6.78 | 14.013 / 2.026 / 2.262 / 1.755 | 6.92 / 6.20 / 7.98 |
| 360 | min-even | 722 | batch32 | 32 | 11.756 / 2.432 / 2.052 / 1.762 | 4.83 / 5.73 / 6.67 | 13.942 / 2.050 / 2.276 / 1.766 | 6.80 / 6.12 / 7.90 |
| 360 | current | 723 | single | 1 | 29.961 / 2.128 / 17.852 / 12.065 | 14.08 / 1.68 / 2.48 | 30.984 / 2.609 / 18.137 / 12.177 | 11.88 / 1.71 / 2.54 |
| 360 | current | 723 | batch32 | 32 | 29.912 / 2.157 / 17.904 / 12.056 | 13.87 / 1.67 / 2.48 | 30.645 / 2.624 / 18.125 / 12.160 | 11.68 / 1.69 / 2.52 |
| 360 | friendly-even | 726 | single | 1 | 7.934 / 1.562 / 1.795 / 1.508 | 5.08 / 4.42 / 5.26 | 9.418 / 1.157 / 1.937 / 1.525 | 8.14 / 4.86 / 6.18 |
| 360 | friendly-even | 726 | batch32 | 32 | 7.902 / 1.568 / 1.814 / 1.517 | 5.04 / 4.36 / 5.21 | 9.397 / 1.153 / 1.952 / 1.529 | 8.15 / 4.81 / 6.14 |
| 511 | min-odd | 1023 | single | 1 | 15.942 / 4.436 / 2.916 / 2.269 | 3.59 / 5.47 / 7.03 | 17.444 / 5.084 / 3.079 / 2.269 | 3.43 / 5.67 / 7.69 |
| 511 | min-odd | 1023 | batch32 | 32 | 15.980 / 4.452 / 2.946 / 2.276 | 3.59 / 5.42 / 7.02 | 17.423 / 5.089 / 3.119 / 2.287 | 3.42 / 5.59 / 7.62 |
| 511 | min-even | 1024 | single | 1 | 2.672 / 1.838 / 2.100 / 1.848 | 1.45 / 1.27 / 1.45 | 2.239 / 1.267 / 2.059 / 1.786 | 1.77 / 1.09 / 1.25 |
| 511 | min-even | 1024 | batch32 | 32 | 2.667 / 1.867 / 2.128 / 1.878 | 1.43 / 1.25 / 1.42 | 2.252 / 1.256 / 2.093 / 1.814 | 1.79 / 1.08 / 1.24 |
| 511 | current | 1025 | single | 1 | 11.911 / 3.667 / 3.088 / 2.310 | 3.25 / 3.86 / 5.16 | 12.262 / 4.306 / 3.309 / 2.270 | 2.85 / 3.71 / 5.40 |
| 511 | current | 1025 | batch32 | 32 | 11.916 / 3.676 / 3.057 / 2.343 | 3.24 / 3.90 / 5.09 | 12.319 / 4.329 / 3.342 / 2.311 | 2.85 / 3.69 / 5.33 |
| 511 | friendly-even | 1024 | single | 1 | 2.681 / 1.843 / 2.102 / 1.853 | 1.46 / 1.27 / 1.45 | 2.219 / 1.265 / 2.058 / 1.790 | 1.75 / 1.08 / 1.24 |
| 511 | friendly-even | 1024 | batch32 | 32 | 2.687 / 1.840 / 2.134 / 1.885 | 1.46 / 1.26 / 1.43 | 2.277 / 1.260 / 2.086 / 1.831 | 1.81 / 1.09 / 1.24 |
| 720 | min-odd | 1441 | single | 1 | 53.174 / 8.281 / 15.682 / 13.893 | 6.42 / 3.39 / 3.83 | 56.237 / 9.231 / 16.027 / 13.576 | 6.09 / 3.51 / 4.14 |
| 720 | min-odd | 1441 | batch32 | 32 | 53.216 / 8.314 / 15.727 / 13.907 | 6.40 / 3.38 / 3.83 | 56.342 / 9.241 / 15.982 / 13.607 | 6.10 / 3.52 / 4.14 |
| 720 | min-even | 1442 | single | 1 | 83.187 / 6.183 / 12.827 / 11.328 | 13.46 / 6.49 / 7.34 | 84.406 / 5.350 / 12.990 / 11.395 | 15.78 / 6.50 / 7.41 |
| 720 | min-even | 1442 | batch32 | 32 | 83.284 / 6.210 / 12.886 / 11.341 | 13.41 / 6.46 / 7.34 | 84.520 / 5.454 / 13.023 / 11.403 | 15.50 / 6.49 / 7.41 |
| 720 | current | 1443 | single | 1 | 24.578 / 7.593 / 4.578 / 3.635 | 3.24 / 5.37 / 6.76 | 28.304 / 8.655 / 4.568 / 3.554 | 3.27 / 6.20 / 7.96 |
| 720 | current | 1443 | batch32 | 32 | 24.639 / 7.695 / 4.604 / 3.649 | 3.20 / 5.35 / 6.75 | 28.327 / 8.609 / 4.602 / 3.589 | 3.29 / 6.16 / 7.89 |
| 720 | friendly-even | 1452 | single | 1 | 7.476 / 3.365 / 5.699 / 4.185 | 2.22 / 1.31 / 1.79 | 9.206 / 2.536 / 6.008 / 4.205 | 3.63 / 1.53 / 2.19 |
| 720 | friendly-even | 1452 | batch32 | 32 | 7.481 / 3.376 / 5.755 / 4.237 | 2.22 / 1.30 / 1.77 | 9.227 / 2.551 / 6.040 / 4.244 | 3.62 / 1.53 / 2.17 |

f32 reference validation compares f32-rounded inputs and outputs with independent f64 direct DFTs where feasible and independent f64 FFT references at the larger lengths. The table aggregates maximum observed errors over tested smooth, general-prime, Bluestein and SHT sizes; the per-length values, including N=241, 257, 723, 1025, 1443 and 4097, are in [`m5-f32-accuracy.csv`](../benches/results/m5-f32-accuracy.csv).

| operation | max absolute error | max RMS error | max relative L2 | max scaled maximum | nonfinite outputs |
|---|---:|---:|---:|---:|---:|
| R2C | 1.838824e-04 | 1.043319e-05 | 2.543863e-07 | 1.845495e-07 | 0 |
| C2R | 1.008106e-07 | 9.028811e-09 | 2.049111e-07 | 1.008106e-07 | 0 |

No nonfinite outputs occurred. The f32 R2C maximum relative L2 error is `2.544e-7`; C2R maximum relative L2 error is `2.049e-7`. This is numerical evidence, not an f32 performance win from the new general-prime AVX2 kernel: the new kernel is f64-only and f32 uses its existing scalar general-prime path. The current T127/T360 odd-length f32 gaps are large, especially where Bluestein is selected.

## Meridional C2C results

Each cell gives `RustSHT-FFT / RustFFT / DUCC portable / DUCC native`, in microseconds per transform. The `sht_rows` class uses the measured RustSHT meridional row count and reuses plan/workspace. C2C has no real packing adapter. Ratios use the same library order as the f64 real results.

| T | meridional length role | N | call pattern | rows | C2C forward times, µs (DFFT / RustFFT / DUCC portable / DUCC native) | forward ratios | C2C inverse times, µs | inverse ratios |
|---:|---|---:|---|---:|---|---|---|---|
| 72 | original | 146 | single | 1 | 8.916 / 1.032 / 3.136 / 3.196 | 8.64 / 2.84 / 2.79 | 9.067 / 0.882 / 3.086 / 2.896 | 10.28 / 2.94 / 3.13 |
| 72 | original | 146 | sht_rows | 73 | 8.735 / 0.737 / 2.279 / 2.338 | 11.85 / 3.83 / 3.74 | 8.933 / 0.779 / 2.254 / 2.310 | 11.46 / 3.96 / 3.87 |
| 72 | prepared | 150 | single | 1 | 1.182 / 0.331 / 0.461 / 0.291 | 3.57 / 2.56 / 4.06 | 1.392 / 0.371 / 0.531 / 0.341 | 3.75 / 2.62 / 4.08 |
| 72 | prepared | 150 | sht_rows | 73 | 1.002 / 0.277 / 0.443 / 0.233 | 3.62 / 2.26 / 4.30 | 1.215 / 0.311 / 0.483 / 0.280 | 3.91 / 2.52 / 4.34 |
| 72 | full | 300 | single | 1 | 3.016 / 0.621 / 1.192 / 0.641 | 4.86 / 2.53 / 4.71 | 3.426 / 0.702 / 1.132 / 0.711 | 4.88 / 3.03 / 4.82 |
| 72 | full | 300 | sht_rows | 73 | 2.051 / 0.529 / 0.940 / 0.488 | 3.87 / 2.18 / 4.21 | 2.513 / 0.604 / 1.055 / 0.585 | 4.16 / 2.38 / 4.30 |
| 127 | shared | 256 | single | 1 | 1.052 / 0.271 / 0.781 / 0.762 | 3.88 / 1.35 / 1.38 | 1.502 / 0.361 / 0.872 / 0.671 | 4.16 / 1.72 / 2.24 |
| 127 | shared | 256 | sht_rows | 128 | 0.746 / 0.293 / 0.643 / 0.584 | 2.54 / 1.16 / 1.28 | 1.109 / 0.353 / 0.716 / 0.610 | 3.14 / 1.55 / 1.82 |
| 255 | shared | 512 | single | 1 | 0.982 / 0.681 / 1.683 / 1.342 | 1.44 / 0.58 / 0.73 | 1.673 / 1.082 / 1.853 / 1.272 | 1.55 / 0.90 / 1.31 |
| 255 | shared | 512 | sht_rows | 256 | 0.866 / 0.721 / 1.459 / 0.849 | 1.20 / 0.59 / 1.02 | 1.587 / 0.842 / 1.653 / 0.937 | 1.89 / 0.96 / 1.69 |
| 360 | original | 722 | single | 1 | 26.790 / 6.713 / 7.744 / 5.100 | 3.99 / 3.46 / 5.25 | 27.531 / 7.053 / 7.474 / 4.809 | 3.90 / 3.68 / 5.72 |
| 360 | original | 722 | sht_rows | 361 | 23.389 / 6.854 / 4.959 / 3.339 | 3.41 / 4.72 / 7.00 | 24.494 / 6.965 / 5.185 / 3.410 | 3.52 / 4.72 / 7.18 |
| 360 | prepared | 726 | single | 1 | 8.616 / 2.014 / 3.918 / 2.595 | 4.28 / 2.20 / 3.32 | 9.077 / 2.294 / 4.699 / 3.136 | 3.96 / 1.93 / 2.89 |
| 360 | prepared | 726 | sht_rows | 361 | 7.120 / 1.657 / 3.529 / 2.537 | 4.30 / 2.02 / 2.81 | 8.061 / 1.826 / 3.895 / 2.640 | 4.42 / 2.07 / 3.05 |
| 360 | full | 1452 | single | 1 | 15.459 / 3.788 / 8.436 / 4.538 | 4.08 / 1.83 / 3.41 | 16.812 / 3.777 / 9.047 / 5.250 | 4.45 / 1.86 / 3.20 |
| 360 | full | 1452 | sht_rows | 361 | 14.889 / 3.423 / 7.561 / 3.292 | 4.35 / 1.97 / 4.52 | 16.810 / 3.760 / 8.328 / 3.617 | 4.47 / 2.02 / 4.65 |
| 511 | shared | 1024 | single | 1 | 2.575 / 1.513 / 4.037 / 2.405 | 1.70 / 0.64 / 1.07 | 3.656 / 1.904 / 4.198 / 2.524 | 1.92 / 0.87 / 1.45 |
| 511 | shared | 1024 | sht_rows | 512 | 2.232 / 1.625 / 3.364 / 1.733 | 1.37 / 0.66 / 1.29 | 3.706 / 1.882 / 3.799 / 1.911 | 1.97 / 0.98 / 1.94 |
| 720 | original | 1442 | single | 1 | 118.662 / 12.484 / 24.927 / 17.723 | 9.51 / 4.76 / 6.70 | 120.094 / 12.804 / 24.697 / 16.170 | 9.38 / 4.86 / 7.43 |
| 720 | original | 1442 | sht_rows | 721 | 124.571 / 12.813 / 24.953 / 18.072 | 9.72 / 4.99 / 6.89 | 126.253 / 13.182 / 25.103 / 18.308 | 9.58 / 5.03 / 6.90 |
| 720 | prepared | 1452 | sht_rows | 721 | 14.900 / 3.464 / 7.571 / 3.329 | 4.30 / 1.97 / 4.48 | 16.895 / 3.814 / 8.418 / 3.746 | 4.43 / 2.01 / 4.51 |
| 720 | full | 2904 | single | 1 | 31.158 / 6.762 / 18.565 / 10.139 | 4.61 / 1.68 / 3.07 | 34.855 / 7.414 / 19.126 / 10.980 | 4.70 / 1.82 / 3.17 |
| 720 | full | 2904 | sht_rows | 721 | 31.900 / 7.623 / 16.677 / 8.072 | 4.18 / 1.91 / 3.95 | 35.391 / 8.035 / 17.305 / 9.064 | 4.41 / 2.04 / 3.90 |

At the T720 original profile length N=1442, the `sht_rows` case executes 721 transforms: forward is 124.571 / 12.813 / 24.953 / 18.072 µs, and inverse is 126.253 / 13.182 / 25.103 / 18.308 µs. The current C2C factorization exposes a terminal general-prime radix 103. Callgrind on N=1442 attributes about 91.3% of instruction references to the general radix transform under `execute_pass_inner::<0>` and about 96.2% to the inclusive pass stack; memcpy accounts for about 0.8%. This points to the scalar O(p²) general-prime C2C pass as the main gap, not output copyback. The phase-index optimization below removes repeated modulo work from this pass but leaves its O(p²) butterfly count unchanged. An `ido == 1` C2C SIMD prototype remains a possible next step only after an uncontended rerun establishes the scalar change's effect.

The f64 C2C code-path direct-DFT regression covers N=146, 722, 1442 and 1452 in both directions. The phase-table validation also directly covers f64 radices 13–109 and f32 radix 103, and a new f32/f64 N=103 inverse test exercises the maximum-finite DC fallback. No f32 C2C performance matrix was collected, so f32 C2C remains unmeasured.

### M6 C2C phase-index optimization

The measured C2C bottleneck used `bounded_product_mod(branch, frequency, radix)` in the innermost direct-prime butterfly. Plan construction now precomputes the phase `(branch × frequency) mod radix` as a `u8` table for odd general radices 13–109. The execution kernel uses that phase to select the existing global root and retains the existing forward conjugation, inverse sign, normalization, prescaling and residual-correction paths. This adds no unsafe code, dependency, public API or workspace scratch. The N=722 plan has two radix-19 tables (342 payload bytes); N=1442 has one radix-103 table (5,253 payload bytes).

The table lookup is exercised by direct-DFT tests for f64 radices 13, 17, 19, 23, 31, 41, 53, 73, 103, 107 and 109, plus the existing f64 N=722 and N=1442 meridional tests. f32 direct-DFT coverage includes radix 103. Radix 113 and larger still use Bluestein. Both directions are checked; the general-prime tests include normal deterministic data, and existing inverse extreme-value regressions still exercise the unchanged safety paths.

The source baseline was commit `84c6c437a2af3661388be922d460d9cc9acc4bda`. Baseline and candidate used Rust 1.99.0, the same Cargo bench profile (`opt-level=3`, no LTO or custom `RUSTFLAGS`), the portable runtime-dispatch build, deterministic inputs, one FFT worker, reusable plans and workspaces, and `taskset -c 30`. Three alternating baseline/candidate process pairs were collected for each length. The host was heavily contested: nearby load-average snapshots were 58.75/47.37/38.81 and 44.05/47.44/40.98, and `ps` showed concurrent RustSHT execution and multiple JAX C++ compilation processes. Interpret the paired ratios as screening evidence and the absolute values as provisional. The exact samples are in [`m6-c2c-phase-index-process-samples.csv`](../benches/results/m6-c2c-phase-index-process-samples.csv).

| N | direction | rows | baseline median, µs/transform | candidate median, µs/transform | paired baseline/candidate ratio (range) |
|---:|---|---:|---:|---:|---:|
| 722 | forward | 1 | 51.746 | 42.779 | 1.221× (1.206–1.279×) |
| 722 | forward | 361 | 51.353 | 42.565 | 1.205× (1.199–1.299×) |
| 722 | inverse | 1 | 51.736 | 40.234 | 1.286× (1.212–1.468×) |
| 722 | inverse | 361 | 51.572 | 42.955 | 1.201× (1.184–1.296×) |
| 1442 | forward | 1 | 234.204 | 189.000 | 1.272× (1.152–1.336×) |
| 1442 | forward | 721 | 236.231 | 191.486 | 1.234× (1.229–1.244×) |
| 1442 | inverse | 1 | 233.944 | 195.643 | 1.199× (1.196–1.239×) |
| 1442 | inverse | 721 | 239.440 | 193.571 | 1.242× (1.171–1.266×) |

Forward plan setup medians were 21.472/22.634 µs (baseline/candidate) for a single N=722 plan and 18.928/19.960 µs in the row-batch benchmark. At N=1442, forward setup was 39.112/55.049 µs for a single plan and 36.657/51.914 µs in the row-batch benchmark. Inverse setup medians were 18.581/19.929 µs and 18.611/19.640 µs at N=722, and 36.171/53.402 µs and 36.988/66.728 µs at N=1442. Workspace allocation medians were noisy, but workspace sizes remained exactly 12,160 bytes at N=722 and 26,368 bytes at N=1442. The N=1442 forward measurements show about 41% more one-time plan construction; the inverse row-batch setup result is noisier and higher still. These costs are relevant to RustSHT's plan-reuse pattern, not one-shot setup/transform calls.

The optimized `radix_general_prime_precomputed_phases::<f64>` symbol in the release benchmark binary is 698 bytes and contains no integer `div`/`idiv` instruction; phase lookup and the root-stride multiply remain scalar. The old source performed runtime modulo in the nested branch/frequency loop. The current cross-library ratios in the preceding M5 tables describe the pre-optimization C2C source baseline; no new candidate-vs-RustFFT/DUCC process campaign was run under this host load. The source-relative improvement does not close the remaining C2C gap. This optimization is retained as a low-complexity scalar improvement, with an uncontended paired rerun recommended before making a final percentage claim.

## Longitude sampling comparison

Both minimum counts preserve all Fourier orders through L. The table below shows the f64 scalar row call for each candidate N as `RustSHT-FFT / RustFFT / DUCC native` in microseconds per row. C2R is separately shown. It is the same data as the full matrix above, regrouped by sampling role to make length alternatives explicit. The row count is `L+1`; it is not a complete SHT timing.

| T | role | N | scalar rows | R2C RustSHT-FFT / RustFFT / DUCC native, µs | C2R RustSHT-FFT / RustFFT / DUCC native, µs |
|---:|---|---:|---:|---|---|
| 72 | min-odd | 145 | 73 | 0.711 / 0.906 / 0.686 | 0.769 / 0.921 / 0.660 |
| 72 | min-even | 146 | 73 | 1.310 / 0.559 / 2.242 | 1.462 / 0.487 / 2.264 |
| 72 | current | 147 | 73 | 1.139 / 0.350 / 0.491 | 1.294 / 0.376 / 0.446 |
| 72 | friendly-even | 150 | 73 | 0.457 / 0.385 / 0.333 | 0.430 / 0.310 / 0.245 |
| 127 | min-odd | 255 | 128 | 0.946 / 0.872 / 0.761 | 1.113 / 0.908 / 0.691 |
| 127 | min-even;friendly-even | 256 | 128 | 0.677 / 0.499 / 0.509 | 0.560 / 0.386 / 0.493 |
| 127 | current | 257 | 128 | 10.378 / 1.226 / 2.838 | 10.554 / 1.250 / 2.864 |
| 255 | min-odd | 511 | 256 | 6.142 / 2.892 / 4.516 | 7.639 / 2.949 / 4.499 |
| 255 | min-even;friendly-even | 512 | 256 | 1.416 / 1.028 / 1.201 | 1.193 / 0.796 / 1.018 |
| 255 | current | 513 | 256 | 2.306 / 2.332 / 1.612 | 2.604 / 2.397 / 1.349 |
| 360 | min-odd | 721 | 361 | 11.138 / 6.309 / 8.420 | 13.135 / 6.359 / 8.421 |
| 360 | min-even | 722 | 361 | 10.588 / 4.323 / 2.389 | 12.445 / 4.031 / 2.156 |
| 360 | current | 723 | 361 | 32.942 / 4.700 / 11.692 | 33.426 / 4.773 / 11.585 |
| 360 | friendly-even | 726 | 361 | 8.241 / 1.833 / 2.227 | 9.338 / 1.513 / 1.903 |
| 511 | min-odd | 1023 | 512 | 11.402 / 7.567 / 3.529 | 13.820 / 7.670 / 3.208 |
| 511 | min-even;friendly-even | 1024 | 512 | 3.003 / 2.233 / 2.496 | 2.492 / 1.725 / 2.164 |
| 511 | current | 1025 | 512 | 6.036 / 6.171 / 3.684 | 7.281 / 6.290 / 3.333 |
| 720 | min-odd | 1441 | 721 | 30.201 / 13.527 / 16.697 | 34.991 / 13.464 / 16.769 |
| 720 | min-even | 1442 | 721 | 63.255 / 8.449 / 11.850 | 64.793 / 7.801 / 12.164 |
| 720 | current | 1443 | 721 | 18.180 / 14.205 / 5.529 | 22.174 / 14.294 / 5.319 |
| 720 | friendly-even | 1452 | 721 | 8.629 / 3.939 / 4.132 | 10.190 / 3.326 / 4.472 |

The experiment is a fixed-length FFT comparison. It does not change the RustSHT grid, sampling default or public API, and does not include complete Legendre, grid-weight and harmonic-work costs. Mathematically, both shorter counts retain the required Fourier orders, with the unused even Nyquist mode at L+1. A future change to RustSHT sampling should be justified by full SHT validation and end-to-end timing, not by this row benchmark alone.

## Portable runtime dispatch versus native-target build

The separate Rust native-target screen used the same source and benchmark but set `RUSTFLAGS='-C target-cpu=native'`. Over 54 selected f64 real cells, the median portable-build / native-target ratio is 1.035× (range 0.934–1.179×; four cells favor the portable build). This is a modest and workload-dependent gain relative to the general-prime kernel changes. It does not justify imposing `target-cpu=native` on distributed builds. The production portable build uses runtime AVX2 detection and retains a scalar fallback. Full per-cell results are in [`m5-rust-native-target-summary.csv`](../benches/results/m5-rust-native-target-summary.csv).

LTO was not separately tested in this session. The baseline and candidate Rust binaries use the same compiler profile and compiler flags; no SIMD conclusion is based on comparing a portable Rust binary against a native-target C++ binary without labeling the ISA difference.

## Admiral algorithm assessment

Admiral source inspected at commit `ce67bc00ff9e6de96bf5b75f7a789581105ddb58`: [`rader.hpp`](https://github.com/DiamonDinoia/admiral/blob/ce67bc00ff9e6de96bf5b75f7a789581105ddb58/include/admiral/detail/rader.hpp), [`plan.hpp`](https://github.com/DiamonDinoia/admiral/blob/ce67bc00ff9e6de96bf5b75f7a789581105ddb58/include/admiral/detail/plan.hpp), [`base_cost_model.hpp`](https://github.com/DiamonDinoia/admiral/blob/ce67bc00ff9e6de96bf5b75f7a789581105ddb58/include/admiral/detail/base_cost_model.hpp), [`four_step.hpp`](https://github.com/DiamonDinoia/admiral/blob/ce67bc00ff9e6de96bf5b75f7a789581105ddb58/include/admiral/detail/four_step.hpp), and [`good_thomas.hpp`](https://github.com/DiamonDinoia/admiral/blob/ce67bc00ff9e6de96bf5b75f7a789581105ddb58/include/admiral/detail/good_thomas.hpp). No Admiral code was copied and no dependency was added.

### Source-level comparison

| Design point | Admiral | Pinned DUCC / current RustSHT-FFT |
|---|---|---|
| Rader route | Supports prime p when p−1 has an available codelet, DIF or four-step inner route; precomputes primitive-root permutations and a transformed convolution kernel. | DUCC's 1D planner uses its fixed direct-prime and Bluestein families; RustSHT-FFT follows its separate complex and real Bluestein thresholds. Neither has a Rader plan in this source revision. |
| Prime convolution | Cyclic convolution length p−1, implemented with a selected inner FFT route. | Bluestein uses a padded convolution (current observed sizes 525 for p=257 and 484 for p=241). |
| Route selection | A route catalogue and a hardware/build-specific cost model rank codelet, DIF, four-step, Good–Thomas, Rader and Bluestein. An optional measured-effort mode can benchmark candidates at planning time. | Fixed DUCC-compatible factorization and thresholds; no generic cost model or plan-time autotuning. The screen below does not justify replacing that with a model. |
| Good–Thomas / four-step | Catalogued CRT permutations for selected coprime factors; four-step uses transpose/row FFT/twiddle/column FFT structure. | Current multipass execution uses DUCC pass descriptors and buffer alternation. The bounded M5 screen only reversed existing factor order; no Good–Thomas or four-step engine was implemented. |
| Scratch and ownership | The inspected Rader execution creates local SoA scratch; the route has Admiral-specific inner scratch. | The experiment reuses `ComplexFftPlan` and caller-owned `ComplexFftWorkspace`/real workspace scratch. No allocation occurs inside timed execution. |

Admiral's model and published CPU-specific measurements are not treated as Ryzen 9 5950X evidence. Its inspected source is BSD-3-Clause-Attribution; the prototype below was implemented independently and has no source-expression reuse.

### Admiral Rader eligibility and route selection

At the inspected revision, `rader_supported(p)` requires a prime p outside the direct codelet catalogue and an available route for p−1. The inner transform is a direct codelet when p−1 is catalogued, iterative DIF when p−1 is a power of two or factors only into 2, 3, 5, 7 and 11, and four-step otherwise. Both p=257 (p−1=256) and p=241 (p−1=240) meet the inner-route condition. The plan builds `gpow` and `ginvpow`, permutes nonzero input bins by powers of a primitive root, transforms the direction-specific permuted kernel once at plan construction, then writes nonzero outputs through the inverse permutation. The inner inverse is normalized by 1/(p−1); the outer inverse contributes 1/p.

Admiral's route selector ranks codelet, DIF, Good–Thomas, four-step, Rader and Bluestein candidates. Its Rader gate compares a modeled pair of inner transforms plus a per-element cost against Bluestein; the f64 threshold depends on build vector width (1.30 for the inspected narrow-width branch and 1.06 for width at least eight lanes). This is a build-specific model, not a reason to transplant its thresholds to RustSHT-FFT's portable runtime-dispatch build. Admiral's inspected Rader `execute` also creates local SoA scratch; the four-step inner route creates a temporary vector. RustSHT-FFT's prototype instead reuses its nested plan workspace and scratch across calls.

Good–Thomas in Admiral replaces twiddles with CRT index maps for selected coprime factors, while four-step uses transpose, row transforms, a twiddle stage and column transforms. Those are materially different schedules from simply reversing RustSHT-FFT's factors. The existing bounded factor-order experiment did not implement either decomposition, and its noisy/near-neutral results do not justify their additional permutation and buffer machinery in this session.

### Rader versus Bluestein for p=257 and p=241

The asymptotic setup is favorable for these two primes: Rader replaces a length-525 Bluestein convolution with a cyclic convolution of length 256 for p=257; p=241 uses length 240 rather than a length-484 Bluestein convolution. Both p−1 lengths are smooth and already supported by `ComplexFftPlan`. Rader still requires primitive-root permutations, direction-specific transformed kernels, a sum output, and permutation-aware scratch. For the T360 longitude N=723=3×241, the prototype adds an explicit radix-3 decomposition around three Rader length-241 transforms.

The prototype uses reusable complex plans and scratch. It computes the transformed direction-specific kernel at plan construction, performs input and output permutations, runs inner forward FFT → pointwise product → normalized inner inverse FFT, and applies the outer normalization required by the real/mixed wrapper. Plan and scratch values below are medians from three alternating baseline/prototype process pairs. Rader timing is full public R2C/C2R call time, not only inner convolution time.

For the standalone prime cases, the prototypes use a cyclic convolution of length 256 for p=257 and 240 for p=241, versus the production Bluestein lengths shown above. For N=723=3×241, the mixed-radix adapter performs three prime transforms and the radix-3 combination. The direct prime prototype tests both f64 directions against independent direct DFTs; the public real wrappers compare normal and cancellation-heavy data against direct DFTs, RustFFT and DUCC output dumps. Separate cancellation, high-dynamic-range, near-overflow (inputs scaled by transform length), minimum-normal and subnormal patterns are checked for finite results and agreement with the production real plan. `cargo bench --manifest-path crates/rustsht-fft/Cargo.toml --bench rader_real_prototype -- validate` passed again during this follow-up.

| N | row case | rows | direction | Bluestein public, µs | Rader public, µs | paired speedup (range) | plan µs B/R | workspace allocation µs B/R | scratch B/R | Rader tables B* |
|---:|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 257 | single | 1 | c2r | 10.374 | 3.112 | 3.326× (3.274–4.268×) | 19.967 / 15.349 | 0.842 / 0.431 | 37584 / 24896 | 12432 |
| 257 | single | 1 | r2c | 10.219 | 2.542 | 4.041× (3.096–4.083×) | 19.757 / 12.033 | 0.832 / 0.370 | 37584 / 24896 | 12432 |
| 257 | scalar | 128 | c2r | 10.545 | 2.556 | 4.136× (4.123–4.158×) | 20.197 / 12.253 | 0.862 / 0.381 | 37584 / 24896 | 12432 |
| 257 | scalar | 128 | r2c | 10.381 | 2.553 | 4.068× (4.032–4.139×) | 20.038 / 12.122 | 0.852 / 0.400 | 37584 / 24896 | 12432 |
| 257 | spin1 | 256 | c2r | 10.676 | 2.577 | 4.156× (4.102–4.172×) | 20.148 / 12.233 | 0.842 / 0.371 | 37584 / 24896 | 12432 |
| 257 | spin1 | 256 | r2c | 10.535 | 2.549 | 4.132× (4.128–4.137×) | 20.158 / 12.092 | 0.861 / 0.371 | 37584 / 24896 | 12432 |
| 723 | single | 1 | c2r | 33.429 | 14.465 | 2.310× (2.304–2.348×) | 27.592 / 35.747 | 0.952 / 0.601 | 48640 / 50352 | 57936 |
| 723 | single | 1 | r2c | 33.496 | 14.751 | 2.279× (2.158–2.338×) | 27.291 / 35.747 | 0.882 / 0.611 | 48640 / 50352 | 57936 |
| 723 | scalar | 361 | c2r | 33.782 | 14.764 | 2.288× (2.283–2.333×) | 27.381 / 35.727 | 0.882 / 0.611 | 48640 / 50352 | 57936 |
| 723 | scalar | 361 | r2c | 33.524 | 14.476 | 2.316× (2.285–2.332×) | 27.261 / 35.757 | 0.861 / 0.591 | 48640 / 50352 | 57936 |
| 723 | spin1 | 722 | c2r | 33.906 | 14.698 | 2.295× (2.262–2.307×) | 27.101 / 35.456 | 0.872 / 0.581 | 48640 / 50352 | 57936 |
| 723 | spin1 | 722 | r2c | 33.466 | 14.408 | 2.318× (2.312–2.329×) | 27.171 / 35.827 | 0.872 / 0.601 | 48640 / 50352 | 57936 |

`rader_table_bytes` counts the prototype's permutation, kernel and mixed-radix tables; it excludes root tables held by its nested complex plan. At N=257, Rader uses 24,896 scratch bytes versus 37,584 for the current plan, and its table count is 12,432 bytes. At N=723, scratch rises slightly from 48,640 to 50,352 bytes and the Rader/mix tables use 57,936 bytes. N=257 plan construction falls from about 20 µs to 12–15 µs. N=723 plan construction rises from about 27 µs to 35–36 µs, so setup is a real tradeoff even though the execution is faster.

For ordinary deterministic input/spectrum data, Rader outputs were compared against direct DFTs, RustFFT 6.4.1, and pinned DUCC. Rader's normal/cancellation cases agree with direct f64 DFT and RustFFT within the test bounds. Compared with DUCC portable and native dumps, scaled maximum differences were:

| N | operation | bins/samples | max absolute difference | scaled maximum |
|---:|---|---:|---:|---:|
| 257 | R2C | 129 | 2.182e-14 | 1.664e-16 |
| 257 | C2R | 257 | 2.776e-17 | 2.278e-16 |
| 723 | R2C | 362 | 9.670e-14 | 2.708e-16 |
| 723 | C2R | 723 | 5.378e-17 | 4.573e-16 |

The extreme-value checks compare against the production path and verify finite outputs; they do not compare those extreme cases against an independently computed direct DFT. The candidate also does not implement RustSHT-FFT's conditional inverse prescaling and residual-correction behavior. It is an isolated timing prototype, so it does not change production fallback behavior, but its extreme-value validation is not sufficient for promotion. A production route would need to delegate unsafe magnitudes to the existing Bluestein path or preserve the same proven safeguards; this prototype does neither. This gap, together with its 800+ lines of real/mixed-radix scaffolding, is why the measured execution gain is documented but not accepted into the production planner.

### Candidate longitude counts

The valid alternatives and their measured scalar-row results are in the preceding [longitude sampling comparison](#longitude-sampling-comparison). The full per-size and row-case table is also in `m5-sht-real-f64-summary.csv`. This fixed-length comparison separates sampling changes from FFT algorithm changes. Shorter N=256 is much faster than prime N=257, and N=1024 is faster than N=1025. No RustSHT sampling change is proposed without a full SHT-level comparison.

### Factor-order screen

The current target factors are N=513 `[3,3,3,19]`, N=1025 `[5,5,41]`, N=1443 `[3,13,37]`, and the slow meridional C2C N=1442 `[2,7,103]`. The bounded alternative reversed the first three lists to `[19,3,3,3]`, `[41,5,5]` and `[37,13,3]`; for N=1442 it compared `[2,7,103]` with `[103,7,2]`. It used existing kernels only and three process pairs. The ratio is current time / reversed time; above 1 means the reversed screen was faster.

| mode | N | rows / case | current, µs | reversed, µs | current / reversed (range) |
|---|---:|---|---:|---:|---:|
| c2c_forward | 1442 | 1 / single | 13.585 | 13.355 | 1.017× (1.003–1.020×) |
| c2c_forward | 1442 | 721 / meridional | 13.455 | 13.345 | 1.019× (0.897–1.288×) |
| c2c_forward | 1452 | 1 / single | 4.388 | 3.436 | 1.292× (1.018–1.317×) |
| c2c_forward | 1452 | 721 / meridional | 3.630 | 3.563 | 1.019× (0.989–1.169×) |
| c2c_forward | 2904 | 1 / single | 6.813 | 6.812 | 0.999× (0.994–1.012×) |
| c2c_forward | 2904 | 721 / meridional | 7.838 | 7.732 | 1.014× (0.858–1.145×) |
| c2c_inverse | 1442 | 1 / single | 13.675 | 14.186 | 0.992× (0.583–1.011×) |
| c2c_inverse | 1442 | 721 / meridional | 15.705 | 13.421 | 1.050× (1.007–1.175×) |
| c2c_inverse | 1452 | 1 / single | 3.797 | 3.867 | 0.982× (0.908–1.060×) |
| c2c_inverse | 1452 | 721 / meridional | 3.919 | 3.838 | 1.021× (0.997–1.036×) |
| c2c_inverse | 2904 | 1 / single | 7.725 | 7.514 | 1.041× (1.007–1.219×) |
| c2c_inverse | 2904 | 721 / meridional | 8.203 | 8.560 | 0.947× (0.941–1.068×) |
| c2r | 513 | 1 / single | 2.455 | 2.464 | 1.021× (0.949–1.030×) |
| c2r | 513 | 256 / scalar | 2.403 | 2.425 | 0.990× (0.988–0.991×) |
| c2r | 513 | 512 / spin1 | 2.462 | 2.443 | 1.009× (0.974–1.134×) |
| c2r | 1025 | 1 / single | 7.094 | 6.872 | 1.076× (0.866–1.079×) |
| c2r | 1025 | 512 / scalar | 6.400 | 6.319 | 1.009× (1.002–1.015×) |
| c2r | 1025 | 1024 / spin1 | 6.737 | 6.714 | 1.003× (0.995–1.015×) |
| c2r | 1443 | 1 / single | 13.194 | 13.234 | 1.004× (0.990–1.005×) |
| c2r | 1443 | 721 / scalar | 14.197 | 14.181 | 1.001× (0.984–1.016×) |
| c2r | 1443 | 1442 / spin1 | 15.111 | 15.676 | 0.967× (0.964–1.002×) |
| r2c | 513 | 1 / single | 2.345 | 2.335 | 1.004× (1.000–1.230×) |
| r2c | 513 | 256 / scalar | 2.342 | 2.358 | 0.995× (0.986–0.999×) |
| r2c | 513 | 512 / spin1 | 2.353 | 2.376 | 0.989× (0.989–0.991×) |
| r2c | 1025 | 1 / single | 6.201 | 6.983 | 0.885× (0.874–1.153×) |
| r2c | 1025 | 512 / scalar | 6.291 | 6.316 | 1.005× (0.991–1.018×) |
| r2c | 1025 | 1024 / spin1 | 6.711 | 6.559 | 1.010× (1.008–1.023×) |
| r2c | 1443 | 1 / single | 12.984 | 12.994 | 1.001× (0.995–1.005×) |
| r2c | 1443 | 721 / scalar | 13.954 | 14.195 | 0.993× (0.976–0.996×) |
| r2c | 1443 | 1442 / spin1 | 14.488 | 14.686 | 0.987× (0.979–1.012×) |

No single reversed schedule consistently improves both directions, single calls and repeated rows. Real N=513/1025/1443 mostly differ by around 1%; N=1443 directions and batch classes straddle 1.0. C2C N=1442 inverse row batches and the isolated N=1452 single forward case favor the reversed schedule, but same-length single/batch comparisons and other directions do not reproduce those gains, and pair ranges are broad. The results do not justify a factorization change. Current DUCC ordering also leaves the real prime stage terminal with `ido==1`, which enables the new f64 general-prime kernel.

### Candidate disposition and maintenance cost

- **Kept in production:** the f64 AVX2 real general-prime `ido==1` kernel, phase tables created only for eligible passes, runtime feature dispatch, and scalar fallback. It produces repeated paired gains at important N=513, 1025 and 1443 without changing the planner. Direct DFT tests cover radices 17, 37, 41, 73, 103 and 131, several `l1` values, both directions, scaling and surrounding guard values. f32 keeps the scalar path.
- **Kept in production:** the scalar C2C general-prime phase-index table for odd factors 13–109. It adds no unsafe code or workspace scratch and shows 1.20–1.24× median paired gains in repeated N=722/N=1442 row measurements, with a larger one-time N=1442 plan cost. Because the host was heavily loaded, treat the gain as provisional and rerun before making an uncontended performance claim.
- **Kept as bench-only evidence:** the real Rader prototype for N=257 and N=723. It is explicitly under `benches/prototypes/` and does not alter `RealFftPlan` or planner selection. It is 800+ lines including prototype math, validation, dump and benchmark code; this is meaningful experiment complexity and would be too much to pull into production without consolidating the plan and scratch design.
- **Rejected for production:** reversed factor order. It has no stable performance advantage across relevant modes/row counts.
- **Not promoted:** `-C target-cpu=native`. It showed a small median gain in a separate build but regressed several cells and is not valid for portable distribution.
- **Not revisited:** across-row SIMD. Earlier measurements found packing/unpacking costs larger than its benefit; the row loop executes transforms sequentially with reused scratch.
- **Not attempted:** Good–Thomas framework, four-step engine, cost model or measured planner. These would add broad planner and permutation infrastructure without a measured M5 need.

The production general-prime kernel adds approximately 400 net source/test lines across `simd.rs`, `real_multipass.rs`, `real.rs` and `tests/real.rs`; it adds one specialized kernel rather than a second general FFT architecture. The Rader file stays outside the production module graph. The new workload test and benchmark entry points reuse existing crate dependencies and introduce no dependency or public API change.

## Numerical validation and checks

The M5 default-target crate tests passed: 11 library/unit tests, 2 across-row tests, 24 complex integration tests and 30 real integration tests. Coverage includes f32/f64 direct DFTs, large smooth/general-prime/Bluestein/SHT lengths, extreme inverse paths, normalization, endpoint handling and separate workspaces. New f64 C2C direct DFT cases cover 146/722/1442/1452. New f32 large-reference cases include the target workload lengths and N=4097. The Rader prototype validation passed direct C2C DFTs for p=241/257, real N=257/723 direct comparisons, RustFFT agreement, pinned DUCC output dumps and finite-range comparisons.

The M6 closeout added a concurrent f32 real Bluestein test for shared immutable plans with independent workspaces. The C2C phase-index change adds direct-prime coverage and a maximum-finite inverse fallback test for both precisions, with an actual `MAX` DC input. The current crate suite contains 69 tests: 11 library/unit, 2 across-row, 25 complex, and 31 real. The portable and release tests used `cargo test --manifest-path crates/rustsht-fft/Cargo.toml --all-features` and `cargo test --release --manifest-path crates/rustsht-fft/Cargo.toml --all-features`. Both suites passed after this change. The same suite had passed before this change with `CARGO_TARGET_DIR=/tmp/rustsht-fft-final-native-target RUSTFLAGS='-C target-cpu=native'`; the native-target suite was not rerun for the phase-index follow-up. The crate Clippy and formatting checks passed. Earlier rustdoc with warnings denied and standalone example commands are listed in [`standalone-readiness.md`](standalone-readiness.md) and passed. `cargo package --list` includes the API example and readiness guide. Exact future RustSHT workspace and adapter steps are also recorded there.

The workspace-wide RustSHT tests and Python binding tests were not run: all modifications are under `crates/rustsht-fft/**`, and no SHT or binding source changed. No test tolerances were weakened. The measured row batches are not complete SHT wall times. RustSHT-FFT is functionally implemented but performance-incomplete, and the package remains `publish = false` pending the gaps listed below and full SHT integration measurements.

## Final standalone review

The read-only R37 readiness review accepted the isolated manifest, runtime dependency set, attribution, normalization/layout contract, performance caveats, and standalone integration guide. It requested two documentation corrections; both are applied and recorded in [`review-log.md`](review-log.md). The added f32 concurrent real Bluestein test was confirmed to use independent forward and inverse DFT references and one workspace per worker. No additional M6 blocker was found; the documented performance gaps still prevent a performance-complete publication recommendation. Earlier checkpoint dispositions remain in the review log.

## Remaining bottlenecks and ranked follow-up

1. **Real Rader production route for the two measured cases.** Expected impact is highest at T127 N=257 (about 4× in the isolated public R2C/C2R prototype) and useful for T360 N=723 (about 2.3×). First design one reusable prime-pass representation that owns nested complex plan and scratch, then preserve the production normalization and extreme-value fallback. Require direct DFT, RustFFT, pinned DUCC, maximum-finite, cancellation, dynamic-range and subnormal tests before changing planner choice.
2. **C2C general-prime terminal pass.** N=1442 remains the largest measured meridional gap after the scalar phase-index change; the measured 1.20–1.24× repeated-row gain is provisional. Repeat the source comparison under low host load, then re-profile the radix-103 pass before deciding whether a bounded `ido==1` SIMD prototype can justify its added complexity.
3. **Full SHT sampling evaluation.** Measure the complete scalar and spin-1 forward/inverse operation at the minimum odd/even counts and current `2L+3`. Keep all longitude orders through L and verify analysis/adjoint identities at each candidate grid.
4. **f32 general-prime/Bluestein gap.** The new kernel is f64-only. T127/T360 f32 still spend heavily in the same prime routes. Reuse the numerical guard and cost evidence before writing f32 SIMD.
5. **Remaining smooth-path differences.** Continue only with a specific profile-supported bottleneck; the present M5 data does not justify a planner rewrite or another generic decomposition framework.
6. **External worker scaling.** The crate safely supports shared immutable plans with separate workspaces, but eight-worker throughput is unmeasured. Benchmark the RustSHT scheduler's real row partition after integration rather than adding a worker pool to this crate.

## Detailed results index

- [`m5-sht-real-f64-summary.csv`](../benches/results/m5-sht-real-f64-summary.csv): all target T, longitude roles, unique N, single/scalar/spin-1 rows, R2C/C2R timings, ratios, candidate pair ranges, plan and scratch.
- [`m5-sht-real-f32-summary.csv`](../benches/results/m5-sht-real-f32-summary.csv): f32 target rows and setup/scratch.
- [`m5-sht-c2c-summary.csv`](../benches/results/m5-sht-c2c-summary.csv): C2C forward/inverse, single and meridional row calls.
- [`m5-phase-breakdown-screen.csv`](../benches/results/m5-phase-breakdown-screen.csv): phase-separated cross-library timing screen for current longitude sizes.
- [`m5-rader-prototype-summary.csv`](../benches/results/m5-rader-prototype-summary.csv): full-call Rader/Bluestein prototype comparison, setup and scratch.
- [`m5-factor-order-screen.csv`](../benches/results/m5-factor-order-screen.csv): current/reversed factor-order results.
- [`m5-rust-native-target-summary.csv`](../benches/results/m5-rust-native-target-summary.csv): portable runtime-dispatch versus target-native screen.
- [`m5-f32-accuracy.csv`](../benches/results/m5-f32-accuracy.csv): f32 per-length accuracy metrics.
- [`m6-c2c-phase-index-process-samples.csv`](../benches/results/m6-c2c-phase-index-process-samples.csv): alternating source/candidate C2C samples for N=722 and N=1442, including single and row-batch setup, execution, scratch and checksums.
