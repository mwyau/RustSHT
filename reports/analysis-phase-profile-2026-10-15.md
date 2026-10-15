# Scalar and spin-1 analysis phase profile

## Method

A test-only release profile timed scalar and spin-1 GL STANDARD analysis at T511 and T1023 with full `mmax`, `ntheta=lmax+1`, `nphi=2*lmax+3`, and `phi0=0.37`. It used explicit q1 and q6 Rayon pools and five successive calls per case after plan and workspace construction. The first call was included; no warm-up was discarded.

The timers separate forward longitude FFTs, latitude preparation, scalar order-spectrum preparation, coefficient initialization and cleanup, the analysis order-work prefix, and Legendre recurrence with coefficient accumulation. The FFT phase covers the existing RustFFT calls in their current serial ring loops. It does not profile RustFFT internals.

The compiler was rustc `1.99.0 (b940084d7 2026-09-28)` with LLVM `23.1.1`, using `cargo test --release`. The host was an AMD Ryzen 9 5950X with 16 cores and 32 logical CPUs, running Linux `7.0.14-20-pve`. The process inherited CPUs 0–31 and had no explicit affinity. The CPU exposes AVX2 and FMA; Pulp runtime dispatch was enabled, but the diagnostic did not log the selected ISA. No peak RSS was collected. The phase values are diagnostic samples from one process, not paired performance measurements.

The raw output is in [`profile.log`](data/analysis-phase-profile-2026-10-15/profile.log). Reproduce it with:

```text
cargo test --release -p rustsht --all-features m5_analysis_phase_profile -- --ignored --test-threads=1 --nocapture
```

## Results

Times are medians across five samples in milliseconds. The wall range is the minimum-to-maximum sample range; phase ranges and per-sample values are preserved in the raw log.

| Operation | Degree | Threads | Whole call | Forward FFT | Recurrence and accumulation |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar | T511 | 1 | `22.02 (21.74–28.00)` | `3.65` | `17.94` |
| Scalar | T511 | 6 | `8.41 (8.05–15.51)` | `3.39` | `4.60` |
| Scalar | T1023 | 1 | `171.22 (168.22–172.77)` | `38.14` | `127.99` |
| Scalar | T1023 | 6 | `75.50 (73.69–77.45)` | `39.60` | `33.86` |
| Spin 1 | T511 | 1 | `35.26 (35.17–42.71)` | `7.49` | `27.84` |
| Spin 1 | T511 | 6 | `15.57 (15.01–16.13)` | `6.81` | `8.29` |
| Spin 1 | T1023 | 1 | `269.54 (265.73–274.72)` | `76.88` | `190.47` |
| Spin 1 | T1023 | 6 | `139.05 (133.02–148.38)` | `79.74` | `52.31` |

At q6 T1023, the serial row FFT phase accounts for 52% of the scalar whole-call median and 57% of the spin-1 median. Recurrence and accumulation account for 45% and 38%, respectively. At q1 T1023, recurrence and accumulation account for about 75% of scalar time and 71% of spin-1 time. The order-spectrum preparation phase adds a median 2.48 ms for scalar T1023 q6; latitude setup, the order-work prefix, and coefficient initialization each take less than 0.6 ms in these cases.

The phase sums match whole-call time within 0.07 ms in every sample. The profile locates a second parallelism target at moderate and high worker counts: the recurrence already uses multiple Rayon tasks, while the independent row FFT calls execute one after another.

## Next experiment

Schedule independent longitude rows across the workspace's requested execution capacity, with one private `RealFftWorkspace` per worker. Keep the existing RustFFT plan, packing, scaling, and per-row transform unchanged. Start with GL scalar and spin-1 analysis at T511/T1023, compare q1 and q6 outputs, and measure workspace memory and whole-call time. Any accepted change needs alternating baseline/candidate process pairs and the existing DUCC and analytic correctness checks.

This profile does not establish the benefit of parallel row FFTs. It does not cover CC/F1 meridional resampling, synthesis, other pool sizes, or independent-field throughput.

The ignored release profile test passed. `cargo fmt --all -- --check` and workspace Clippy with warnings denied passed. The profile uses deterministic map inputs but does not compare coefficients against DUCC or an analytic field; the existing numerical suite was unchanged by this test-only instrumentation.
