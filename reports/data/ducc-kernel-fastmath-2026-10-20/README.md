# Matched scalar-analysis kernel build with DUCC production flags

DUCC source revision: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`
(0.42.0). RustSHT source revision: `d6382614d1726e28a8fc0ae1208d3d15516fac39`.
The run used Rust 1.99.1 (LLVM 23.1.1), Pulp 0.22.3 runtime V3, GCC 15.2.0
or Clang 21.1.8, and an AMD Ryzen 9 5950X with AVX2/FMA. The full measured
rows and process-paired summaries are in `summary.csv`.

## Flag verification

The upstream DUCC CMake `compile_commands.json` for `src/ducc0/sht/sht.cc`
showed `-O3 -ffast-math -march=x86-64-v3` for the v3 SHT object with both
compilers. Its v1 and v4 objects use `-march=x86-64` and `-march=x86-64-v4`.
The standalone kernel makes contraction explicit and was compiled separately
from the benchmark driver:

```text
kernel: -O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3
driver: -O3 -DNDEBUG -fno-fast-math -ffp-contract=fast -march=x86-64-v3
```

The driver contains the fixture reader, `std::isfinite` checks, warmup/result
checks and output comparison. Rust used
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; Rust fast-math was not enabled.
The active runner prints the compiler's full object and link commands, and
uses the same requested flags for GCC and Clang.

The CMake command lines contain `-ffast-math` but do not spell out
`-ffp-contract=fast`. The standalone adds the latter explicitly to make the
requested floating-point mode unambiguous.

## Workload and timing

- T720 GL scalar analysis recurrence, `m=1`, f64, one thread.
- 64, 128 and 256 active latitude lanes. Both executables use four f64 values
  per AVX2 vector; Rust reports the actual Pulp V3 dispatch.
- The same Rust-generated input fixture is read by DUCC. The recurrence
  coefficients agree within a maximum scaled difference of `1.970e-16`.
- Ten independent Rust/DUCC process pairs per compiler and lane count. Backend
  order alternates across pairs; each backend contributes 60 calibrated
  batches per lane count.
- A single allowed physical core is used for both backends within each pair.
  Operation batches were 20–100 ms; no final batch fell outside the window.
- Rust reports the checked route and the grouped ordinary route without its
  post-pair range traversal. DUCC calls `map2alm_kernel`. Both timings include
  coefficient accumulation; Rust's staged result is committed to the output.
  Setup is outside the timed region. State/output reset cost is measured
  separately and removed from the reset-excluded estimate. Reset-inclusive
  results also remain in the local complete run logs.

Median CPU frequency was 3.96 GHz for both compiler runs. Median run-queue
wait was 1.6% for GCC and 1.5% for Clang; the maximum sampled process wait was
70.6% and 87.6%. Paired-process medians remained stable across lane counts.
No timing samples were filtered. Exact process captures are local and are not
part of this artifact.

## Reproduction

Set `DUCC_SRC` and `DUCC_GIT` to a clean checkout of the pinned revision, then
run from the RustSHT repository root:

```sh
DUCC_SRC="$DUCC_SRC" DUCC_GIT="$DUCC_GIT" CXX=g++-15 \
  python3 scripts/benchmark_scalar_analysis_kernel.py

DUCC_SRC="$DUCC_SRC" DUCC_GIT="$DUCC_GIT" CXX=clang++-21 \
  python3 scripts/benchmark_scalar_analysis_kernel.py
```

The runner prints the resolved compiler versions, actual driver/kernel/link
commands, calibration counts, timing distributions, numerical errors, batch
durations, CPU-frequency samples summarized in aggregate form, and
scheduling summaries. The benchmark result discussion is in
[`../../ducc-upgrade-baseline-2026-10-20.md`](../../ducc-upgrade-baseline-2026-10-20.md).
