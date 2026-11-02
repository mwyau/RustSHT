# B3 f64 current-source control — 2026-11-02

This package re-establishes the warm whole-transform B3 spin-1 analysis baseline at RustSHT <code>85f74d6d91863278f06891205d22d30207e2649d</code> against pinned native DUCC 0.42.0. It narrows the existing <code>scripts/benchmark_ducc_spin1_native.py</code> runner to the exact GL T720 full/reduced geometries and analysis only. No production source was changed.

## Results

Three interleaved process pairs per cell; ratios are paired Rust/DUCC medians. Intervals are 95% Student-t intervals on the three log paired ratios (df=2); these are screens, not promotion-grade 15-pair results.

| GL T720 B3 f64 analysis | Rust ms | DUCC ms | Rust / DUCC (95% interval) | Screen |
|---|---:|---:|---:|---|
| Full q1 | 32.929 | 29.768 | 1.109 [1.094, 1.120] | GAP |
| Full q8 | 4.853 | 4.282 | 1.139 [1.108, 1.164] | GAP |
| Reduced q1 | 12.823 | 11.849 | 1.095 [1.046, 1.151] | GAP |
| Reduced q8 | 1.925 | 1.734 | 1.130 [1.058, 1.185] | GAP |

Each interval is above parity. Rust and DUCC outputs were finite; repeat and workspace-reset checks passed in every process. Maximum relative L2 between Rust and DUCC was <code>4.76e-14</code> full and <code>5.21e-14</code> reduced. The fixtures, times, diagnostics, and per-batch records are in <code>raw/results.json</code> and <code>raw/batch-samples.csv</code>; <code>raw/failures.json</code> is empty.

## Provenance

- Rust source: <code>85f74d6d91863278f06891205d22d30207e2649d</code>.
- B3 source blobs: <code>src/transform/spin/analysis.rs</code> <code>88a30c434410d8e44f4b690321380fa3671110ed</code>; <code>src/legendre/scalar_paired/analysis.rs</code> <code>a8a8f18842af6d7507d1c5174be4aa0331e893c3</code>; <code>src/legendre/spin_fast.rs</code> <code>64f3e94db724b8fdfc0d71c9d96a6ae6d5f33ba3</code>.
- Rust binary SHA-256: <code>121e72638ea3038a0ab7db0d7ea6a08c81d405a37ac7df96986707786ec31ee5</code>.
- DUCC source: <code>42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b</code>; binary SHA-256 <code>b646183e9df8aac030f796e2357b8d92e3b4a194d21be98b460eae043ab63443</code>.
- CPU: AMD Ryzen 9 5950X. Rust 1.99.1 / LLVM 23.1.1, <code>-C target-cpu=x86-64-v3 -Aunexpected_cfgs</code>; DUCC g++ 15.2.0 with SHT <code>-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3</code>. q1 is pinned to CPU 0; q8 to CPUs 0–7.
- Geometry and normalization: GL T720, <code>ntheta=721</code>, <code>phi0=0.23</code>; full M720/<code>nphi=1458</code>, reduced M180/<code>nphi=378</code>; warm reused plan/workspace, native f64 inputs and outputs.
- Exact deterministic fixture hashes are recorded in <code>raw/results.json</code>. The standard runner records affinity, CPU topology, runqueue observations, build settings, executable hashes, per-process ratios, and output checks.

## Source comparison and interpretation

The prior exact-source screen was at <code>4a558596</code>. Between that source and <code>85f74d6d</code>, the B3 hot file <code>scalar_paired/analysis.rs</code> changed as rejected B1 experimental range-check paths were removed. Nevertheless, clean release builds made with the same toolchain have byte-identical <code>.text</code> sections: SHA-256 <code>28b92a70ce8fa47b31a413757ce48a595de8cde438e2f0de652db5860cdf3a76</code>. The active V3 B3 AVX2/FMA leaf is 11,160 bytes at the same address in both binaries and has identical instruction bytes (SHA-256 <code>8da541e7f4207570321326aa904381d02884b37e7ad4f6b2454af003625ac119</code>). The cleanup therefore did not change the emitted release code on this toolchain.

The new screen remains consistent with a B3 gap. Its q1 full ratio differs from the earlier three-pair screen while the entire executable text is identical; the Rust time is stable and DUCC ran about 3% faster in this repeat. Treat this as screen-level timing variation, not a code regression or a source improvement. The accepted 15-pair seed-handoff measurements remain the stronger historical performance result. The previously measured 11.16KB leaf instruction excess, matched recurrence work, and lack of YMM stack traffic remain relevant, but none separates frontend pressure from recurrence dependencies or accumulation throughput. Hardware PMU access remains unavailable (<code>perf_event_paranoid=4</code>).

No kernel candidate is justified by this rebaseline. B1's bounded attempts and the B3 candidate stop rule remain in force.

## Reproduction

Build the Rust example from the exact source with <code>RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo build --release --example ducc_spin1_full_transform_parity --features parallel</code>. Build DUCC at the pinned revision with the flags above. Set <code>RUSTSHT_ROOT</code>, <code>DUCC_SRC</code>, <code>DUCC_BUILD_DIR</code>, and optionally <code>RESULTS_DIR</code>, then run <code>python3 repro/run_exact_source_screen.py</code>. The small wrapper only filters the existing runner to the two exact geometries and the analysis operation.
