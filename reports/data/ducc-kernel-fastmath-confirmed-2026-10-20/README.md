# DUCC SHT kernel floating-point flags, rechecked at current source

RustSHT source: `6682095c2e24516c5f26a14fb2bbd1f0f7156938`.
DUCC source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
The pinned upstream checkout was configured in Release multiarch mode. Its
actual GCC 15.2 and Clang 21.1.8 `compile_commands.json` entries for
`src/ducc0/sht/sht.cc` define `DUCC0_MULTIARCH`, use the `ducc0_v3` namespace,
and contain `-O3 -ffast-math -march=x86-64-v3`. Upstream CMake does not print
`-ffp-contract=fast`; the standalone kernel explicitly adds it as required.
The v1 and v4 entries use their respective x86-64 and x86-64-v4 targets.

The standalone build compiled two separate translation units with both
compilers. The recorded command lines were checked after compilation:

```text
GCC kernel: g++-15 -std=c++17 -O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3 -c <DUCC kernel TU>
GCC driver: g++-15 -std=c++17 -O3 -DNDEBUG -fno-fast-math -ffp-contract=fast -march=x86-64-v3 -c <benchmark driver TU>
Clang kernel: clang++-21 -std=c++17 -O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3 -c <DUCC kernel TU>
Clang driver: clang++-21 -std=c++17 -O3 -DNDEBUG -fno-fast-math -ffp-contract=fast -march=x86-64-v3 -c <benchmark driver TU>
```

The driver owns fixture input, `std::isfinite` checks, timing orchestration,
and output validation. Fast-math is explicitly disabled for it. Rust 1.99.1 uses
`-C target-cpu=x86-64-v3` with Rust fast-math disabled; the C++ fast-math
options were not translated to Rust LLVM flags.

## Matched kernel run

The workload is T720 GL scalar f64 analysis at `m=1`, q1, for 64, 128, and
256 active latitude lanes. Rust used Pulp V3; DUCC used the compiled v3
kernel. Each compiler/lane case has ten interleaved independent process pairs
and 60 calibrated batches per backend. Batch timing targets 20–100 ms. The
reported kernel-plus-commit boundary includes Rust coefficient commit and
DUCC direct accumulation; state/output reset is measured separately and
excluded. No timing samples were removed.

| Compiler | lanes | Rust warm µs (p10–p90) | DUCC µs (p10–p90) | paired Rust/DUCC ratio (p10–p90) | paired excess µs (p10–p90) | relative L2 | max abs |
|---|---:|---:|---:|---:|---:|---:|---:|
| GCC 15.2 | 64 | 7.939 (7.766–8.055) | 8.028 (7.648–8.259) | 0.991 (0.975–1.024) | -0.070 (-0.204–0.183) | 6.670e-14 | 5.009e-12 |
| GCC 15.2 | 128 | 14.155 (13.961–14.370) | 15.648 (14.568–15.883) | 0.906 (0.894–0.961) | -1.468 (-1.676–-0.570) | 1.628e-13 | 1.896e-11 |
| GCC 15.2 | 256 | 26.356 (25.723–26.727) | 30.756 (30.102–31.733) | 0.860 (0.846–0.866) | -4.297 (-4.851–-4.058) | 5.928e-14 | 2.559e-11 |
| Clang 21.1.8 | 64 | 7.983 (7.822–8.081) | 6.629 (6.318–6.809) | 1.204 (1.193–1.226) | 1.358 (1.284–1.474) | 6.670e-14 | 5.009e-12 |
| Clang 21.1.8 | 128 | 14.175 (14.060–14.348) | 12.375 (12.175–12.748) | 1.147 (1.137–1.160) | 1.827 (1.707–1.972) | 1.628e-13 | 1.896e-11 |
| Clang 21.1.8 | 256 | 26.530 (26.140–26.808) | 24.142 (23.509–24.636) | 1.098 (1.088–1.115) | 2.371 (2.153–2.770) | 5.928e-14 | 2.559e-11 |

All coefficient maximum scaled differences were `1.970e-16`. GCC's measured
operation batches ranged from 45.27 to 85.07 ms; Clang's ranged from 39.88 to
84.98 ms. All 1,440 operation batches per compiler remained within the target
window. The process-median stability gate passed, but host scheduling
interference was substantial: median run-queue wait was 0.8% for GCC and
12.5% for Clang, with maxima of 84.2% and 99.5%. Median sampled frequencies
were 3.968 and 3.943 GHz. The load average was about 30 at the start of the
GCC run. These conditions can bias paired wall-clock timings; the opposite
GCC/Clang ratio directions are not used to attribute a compiler or algorithm
advantage. The flag check is conclusive; this timing sample is not a clean
compiler comparison.

## Reproduction

Use a clean checkout of the pinned DUCC revision and the repository's
`scripts/benchmark_scalar_analysis_kernel.py` from RustSHT source
`6682095c`. The harness validates the required kernel/driver flag split and
prints the exact object and link commands before running:

```sh
DUCC_SRC="$DUCC_SRC" DUCC_GIT="$DUCC_GIT" CXX=g++-15 \
  python3 scripts/benchmark_scalar_analysis_kernel.py
DUCC_SRC="$DUCC_SRC" DUCC_GIT="$DUCC_GIT" CXX=clang++-21 \
  python3 scripts/benchmark_scalar_analysis_kernel.py
```

Aggregate samples and host diagnostics are in `summary.csv`. Full process
captures and transient host telemetry remain local and untracked.
