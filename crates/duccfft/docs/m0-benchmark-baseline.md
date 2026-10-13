# M0 benchmark baseline

Date: 2026-10-12. This baseline measures DUCC and RustFFT before native
DUCCFFT transforms exist. It is reference data, not a DUCCFFT performance
result.

## Build and measurement conditions

- DUCC source revision: `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.
- Host: AMD Ryzen 9 5950X, 16 cores / 32 logical CPUs, Linux x86_64.
- C++ compiler: g++ 15.2.0. Tuned DUCC build flags:
  `-std=c++20 -O3 -DNDEBUG -march=native -pthread`.
- Portable DUCC build flags: `-std=c++20 -O3 -DNDEBUG -pthread`.
- Rust compiler: rustc 1.99.0. RustFFT used the Cargo bench release profile
  (`opt-level=3`) without `target-cpu=native`.
- DUCC direct plans use one thread per transform. The scalar planner leaves
  `vectorize=false`; the tuned vectorized planner sets `vectorize=true`. The
  compiler's `-march=native` setting also enables DUCC's compile-time ISA
  specializations. RustFFT remains on the portable compiler target.
- Processes were pinned to logical CPU 30. Standard matrix batches contain one
  or 32 rows, with four rows for lengths above 10,000. The SHT workload runs
  use the corresponding GL latitude counts: 128, 512, 1,024, and 2,048 rows.
- Each transform measurement has one warm-up operation and seven timed samples;
  the median elapsed time is divided by row count. Plan construction and
  scratch allocation use 20 repetitions. Inputs use the deterministic sine and
  cosine formulas in the two benchmark programs.
- C2C timing includes the transform and inverse scaling. R2C/C2R whole-call
  timing includes the adapter to or from DUCC's packed real layout or the
  RustFFT adapter's half-complex representation. Separate execution and
  packing measurements are included in the CSV files.

## RustSHT longitude workload

These are single process runs at the actual row counts. Values are microseconds
per row for whole-call R2C/C2R timing. DUCC and RustFFT each reuse a plan and
scratch across rows.

| Workload | nphi × rows | Portable DUCC R2C / C2R | Tuned DUCC scalar R2C / C2R | Tuned DUCC vectorized R2C / C2R | RustFFT R2C / C2R |
| --- | ---: | ---: | ---: | ---: | ---: |
| T127 | 257 × 128 | 4.820 / 4.893 | 2.873 / 2.896 | 2.832 / 2.860 | 1.237 / 1.273 |
| T511 | 1,025 × 512 | 4.854 / 4.593 | 3.676 / 3.371 | 3.688 / 3.334 | 6.142 / 6.234 |
| T1023 | 2,049 × 1,024 | 49.922 / 51.459 | 39.350 / 39.834 | 39.005 / 39.079 | 37.240 / 37.671 |
| T2047 | 4,097 × 2,048 | 106.889 / 107.112 | 77.060 / 76.262 | 77.278 / 75.926 | 62.740 / 62.503 |

The observed ordering changes with transform length. In these runs, RustFFT has
lower whole-call latency at nphi 257, 2,049, and 4,097. Tuned DUCC has lower
latency at nphi 1,025. The vectorized planner does not improve every length.
These measurements do not establish an overall performance winner.

## Standard length matrix and numerical comparison

The matrix includes 30 lengths, four directions (C2C forward, C2C inverse,
R2C, and C2R), and one-row and 32-row cases. For lengths above 10,000, the
32-row case is capped at four rows. It includes the required SHT lengths and
prime and threshold probes such as 107, 109, 113, 1,013, 10,000, and 10,001.

The direct comparison checks 151,606 scalar output components against pinned
DUCC for each planner mode. Both modes passed with `atol=2e-11` and
`rtol=2e-11`. Maximum absolute error was `2.956e-12`. The maximum scaled error
`error / max(1, abs(DUCC), abs(RustFFT))` was `1.085e-12`. The comparison
rejects nonfinite values from either implementation.

Raw results are in [`benches/results/`](../benches/results/):

- `m0-native-scalar.csv` and `m0-native-vectorized.csv` contain two
  process-level standard matrix runs per DUCC planner mode.
- `m0-portable-scalar.csv` contains the portable compiler-target run.
- `m0-sht-scalar.csv`, `m0-sht-vectorized.csv`, and
  `m0-sht-portable-scalar.csv` contain the actual-row-count runs.

## Limits

These measurements are provisional. The host runs other RustSHT work, and a
compute-heavy Legendre job was active during part of the first standard matrix
run. Later process checks showed no comparable CPU-bound job, and all FFT
processes used CPU 30, but the actual-row-count runs have only one process-level
sample each. Repeat those workload runs on an idle host before using small
differences as performance evidence. No native DUCCFFT performance has been
measured yet.
