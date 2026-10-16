# M34 Rader phase measurements and radix-five candidate

## Result

M32 remains the accepted production source at `234ab2d30b8d542457398e2f7be17e7e7b9aa25e`. The M34 phase measurements locate most N=723 and N=4097 radix-241 stage time in the two repeated 240-point complex transforms. A symmetric-pair AVX2 radix-five kernel improved those real rows, but each tested dispatch form produced a repeatable regression in at least one unrelated workload. The kernel was removed; no production FFT source changed.

## Phase method

The `profile-phases` mode in [`rader_real.rs`](../benches/prototypes/rader_real.rs) extends the existing Rader prototype. It times input permutation, the 240-point forward FFT, pointwise convolution, the 240-point inverse FFT, scatter and scaling, real spectrum packing or reconstruction, and the complete radix-241 pass. The 240-point transforms use the public `ComplexFftPlan<f64>` implementation. The prototype models the production Rader group layout and validates every forward and inverse pass against an independently evaluated direct DFT at N=723 and N=4097. Each phase capture used six fresh processes per N/direction cell, seven timed samples per process, 256 repetitions per sample, one transform thread, and one CPU core on an x86-64 CPU. Rust 1.99.0 built the optimized benchmark with portable x86_64 target flags; runtime AVX2 was available. Load-average telemetry was retained only in local diagnostics. The binary hash was `b04b5a83a62f59bf083f80d993952f9c6f90a511a7b04ff21667904bbd295015`. The table reports median microseconds per complete transform equivalent. Brackets contain the 95% bootstrap interval across six process medians. The detailed CSV includes every measured phase and interval.

| N and operation | Complete Rader pass | Inner forward FFT | Pointwise product | Inner inverse FFT |
| --- | ---: | ---: | ---: | ---: |
| 723 R2C | 8.372 [8.361, 8.385] | 2.798 [2.764, 2.805] | 0.348 [0.340, 0.369] | 3.941 [3.920, 3.947] |
| 723 C2R | 8.426 [8.360, 8.487] | 2.789 [2.778, 2.826] | 0.346 [0.327, 0.370] | 3.995 [3.978, 4.027] |
| 4097 R2C | 47.517 [47.462, 47.846] | 15.713 [15.668, 15.800] | 2.017 [1.879, 2.054] | 22.411 [22.248, 22.434] |
| 4097 C2R | 47.565 [47.508, 48.314] | 15.703 [15.562, 15.889] | 1.870 [1.852, 1.974] | 22.459 [22.369, 22.677] |

The forward and inverse 240-point transforms account for about 80% of the measured Rader pass at both lengths. Other phase medians, also in microseconds per transform equivalent, are:

| N and operation | Gather or Hermitian permutation | Scatter and output write | Public pack or reconstruction |
| --- | ---: | ---: | ---: |
| 723 R2C | 0.481 | 0.321 | 0.084 reconstruction |
| 723 C2R | 0.700 | 0.390 | 0.562 pack and safety scan |
| 4097 R2C | 2.693 | 1.822 | 0.614 reconstruction |
| 4097 C2R | 3.921 | 2.207 | 3.170 pack and safety scan |

Odd-length R2C does not need public real-input packing in this prototype. The complete pass time is measured separately from its component loops, so it need not equal the sum of their medians. The phase times come from this prototype model; the public operation latencies come from the production real-transform benchmark. Raw samples, per-sample direct-DFT validation messages, binary metadata, and summary intervals are in [`m34-rader-phase-timing/`](../benches/results/m34-rader-phase-timing/). Reproduce one cell with:

```sh
TMPDIR=<scratch-path> taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo bench --bench rader_real_prototype -- profile-phases 723 r2c 256
```

## Radix-five candidate

The phase results motivated a fixed f64 AVX2 radix-five kernel for the `(radix, l1, ido) = (5, 48, 1)` pass in the 240-point inner transform. It paired symmetric frequencies across adjacent outer groups, used 128-bit loads for the final branch at each group boundary, and used the existing scalar group for odd tails. Candidate tests compared both directions with direct DFTs across even and odd group counts, checked guards and extreme finite values, and confirmed that safe N=240 transforms selected the route. Large inverse inputs selected the existing guarded scalar path. No permanent scratch or plan data was added. Three dispatch forms were screened against the accepted M32 executable. Each full matrix capture used six alternating process pairs, seven warmed samples, the established 34-cell workload, and one CPU core. The raw JSONL captures record source and binary hashes, compiler, affinity, load, plan time, workspace time, scratch bytes, and transform latency.

