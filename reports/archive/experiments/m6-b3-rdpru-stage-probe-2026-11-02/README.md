# B3 RDPRU stage probe — exploratory only

**Classification:** test-only, instrumented diagnostic. It is not a production change, a DUCC comparison, a cycle-accurate profile, or an optimization result. It does not justify a kernel change.

## Provenance and reproduction

- Rust source base: `41caa81ab01f32fd28630d4a1e1ebe748454e6f8`.
- Test-only patch: [RDPRU instrumentation patch](patches/rdpru-profile.patch.gz); example runner: [m6_b3_rdpru_probe.rs](examples/m6_b3_rdpru_probe.rs).
- Geometry: GL T720, full M720, spin 1, 721 latitude rows, 1458 longitudes, phi0 0.23; q1 pinned to logical CPU 0. The Rust f64 input maps were widened from existing f32 fixtures.
- Fixture SHA-256: map0 `2f5c74d4144bb78cd6b4b7507ab7fedf1d5605b848c9f88a4bbbf4d8099bc032`; map1 `14bfa8b7db3131a688e18b8477dca389e189aa66966d0360b9a1ac786895fb9a`.
- Toolchain: rustc 1.99.1, LLVM 23.1.1; target flags included `-C target-cpu=x86-64-v3`. The instrumentation cfg was `m6_rdpru_probe`.
- To reproduce the runner, check out the source revision above in a disposable worktree, apply the saved patch, then build and run the example with the two matching map files and thread count 1. The original probe binaries are intentionally not archived; their hashes are in `raw/summary.json`.

The host was an AMD Ryzen 9 5950X. Linux perf_event_open PMU access was blocked by perf_event_paranoid=4. The probe used RDPRU to sample a limited APERF-like per-logical-core counter; this is not a general retired-instruction, branch, cache, or PMU event profile and must not be described as those measurements.

## Observations and limits

The outer-only instrumented profile covers 200 warmed calls. Median total was 152.152 million counter units; FFT was 29.495 million (19.39%) and Legendre was 121.469 million (79.83%). Preparation and cleanup were each below 0.1%. A nine-pair instrumented/plain control was unresolved: geometric ratio 0.997 [0.986, 1.009].

The finer block profile covers 100 warmed calls. Its fused blocks measured 87.570 million units (56.62% of median total), block preparation 17.625 million (11.39%), order setup 6.131 million (3.96%), postprocessing 10.262 million (6.63%), and FFT 29.576 million (19.12%). Its nine-pair instrumented/plain control measured 1.030 [1.024, 1.035], indicating about 3% probe overhead. These are instrumented counter shares, not elapsed-time or DUCC shares.

A deeper leaf probe produced one-process samples only and had no probe-overhead control. Its reported 75.494 million leaf-SIMD counter units and 2,980 normal versus 599 scaled seeds are preserved in the raw summary as preliminary observations; they are not used to rank bottlenecks or support an optimization. The disassembly captures are included for future inspection, not as evidence of cycle cost.

The compact JSON, stage logs, disassembly, patch, and this note are covered by [SHA256SUMS](SHA256SUMS). Per-process instrumentation stderr captures are retained locally as untracked files per repository review; their parsed numeric observations are in `raw/summary.json`. No production source or benchmark binary is included. No numerical acceptance or performance-parity conclusion follows from this probe.
