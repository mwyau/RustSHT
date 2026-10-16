# M17 cross-library measurements

Date: 2026-10-15. RustSHT-FFT source: M17 commit `f3f03a3`.

## Method

The first matrix uses the crate's exact complex and real SHT longitude lengths
and row counts: 11 complex sizes and six real sizes, for 34 direction/size
cells per reference. The focused matrix measures standalone C2C operations at
length 682, the FFT length used by the real Rader route, with 32 rows in both
directions. Each process reports the median of seven warmed samples per cell. Six
alternating process pairs ran on
an x86-64 CPU, pinned to logical one CPU core. RustSHT used Rust 1.99.0,
the Cargo bench profile, a portable target and runtime AVX2 dispatch. Each
implementation used one transform thread. Ratios are `reference latency / RustSHT-FFT M17 latency`; values above one
favor M17. Intervals are percentile-bootstrap 95% intervals over paired
process ratios. The measurements cover public 1D FFT operations with the
benchmark adapters and their packing/scaling work. They are not end-to-end
SHT timings. The references were RustFFT 6.4.1 from the same Rust benchmark build; DUCC
revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` in a portable scalar
planner build and a native `-march=native` build with DUCC planner vectorizing
enabled; system FFTW 3.3.10 with `FFTW_ESTIMATE` and `FFTW_PRESERVE_INPUT`;
and Admiral revision `ce67bc00ff9e6de96bf5b75f7a789581105ddb58` in its
native Release estimate configuration. The system FFTW package did not expose
its compiler flags. Exact binary hashes and invocations are in the configs.

## Exact SHT target matrix

Across the 34 target cells, the 95% paired intervals favor M17 in 3 cells
against RustFFT, 12 against portable DUCC, six against native DUCC, five
against FFTW and one against Admiral. The intervals favor the reference in
the remaining cells. This matrix shows that the N=2049 optimization does not
close the broader performance gap across the crate's SHT target set. At the 1,024-row N=2049 real target, M17 improves both directions against
RustFFT, both DUCC builds and FFTW. Admiral remains faster for these two
operations.

| Reference | R2C M17/reference (µs) | R2C ratio (95% interval) | C2R M17/reference (µs) | C2R ratio (95% interval) |
|---|---:|---:|---:|---:|
| RustFFT 6.4.1 | 32.514 / 37.027 | 1.136 (1.129–1.160) | 35.784 / 36.760 | 1.029 (1.014–1.032) |
| DUCC portable | 32.299 / 49.753 | 1.544 (1.525–1.601) | 35.621 / 50.735 | 1.425 (1.406–1.452) |
| DUCC native + vectorize | 32.351 / 38.251 | 1.181 (1.147–1.220) | 35.575 / 38.255 | 1.082 (1.053–1.107) |
| FFTW 3.3.10 | 32.314 / 42.722 | 1.326 (1.313–1.336) | 35.416 / 42.807 | 1.201 (1.178–1.228) |
| Admiral | 32.445 / 28.096 | 0.871 (0.854–0.878) | 35.910 / 28.522 | 0.797 (0.765–0.800) |

## C2C length 682

The 32-row C2C result compares the standalone FFT length used by the real
Rader route; it does not measure the complete convolution. This length is not
an SHT longitude length. M17 beats RustFFT, portable DUCC and FFTW in both
directions, beats native DUCC forward, and trails native DUCC inverse and
Admiral in both directions.

| Reference | Forward ratio (95% interval) | Inverse ratio (95% interval) |
|---|---:|---:|
| RustFFT 6.4.1 | 1.391 (1.343–1.409) | 1.109 (1.099–1.126) |
| DUCC portable | 1.556 (1.535–1.588) | 1.261 (1.254–1.265) |
| DUCC native + vectorize | 1.162 (1.154–1.169) | 0.822 (0.786–0.829) |
| FFTW 3.3.10 | 1.624 (1.610–1.635) | 1.302 (1.296–1.310) |
| Admiral | 0.823 (0.812–0.916) | 0.637 (0.623–0.701) |

## Data and limits

All 120 process records exited successfully: six alternating pairs for each
of five references on the exact target matrix, plus six pairs for each
reference on C2C length 682. Load-average telemetry was retained only in local diagnostics. Pairing and CPU affinity reduce measurement variation but do
not remove unrelated host activity. Treat the results as specific to this
CPU, library builds and benchmark adapters. Workspace-byte values use
library-specific accounting and are not compared here. Raw JSONL, six-pair configurations and generated CSV summaries are in
[`the M17 cross-library result directory`](../benches/results/m17-cross-library-sht-targets/).
