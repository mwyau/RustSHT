# M8 real Rader route at N=2049

Date: 2026-10-15. Baseline: M7 commit `d9666bf`.

## Result

The f64 real plan at N=2049 now uses Rader for its terminal radix-683 pass
(`l1=3`, `ido=1`). The Rader cyclic-convolution length is 682. The plan retains
the radix-683 Bluestein route with convolution length 1372 for inputs outside
the Rader safety range. Other precisions and lengths retain their existing
routes. Six alternating process pairs improved the 1,024-row target by 19.8% for R2C
and 17.5% for C2R against M7; both paired 95% intervals are above parity. Both directions also improved at one and 32 rows. The N=241, N=257 and N=723
controls had no median regression above 1.4%, below the predeclared 5% limit. The N=2049 route adds 61,697 bytes to retained plan payload and 22,816 bytes
to the reported workspace payload. The payload counts initialized elements
and exclude allocator metadata. Plan and workspace construction are reported
separately in the paired samples. These one-dimensional FFT measurements do
not establish a whole-SHT speedup.

## Correctness and dispatch

`rader_2049_matches_independent_real_transform_references` checks R2C and
C2R against direct DFT references and RustFFT 6.4.1. It also checks finite
high-amplitude outputs, the high-amplitude constant's DC value, and workspace
reuse after an extreme input. A four-thread test shares the immutable plan
with one workspace per worker and checks impulse transforms in both
directions. Unit tests verify the selected Rader route and Bluestein fallback
plan, ordinary execution, and safety fallback for high-amplitude and
subnormal inputs. The plan trace records Rader-683 for the real pass and Bluestein-1372 as its
fallback. The convolution FFT has length 682 (`2 * 11 * 31`): its radix-2
prefix executes 340 AVX2 lanes and one scalar tail on this host, followed by a
radix-11 pass and a general radix-31 pass. Runtime feature detection guards
the AVX2 path. The full trace is in
[`m8-rader-683-route-diagnostics.txt`](../benches/results/m8-rader-683-route-diagnostics.txt). The benchmark-only prototype screen used two alternating pairs and favored
the prototype by a median 1.15x for R2C. C2R favored it by 1.41x, but that
screen ran at elevated host load and was treated as provisional. The
production acceptance campaign below is the decision evidence. The prototype
kept the direct-reference tolerance unchanged; its N=2049 dense inverse case
was checked against production while sparse inverse inputs were checked
against an independent direct transform. Raw screen samples and the
two-pair summary are in the [R2C capture](../benches/results/m8-rader-683-screen-r2c-two-pairs.jsonl),
[C2R capture](../benches/results/m8-rader-683-screen-c2r-two-pairs.jsonl), and
[screen summary](../benches/results/m8-rader-683-screen-summary.csv). The read-only Sol High review found no actionable issue with primitive-root
selection, convolution signs or scaling, workspace ownership, the `l1=3`,
`ido=1` layout, precision gating, or safety fallback. No tests were run by the
reviewer.

## Paired latency against M7

Each process reported the median of seven warmed samples. Six alternating
process pairs were pinned to logical one CPU core. Ratios are `M7 time / M8 time`,
so values above one favor M8. Intervals are 95% bootstrap intervals over the
six paired ratios. Plan and workspace construction were measured separately.

| N | Rows | R2C speedup (95% interval) | C2R speedup (95% interval) |
|---:|---:|---:|---:|
| 2049 | 1 | 1.196 (1.189–1.219) | 1.185 (1.172–1.190) |
| 2049 | 32 | 1.197 (1.189–1.213) | 1.182 (1.175–1.191) |
| 2049 | 1024 | 1.198 (1.191–1.214) | 1.175 (1.166–1.184) |

At 1,024 rows, median public-operation times were 75.383 µs R2C and
77.793 µs C2R for M8, compared with 90.221 µs and 91.490 µs for M7. The
N=2049 plan-construction median was 118.194 µs for M8 and 74.602 µs for M7;
workspace construction was 2.489 µs and 2.218 µs, respectively. The control medians use `M7 time / M8 time`; values below one indicate a
regression:

| N | Rows | R2C ratio | C2R ratio |
|---:|---:|---:|---:|
| 241 | 120 | 1.000 | 1.004 |
| 257 | 128 | 1.004 | 1.015 |
| 723 | 361 | 0.987 | 1.009 |

The largest control median regression was 1.4% at N=723 R2C. Its paired
interval was 0.977–0.996; no control median regression exceeded 5%. Raw samples, per-process configurations and summaries are recorded in
[`m8-rader-683-acceptance-target-six-pairs.jsonl`](../benches/results/m8-rader-683-acceptance-target-six-pairs.jsonl),
[`m8-rader-683-controls-summary.csv`](../benches/results/m8-rader-683-controls-summary.csv),
and the matching `m8-rader-683-control-*-six-pairs.jsonl` files in
[`benches/results`](../benches/results/).

## Comparison with other FFT libraries

The exact SHT-row target workload was measured in six alternating process
pairs against RustFFT 6.4.1, DUCC revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf` in portable and native builds,
system FFTW 3.3.10, and Admiral revision
`ce67bc00ff9e6de96bf5b75f7a789581105ddb58`. The table gives `reference time /
M8 time` at N=2049 and 1,024 rows. Values below one mean the reference is
faster; intervals are paired 95% bootstrap intervals.

| Reference | R2C ratio (95% interval) | C2R ratio (95% interval) |
|---|---:|---:|
| RustFFT 6.4.1 | 0.487 (0.479–0.493) | 0.469 (0.465–0.472) |
| DUCC portable | 0.679 (0.675–0.683) | 0.665 (0.661–0.669) |
| DUCC native | 0.515 (0.512–0.528) | 0.497 (0.492–0.514) |
| FFTW 3.3.10 | 0.565 (0.558–0.565) | 0.547 (0.539–0.552) |
| Admiral | 0.372 (0.362–0.378) | 0.369 (0.366–0.374) |

All five references were faster in both directions at this workload. The full
target matrix, including C2C operations and the other target lengths, is in
[`m8-crosslib-sht-targets-summary.csv`](../benches/results/m8-crosslib-sht-targets-summary.csv). Raw samples and reference configurations are recorded for
[RustFFT](../benches/results/m8-crosslib-rustfft-6.4.1-six-pairs.jsonl),
[portable DUCC](../benches/results/m8-crosslib-ducc-c1ed01b8-portable-six-pairs.jsonl),
[native DUCC](../benches/results/m8-crosslib-ducc-c1ed01b8-native-six-pairs.jsonl),
[FFTW](../benches/results/m8-crosslib-fftw-system-3.3.10-six-pairs.jsonl), and
[Admiral](../benches/results/m8-crosslib-admiral-ce67bc00-six-pairs.jsonl).

## Build and validation

Measurements used an x86-64 CPU under Linux x86-64, Rust 1.99.0
(`rustc 1.99.0`, LLVM 23.1.1), the release bench profile, a portable compiler
target, runtime AVX2, and one process pinned to one CPU core. The M7 baseline
executable SHA-256 is
`028a36f5cd4a32f8c5d180341623b27c0ece10e709e44ac85bee81714a268401`. The M8
candidate executable SHA-256 is
`162c081879b49e38a7221d9038ffb5fc82847320bc4cb3f3b2ba43fe2b270156`. The following checks passed in `crates/rustsht-fft/`:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features` (97 passed)
- `cargo test --release --lib m8_real_2049_uses_rader_683_and_preserves_bluestein_fallback`
- `cargo test --release --test real rader_2049` (2 passed)
- `cargo bench --locked --bench rustfft --no-run`
