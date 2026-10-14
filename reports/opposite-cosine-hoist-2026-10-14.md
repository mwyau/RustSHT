# Opposite-parity cosine hoist experiment

## Hypothesis

Mirrored scalar analysis accumulates the odd-parity term as `opposite * basis * cos(theta)` for every degree. At a fixed ring pair, the Fourier term and cosine do not depend on degree. Precomputing `opposite * cos(theta)` once per pair should remove two multiplies per latitude lane from each degree's real/imaginary accumulation. The temporary implementation preweighted the existing opposite-parity component buffers only when both components remained normal (or exactly zero) after multiplication. It retained the original path when preweighting could create a subnormal, and kept the existing scaled multiplication helper for recurrence lanes with binary exponents. Scalar and spin-1 mirrored analysis were included; no workspace allocation was added. The change altered floating-point multiplication order.

## Result

The candidate did not improve whole-transform time and was removed. Five paired T511 process runs showed a 1.26% scalar-analysis regression at one thread and no change at eight threads. Spin-1 analysis changes were within measurement variability. Three T720 pairs showed no stable gain. The additional block scan and range checks offset the expected reduction in repeated multiplication. Native timings used f64 GL, `ntheta=lmax+1`, `nphi=2*lmax+3`, full `mmax`, `phi0=0.37`, reused plan/workspace, one warm-up, five timed operations per thread pool, and paired alternating processes 8 pinned CPU cores. Each `m1b_bench` process measured one-thread and eight-thread paths. The baseline was commit `5e9cc6a`; the candidate was an uncommitted prototype on that source. The host was an x86-64 CPU, Linux, Rust `1.99.0 (b940084d7 2026-09-28)` / LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch (AVX2/FMA on this CPU), no `target-cpu=native`. Values are medians of process medians in milliseconds. The paired change column uses the median percentage change within each alternating baseline/candidate pair; ranges show all paired changes.

| Workload | Threads | Baseline | Candidate | Paired change (range) |
| --- | ---: | ---: | ---: | ---: |
| T511 scalar analysis | 1 | 23.529 | 23.904 | +1.26% (-0.99% to +2.67%) |
| T511 scalar analysis | 8 | 7.326 | 7.407 | -0.12% (-1.34% to +2.28%) |
| T511 spin-1 STANDARD analysis | 1 | 39.433 | 39.656 | +0.30% (-1.22% to +1.34%) |
| T511 spin-1 STANDARD analysis | 8 | 12.632 | 12.710 | -0.26% (-2.44% to +2.91%) |
| T720 scalar analysis | 1 | 64.297 | 64.258 | -0.06% (-0.13% to +0.04%) |
| T720 scalar analysis | 8 | 20.011 | 19.968 | +0.96% (-1.47% to +4.36%) |
| T720 spin-1 STANDARD analysis | 1 | 107.219 | 107.190 | +0.33% (-1.10% to +1.03%) |
| T720 spin-1 STANDARD analysis | 8 | 34.959 | 35.201 | +1.10% (-1.83% to +1.32%) |

Raw samples for all 32 process runs are in [`paired-process-samples.txt`](data/opposite-cosine-hoist-2026-10-14/paired-process-samples.txt). No memory change was measured or expected; the temporary path reused existing worker arrays.

## Numerical checks and profiling context

The prototype passed its focused ordinary/mixed-scaled-lane comparison and preweight underflow guard. It also passed the T255 scalar DUCC coefficient and adjoint test, the spin-1 all-grid scalar-conversion/general recurrence and adjoint comparison, and the pinned DUCC GL spin-1 analysis test. The full high-band and workspace suites were not run for a prototype that showed no whole-transform improvement; the committed baseline remains covered by the preceding full validation. No source change, public API, numerical convention or FFT code was retained. The previous scalar normalization fast path at `5e9cc6a` remains unchanged.
