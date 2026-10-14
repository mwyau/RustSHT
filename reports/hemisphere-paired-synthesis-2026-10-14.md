# Hemisphere-paired scalar synthesis experiment

## Decision

The test-only north/south recurrence-sharing prototype was rejected. It preserved the tested output to approximately `1–2e-14` relative L2, but it was slower than the current recurrence at both one and eight threads. No production source or API changed.

The experiment used RustSHT baseline commit `acced0f` on `main`. The current compact Legendre table already stores only northern latitudes and reconstructs the southern values by degree parity. The table is capped at T255; the recurrence comparison therefore used T511, where scalar STANDARD synthesis runs the on-the-fly recurrence. The existing mirrored-ring spin-1 GL analysis path is separate and unchanged. Scalar parity was not applied to spin-weighted synthesis.

## Hypothesis and prototype

For scalar harmonics,

`Y_lm(pi - theta, phi) = (-1)^(l+m) Y_lm(theta, phi)`.

In the paired-degree recurrence, degrees `l=m+2k` have even reflection parity and intervening degrees `l=m+2k+1` have odd reflection parity. For an eligible northern latitude block, the prototype evaluated the recurrence once and sent the normal even/odd coefficients to one output channel and the same even coefficients plus negated odd coefficients to the other channel. The second channel was scattered to the mirrored southern rings. It used the existing dual-channel fused synthesis kernel and `workspace.spin_spectra` as temporary storage; it added no workspace allocation.

Pairing was restricted to blocks where every north/south ring pair retained all orders under the current polar cutoff and every northern sine was at least `0.01`. Other blocks used the unchanged recurrence independently on both hemispheres. If coefficient preparation or the normal-range kernel declined a block, the prototype cleared partial output and used that same fallback. For odd latitude counts it handled the equatorial singleton through the ordinary one-ring recurrence.

## Measurement method

- CPU: AMD Ryzen 9 5950X, 16 physical cores, AVX2/FMA, no AVX-512.
- OS: Linux `7.0.14-20-pve`. Processes were pinned to physical CPUs 0–7; the host was shared.
- Rust `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO off, no `target-cpu=native`; Pulp used AVX2/FMA runtime dispatch.
- f64 GL T511, `ntheta=512`, `nphi=1025`, `mmax=511`, `phi0=0.37`; reused plan and workspace. q=1 and q=8 pools were constructed independently.
- Seven interleaved warm measurements per method per process, with three process repetitions. Each run compared complete maps against the ordinary Rust transform before timing.
- Exact command: `taskset -c 0-7 cargo test --release -p rustsht --lib benchmark_scalar_hemisphere_paired_synthesis_experiment -- --ignored --nocapture`.
- Full per-sample output is in [`prototype-samples.txt`](data/hemisphere-paired-synthesis-2026-10-14/prototype-samples.txt).

`perf_event_paranoid` was 4, so the prototype was judged from whole-transform wall time rather than hardware counters. The benchmark includes recurrence, fallback, Fourier scaling, the longitude inverse FFTs and map writes. This provides an end-to-end comparison under the same plan and workspace. It does not isolate per-stage time.

## Results

Times are process medians in milliseconds; each entry is `ordinary recurrence / paired prototype`.

| Threads | Process 1 | Process 2 | Process 3 | Median of process medians | Paired-path result |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 41.143 / 45.412 | 43.166 / 46.456 | 41.360 / 45.644 | 41.360 / 45.644 | 10.4% slower |
| 8 | 11.222 / 12.359 | 11.866 / 12.605 | 12.197 / 12.889 | 11.866 / 12.605 | 6.2% slower |

The cutoff and lane-size conditions limited the candidate to `64/256` mirrored pairs at q=1 and `96/256` at q=8. This corresponds to an estimated maximum reduction of 14.1% and 21.2%, respectively, in active latitude-degree recurrence updates. It does not include the extra paired accumulators, repeated factor preparation, fallback work, or scatter. The estimate is a work count, not a timing prediction.

The prototype processes latitude blocks outside the order loop. Each block therefore regenerates order factors and transforms coefficient streams. In contrast, the production recurrence prepares those values once per order and processes its latitude group. The prototype also accumulates two complex streams and scatters the reflected channel. The measured timings show that these costs exceed the recurrence work saved for this implementation.

### Numerical checks

For the exact T511 grid, relative-L2 difference from ordinary Rust synthesis was `1.274e-14` at q=1 and `1.936e-14` at q=8. The input used the same deterministic coefficient construction as the pinned DUCC Python benchmark. The direct candidate-to-DUCC output was not measured; the existing ordinary Rust T511 comparison to DUCC 0.41.0 reports `6.632e-14` relative L2 for this scalar synthesis workload in the [batch study](batched-transform-study-2026-10-14.md).

The prototype also passed an over-resolved GL control with an odd latitude count and even longitude count (`513 x 1022`, T511, `mmax=511`, nonzero `phi0`). This exercised the equatorial singleton and allowed Nyquist-order synthesis. Relative-L2 difference from ordinary synthesis was `1.309e-14` at q=1 and q=8. The q=8 lane block changed to 64 in this configuration, so those timings were not used as a matched performance result.

## Disposition

No code change was accepted. The prototype was removed after testing. This result rejects the coarse fully-active-block pairing strategy at T511; it does not establish that every order-major active-pair design is slower. A finer compaction or order-major design would need to avoid repeated factor preparation and demonstrate an end-to-end gain before further work. It is deferred because the measured candidate regressed consistently, the safe full-active region covered only a fraction of total active work, and it would introduce another scheduling/layout path. The next experiment should remain on the approved spin-0/1 sequence and be measured independently.
