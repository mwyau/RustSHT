# Current-source native RustSHT/DUCC scalar GL sweep (2026-10-21)

Production Rust source: `e487b85bc98273dd428a94529689124a5c9fa1e6`; benchmark harness commit: `9b728a904d499b1ccb9a6921a3d4da574983cf0a`.
Pinned DUCC 0.42.0 source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
Host: AMD Ryzen 9 5950X 16-Core Processor; selected q1 CPU `0`, q8 CPUs `0,1,2,3,4,5,6,7`; q8 L3 cache IDs `['0', '0', '0', '0', '0', '0', '0', '0']` (one CCD on this host).
Toolchains: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1, Cargo 1.99.1, Pulp 0.22.3, with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC used `g++ (Ubuntu 15.2.0-16ubuntu1) 15.2.0`.
DUCC SHT translation units used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the native benchmark driver used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`. The actual commands were verified in `compile_commands.json` and are preserved in the result JSON.
The production Rust source matches the frozen baseline: `True`; changed production paths since it: `[]`. Executed Rust Pulp architectures: `['V3(V3 { sse: Sse, sse2: Sse2, fxsr: Fxsr, sse3: Sse3, ssse3: Ssse3, sse4_1: Sse4_1, sse4_2: Sse4_2, popcnt: Popcnt, avx: Avx, avx2: Avx2, bmi1: Bmi1, bmi2: Bmi2, fma: Fma, lzcnt: Lzcnt })']`, f64 lane widths `[4]`; DUCC runtime label: `['x86-64-v3 fixed build']`.
The exact `rustc -Vv`, DUCC SHT compile command, and strict benchmark-driver compile command are preserved in the result JSON metadata.

## Scope and method

The case manifest contains 376 distinct GL geometries: every `lmax=0…128` plus the requested 28 sampled higher degrees. For each applicable degree it covers full and quarter-order `mmax`, with even FFT-smooth longitude counts throughout and odd-minimum counts at the selected small, boundary and larger degrees. Each transform uses `ntheta=lmax+1`, `phi0=0.23`, the full permitted degree/order triangle and deterministic PCG64 fixtures shared by both native libraries.
Both analysis and synthesis were run at q1/q8. Short calls use 3 interleaved process pairs and 10 calibrated batches per backend process, targeting 25 ms; long calls use the existing individually timed rule. Rust plan/workspace setup, first-call and warmed timings are separate. The saved JSON contains every invocation, batch, numerical comparison, validation, frequency sample and scheduler observation; CSV rows are computed from paired process data. Failure records are retained in `failures.csv` and JSON.
The q1/q8 CPU affinity was pinned to the physical core sets shown above. q8 uses the first eight physical cores visible to the process (one CCD on this host). RustSHT and DUCC receive the same fixtures and reusable caller buffers; DUCC's public API performs its own internal setup on each call.

## Coverage audit

The low-degree audit includes 129/129 integer degrees, with no omitted `lmax` values. The required higher list has 28 values; case selection check: `True`. Low-degree even-smooth full/reduced coverage omissions: 0. Across 1504 operation/thread groups, statuses are `{'complete': 1504, 'partial': 0, 'failed': 0, 'missing': 0}`. Failed or partial groups are listed explicitly rather than silently removed.

## Warmed whole-transform summary

Ratios below 1 mean RustSHT was faster. Each table cell reports median ratio, geometric mean ratio, and completed paired case count. Odd-minimum cases are excluded from this even-smooth summary.

| lmax band | operation | mmax | threads | median ratio | geomean ratio | cases |
|---|---|---|---:|---:|---:|---:|
| 1–32 | analysis | full | q1 | 1.181 | 0.987 | 32 |
| 1–32 | analysis | full | q8 | 0.621 | 0.647 | 32 |
| 1–32 | analysis | reduced | q1 | 0.779 | 0.718 | 31 |
| 1–32 | analysis | reduced | q8 | 0.401 | 0.518 | 31 |
| 1–32 | synthesis | full | q1 | 0.933 | 0.826 | 32 |
| 1–32 | synthesis | full | q8 | 0.554 | 0.554 | 32 |
| 1–32 | synthesis | reduced | q1 | 0.733 | 0.653 | 31 |
| 1–32 | synthesis | reduced | q8 | 0.395 | 0.438 | 31 |
| 33–63 | analysis | full | q1 | 1.329 | 1.303 | 31 |
| 33–63 | analysis | full | q8 | 0.857 | 0.865 | 31 |
| 33–63 | analysis | reduced | q1 | 1.016 | 1.017 | 31 |
| 33–63 | analysis | reduced | q8 | 0.500 | 0.495 | 31 |
| 33–63 | synthesis | full | q1 | 1.118 | 1.105 | 31 |
| 33–63 | synthesis | full | q8 | 0.720 | 0.745 | 31 |
| 33–63 | synthesis | reduced | q1 | 0.945 | 0.965 | 31 |
| 33–63 | synthesis | reduced | q8 | 0.477 | 0.467 | 31 |
| 64–128 | analysis | full | q1 | 1.133 | 1.099 | 65 |
| 64–128 | analysis | full | q8 | 2.096 | 2.108 | 65 |
| 64–128 | analysis | reduced | q1 | 1.088 | 1.091 | 65 |
| 64–128 | analysis | reduced | q8 | 0.801 | 0.821 | 65 |
| 64–128 | synthesis | full | q1 | 1.146 | 1.151 | 65 |
| 64–128 | synthesis | full | q8 | 1.448 | 1.381 | 65 |
| 64–128 | synthesis | reduced | q1 | 1.069 | 1.061 | 65 |
| 64–128 | synthesis | reduced | q8 | 0.708 | 0.711 | 65 |
| 129–256 | analysis | full | q1 | 1.284 | 1.473 | 8 |
| 129–256 | analysis | full | q8 | 1.703 | 1.913 | 8 |
| 129–256 | analysis | reduced | q1 | 1.093 | 1.077 | 8 |
| 129–256 | analysis | reduced | q8 | 1.612 | 1.533 | 8 |
| 129–256 | synthesis | full | q1 | 1.130 | 1.134 | 8 |
| 129–256 | synthesis | full | q8 | 1.079 | 1.096 | 8 |
| 129–256 | synthesis | reduced | q1 | 1.076 | 1.070 | 8 |
| 129–256 | synthesis | reduced | q8 | 1.229 | 1.197 | 8 |
| 257–720 | analysis | full | q1 | 1.144 | 1.143 | 10 |
| 257–720 | analysis | full | q8 | 1.084 | 1.091 | 10 |
| 257–720 | analysis | reduced | q1 | 1.080 | 1.079 | 10 |
| 257–720 | analysis | reduced | q8 | 1.388 | 1.345 | 10 |
| 257–720 | synthesis | full | q1 | 1.064 | 1.061 | 10 |
| 257–720 | synthesis | full | q8 | 1.085 | 1.077 | 10 |
| 257–720 | synthesis | reduced | q1 | 1.074 | 1.079 | 10 |
| 257–720 | synthesis | reduced | q8 | 1.189 | 1.202 | 10 |
| 721–1536 | analysis | full | q1 | 1.073 | 1.054 | 6 |
| 721–1536 | analysis | full | q8 | 1.052 | 1.004 | 6 |
| 721–1536 | analysis | reduced | q1 | 1.069 | 1.071 | 6 |
| 721–1536 | analysis | reduced | q8 | 1.035 | 1.032 | 6 |
| 721–1536 | synthesis | full | q1 | 1.055 | 1.031 | 6 |
| 721–1536 | synthesis | full | q8 | 1.048 | 1.006 | 6 |
| 721–1536 | synthesis | reduced | q1 | 1.066 | 1.067 | 6 |
| 721–1536 | synthesis | reduced | q8 | 1.049 | 1.053 | 6 |
| 1537–3072 | analysis | full | q1 | 0.987 | 0.987 | 4 |
| 1537–3072 | analysis | full | q8 | 0.846 | 0.857 | 4 |
| 1537–3072 | analysis | reduced | q1 | 1.077 | 1.071 | 4 |
| 1537–3072 | analysis | reduced | q8 | 1.053 | 1.029 | 4 |
| 1537–3072 | synthesis | full | q1 | 0.945 | 0.948 | 4 |
| 1537–3072 | synthesis | full | q8 | 0.876 | 0.875 | 4 |
| 1537–3072 | synthesis | reduced | q1 | 1.085 | 1.080 | 4 |
| 1537–3072 | synthesis | reduced | q8 | 1.043 | 1.027 | 4 |

### Fast and slow measured cases

| case | operation | q | Rust ms | DUCC ms | ratio | relative L2 |
|---|---|---:|---:|---:|---:|---:|
| **Largest ratios** |  |  |  |  |  |  |
| T254 m254 even_smooth | analysis | q8 | 0.660638 | 0.221093 | 2.988 | 1.726e-14 |
| T248 m248 even_smooth | analysis | q8 | 0.618952 | 0.211467 | 2.927 | 1.841e-14 |
| T124 m124 even_smooth | analysis | q8 | 0.204732 | 0.073968 | 2.768 | 8.268e-15 |
| T122 m122 even_smooth | analysis | q8 | 0.198483 | 0.0727063 | 2.730 | 8.183e-15 |
| T254 m254 even_smooth | analysis | q1 | 3.24191 | 1.18809 | 2.729 | 1.726e-14 |
| T121 m121 even_smooth | analysis | q8 | 0.194345 | 0.0716138 | 2.714 | 7.561e-15 |
| T120 m120 even_smooth | analysis | q8 | 0.192242 | 0.071095 | 2.704 | 8.327e-15 |
| T126 m126 even_smooth | analysis | q8 | 0.18662 | 0.0716867 | 2.603 | 9.123e-15 |
| **Smallest ratios** |  |  |  |  |  |  |
| T0 m0 even_smooth | synthesis | q8 | 0.000184812 | 0.000940424 | 0.197 | 0.000e+00 |
| T63 m63 odd_minimum | analysis | q1 | 0.0695448 | 0.34042 | 0.204 | 3.440e-15 |
| T0 m0 odd_minimum | synthesis | q8 | 0.000186382 | 0.00088642 | 0.210 | 0.000e+00 |
| T0 m0 even_smooth | analysis | q8 | 0.000229389 | 0.00104025 | 0.221 | 0.000e+00 |
| T0 m0 odd_minimum | analysis | q1 | 0.000226353 | 0.00101654 | 0.223 | 0.000e+00 |
| T0 m0 even_smooth | analysis | q1 | 0.000233511 | 0.00104415 | 0.224 | 0.000e+00 |
| T0 m0 odd_minimum | analysis | q8 | 0.000225046 | 0.000996065 | 0.226 | 0.000e+00 |
| T0 m0 even_smooth | synthesis | q1 | 0.000254176 | 0.000942357 | 0.270 | 0.000e+00 |

### Longitude-shape sensitivity

- analysis q1: odd/even median runtime factor Rust 1.124×, DUCC 1.184× across 64 matched cases.
- analysis q8: odd/even median runtime factor Rust 1.107×, DUCC 1.035× across 64 matched cases.
- synthesis q1: odd/even median runtime factor Rust 1.213×, DUCC 1.170× across 64 matched cases.
- synthesis q8: odd/even median runtime factor Rust 1.277×, DUCC 1.048× across 64 matched cases.

### Crossover and numerical results

- analysis full q1: 34 observed points below parity and 123 above; lowest-ratio degree 0, highest-ratio degree 254; observed parity crossings T10→T11, T62→T63, T65→T66, T66→T67, T67→T68, T70→T71, T71→T72, T74→T75, T75→T76, T78→T79, T79→T80, T82→T83, T83→T84, T86→T87, T87→T88, T90→T91, T91→T92, T94→T95, T95→T96, T98→T99, T99→T100, T102→T103, T103→T104, T106→T107, T107→T108, T110→T111, T111→T112, T114→T115, T115→T116, T118→T119, T119→T120, T122→T123, T123→T124, T126→T127, T127→T128, T1280→T1536, T2048→T3072 (sampled points only).
- analysis full q8: 60 observed points below parity and 97 above; lowest-ratio degree 0, highest-ratio degree 254; observed parity crossings T0→T1, T3→T4, T57→T58, T1280→T1536 (sampled points only).
- analysis reduced q1: 46 observed points below parity and 109 above; lowest-ratio degree 2, highest-ratio degree 76; observed parity crossings T27→T28, T29→T30, T35→T36, T39→T40, T41→T42, T45→T46, T51→T52, T53→T54, T55→T56, T79→T80, T81→T82, T109→T110, T111→T112 (sampled points only).
- analysis reduced q8: 115 observed points below parity and 40 above; lowest-ratio degree 31, highest-ratio degree 384; observed parity crossings T3→T4, T108→T109, T109→T110, T112→T113, T119→T120, T123→T124, T125→T126, T126→T127, T448→T511, T511→T512, T2048→T3072 (sampled points only).
- synthesis full q1: 27 observed points below parity and 130 above; lowest-ratio degree 0, highest-ratio degree 72; observed parity crossings T17→T18, T19→T20, T21→T22, T31→T32, T32→T33, T513→T640, T640→T720, T1280→T1536 (sampled points only).
- synthesis full q8: 70 observed points below parity and 87 above; lowest-ratio degree 0, highest-ratio degree 123; observed parity crossings T62→T63, T69→T70, T71→T72, T1280→T1536 (sampled points only).
- synthesis reduced q1: 54 observed points below parity and 101 above; lowest-ratio degree 2, highest-ratio degree 98; observed parity crossings T35→T36, T39→T40, T55→T56, T60→T61, T61→T62, T79→T80, T80→T81, T82→T83, T83→T84, T127→T128, T128→T129 (sampled points only).
- synthesis reduced q8: 129 observed points below parity and 26 above; lowest-ratio degree 27, highest-ratio degree 384; observed parity crossings T129→T159, T2048→T3072 (sampled points only).
Maximum random-output relative L2 error: `2.536e-13`; maximum absolute error: `1.156e-08`.
Validation ran for 752 case/thread configurations: finite edge outputs 752/752, reused-output reset checks 752/752; maximum roundtrip relative L2 `5.563e-13`, selected-basis recovery `5.394e-12`, and adjoint residual `3.427e-12`. Sidecar results include zero, subnormal and extreme-finite checks.
Observed CPU frequency median/p10/p90 `4.467/1.746/4.766` GHz; scheduler wait median/p90 `0.008419/0.2696`; involuntary-switch median/max `63/134844`. No batch samples are removed from the saved distributions.
There were 0 recorded failure/warning events. Inspect `data/ducc-current-full-sweep-2026-10-21/failures.csv` and the JSON `failures` array; unsupported cases are labeled separately from worker and numerical-validation errors.

## Principal figures

- [Analysis, full mmax](./data/ducc-current-full-sweep-2026-10-21/ratio-analysis-full-mmax.svg)
- [Analysis, reduced mmax](./data/ducc-current-full-sweep-2026-10-21/ratio-analysis-reduced-mmax.svg)
- [Synthesis, full mmax](./data/ducc-current-full-sweep-2026-10-21/ratio-synthesis-full-mmax.svg)
- [Synthesis, reduced mmax](./data/ducc-current-full-sweep-2026-10-21/ratio-synthesis-reduced-mmax.svg)

The four figures plot measured warmed RustSHT/DUCC ratios for even-smooth longitude cases only; q1 and q8 are separate curves and points are not smoothed. Odd-minimum cases remain in the data and shape summary.

## Reproduction and attribution

The machine-readable artifacts are [`results.csv`](./data/ducc-current-full-sweep-2026-10-21/results.csv), [`results.json`](./data/ducc-current-full-sweep-2026-10-21/results.json), and [`coverage-audit.json`](./data/ducc-current-full-sweep-2026-10-21/coverage-audit.json). Rust plan setup/first-call fields, calibrated warm batch distributions, process pairing, per-case numerical errors, host scheduling/frequency diagnostics, and every requested geometry are retained. The JSON records the full `rustc -Vv` output, selected runtime backends, binary hashes, and exact compiler/build invocations.

```sh
REPO=/path/to/RustSHT
DUCC_SRC=/path/to/ducc-42ce8ab5c5d7
DUCC_BUILD=$(mktemp -d)
SHARD_ROOT=$(mktemp -d)
cd "$REPO"
git checkout 9b728a904d499b1ccb9a6921a3d4da574983cf0a
git -C "$DUCC_SRC" checkout --detach 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b
# Rust benchmark executable (the production source must match e487b85bc98273dd428a94529689124a5c9fa1e6).
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo build --release --example ducc_full_transform_parity --features parallel
# DUCC native benchmark; the driver restores strict floating-point checks after the DUCC SHT flags.
cmake -S reports/data/ducc-full-transform-parity-2026-10-20 -B "${DUCC_BUILD}" -DDUCC_SRC="$DUCC_SRC" -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=g++ -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
cmake --build "${DUCC_BUILD}" --parallel 4
# The verified SHT and driver compile commands are in results.json metadata fields
# ducc_sht_compile_command and ducc_driver_compile_command.
# Each shard used the same binaries and this invocation shape (the report lists all eight exact degree selections):
/usr/bin/python3 "$REPO"/scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --results-dir "${SHARD_ROOT}/chunk-01-T0-T32" --report-path "${SHARD_ROOT}/chunk-01-T0-T32/unused.md" --no-report --skip-build --dense-sweep --production-source-revision e487b85bc98273dd428a94529689124a5c9fa1e6 --sweep-id current-source-dense-2026-10-21 --lmax 0 --lmax 1 --lmax 2 --lmax 3 --lmax 4 --lmax 5 --lmax 6 --lmax 7 --lmax 8 --lmax 9 --lmax 10 --lmax 11 --lmax 12 --lmax 13 --lmax 14 --lmax 15 --lmax 16 --lmax 17 --lmax 18 --lmax 19 --lmax 20 --lmax 21 --lmax 22 --lmax 23 --lmax 24 --lmax 25 --lmax 26 --lmax 27 --lmax 28 --lmax 29 --lmax 30 --lmax 31 --lmax 32
/usr/bin/python3 "$REPO"/scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --results-dir "${SHARD_ROOT}/chunk-02-T33-T64" --report-path "${SHARD_ROOT}/chunk-02-T33-T64/unused.md" --no-report --skip-build --dense-sweep --production-source-revision e487b85bc98273dd428a94529689124a5c9fa1e6 --sweep-id current-source-dense-2026-10-21 --lmax 33 --lmax 34 --lmax 35 --lmax 36 --lmax 37 --lmax 38 --lmax 39 --lmax 40 --lmax 41 --lmax 42 --lmax 43 --lmax 44 --lmax 45 --lmax 46 --lmax 47 --lmax 48 --lmax 49 --lmax 50 --lmax 51 --lmax 52 --lmax 53 --lmax 54 --lmax 55 --lmax 56 --lmax 57 --lmax 58 --lmax 59 --lmax 60 --lmax 61 --lmax 62 --lmax 63 --lmax 64
/usr/bin/python3 "$REPO"/scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --results-dir "${SHARD_ROOT}/chunk-03-T65-T96" --report-path "${SHARD_ROOT}/chunk-03-T65-T96/unused.md" --no-report --skip-build --dense-sweep --production-source-revision e487b85bc98273dd428a94529689124a5c9fa1e6 --sweep-id current-source-dense-2026-10-21 --lmax 65 --lmax 66 --lmax 67 --lmax 68 --lmax 69 --lmax 70 --lmax 71 --lmax 72 --lmax 73 --lmax 74 --lmax 75 --lmax 76 --lmax 77 --lmax 78 --lmax 79 --lmax 80 --lmax 81 --lmax 82 --lmax 83 --lmax 84 --lmax 85 --lmax 86 --lmax 87 --lmax 88 --lmax 89 --lmax 90 --lmax 91 --lmax 92 --lmax 93 --lmax 94 --lmax 95 --lmax 96
/usr/bin/python3 "$REPO"/scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --results-dir "${SHARD_ROOT}/chunk-04-T97-T128" --report-path "${SHARD_ROOT}/chunk-04-T97-T128/unused.md" --no-report --skip-build --dense-sweep --production-source-revision e487b85bc98273dd428a94529689124a5c9fa1e6 --sweep-id current-source-dense-2026-10-21 --lmax 97 --lmax 98 --lmax 99 --lmax 100 --lmax 101 --lmax 102 --lmax 103 --lmax 104 --lmax 105 --lmax 106 --lmax 107 --lmax 108 --lmax 109 --lmax 110 --lmax 111 --lmax 112 --lmax 113 --lmax 114 --lmax 115 --lmax 116 --lmax 117 --lmax 118 --lmax 119 --lmax 120 --lmax 121 --lmax 122 --lmax 123 --lmax 124 --lmax 125 --lmax 126 --lmax 127 --lmax 128
/usr/bin/python3 "$REPO"/scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --results-dir "${SHARD_ROOT}/chunk-05-T129-T257" --report-path "${SHARD_ROOT}/chunk-05-T129-T257/unused.md" --no-report --skip-build --dense-sweep --production-source-revision e487b85bc98273dd428a94529689124a5c9fa1e6 --sweep-id current-source-dense-2026-10-21 --lmax 129 --lmax 159 --lmax 191 --lmax 223 --lmax 248 --lmax 254 --lmax 255 --lmax 256 --lmax 257
/usr/bin/python3 "$REPO"/scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --results-dir "${SHARD_ROOT}/chunk-06-T300-T513" --report-path "${SHARD_ROOT}/chunk-06-T300-T513/unused.md" --no-report --skip-build --dense-sweep --production-source-revision e487b85bc98273dd428a94529689124a5c9fa1e6 --sweep-id current-source-dense-2026-10-21 --lmax 300 --lmax 320 --lmax 384 --lmax 448 --lmax 511 --lmax 512 --lmax 513
/usr/bin/python3 "$REPO"/scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --results-dir "${SHARD_ROOT}/chunk-07-T640-T1024" --report-path "${SHARD_ROOT}/chunk-07-T640-T1024/unused.md" --no-report --skip-build --dense-sweep --production-source-revision e487b85bc98273dd428a94529689124a5c9fa1e6 --sweep-id current-source-dense-2026-10-21 --lmax 640 --lmax 720 --lmax 721 --lmax 768 --lmax 896 --lmax 1024
/usr/bin/python3 "$REPO"/scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --results-dir "${SHARD_ROOT}/chunk-08-T1280-T3072" --report-path "${SHARD_ROOT}/chunk-08-T1280-T3072/unused.md" --no-report --skip-build --dense-sweep --production-source-revision e487b85bc98273dd428a94529689124a5c9fa1e6 --sweep-id current-source-dense-2026-10-21 --lmax 1280 --lmax 1536 --lmax 1792 --lmax 2047 --lmax 2048 --lmax 3072
```

The October 20 sweep is retained as a historical illustration only. It used a different Rust source/toolchain combination; its points are not merged into this dataset or used to compute the ratios above.
