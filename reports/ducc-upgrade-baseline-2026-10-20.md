# RustSHT/DUCC post-upgrade baseline

This report records the first representative full-transform and ordinary
Legendre-kernel comparisons after the Rust and DUCC updates. It does not mix
these results with the earlier Rust 1.99.0 / DUCC `80388904` campaign.

## Builds and method

- RustSHT source and benchmark harness: `b8c1b85a`, release mode, Rust
  1.99.1 (`58cf8f9c3`, LLVM 23.1.1), Pulp 0.22.3 runtime V3 (AVX2/FMA),
  `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`.
- DUCC: immutable source revision
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0). The full-transform
  native workers used fixed x86-64-v3 builds with GCC 15.2.0 and Clang 21.1.8;
  the separately inspected upstream CMake multiarch extensions selected v3 at
  runtime. DUCC translation units used
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; only the benchmark
  driver used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3` so its
  finite-value checks remain meaningful. Both libraries ran four f64 lanes.
- The full-transform comparison covered T255, T300 and T720 GL grids, odd
  minimum and even smooth longitude counts, `m=1` where selected, quarter and
  full `mmax`, analysis and synthesis, q1/q8. Each compiler run collected
  three interleaved Rust/DUCC process pairs and ten calibrated batches per
  process for short operations. Setup, first-call, warmed-call, fixture and
  output-comparison costs are separate in the machine-readable rows.
- A same-source Rust 1.99.0 control covered all T720 cases with the GCC DUCC
  build. It used the same v3 target flags and benchmark inputs. It is a
  compiler control, not part of the Rust 1.99.1 baseline.
- Machine-readable results retain batch and paired process quantiles, invocation
  counts and numerical checks. Individual batch records and per-process RSS
  samples remain local; aggregate peak RSS is retained.
  The datasets are in
  [`reports/data/ducc-upgrade-baseline-2026-10-20/`](data/ducc-upgrade-baseline-2026-10-20/).

Reproduce the full-transform runs from the repository root with a clean DUCC
checkout at the recorded SHA:

```sh
DUCC_SRC="$DUCC_SRC" CXX=g++ python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 255 --lmax 300 --lmax 720 --results-dir "$RESULT_ROOT/gcc-v3" --no-report

DUCC_SRC="$DUCC_SRC" CXX=clang++-21 python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 255 --lmax 300 --lmax 720 --results-dir "$RESULT_ROOT/clang-v3" --no-report

RUSTUP_TOOLCHAIN=1.99.0 DUCC_SRC="$DUCC_SRC" CXX=g++ \
  python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --lmax 720 --results-dir "$RESULT_ROOT/rust-1.99.0-gcc-v3" --no-report
```

The ordinary-kernel control used
`DUCC_SRC="$DUCC_SRC" DUCC_GIT="$DUCC_GIT" CXX=g++` (then
`CXX=clang++-21`) with `python3 scripts/benchmark_scalar_analysis_kernel.py`.
Its compile flags and limitations are listed below.

## Full-transform results

Cells show Rust/DUCC median milliseconds, the Rust/DUCC median ratio, and the
paired process-ratio p10–p90 interval. Even-smooth longitude cases are shown;
the JSON/CSV files contain odd-minimum cases and all other measurements.

| DUCC compiler | T720 `mmax` | A q1 | A q8 | S q1 | S q8 |
|---|---:|---|---|---|---|
| GCC | 720 | 27.689/27.239, 1.016× (0.979–1.091) | 14.606/14.189, 1.029× (0.556–1.164) | 34.404/27.083, 1.270× (1.238–1.287) | 23.828/15.236, 1.564× (1.032–1.761) |
| GCC | 180 | 10.782/11.685, 0.923× (0.904–0.944) | 5.857/7.540, 0.777× (0.517–1.219) | 14.845/11.646, 1.275× (1.244–1.321) | 10.128/10.556, 0.960× (0.674–1.434) |
| Clang | 720 | 27.926/24.164, 1.156× (1.133–1.181) | 16.452/14.966, 1.099× (0.834–1.755) | 34.346/28.655, 1.199× (1.190–1.219) | 25.795/20.631, 1.250× (0.655–1.511) |
| Clang | 180 | 10.674/10.054, 1.062× (1.028–1.082) | 8.346/8.953, 0.932× (0.528–1.545) | 14.859/12.031, 1.235× (1.224–1.246) | 11.358/12.361, 0.919× (0.695–2.093) |

At T300, even-smooth q1 full-order analysis/synthesis ratios were 1.158×/1.225×
with GCC and 1.216×/1.094× with Clang. For reduced `mmax=75`, they were
1.112×/1.303× and 1.252×/1.247×, respectively. Several reduced-order q1
intervals are wider than the full-order intervals. T255 and odd-minimum results
are available in the result files.

Numerical comparisons completed for 192 output rows and 32 additional
validation suites per DUCC compiler. Maximum relative L2 error was
`1.841e-13` for GCC and `3.575e-13` for Clang. The suites include selected
basis modes, roundtrips, edge values, finite checks and adjoint identities.
The Rust 1.99.0 T720 control had maximum relative L2 error `1.841e-13`; at q1,
its full/reduced T720 point-median ratios versus Rust 1.99.1 were respectively
0.992/0.973 for analysis and 1.015/1.025 for synthesis. This shows no clear q1
compiler-version shift in this screen. The q8 compiler-control differences
were too variable to interpret.

## Host limits and kernel screen

Scheduling interference was substantial in the full-transform runs. The
summarized run-queue wait fraction was 33.1% median / 78.6% p90 for GCC and
40.0% / 79.1% for Clang. CPU-frequency telemetry was available, but its
numeric readings remain local under the artifact policy. The q8 paired
intervals are correspondingly broad and include both sides of parity. No timed
observations were filtered. Treat q1 medians as a provisional baseline and do
not infer a stable q8 performance ranking from this run.

### DUCC floating-point flags

The pinned upstream multiarch CMake builds were checked using their actual
`compile_commands.json`, not just CMake settings. For `src/ducc0/sht/sht.cc`,
both GCC 15.2.0 and Clang 21.1.8 v3 commands include
`-O3 -ffast-math -march=x86-64-v3`, `DUCC0_MULTIARCH`, and the v3 namespace.
The corresponding v1/v4 objects use `-march=x86-64` and `-march=x86-64-v4`.
The upstream commands do not spell out `-ffp-contract=fast`; the standalone
comparison below adds that flag explicitly as requested. The actual command
summaries are in
[`reports/data/ducc-multiarch-2026-10-20/README.md`](data/ducc-multiarch-2026-10-20/README.md).

The corrected standalone build compiles the DUCC kernel and driver as separate
objects. The numerical kernel uses
`-O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver
uses `-O3 -DNDEBUG -fno-fast-math -ffp-contract=fast -march=x86-64-v3` so
`std::isfinite` and output validation retain strict semantics. GCC 15 and
Clang 21 receive the same flags. Rust remains Rust 1.99.1 with
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; no Rust fast-math options are
enabled. The runner prints each actual compile and link command.

