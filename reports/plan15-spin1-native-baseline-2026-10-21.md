# Plan 15 native spin-1 GL/CC/F1 baseline

**Status:** Phase A baseline complete; Phase B GL diagnosis starts from this
source. **Rust production source:** `549bd2f0b692c7035139d2942f46e57962b7adfc`
(the later `bacc8cbe` commit changes only scalar diagnostic wording).
**DUCC:** 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Matched implementation and protocol

The benchmark exercises the native production-dispatched f64 spin-1 STANDARD
E/B transform: two real spatial channels, both complete complex harmonic
triangles, the GL/CC/F1 meridional path, and the full longitude FFT. Rust reuses
one plan and workspace per worker process; DUCC calls the corresponding native
analysis/synthesis entry point with the same fixtures and thread count. Setup,
first call and warmed execution are recorded separately. Each timed call resets
its output, and checksums plus numerical validation run outside the timed
interval.

Rust 1.99.1 / LLVM 23.1.1 and Pulp 0.22.3 used release mode with
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; production code reaches Pulp's
runtime `Arch::new()` dispatch. A separate probe using the same build flags
and host returned Pulp V3 (four f64 lanes); the spin-1 worker itself does not
serialize its selected-architecture label. DUCC used GCC 15.2.0, with SHT sources built
using `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the validation
driver adds `-fno-fast-math`. Both binaries and the exact compiler commands
are identified by SHA-256 and recorded in the machine-readable metadata.
The host was an AMD Ryzen 9 5950X. q1 used one pinned physical core; q8 used
eight pinned physical cores within one L3 domain.

For short operations, each case used three interleaved process pairs and ten
calibrated batches near 25 ms per process, giving 30 timed batch samples per
backend. Operations exceeding the runner's 100 ms long-call threshold were
timed individually; the per-backend sample counts are included in each result
row. RustSHT and DUCC order alternated across process pairs. The published
dataset contains median, p10 and p90 latency, paired process-median ratio,
absolute excess, first-call/setup medians and numerical errors. The local raw
worker logs include process and host telemetry and are intentionally not
published.

## Coverage and numerical checks

| Grid | Geometry cases | Measurement rows | Comparisons | Failed cases | Maximum relative L2 | Maximum absolute error |
|---|---:|---:|---:|---:|---:|---:|
| GL | 54 | 216 | 648 | 0 | `5.64e-13` | `6.88e-9` |
| CC | 62 | 248 | 744 | 0 | `1.36e-12` | `2.20e-8` |
| F1 | 54 | 216 | 648 | 0 | `1.63e-12` | `2.44e-8` |
| **Total** | **170** | **680** | **2,040** | **0** | **1.63e-12** | **2.44e-8** |

The degree samples are `1, 4, 16, 32, 63, 64, 65, 127, 128, 159, 255,
256, 511, 720, 1023`, with full and quarter-order transforms, q1/q8, and
even-smooth plus selected odd-minimum longitude counts. GL and F1 include
exact and 2x-latitude over-resolved controls. CC includes exact sampling and
latitudes just below and at its resampling-skip threshold. Every result was
finite, repeat-consistent and output-reset-consistent. The harness also ran
roundtrip, selected basis, zero, subnormal, extreme-finite and S/A adjoint
checks; no comparison exceeded the existing numerical checks.

## End-to-end performance summary

The median ratio below is the median of per-configuration paired Rust/DUCC
ratios within each grid, operation and thread count. It pools full/reduced
order and the sampled longitude/latitude geometries, so it is a compact
description rather than a parity score. The last column identifies the
largest positive absolute excess in that group; all times are warmed-call
medians.

| Grid | Operation | Threads | Configs | Median paired ratio | Largest excess case (Rust vs DUCC; excess) |
|---|---|---:|---:|---:|---|
| GL | Analysis | q1 | 54 | 1.651× | T1023 full exact: 171.164 vs 86.003 ms; +85.161 ms |
| GL | Analysis | q8 | 54 | 1.899× | T1023 full exact: 28.745 vs 15.305 ms; +13.440 ms |
| GL | Synthesis | q1 | 54 | 1.074× | T1023 full exact: 260.559 vs 121.636 ms; +138.923 ms |
| GL | Synthesis | q8 | 54 | 2.821× | T720 full over-resolved odd: 76.483 vs 15.754 ms; +60.729 ms |
| CC | Analysis | q1 | 62 | 1.509× | T1023 reduced exact: 290.213 vs 94.699 ms; +195.513 ms |
| CC | Analysis | q8 | 62 | 1.008× | T1023 full exact: 201.681 vs 67.215 ms; +134.465 ms |
| CC | Synthesis | q1 | 62 | 2.262× | T720 full resampling-active: 293.856 vs 95.123 ms; +198.733 ms |
| CC | Synthesis | q8 | 62 | 4.538× | T1023 full exact: 234.293 vs 30.944 ms; +203.349 ms |
| F1 | Analysis | q1 | 54 | 1.418× | T1023 full exact: 505.730 vs 259.020 ms; +246.710 ms |
| F1 | Analysis | q8 | 54 | 1.013× | T1023 full exact: 195.409 vs 66.340 ms; +129.069 ms |
| F1 | Synthesis | q1 | 54 | 1.292× | T1023 full exact: 500.873 vs 157.690 ms; +343.182 ms |
| F1 | Synthesis | q8 | 54 | 2.702× | T1023 full exact: 131.058 vs 30.281 ms; +100.777 ms |

GL q8 full synthesis has the clearest immediate Phase B target: T720 exact
odd is 52.631 vs 8.282 ms (+44.349 ms), and the 2x-latitude odd control is
76.483 vs 15.754 ms (+60.729 ms). GL q1 T1023 synthesis has a larger absolute
excess (+138.923 ms), so it also needs to remain in the Phase B evaluation.
The large CC/F1 q8 synthesis gaps are stronger overall, but those operations
include grid-specific meridional preparation and will be attributed in their
own phases rather than treated as evidence for a GL kernel change.

The q8 row-wise medians hide wide case variation: e.g. CC and F1 analysis have
near-unity median ratios while their full T1023 cases retain 3x gaps. Host
wait was elevated in the tail, especially during F1 (aggregate median/p90
wait fractions 17%/40%; GL 0.4%/27%, CC 3.8%/27%). The dataset supports
ranking large excesses but not accepting small percentage differences; repeat
ambiguous A/B cases under lower contention.

## Reproduction and artifacts

The benchmark runner and C++ driver are in
[`reports/data/plan15-spin1-native-baseline-2026-10-21/`](data/plan15-spin1-native-baseline-2026-10-21/).
The sanitized aggregate files are
[`full-transform-summary.csv`](data/plan15-spin1-baseline-2026-10-21/published/full-transform-summary.csv)
and
[`full-transform-summary.json`](data/plan15-spin1-baseline-2026-10-21/published/full-transform-summary.json).
They retain all 680 aggregate rows and per-case uncertainty, cold/setup
costs and numerical error, while omitting raw process IDs, CPU-frequency
samples and scheduler logs.

With the DUCC checkout at the immutable revision above, run:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
python3 scripts/benchmark_ducc_spin1_native.py \
  --ducc-src "$DUCC_SRC" --process-pairs 3 --target-ms 25 \
  --build-dir "$BUILD_DIR" \
  --grid GL --results-dir "$RESULTS_DIR/GL"

RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
python3 scripts/benchmark_ducc_spin1_native.py \
  --ducc-src "$DUCC_SRC" --process-pairs 3 --target-ms 25 \
  --build-dir "$BUILD_DIR" --skip-build \
  --grid CC --results-dir "$RESULTS_DIR/CC"

RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
python3 scripts/benchmark_ducc_spin1_native.py \
  --ducc-src "$DUCC_SRC" --process-pairs 3 --target-ms 25 \
  --build-dir "$BUILD_DIR" --skip-build \
  --grid F1 --results-dir "$RESULTS_DIR/F1"

python3 scripts/summarize_ducc_spin1_baseline.py \
  --input-dir "$RESULTS_DIR" --output-dir "$RESULTS_DIR/published"
```

The Rust binary was built from source revision `549bd2f0`; pinned DUCC's
source revision and both optimized binary hashes are in the JSON metadata.
Baseline results are a starting point, not a stage attribution. Phase B begins
by decomposing GL spin-1 analysis/synthesis, with priority on the confirmed
whole-call excesses above.
