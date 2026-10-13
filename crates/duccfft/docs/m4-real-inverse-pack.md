# Real inverse eligibility and spectrum packing

Commit `8ed5c87` combines the real multipass inverse overflow check with the
conversion from the public half-spectrum to the packed real buffer. Before
this change, an eligible inverse read the spectrum once in
`inverse_normalization_is_safe` and again in `pack_spectrum`. The combined
pass checks the existing eligibility condition while writing the packed
buffer. It runs the multipass transform only when every used component is
finite and the maximum magnitude is at most `MAX/(4*N)`. The check ignores imaginary DC and, for even lengths, imaginary Nyquist, as
the public inverse operation does. If any used value is nonfinite or exceeds
the bound, the packed buffer is not consumed; execution uses the existing
residual-corrected recursive path. Plans without a real multipass path retain
the previous normalization check. No plan or workspace storage changed.

## Numerical validation

The real regression test now supplies NaN imaginary DC and infinite imaginary
Nyquist values for both f32 and f64. The inverse still returns zeros for the
zero real spectrum. Existing tests continue to cover maximum finite values,
subnormals, cancellation, dynamic-range mixtures, direct DFT comparisons,
large smooth lengths, primes, Bluestein, complexify plans, and f32 references. Validation on the candidate source passed:

- `cargo test --manifest-path crates/duccfft/Cargo.toml --all-features`: 61 tests passed.
- `cargo clippy --manifest-path crates/duccfft/Cargo.toml --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --manifest-path crates/duccfft/Cargo.toml --all -- --check`: passed. All paired baseline and candidate benchmark checksums match. The benchmark
checksum observes the first output value; the numerical tests compare full
outputs against independent references.

## Paired public-operation measurements

The baseline source was `5f8bf5f`; the candidate source is `8ed5c87`. Both
used the same `benches/rustfft.rs` source (SHA-256
`bbbcb779effb7c1121f4ad08411eea0207c7ec38eccc75278f57d0fd46784457`). Rust
1.99.0 (`b940084d7`, LLVM 23.1.1) and Cargo 1.99.0 built the `bench` profile
at `opt-level=3`, with no custom `RUSTFLAGS`, LTO, or target CPU setting. The
baseline and candidate benchmark binary SHA-256 values are
`691222ff38f890c02e9436368ca0291e18a07b6db07c57780ef3435a733eef26` and
`d7eeea36b68d2cdd924a668345f5b8737dfe7132944698a6a20ba27dea44b980`. The CPU was an x86-64 CPU. Each process was pinned to logical one CPU core
and used one execution thread. Twelve process pairs were collected at each
length; baseline-first and candidate-first order alternated. Each process
warmed once and reported the median of seven timed calls. The input spectra
use `spectrum_value(index, n, row)` from the benchmark harness. A 32-row batch
reuses one plan and workspace. Each plan and workspace setup value is an
average across 20 constructions or allocations. The host also ran the
Legendre workload, so the small wall-time changes remain provisional. The table reports per-transform microseconds. Paired latency reductions are
the medians of `1 - candidate / baseline` over the twelve process pairs;
negative values mean that the candidate was slower. R2C is an unchanged
timing control.

| N | C2R single, baseline → candidate | C2R paired latency reduction | C2R batch 32, per transform, baseline → candidate | C2R paired latency reduction | R2C single control, baseline → candidate | R2C batch 32 control, baseline → candidate |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 0.1610 → 0.1610 | 0.00% | 0.1349 → 0.1400 | −4.07% | 0.1800 → 0.1800 | 0.1583 → 0.1583 |
| 1,024 | 2.5500 → 2.5050 | 1.76% | 2.5371 → 2.5089 | 1.25% | 3.0310 → 3.0560 | 2.8990 → 2.9198 |
| 1,920 | 5.7210 → 5.5900 | 2.62% | 5.7133 → 5.5928 | 2.44% | 6.4170 → 6.4120 | 6.1462 → 6.1793 |
| 4,096 | 11.4815 → 11.2660 | 1.96% | 11.5528 → 11.3398 | 1.60% | 13.2895 → 13.3600 | 13.3953 → 13.4060 |

The C2R single-call and batch timings favored the candidate in 11/12 and
10/12 pairs at N=1,024, 12/12 pairs at N=1,920, and 11/12 and 10/12 pairs at
N=4,096. N=64 did not benefit. Normalizing each C2R result by its same-process
R2C control gives median gains of 2.21%/1.88% at N=1,024, 2.50%/2.98% at
N=1,920, and 1.93%/1.73% at N=4,096 (single/batch). This control-adjusted
comparison supports a small improvement for longer multipass inverses, not a
broad latency reduction at every length. Plan and workspace setup remain unchanged in behavior. Medians across the
12 process samples and reusable workspace bytes are:

| N | Baseline plan / workspace, µs | Candidate plan / workspace, µs | Scratch bytes, both |
|---:|---:|---:|---:|
| 64 | 1.787 / 0.492 | 1.790 / 0.486 | 2,048 |
| 1,024 | 17.910 / 1.100 | 18.042 / 1.181 | 27,776 |
| 1,920 | 33.071 / 1.868 | 33.199 / 1.798 | 56,816 |
| 4,096 | 69.318 / 3.225 | 69.416 / 3.212 | 109,760 |

The 384 raw process records, including timings, setup, scratch, process order,
and checksums, are in
[`m4-real-inverse-pack-paired-process-samples.csv`](../benches/results/m4-real-inverse-pack-paired-process-samples.csv).

## Instruction counts

each build. The counts include plan construction, workspace allocation, and
input setup; they are instruction references, not timings. For the ordinary
`spectrum_value` input, references fell from 27,367,075 to 26,733,067 (2.32%). For the high-range fallback input, references rose from 253,662,416 to
256,619,655 (1.17%). The latter includes the extra packing pass before the
recursive fallback. The correctness benefit and ordinary-path gain retain
the high-range correction path; this small fallback cost is recorded rather
than hidden. The four compressed profiles are
and