### Matched ordinary-kernel results

The corrected screen uses T720 GL, `m=1`, four f64 values per AVX2 vector,
64/128/256 active latitude lanes, and the production-dispatched Rust fused
analysis kernel. Each compiler run has ten interleaved Rust/DUCC process pairs
and 60 calibrated batches per backend/lane. The Rust warm column disables the
ordinary recurrence group's post-pair range traversal; the checked column
retains it. Both Rust columns include coefficient commit, matching DUCC's
direct accumulation boundary. Setup and input preparation are outside timing;
the output and recurrence-state reset cost is measured separately and
subtracted from operation batches. Results are paired by process invocation.

| Compiler | lanes | Rust warm µs (p10–p90) | DUCC µs (p10–p90) | warm/DUCC paired ratio (p10–p90) | excess µs | checked/DUCC | relative L2 |
|---|---:|---:|---:|---:|---:|---:|---:|
| GCC 15.2 | 64 | 8.075 (7.931–8.155) | 8.134 (7.897–8.326) | 0.990 (0.982–0.999) | -0.081 | 1.374 | 6.670e-14 |
| GCC 15.2 | 128 | 14.405 (14.283–14.531) | 15.888 (14.825–16.680) | 0.906 (0.899–0.950) | -1.491 | 1.269 | 1.628e-13 |
| GCC 15.2 | 256 | 26.746 (26.370–26.985) | 30.955 (29.694–31.568) | 0.868 (0.857–0.883) | -4.075 | 1.238 | 5.928e-14 |
| Clang 21.1.8 | 64 | 8.008 (7.860–8.112) | 6.624 (6.489–7.050) | 1.211 (1.201–1.217) | 1.393 | 1.679 | 6.670e-14 |
| Clang 21.1.8 | 128 | 14.331 (14.018–14.529) | 12.478 (12.304–12.865) | 1.145 (1.141–1.160) | 1.818 | 1.615 | 1.628e-13 |
| Clang 21.1.8 | 256 | 26.651 (26.384–26.917) | 23.953 (23.202–24.321) | 1.112 (1.105–1.122) | 2.685 | 1.590 | 5.928e-14 |

The coefficient transform maximum scaled difference was `1.970e-16`; maximum
absolute output error was `2.559e-11`. Every final batch was 20–100 ms (GCC
37.91–80.22 ms; Clang 35.73–88.44 ms), with no batches outside the window.
Median process run-queue wait was 1.6% for GCC and 1.5% for Clang; maxima
reached 70.6% and 87.6%, respectively. Median sampled CPU frequency was
3.96 GHz for both runs. Paired-process ratio spreads were at most 8.4% for
GCC and 3.8% for Clang; keep the maximum scheduling-delay cases in mind.

The matched production flags change the attribution picture: Rust's warm
256-lane route is 13% faster than GCC DUCC but 11% slower than Clang DUCC.
The GCC and Clang DUCC medians differ by about 29% at that lane count under
the same flags, while Rust timings are nearly unchanged. This is a compiler
code-generation difference to investigate in the exact executed loops, not
evidence that Rust or either compiler is generally faster. The earlier
single-translation-unit kernel screen omitted fast-math, had high Rust queue
delay, and is retained only as historical data in
`ordinary-kernel-summary.csv`; it is not used for the current conclusion.
Machine-readable paired distributions and commands are in
[`reports/data/ducc-kernel-fastmath-2026-10-20/`](data/ducc-kernel-fastmath-2026-10-20/).

## Next step

Keep this baseline separate from historical measurements. Next, inspect the
actual GCC and Clang v3 kernel loops generated by these corrected builds and
compare their instruction scheduling, spills, vector reductions and memory
traffic with Rust's executed specialization. Isolate recurrence,
coefficient-accumulation and horizontal-reduction costs before changing
production source. The existing v4 SHT disassembly remains static inspection
only; no AVX-512 code ran on this AVX2-only host. See
[`reports/data/ducc-multiarch-2026-10-20/README.md`](data/ducc-multiarch-2026-10-20/README.md)
for the v3/v4 build and symbol evidence.
