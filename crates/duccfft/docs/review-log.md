# Review log

| Checkpoint | Date | Finding | Disposition |
| --- | --- | --- | --- |
| R0 | 2026-10-12 | The comparison script reported the scaled error at the maximum-absolute-error component instead of the true maximum, and did not reject a nonfinite DUCC reference value. | Fixed by tracking both maxima independently, reporting each location, and rejecting nonfinite values on either side. The full 151,606-value comparison passes in scalar and vectorized DUCC modes. |
| R0 | 2026-10-12 | RootCalc precision was described as long double throughout. | Corrected: the initial angle expression uses a long-double pi constant, then f32/f64 roots store the angle and perform trigonometry in double precision. |
| R0 | 2026-10-12 | The source map incorrectly excluded threading within a direct 1D plan. | Corrected: large complex factorizations can enter nested multipass scheduling when SIMD support is compiled; the reference harness explicitly passes one thread. Rust execution remains caller-scheduled by design. |
| R0 | 2026-10-12 | Real forward pass order, real Bluestein nesting, and the complex alignment allowance were inaccurate or omitted. | Corrected: real forward traverses in reverse construction order, real inverse in construction order; `rfftpblue` creates an `ip` complex plan whose complex Bluestein subplan uses `good_size_cmplx(2*ip-1)`; complex `bufsize()` includes 32 complex alignment values for lengths divisible by 1,024. |
| R0 | 2026-10-12 | The harness could not exercise DUCC's optional vectorized planner ceiling. | Added `DUCCFFT_DUCC_VECTORIZE=1` to pass `vectorize=true` into direct C2C/R2C/C2R plans and recorded the mode in benchmark and comparison output. |
| R0 | 2026-10-12 | Acknowledging an external checkout's license text would be insufficient if source code is translated. | Added the complete BSD-3-Clause conditions and disclaimer plus the applicable pinned-source copyright attributions in `LICENSE-THIRD-PARTY`. |
