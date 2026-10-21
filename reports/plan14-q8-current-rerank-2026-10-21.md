# Plan 14: current-source q8 rerank and odd inverse screen

**Source:** RustSHT `1d39a28829b8f9d963a024b69af7a27e2b56596`; Rust 1.99.1 / LLVM 23.1.1, release with strict FP and `-C target-cpu=x86-64-v3`, runtime Pulp V3. **Reference:** DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2 x86-64-v3 production SHT flags. Host: Ryzen 9 5950X, q8 pinned to eight physical cores within one CCD.

## Current whole-transform rank

The q8 cases below were selected from Plan 14's T720 and large-bandlimit targets and remeasured on the same source. T720 cases use three interleaved process pairs and 30 calibrated batches per backend/case. T3072 calls exceed 100 ms, so three interleaved process pairs collected five individual warm calls per backend per process. Inputs and output-reset semantics match the native parity harness. Rust selected Pulp V3; DUCC used its fixed x86-64-v3 build.

| GL workload | Operation | Rust warm ms (p10–p90) | DUCC warm ms (p10–p90) | Rust/DUCC | Excess ms | Relative L2 | Cold first Rust / DUCC ms |
|---|---|---:|---:|---:|---:|---:|---:|
| T720, full `mmax`, `nphi=1458` | Analysis | 2.244 (2.209–3.022) | 2.121 (2.111–2.282) | 1.058× | +0.122 | 4.63e-14 | 3.419 / 7.110 |
| T720, `mmax=180`, `nphi=378` | Analysis | 0.967 (0.927–1.055) | 0.931 (0.914–1.010) | 1.038× | +0.036 | 4.91e-14 | 1.461 / 2.785 |
| T720, `mmax=180`, `nphi=361` | Synthesis | 1.695 (1.635–1.769) | 1.288 (1.254–1.417) | 1.315× | **+0.407** | 5.19e-14 | 1.904 / 2.831 |
| T3072, full `mmax`, `nphi=6174` | Analysis | 105.052 (104.241–106.122) | 115.697 (113.524–117.551) | 0.908× | −10.645 | 1.96e-13 | 132.645 / 115.820 |

The T3072 current-source result overturns the older q8 ranking: this case is faster than DUCC by 10.645 ms, with low recorded runqueue wait (median 0.23%, p90 0.55%). It is not the current bottleneck. T720 full analysis had higher runqueue wait (2.40%, p90 4.53%); reduced analysis 2.03%/2.86%; odd reduced synthesis 1.99%/7.16%. The T720 odd reduced synthesis case remains the largest positive excess in this selected current-source set. Per-process distributions, cold/setup details and fixtures are preserved in the [T720 odd-synthesis results](data/plan14-q8-current-t720-reduced-synthesis-2026-10-21/results.json) and neighboring T720/T3072 data directories.

## Odd inverse row FFT screen

The production odd-length row path in `src/fft.rs` reconstructs a Hermitian length-361 complex row, runs RustFFT's inverse C2C, and writes real samples. A focused q8 screen compared that exact operation structure with the existing `rustsht-fft::RealFftPlan::inverse` real C2R engine on the same deterministic 721-row Hermitian input. Each backend reused plans and eight workspaces; rows were partitioned into eight contiguous worker chunks. Output reset is implicit because every output sample is overwritten. Each process warmed both paths, calibrated batches near 30 ms, then collected 30 interleaved batches. Three independently invoked processes were pinned to the same set of eight physical cores. Plan construction was outside timing. The alternative's normalized output was multiplied by 361 to match RustSHT's unnormalized `inverse_series` convention.

| Process | RustFFT current p10 / median / p90 ms | Existing real C2R p10 / median / p90 ms | C2R/current |
|---:|---:|---:|---:|
| 1 | 0.353 / 0.364 / 0.392 | 0.384 / 0.396 / 0.435 | 1.088× |
| 2 | 0.357 / 0.364 / 0.374 | 0.387 / 0.398 / 0.418 | 1.095× |
| 3 | 0.357 / 0.368 / 0.396 | 0.387 / 0.404 / 0.450 | 1.100× |

The two engines agreed to relative L2 `5.53e-16` and maximum absolute error `8.53e-14`. The existing real engine was 8.8–10.0% slower in all three q8 process comparisons, despite avoiding full Hermitian expansion. It is rejected as a production replacement. This is an FFT-engine A/B only: the synthetic row spectra do not include SHT recurrence, and this does **not** directly compare DUCC pocketfft or establish that FFT arithmetic explains the whole-transform excess. The earlier Rust/DUCC `leg2map` stage value had a different boundary and predates this source rebaseline; do not subtract it from current whole-call timings.

The benchmark driver and raw batches are in [`benches/plan14_q8_odd_inverse`](../benches/plan14_q8_odd_inverse/) and [`reports/data/plan14-q8-odd-inverse-isolation-2026-10-21`](data/plan14-q8-odd-inverse-isolation-2026-10-21/). Reproduce with:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3' cargo build --release --manifest-path benches/plan14_q8_odd_inverse/Cargo.toml
for run in 1 2 3; do
  taskset -c "${RUSTSHT_Q8_CPUSET:?set eight physical CPU IDs}" benches/plan14_q8_odd_inverse/target/release/plan14-q8-odd-inverse
done
```

The process outputs have been retained as `process-1.csv` through `process-3.csv`; `summary.csv` and `batches.csv` are machine-readable extracts. Exact CPU-frequency samples are excluded; batch spread and runqueue-wait fractions remain visible. No production FFT or recurrence change is justified by this screen.

## Next measurement

Keep the current production implementation. The next q8 FFT step is a direct, matched DUCC n=361 row/pocketfft comparison with equivalent packing, normalization and eight-worker scheduling, alongside current Rust `inverse_series`; remeasure the actual SHT stage on the same source before assigning the +0.407 ms excess. If that does not identify an actionable FFT opportunity, return to the remaining small-q1 T127 fused-analysis residual. Do not promote another FFT engine from isolated Rust-only timings.
