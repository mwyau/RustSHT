# Spin-1 mirrored-ring synthesis

## Decision

Keep a narrow f64 GL STANDARD synthesis path that evaluates eligible north/south latitude pairs together. It is selected for full-band exact GL plans with `319 <= lmax <= 511` when the executing Rayon pool has four or eight threads. Five isolated process runs showed a large, repeatable T319 gain and a smaller but consistent paired-sample gain at T511. Other tested thread counts and T720 remain on the existing path.

This path reuses the spin-1-to-scalar coefficient conversion, current paired scalar recurrence, and existing workspace. It does not change FFT execution, numerical conventions, public APIs, adjoints, GRAD_ONLY, DERIV1, f32, CC/F1, or spins 2–4.

## Algorithm and eligibility

The spin-1 STANDARD coefficients are first converted into two scalar coefficient streams. The scalar reflection identity is then applied to each converted stream: even and odd degrees contribute with the same sign in the north, and the odd-degree contribution changes sign in the south. The two spin components are accumulated together in eight SIMD sums (real and imaginary, even and odd for each component), then divided by the shared `sin(theta)`.

The driver pairs mirrored GL rings while preserving their original output indices. It checks the polar order limit and `sin(theta) >= 0.01` for both rings independently. A block that has an excluded ring, scaled initialization, or a recurrence result outside the normal range uses the existing per-hemisphere recurrence path. An equatorial singleton on odd-`ntheta` grids also uses the existing path. Unsafe fused results are rejected before any Fourier output is scattered.

The deterministic selector requires all of the following:

- f64 paired-scalar support;
- spin-1 STANDARD synthesis with both E and B channels;
- a Gauss–Legendre grid with `ntheta = lmax + 1` and `mmax = lmax`;
- `319 <= lmax <= 511`;
- an executing Rayon pool with four through eight workers.

The band and worker limits reflect the measured results below. The method queries the executing pool at call time; plan construction does not capture a thread count. The prototype uses no calibration or global state.

## Benchmark method

- Baseline source: `56352fd` (`Record spin-1 analysis dispatch results`); final code is in the following implementation commit on `main`.
- CPU: AMD Ryzen 9 5950X, x86-64, AVX2 and FMA, no AVX-512.
- OS: Linux `7.0.14-20-pve`; CPU affinity: `taskset -c 0-15`.
- Toolchain: rustc `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`.
- Build: Cargo release profile, `opt-level=3`, 16 codegen units, LTO disabled, no `target-cpu=native`; Pulp runtime dispatch.
- Workload: f64 GL STANDARD spin-1 synthesis, `phi0=0.37`, full `mmax`, deterministic E/B coefficients scaled by `1e-5`. Plans, coefficients, workspaces and maps were created before timing.
- Exact timed grids: T319 `320x641`, T511 `512x1025`, and the ineligible T720 control `721x1443`. T319 and T511 use exact GL latitude counts; `nphi=2*lmax+3`.
- Thread counts: T319 at 1/2/4/8; T511 at 1/2/4/8/16; T720 at 8. Each case has two warm-up transforms and seven alternating, paired baseline/selected samples. Five separate release-test processes were pinned to CPUs 0–15.
- The timed region covers the complete transform, including recurrence, Fourier scaling, inverse longitude FFT and map writes. Plan and workspace setup are excluded. The selected branch goes through the production dispatch wrapper; ineligible cases execute the baseline path.
- Hardware event access remains restricted (`perf_event_paranoid=4`). No cycle or instruction claims are made.

The benchmark command was:

```sh
taskset -c 0-15 cargo test --release -p rustsht --all-features \
  benchmark_spin1_hemisphere_pair_dispatch -- --ignored --nocapture
```

Each raw process log contains seven timing samples per method and case. [`process-medians.csv`](data/spin1-hemisphere-pairing-2026-10-14/process-medians.csv) contains per-process medians and paired sample changes; [`process-01.log`](data/spin1-hemisphere-pairing-2026-10-14/process-01.log) through [`process-05.log`](data/spin1-hemisphere-pairing-2026-10-14/process-05.log) preserve the raw samples.

## Whole-transform results

Times are milliseconds. Baseline and selected columns are the medians across five process medians. “Paired change” is the median of each process's seven sample-by-sample percentage changes, then the median across five processes. Win counts count selected samples faster than their matched baseline sample. When the selector is false, the selected call deliberately runs the same existing path.

