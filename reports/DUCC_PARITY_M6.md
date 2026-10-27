# M6 — current-head DUCC parity triage

**Source:** `26941cf45564a9637b72b0eca87ff880472c3505` on `dev`<br>
**DUCC:** 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`<br>
**Host:** AMD Ryzen 9 5950X, Linux x86-64; rustc/Cargo 1.99.1, LLVM 23.1.1; GCC 15.2.0.

This first M6 screen covers 14 selected warm whole-transform cells: GL T720 spin-1 analysis, protected T1023 synthesis controls, two reduced synthesis routes, and native-f32 GL T720 scalar/spin-1 analysis and synthesis. The rest of the agreed matrix remains unmeasured. No production algorithm changed during this screen.

## Method

Both Rust workers were built in release mode with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the f32 worker reported Pulp V3 and eight f32 lanes. DUCC numerical sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; both C++ drivers used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`. The pinned DUCC tree was clean at the exact source SHA. Binary and fixture SHA-256 values are recorded in the [per-cell CSV](data/ducc-parity-m6-2026-10-27.csv).

Each cell used three interleaved fresh-process pairs with shared deterministic fixtures, matching geometry, normalization, precision, and output dimensions. Timing covers warm calls using reusable Rust plans/workspaces and direct DUCC APIs; setup, fixture I/O, and output checks are outside the timed batches. The f64 worker targeted 25 ms and the f32 worker 10 ms per batch; each used ten batches for calls below 100 ms and five otherwise. q1 and q8 are thread counts. Workers pinned q1 to CPU 0 and q8 to eight distinct physical cores. f64 q8 used CPUs 0–7; f32 q8 used 0,2,4,6,8,10,12,14.

Three pairs give a triage range, not a confidence interval. The ledger retains each pair's Rust and DUCC medians, ratio, correctness data, fixture hashes, binary hashes, scheduler wait, and frequency samples. f64 comparisons passed finite, repeat, and reset checks. f32 comparisons passed finite, repeat, reset, and output-length checks; the f32 harness records DUCC-relative errors without enforcing a tolerance. Its broader independent-reference, adjoint, and exceptional-value gate remains open.

## f64 cells

| Workload | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Status | Max relative L2 |
|---|---:|---:|---:|---:|---|---:|
| GL T720 full spin-1 analysis q1 | 36.724 | 29.173 | 1.263× | 1.244–1.264× | **GAP** | 4.76e-14 |
| GL T720 full spin-1 analysis q8 | 5.285 | 4.244 | 1.236× | 1.232–1.249× | **GAP** | 4.76e-14 |
| GL T1023 full spin-1 synthesis q1 | 88.272 | 119.637 | 0.737× | 0.736–0.740× | **PASS** | 2.60e-13 |
| GL T1023 full spin-1 synthesis q8 | 14.072 | 19.737 | 0.719× | 0.704–0.720× | **PASS** | 2.60e-13 |
| CC T720 reduced m180 spin-1 synthesis q1 | 30.006 | 18.593 | 1.616× | 1.591–1.658× | **GAP** | 3.62e-13 |
| GL T720 reduced m180 over-resolved synthesis q8, ntheta=1441 | 4.255 | 4.647 | 0.916× | 0.913–0.920× | **PASS** | 2.27e-13 |

The T720 analysis gap is present at both thread counts. Rust q1 scheduler wait was 6.5–6.9% across its three analysis runs, while DUCC wait was low; all paired ratios exceeded 1.23. The CC reduced q1 Rust runs also had about 10% wait, and all three ratios remained above 1.59. The q8 analysis runs had low wait. The T1023 synthesis controls preserve the established speed advantage.

## Native-f32 cells

All cells use GL T720 full `mmax`, `ntheta=721`, `nphi=1458`.

| Transform | q | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Max relative L2 | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Scalar analysis | 1 | 591.691 | 13.922 | 42.515× | 42.398–42.606× | 1.83e-5 | **GAP** |
| Scalar analysis | 8 | 77.904 | 3.434 | 22.139× | 17.287–27.916× | 1.83e-5 | **GAP** |
| Scalar synthesis | 1 | 873.293 | 19.420 | 44.881× | 44.424–45.501× | 1.72e-4 | **GAP** |
| Scalar synthesis | 8 | 339.733 | 3.748 | 91.134× | 65.084–118.232× | 1.72e-4 | **GAP** |
| Spin-1 analysis | 1 | 1084.978 | 28.811 | 37.530× | 37.402–37.740× | 1.57e-5 | **GAP** |
| Spin-1 analysis | 8 | 141.391 | 4.420 | 32.144× | 29.616–32.463× | 1.57e-5 | **GAP** |
| Spin-1 synthesis | 1 | 1535.415 | 40.581 | 37.715× | 36.713–38.063× | 1.43e-4 | **GAP** |
| Spin-1 synthesis | 8 | 279.188 | 6.063 | 46.628× | 41.908–46.691× | 1.43e-4 | **GAP** |

The q1 f32 runs had low scheduler wait. q8 scalar analysis/synthesis saw median wait of about 14.6%/10.5%; spin-1 q8 wait was lower but variable. Those conditions widen the q8 ratios, while every observed f32 ratio remains at least 17× DUCC. The timing classification is a clear **GAP**; the dedicated f32 numerical acceptance gate remains pending.

## Next

Profile the current f64 GL T720 full spin-1 analysis route at q1 and q8. Normalize useful order/latitude/degree work, collect disjoint Callgrind costs, resolve executed basic blocks, and inspect optimized AVX2/FMA assembly before selecting one source change. The selected-cell ledger and bounded harness patch are in [the evidence directory](data/ducc-parity-m6-2026-10-27/README.md). This triage does not certify the broader f64/f32 matrix.
