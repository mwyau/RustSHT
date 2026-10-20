# DUCC fast-math flag confirmation rerun

This is a new run at RustSHT source `be608eb2` (working tree clean before the run), Rust 1.99.1 / LLVM 23.1.1, Pulp 0.22.3 runtime V3, and DUCC 0.42.0 at immutable commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. It is kept separate from `ducc-kernel-fastmath-2026-10-20`, whose Rust source revision was `d6382614`; the Rust kernel benchmark and attribution wrapper changed between those revisions, so their absolute times are not pooled.

## Flag verification

The checked-out DUCC source reports commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` and version 0.42.0. Its upstream `DUCC0_OPTIMIZATION=multiarch` Release `compile_commands.json` has three actual `src/ducc0/sht/sht.cc` compile entries per compiler. The GCC 15.2 and Clang 21.1.8 v3 entries both include `DUCC0_MULTIARCH`, the `ducc0_v3` namespace, `-O3 -ffast-math -march=x86-64-v3`; the v1/v4 entries use the corresponding x86-64/x86-64-v4 targets. Upstream CMake does not spell out `-ffp-contract=fast`; the standalone kernel explicitly does.

The standalone log records the commands actually invoked (same flags with both compilers):

```text
kernel object: -std=c++17 -O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3
strict driver: -std=c++17 -O3 -DNDEBUG -fno-fast-math -ffp-contract=fast -march=x86-64-v3
```

Driver and numerical kernel are distinct translation units. The finite-value checks and output validation stay in the `-fno-fast-math` driver. Rust used `-C target-cpu=x86-64-v3` with Rust fast-math disabled; this is an ISA/CPU target setting, not C++ fast-math equivalence. No Rust LLVM fast-math flags were used.

Compiler command summaries captured from the run:

- GCC: `DUCC_KERNEL_COMPILE g++-15 -std=c++17 -O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3` for the kernel; strict flags as above for the driver.
- Clang: `DUCC_KERNEL_COMPILE clang++-21 -std=c++17 -O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3` for the kernel; strict flags as above for the driver.

## Timing and validation

The harness used identical Rust-generated fixtures at T720 GL, scalar f64 analysis, `m=1`, q1, for 64/128/256 active latitude lanes. Both paths executed four f64 lanes per AVX2 vector. Ten independent paired processes per compiler and lane count interleaved Rust/DUCC order; each backend contributed 60 calibrated batches per lane count. Timings are per-operation medians over 60 batches, with paired process-ratio/excess distributions in `summary.csv`. The primary boundary includes the ordinary recurrence and coefficient accumulation, with Rust coefficient commit included to match DUCC direct accumulation; reset is separately measured. No samples were removed.

| Compiler | Lanes | Rust checked µs | Rust warm/unchecked µs | DUCC µs | Warm/DUCC paired ratio | Warm excess µs | Output relative L2 |
|---|---:|---:|---:|---:|---:|---:|---:|
| GCC 15.2 | 64 | 7.152 | 4.900 | 4.187 | 1.172 (1.166–1.180) | 0.722 | 6.670e-14 |
| GCC 15.2 | 128 | 12.841 | 8.970 | 8.100 | 1.107 (1.101–1.115) | 0.866 | 1.628e-13 |
| GCC 15.2 | 256 | 24.704 | 17.338 | 15.958 | 1.089 (1.081–1.092) | 1.422 | 5.928e-14 |
| Clang 21.1.8 | 64 | 7.074 | 4.867 | 4.190 | 1.161 (1.158–1.167) | 0.676 | 6.670e-14 |
| Clang 21.1.8 | 128 | 12.811 | 8.958 | 8.181 | 1.094 (1.089–1.105) | 0.766 | 1.628e-13 |
| Clang 21.1.8 | 256 | 24.543 | 17.265 | 16.013 | 1.078 (1.073–1.081) | 1.250 | 5.928e-14 |

The C++ coefficient maximum scaled difference was `1.970e-16` for all lane
counts. Rust/DUCC output relative L2 error was `6.670e-14`, `1.628e-13`, and
`5.928e-14` at 64/128/256 lanes; maximum absolute errors were `5.009e-12`,
`1.896e-11`, and `2.559e-11`. `components.csv` records recurrence-only and
recurrence-plus-accumulation timings; their difference estimates accumulation
plus reduction under the same wrapper.

## Host and batch diagnostics

- CPU: AMD Ryzen 9 5950X, 16 physical/32 logical cores, 96 GiB; AVX2/FMA present, AVX-512 absent. Single-thread processes were pinned to one allowed logical CPU on one physical core; exact CPU identifiers are intentionally omitted.
- GCC run: 2,437 frequency samples, median 4.592 GHz (3.593–4.716 GHz); median run-queue wait 0.008%, maximum 0.075%. Operation/reset batches ranged from 32.82 to 105.07 ms; 3/1,800 were outside 20–100 ms. The timing-stability aggregate was `UNSTABLE`; the three over-100-ms batches were retained.
- Clang run: 2,452 frequency samples, median 4.602 GHz (3.604–4.735 GHz); median run-queue wait 0.010%, maximum 0.074%. Operation/reset batches ranged from 35.88 to 82.90 ms; 0/1,800 were outside 20–100 ms. Timing stability was `STABLE`.

The two compiler runs produced similar DUCC medians in this rerun (within 1%
at each size), unlike the earlier `d6382614` source/harness comparison. This is
evidence only for these exact current-source benchmark boundaries. The rerun
confirms that the standalone DUCC kernel uses the upstream DUCC fast-math mode;
Rust remains strict and is not compiled with C++ fast-math options. This does
not establish that C++/Rust whole-transform or q8 workloads are equivalent or
at parity.
Current-source warmed Rust/DUCC ratios are 1.16–1.17× at 64 lanes,
1.09–1.11× at 128 lanes, and 1.06–1.09× at 256 lanes. Next investigate the
measured accumulation/reduction part with a non-spilling attribution method,
then profile larger-order/q8 full transforms as specified in Plan 11.

## Reproduction

From the repository root, with a clean checkout of DUCC at the pinned SHA:

```sh
DUCC_SRC="$DUCC_SRC" DUCC_GIT="$DUCC_GIT" CXX=g++-15 python3 scripts/benchmark_scalar_analysis_kernel.py
DUCC_SRC="$DUCC_SRC" DUCC_GIT="$DUCC_GIT" CXX=clang++-21 python3 scripts/benchmark_scalar_analysis_kernel.py
```

The runner prints the actual compile/link commands and checks numerical output after timing. Raw process logs and transient host telemetry were kept outside the repository; only aggregate results and conclusions are retained here.