| Grid | Threads | Baseline | Selected | Paired change | Faster pairs | Selected |
|---|---:|---:|---:|---:|---:|---|
| T319 `320x641` | 1 | 18.709 | 18.708 | -0.39% | 24/35 | no |
| T319 `320x641` | 2 | 13.997 | 14.145 | +0.81% | 13/35 | no |
| T319 `320x641` | 4 | 12.825 | 9.831 | -22.71% | 35/35 | yes |
| T319 `320x641` | 8 | 8.044 | 6.856 | -12.57% | 35/35 | yes |
| T511 `512x1025` | 1 | 52.944 | 52.962 | -0.64% | 19/35 | no |
| T511 `512x1025` | 2 | 35.602 | 34.954 | +0.13% | 16/35 | no |
| T511 `512x1025` | 4 | 33.853 | 28.853 | -15.04% | 33/35 | yes |
| T511 `512x1025` | 8 | 19.971 | 18.574 | -4.09% | 29/35 | yes |
| T511 `512x1025` | 16 | 18.619 | 19.609 | +3.39% | 13/35 | no |
| T720 `721x1443` | 8 | 43.020 | 42.699 | -1.85% | 21/35 | no |

The q4 T319/T511 gains were present in all five process-level paired medians, with 33–35 of 35 individual samples faster. T319 q8 was faster in every paired sample. T511 q8 was faster in four of five process medians and 29 of 35 paired samples; its median paired reduction was 4.09%, so the selector includes it but the expected benefit is smaller than q4. T720 q8 and q16 T511 were not selected because their observed changes were small or negative and did not support a useful gain.

The host had noticeable frequency and scheduling variation, especially for T511. Pairing methods in the same process and alternating execution order reduces but does not remove this uncertainty. The q8 T511 result should be revisited if a future benchmark on a quieter host fails to reproduce it.

## Numerical validation

The final selected path was compared to the existing general spin-1 recurrence through the production dispatch wrapper. Relative-L2 errors for the first and second map components were:

| Case | First component | Second component |
|---|---:|---:|
| T319 q4 | `1.275e-15` | `1.051e-15` |
| T319 q8 | `1.417e-15` | `1.163e-15` |
| T511 q4 | `1.698e-15` | `1.363e-15` |
| T511 q8 | `1.780e-15` | `1.435e-15` |

Focused regressions also cover E-only, B-only and mixed coefficients; an odd-latitude equatorial singleton; a partial first latitude group; an even-longitude Nyquist order; and an extreme finite `1e300` coefficient that must retain a finite scaled-fallback result. The T319 mixed case satisfies the spin synthesis adjoint identity within `2e-11` relative error.

The T511 q8 sparse sectoral case compares five field samples with stored values from `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, at tolerance `8e-11`. This confirms agreement for that high-order field; it is not a new full-transform timing comparison against DUCC. Existing matched RustSHT/DUCC timings at T511 are recorded separately in the [independent-field study](batched-transform-study-2026-10-14.md).

The read-only Sol High review found no correctness blocker. It requested a partial-lane-group case with a Nyquist order and atomic fallback before output scatter. T321 now exercises the partial group, T319/T321 exercise the even-longitude Nyquist order, and the kernel validates all outputs before scattering any lane.

## Memory and implementation cost

The path adds no plan fields, persistent workspace arrays, duplicate maps, or full spectrum. It uses the existing two spin spectrum buffers and recurrence-worker scratch. Its fixed local reflected-geometry arrays occupy about 1.5 KiB per active worker. No separate peak-RSS measurement was taken because the implementation adds no retained buffer; workspace capacity remains bounded by existing worker sizing.

## Test results and limits

Passed:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo test --workspace --all-features` — 150 Rust tests and 10 f32 tests passed, 9 opt-in tests ignored;
- `cargo test -p rustsht --no-default-features` — 129 Rust tests and 10 f32 tests passed, 7 opt-in tests ignored;
- focused final-dispatch test — 2 passed, benchmark ignored;
- `uv run pytest python/tests` — 16 passed.

The selector does not cover CC/F1, f32, GRAD_ONLY, DERIV1, reduced `mmax`, T720+, one/two threads, or pools above eight threads. Those cases remain on existing kernels. The ignored T1023/T2047 spin tests were not run for this change; the selector stops at T511, and the added T511 regression covers the highest selected band.

No direct Rust-versus-DUCC latency measurement was part of this experiment. It establishes a measured reduction against RustSHT's matched general recurrence while retaining a pinned DUCC field check. The next performance step should profile the scalar-conversion path at q8 on a quiet host before changing the selector again.
