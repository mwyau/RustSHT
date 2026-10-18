# Scaled-prefix exponent-epoch accumulator

## Status

The exponent-epoch accumulator is an isolated, unaccepted experiment. It leaves production routing unchanged. Its recurrence tests and the full Rust workspace suite passed. One instrumented T2047 q1 transform produced route timings, but the experiment has no source-matched parent comparison or stability interval. The timing does not establish a speedup.

## Method

The candidate accumulates each lane's complex prefix products in the recurrence's current exponent domain. When recurrence normalization changes a lane's exponent, it scales that lane's staged sum into the tile output and starts a new accumulation epoch. It flushes all staged sums before entering the ordinary SIMD suffix. Non-normal products or partial sums, unsafe coefficients, and non-finite output sums decline the staged route; the existing scaled path then computes the tile. The new accumulators are reusable workspace arrays.

The patch applies after the archived tile-128 integration and source in [route accounting](../route-accounting/summary.md). Reconstruct that candidate, then apply [`prefix-exponent-epoch-accumulator.patch`](prefix-exponent-epoch-accumulator.patch) with `git apply --unidiff-zero`. Its SHA-256 is `badc0b317705d27f2d846028baf577fd4fb4d880ab3b7e603a92c5bff89c1fb2`. The resulting `synthesis_fast.rs` SHA-256 is `0f12d2aa0881d97dff66fb2b0a63ddbbfae0fb295bc67b066fe9fcced624d072`; the release test binary SHA-256 is `a2c58fa4a1a07017058e5adfa433014d693c35f93f70bd7b5406e5c4a5288e86`.

## Profile

The test used GL f64 scalar synthesis with `lmax=mmax=2047`, a `2048x4096` grid, `phi0=0.37`, one Rayon worker, and deterministic coefficients of magnitude `1e-5`. It made five fixed warm-up transforms and one timed transform with route counters and timers enabled. The worker was pinned to one allowed CPU. Preflight and postflight CPU PSI checks passed; the local review found no competing CPU-intensive process. The profile did not record per-thread placement.

| Component | Time (ms) | Scope |
| --- | ---: | --- |
| Full `synthesis_into` call | 1378.917 | One instrumented call |
| Fast transition, including SIMD suffix | 1227.036 | Summed across completed tiles |
| Ordinary SIMD suffix | 532.543 | Nested in the fast transition |
| Pre-suffix transition estimate | 694.493 | Transition minus suffix; includes setup |
| Scaled fallback | 12.173 | Summed across fallback calls |
| Coefficient-safety scans | 73.172 | Summed across tile scans |
| Peak process RSS | 298592 KiB | Aggregate high-water mark |

The route counters recorded 23,201 completed tiles, no declined tiles, 110,608,856 prefix pair-lanes, 1,622,437,750 suffix pair-lanes, and 1,331,458 fallback pair-lanes. Polar lanes account for the fallback work. Timer scopes overlap and must not be added to the full-call time. The pre-suffix estimate was 50.4% of the instrumented call. This source repeats the coefficient-safety scan for tiles; it does not include the separate coefficient-cache patch.

## Numerical checks and limits

Five focused fast-kernel tests passed, including the scaled-prefix transition against the paired recurrence reference. The whole-map route-accounting test produced bitwise-identical maps across its warm-up and timed calls. The full workspace run passed 201 library tests, 4 extreme-FFT tests, 11 f32 tests, and 1 workspace-compatibility test; 15 tests were ignored. Formatting and Clippy with warnings denied passed.

The profile has one timed call, so it has no coefficient of variation or drift result. It does not compare the experiment with its parent source, clean production, DUCC, or SHTns. No q8, T511, spin-1, or analysis result was collected. The existing tile-128 T511 baseline stability failure continues to block its regression matrix. The experiment remains isolated. The next performance decision requires a source-matched q1 full-transform comparison after the baseline passes the fixed stability and interference gates.