| Candidate | N=723 R2C / C2R speedup | N=4097 R2C / C2R speedup | Other measured result | Decision |
| --- | ---: | ---: | --- | --- |
| 1: added a radix-five function pointer to `SimdKernels` | 1.280 / 1.256 | 1.171 / 1.176 | C2C inverse N=146 ratio 0.921 [0.917, 0.929]; N=722 ratio 0.937 [0.934, 0.942] | Rejected |
| 2: routed radix five through the common AVX2 pass dispatcher | 1.291 / 1.274 | 1.184 / 1.155 | C2C forward N=722 ratio 0.919 [0.915, 0.923]; C2R N=257 ratio 0.815 [0.806, 0.821] | Rejected |
| 3: reused the low-`ido` radix-four dispatch slot | 1.260 / 1.268 | 1.152 / 1.164 | C2C inverse N=722 ratio 0.985 [0.972, 0.991]; R2C N=1443 ratio 0.986 [0.978, 0.989] | Rejected |

Speedup is `M32 latency / candidate latency`; intervals are 95% bootstrap intervals from paired process ratios. Candidate 3 measured N=723 R2C/C2R at 8.020/8.822 µs against 10.108/11.188 µs for its M32 baseline capture. At N=4097 it measured 72.828/84.911 µs against 84.004/98.817 µs. The N=2049 control measured R2C at 32.622 µs against 32.216 µs (ratio 0.983, interval 0.969–1.013) and C2R at 34.821 µs against 35.074 µs (ratio 1.008, interval 0.996–1.017). Candidate 3 used the existing workspace sizes, including 159,856 bytes at N=2049. The three candidate captures are retained in [`m34-inner240-radix5/`](../benches/results/m34-inner240-radix5/). The first capture is in the directory root; candidates 2 and 3 have separate subdirectories. The baseline reference is the accepted M32 binary with SHA-256 `a1036e527f31637d8f0fd9212d1401c2d568155cd1e8dc0faff785aa05333a8c`. Candidate binaries had hashes `990059bddd390c5b39d47f265b2a4665362317744bfe93673f3ddec7ebbcdcb9`, `b51a0687044f000ac89b68cfadacb9062ff1dc698ad2cd39dc05f22c6951fb45`, and `f0ea34e9e857c7f924f3bb4d4fed91605f8ae6533e9fdfd776e1058f9588e202`.

## Cross-library status

No production change was accepted after M32, so the accepted cross-library matrix in [`m33-final-performance-report.md`](m33-final-performance-report.md) remains the final comparison. It covers RustFFT 6.4.1, native and portable DUCC at `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, FFTW 3.3.10, and Admiral at `ce67bc00`. At N=723 RustSHT-FFT remains faster than native SIMD DUCC but 2.15–2.30 times slower than RustFFT. At N=4097 it remains 12% and 34% slower than native DUCC for R2C and C2R. The accepted M32 timings and direct-DFT/DUCC validation support an optional end-to-end SHT experiment. The rejected radix-five candidates add no new integration evidence, and standalone FFT results do not establish SHT performance.

## Final validation

After removing the candidate code, `cargo fmt --all -- --check`, Clippy with warnings denied, and all-feature debug and release tests passed. Each test profile passed 111 tests. The suites cover f32/f64 transforms, direct DFT comparisons, real Rader N=2049 and N=4097 references, inverse normalization, finite extreme values, shared plans, and separate concurrent workspaces. The phase profiler rebuilt and its smoke runs validated N=723 R2C and N=4097 C2R against direct DFTs. Full logs are in [`m34-validation/`](../benches/results/m34-validation/). The M32 source was compared with native and portable DUCC, FFTW 3.3.10, and Admiral at the revisions listed in [`m33-final-performance-report.md`](m33-final-performance-report.md). The host supports AVX2; a non-AVX2 machine was unavailable for fallback execution.
