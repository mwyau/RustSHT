# Exact-zero lanes improve f32 spin-1 q1 analysis by 2.4–2.9% at T511

The f32 spin-1 analysis SIMD path now accepts exact positive and negative zero in Fourier inputs and recurrence basis values. Subnormal values, scaled recurrence lanes, and inactive latitude lanes still use the scalar accumulation path. A T511 GL comparison against the preceding commit measured a repeatable 2.4–2.9% reduction in one-thread analysis time. The eight-thread median also decreased, but one repeat-screen pair was 16.6% slower, so that result is less stable.

## Method

`examples/m4_f32_bench.rs` measured spin-1 transforms on a GL `512x1025` grid at T511, f32, full `mmax`, and `phi0=0.37`. It used a reusable plan and workspace, deterministic coefficients and map inputs, one warmup, and the median of five calls per operation. Each fresh process measured synthesis and analysis with one and eight threads. Five baseline/candidate process pairs ran in alternating order for each screen, pinned to CPUs 0–7, under `target/rustsht-benchmark.lock`.

The baseline was commit `ed9a4af4537781d3446cbd79bd19984a78263069`. The candidate binary included the source patch saved in the data directory. The binaries had distinct SHA-256 hashes, recorded with the raw runs. Both used Rust 1.99.0, LLVM 23.1.1, release optimization, and portable Pulp runtime dispatch. The host was an AMD Ryzen 9 5950X with AVX2/FMA under Linux `7.0.14-20-pve`; it was shared during both screens. Load averages before the first and repeat screens were 2.05/1.73/2.66 and 1.42/1.70/2.51.

## Results

Times are medians of the five per-process call medians. Paired change is the median candidate/baseline percentage across five process pairs; ranges show the full paired spread. Negative values favor the candidate.

| Screen | One-thread analysis, baseline → candidate | Paired change | Eight-thread analysis, baseline → candidate | Paired change |
| --- | ---: | ---: | ---: | ---: |
| First | 870.791 → 847.048 ms | -2.43% (-4.07% to -2.29%) | 164.142 → 159.461 ms | -2.52% (-4.20% to -2.14%) |
| Repeat | 871.545 → 846.319 ms | -2.86% (-3.22% to -2.23%) | 164.691 → 162.535 ms | -1.00% (-3.11% to +16.62%) |

The combined one-thread synthesis-plus-analysis median decreased from 1898.653 to 1842.673 ms in the first screen (-2.74%; paired range -2.95% to -2.60%) and from 1900.036 to 1841.298 ms in the repeat (-3.05%; -3.18% to -1.92%). The synthesis-only one-thread control also decreased by 2.85% and 3.15%. The source change affects analysis eligibility, so the synthesis movement is recorded as a binary-level observation and is not attributed to this change. At eight threads, one repeat pair measured analysis 16.62% slower and combined time 11.02% slower; the cause is undetermined.

Pooled process peak RSS had a 15,558 KiB median (15,284–15,884 KiB) in the first screen and 15,578 KiB (15,092–15,660 KiB) in the repeat. The change adds no workspace arrays. The screen did not include a new DUCC wall-time comparison.

## Validation

The lane tests compare dispatched SIMD accumulation with a lane-wise reference for 64- and 61-lane blocks, including tails, signed zeros, subnormal guards, scaled exponents, inactive cutoffs, and E-only and E/B output. The f32 GL T511 analysis test passes against DUCC 0.41.0, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

`cargo fmt --all -- --check` and workspace Clippy with warnings denied passed. `cargo test --workspace --all-features` passed 184 unit tests, 11 f32 integration tests, and doctests; 12 diagnostic tests were ignored. `cargo test -p rustsht --no-default-features` passed 146 unit tests, 11 f32 integration tests, and doctests; seven diagnostic tests were ignored. The shared `/tmp` volume was full, so the test runs used a temporary directory under `target/`.

Raw process logs, timing files, environment snapshots, binary hashes, the candidate patch, runner scripts, and paired results are in [`reports/data/f32-spin1-zero-lane-simd-2026-10-15`](data/f32-spin1-zero-lane-simd-2026-10-15/).
