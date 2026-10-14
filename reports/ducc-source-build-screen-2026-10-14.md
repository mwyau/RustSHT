# Installed and source-built DUCC0 comparison

## Finding

An optimized DUCC0 Python extension built from source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf` did not run consistently faster
than the installed `ducc0==0.41.0` package on matched f64 GL scalar and
spin-1 STANDARD workloads. The repeatable q1/q8 process medians were generally
within about 1%; the largest absolute median difference was 3.8% and the
paired process ranges usually crossed zero. A preliminary T720 q8 spin-1
synthesis screen suggested a 5–7% source-build gain. Repeating with five
raw timed samples per process removed that result: the paired median was
`+0.1%` (source build relative to installed) with a `-4.8%` to `+4.7%` range.
Compiler flags alone therefore do not explain RustSHT's remaining multi-fold
gap in these operations.

The source revision's `pyproject.toml` identifies the build as DUCC0 0.41.1,
while the installed package reports 0.41.0. This comparison is between the
requested source revision and the installed 0.41.0 package, not two builds
with identical release metadata. Both shared-library `.comment` sections
identify GCC 15.2.0; the installed package's full build flags are unavailable.

## Build and workloads

The source archive was exported from the local DUCC Git object database to a
separate temporary directory. No DUCC checkout was changed. The build used
the source revision's default `DUCC0_OPTIMIZATION=native-strip`, which
configured CMake with `-O3 -ffast-math -march=native -flto=auto` and
`-fno-fat-lto-objects`; link-time IPO was enabled and the extension was
stripped. The corresponding Pulp runtime dispatch in RustSHT used AVX2/FMA
on this host and did not use `target-cpu=native`.

| Item | Value |
| --- | --- |
| CPU / OS | AMD Ryzen 9 5950X; Linux `7.0.14-20-pve` |
| C++ compiler | GCC `15.2.0-16ubuntu1` |
| CMake / scikit-build-core / pybind11 | `4.2.3` / `1.0.3` / `3.1.0` |
| Python / NumPy | `3.11.15` / `2.4.6` |
| Installed DUCC | `ducc0==0.41.0` |
| Source build | revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, reports `0.41.1` |
| Grid and precision | f64 GL; T511 `512x1025`, T720 `721x1443` |
| Harmonics and mode | full `mmax=lmax`, `phi0=0.37`, STANDARD |
| Threading | explicit DUCC `nthreads=1/8`; pinned to CPU 0 or CPUs 0–7 |

Five process pairs alternated installed and source-built libraries. Each
process performed one warm-up and five timed DUCC calls. The same input
generation and operation wrapper from `examples/python_perf_baseline.py`
were used for both builds. `RUSTSHT_PERF_RAW=1` records each timed call as
well as the per-process median. The raw process samples are in
[`process-samples.csv`](data/ducc-source-build-screen-2026-10-14/process-samples.csv).

## Paired DUCC timings

Values are medians of five process medians, in milliseconds. The final
column is the median paired percentage change (source build relative to the
installed package), with the full five-pair range in parentheses.

| Band | Operation | Threads | Installed | Source build | Paired change (range) |
| --- | --- | ---: | ---: | ---: | ---: |
| T511 | Scalar synthesis | 1 | 9.805 | 9.771 | -0.4% (-0.6% to +2.3%) |
| T511 | Scalar synthesis | 8 | 2.790 | 2.766 | -0.3% (-1.7% to +2.3%) |
| T511 | Scalar analysis | 1 | 6.430 | 6.450 | +0.5% (-2.0% to +18.5%) |
| T511 | Scalar analysis | 8 | 1.085 | 1.044 | -1.1% (-14.7% to +8.4%) |
| T511 | Spin-1 synthesis | 1 | 21.425 | 21.359 | -0.2% (-0.9% to +0.7%) |
| T511 | Spin-1 synthesis | 8 | 4.251 | 4.212 | -0.6% (-3.8% to +1.8%) |
| T511 | Spin-1 analysis | 1 | 12.874 | 12.934 | +0.4% (-17.8% to +1.2%) |
| T511 | Spin-1 analysis | 8 | 1.972 | 1.947 | -1.0% (-2.2% to +3.7%) |
| T720 | Scalar synthesis | 1 | 24.097 | 24.491 | +1.6% (-0.1% to +4.5%) |
| T720 | Scalar synthesis | 8 | 6.243 | 6.167 | +0.1% (-7.3% to +3.8%) |
| T720 | Scalar analysis | 1 | 14.841 | 14.987 | +1.0% (-8.0% to +5.3%) |
| T720 | Scalar analysis | 8 | 2.296 | 2.273 | -1.0% (-3.9% to +2.8%) |
| T720 | Spin-1 synthesis | 1 | 52.624 | 52.658 | -0.1% (-0.4% to +1.5%) |
| T720 | Spin-1 synthesis | 8 | 8.975 | 8.954 | +0.1% (-4.8% to +4.7%) |
| T720 | Spin-1 analysis | 1 | 30.666 | 30.326 | -0.7% (-2.4% to +1.3%) |
| T720 | Spin-1 analysis | 8 | 4.584 | 4.445 | -2.9% (-5.5% to +3.3%) |

The wide q8 ranges show shared-host scheduling and frequency variation; no
source-build speedup is accepted from the small median differences. Raw
inner-call samples for each process are retained in the CSV rather than only
the table medians.

## Numerical comparison

The source-built DUCC extension was compared with the current RustSHT plan
using matching deterministic inputs. Relative-L2 errors stayed between
`3.486e-14` and `9.123e-14` across the tested operations, bands, and thread
counts. Maximum absolute, RMS, relative-L2, Python-plan timing, and raw timing
values are in
[`source-ducc-rust-errors.csv`](data/ducc-source-build-screen-2026-10-14/source-ducc-rust-errors.csv).
These comparisons validate numerical agreement; the single-process Python
timings in that file are not used for the source-build performance decision.

The source build leaves the current native Rust-to-DUCC gap materially
unchanged. Using the paired Rust native timings from commit `612a90e` for
operations unaffected by the spin-1 analysis change, and `614db91` for that
analysis change, the approximate T720 ratios against the source-build DUCC
medians are 4.2x/4.5x for scalar synthesis, 4.1x/8.6x for scalar analysis,
2.4x/4.7x for spin-1 synthesis, and 3.2x/7.5x for spin-1 analysis at q1/q8.
These ratios combine native Rust timings with DUCC Python-call timings; the
Python boundary contributes only the direct call and result wrapper in the
timed function, but they are not a single native harness comparison.

## Reproduction

The source archive and build were created outside the DUCC checkout:

```text
git -C /home/albert/ducc archive --format=tar c1ed01b8fda69d2a378190f0915f1a19762dddbf | tar -xf - -C /home/albert/ducc-profile-20261014/source
TMPDIR=/home/albert/ducc-profile-20261014/tmp DUCC0_OPTIMIZATION=native-strip CMAKE_BUILD_PARALLEL_LEVEL=8 uv build --wheel --out-dir /home/albert/ducc-profile-20261014/wheels /home/albert/ducc-profile-20261014/source
```

Representative DUCC-only timing and source-build numerical commands:

```text
RUSTSHT_PERF_RAW=1 RAYON_NUM_THREADS=1 taskset -c 0 uv run python examples/python_perf_baseline.py 511 0 1 GL 1 plan synthesis ducc STANDARD
PYTHONPATH=/home/albert/ducc-profile-20261014/native-package RUSTSHT_PERF_RAW=1 RAYON_NUM_THREADS=8 taskset -c 0-7 uv run python examples/python_perf_baseline.py 720 1 8 GL 1 plan analysis ducc STANDARD
PYTHONPATH=/home/albert/ducc-profile-20261014/native-package RUSTSHT_PERF_RAW=1 RAYON_NUM_THREADS=8 taskset -c 0-7 uv run python examples/python_perf_baseline.py 720 1 8 GL 1 plan analysis both STANDARD
```

`examples/python_perf_baseline.py` retains its previous output by default.
Set `RUSTSHT_PERF_RAW=1` to add the five timed calls for each reported
operation. This allows process medians and within-process variation to be
reconstructed without changing the numerical API.
