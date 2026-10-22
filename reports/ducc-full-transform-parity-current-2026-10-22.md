# Current-source native RustSHT/DUCC scalar GL sweep (2026-10-22)

Production Rust source: `819d1f447043cadbc85b417b5ef3698139bfe875`; benchmark harness commit: `08b938a1d9f89455b9c7a0b8bd81776988ea314a`.
Pinned DUCC 0.42.0 source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
Host: AMD Ryzen 9 5950X 16-Core Processor; selected q1 CPU `0`, q8 CPUs `0,1,2,3,4,5,6,7`; q8 L3 cache IDs `['0', '0', '0', '0', '0', '0', '0', '0']` (one CCD on this host).
Toolchains: Rust `rustc 1.99.1 (58cf8f9c3 2026-10-13)` / 0.22.3 with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC `g++ (Ubuntu 15.2.0-16ubuntu1) 15.2.0` with `DUCC sources: -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; benchmark driver: -O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`.
The production Rust source matches the frozen baseline: `True`; changed production paths since it: `[]`. Executed Rust Pulp architectures: `['V3(V3 { sse: Sse, sse2: Sse2, fxsr: Fxsr, sse3: Sse3, ssse3: Ssse3, sse4_1: Sse4_1, sse4_2: Sse4_2, popcnt: Popcnt, avx: Avx, avx2: Avx2, bmi1: Bmi1, bmi2: Bmi2, fma: Fma, lzcnt: Lzcnt })']`, f64 lane widths `[4]`; DUCC runtime label: `['x86-64-v3 fixed build']`.
The exact `rustc -Vv`, DUCC SHT compile command, and strict benchmark-driver compile command are preserved in the result JSON metadata.

## Scope and method

The case manifest contains 630 distinct GL geometries: every `lmax=0…256` plus 24 selected higher degrees through T3072. Every degree includes full and quarter-order `mmax` with even FFT-smooth longitude where those truncations differ; identical low-degree cases are deduplicated. Selected small, dispatch-boundary and high-resolution cases also include odd-minimum longitude. Each transform uses `ntheta=lmax+1`, `phi0=0.23`, the full permitted degree/order triangle and deterministic PCG64 fixtures shared by both native libraries.
Both analysis and synthesis were run at q1/q8. Short calls use 3 interleaved process pairs and 10 calibrated batches per backend process, targeting 25 ms; long calls use the existing individually timed rule. Rust plan/workspace setup, first-call and warmed timings are separate. The JSON retains per-process batch distributions, numerical comparisons and validation; aggregate CPU-frequency and scheduler diagnostics are reported below. Raw per-process telemetry stays in local shard captures. Failure records are retained in `failures.csv` and JSON.
Timing-count audit: 2465 rows contain 30 calibrated batches (10 in each process pair), 51 contain 15 individually timed long-call observations (five in each pair), and 4 mix two 10-batch pairs with one five-call long pair. The raw JSON records each process's actual timing route; no row is padded to a nominal sample count.
The q1/q8 CPU affinity was pinned to the physical core sets shown above. q8 uses the first eight physical cores visible to the process (one CCD on this host). RustSHT and DUCC receive the same fixtures and reusable caller buffers; DUCC's public API performs its own internal setup on each call.

## Coverage audit

The low-degree audit includes 257/257 integer degrees, with no omitted `lmax` values. The required high-resolution list has 24 values; case selection check: `True`. Low-degree even-smooth full/reduced coverage omissions: 0. Across 2520 operation/thread groups, statuses are `{'complete': 2520, 'partial': 0, 'failed': 0, 'missing': 0}`. Failed or partial groups are listed explicitly rather than silently removed.

## Warmed whole-transform summary

Ratios below 1 mean RustSHT was faster. Each table cell reports median ratio, geometric mean ratio, and completed paired case count. Odd-minimum cases are excluded from this even-smooth summary.

| lmax band | operation | mmax | threads | median ratio | geomean ratio | cases |
|---|---|---|---:|---:|---:|---:|
| 1–32 | analysis | full | q1 | 1.172 | 0.992 | 32 |
| 1–32 | analysis | full | q8 | 0.611 | 0.649 | 32 |
| 1–32 | analysis | reduced | q1 | 0.785 | 0.723 | 31 |
| 1–32 | analysis | reduced | q8 | 0.402 | 0.517 | 31 |
| 1–32 | synthesis | full | q1 | 0.946 | 0.838 | 32 |
| 1–32 | synthesis | full | q8 | 0.558 | 0.554 | 32 |
| 1–32 | synthesis | reduced | q1 | 0.735 | 0.660 | 31 |
| 1–32 | synthesis | reduced | q8 | 0.395 | 0.437 | 31 |
| 33–63 | analysis | full | q1 | 1.307 | 1.288 | 31 |
| 33–63 | analysis | full | q8 | 0.734 | 0.727 | 31 |
| 33–63 | analysis | reduced | q1 | 1.006 | 1.008 | 31 |
| 33–63 | analysis | reduced | q8 | 0.528 | 0.492 | 31 |
| 33–63 | synthesis | full | q1 | 1.133 | 1.116 | 31 |
| 33–63 | synthesis | full | q8 | 0.785 | 0.746 | 31 |
| 33–63 | synthesis | reduced | q1 | 0.941 | 0.967 | 31 |
| 33–63 | synthesis | reduced | q8 | 0.449 | 0.457 | 31 |
| 64–128 | analysis | full | q1 | 1.144 | 1.097 | 65 |
| 64–128 | analysis | full | q8 | 0.881 | 0.882 | 65 |
| 64–128 | analysis | reduced | q1 | 1.077 | 1.087 | 65 |
| 64–128 | analysis | reduced | q8 | 0.692 | 0.694 | 65 |
| 64–128 | synthesis | full | q1 | 1.143 | 1.139 | 65 |
| 64–128 | synthesis | full | q8 | 1.000 | 0.986 | 65 |
| 64–128 | synthesis | reduced | q1 | 1.068 | 1.061 | 65 |
| 64–128 | synthesis | reduced | q8 | 0.750 | 0.717 | 65 |
| 129–256 | analysis | full | q1 | 1.374 | 1.372 | 128 |
| 129–256 | analysis | full | q8 | 1.028 | 1.031 | 128 |
| 129–256 | analysis | reduced | q1 | 1.104 | 1.105 | 128 |
| 129–256 | analysis | reduced | q8 | 0.812 | 0.794 | 128 |
| 129–256 | synthesis | full | q1 | 1.115 | 1.120 | 128 |
| 129–256 | synthesis | full | q8 | 1.087 | 1.086 | 128 |
| 129–256 | synthesis | reduced | q1 | 1.092 | 1.089 | 128 |
| 129–256 | synthesis | reduced | q8 | 0.900 | 0.888 | 128 |
| 257–720 | analysis | full | q1 | 1.151 | 1.143 | 10 |
| 257–720 | analysis | full | q8 | 1.084 | 1.108 | 10 |
| 257–720 | analysis | reduced | q1 | 1.076 | 1.081 | 10 |
| 257–720 | analysis | reduced | q8 | 0.952 | 0.963 | 10 |
| 257–720 | synthesis | full | q1 | 1.084 | 1.094 | 10 |
| 257–720 | synthesis | full | q8 | 1.095 | 1.108 | 10 |
| 257–720 | synthesis | reduced | q1 | 1.068 | 1.080 | 10 |
| 257–720 | synthesis | reduced | q8 | 1.040 | 1.033 | 10 |
| 721–1536 | analysis | full | q1 | 1.078 | 1.060 | 8 |
| 721–1536 | analysis | full | q8 | 1.038 | 1.001 | 8 |
| 721–1536 | analysis | reduced | q1 | 1.060 | 1.061 | 8 |
| 721–1536 | analysis | reduced | q8 | 1.031 | 1.037 | 8 |
| 721–1536 | synthesis | full | q1 | 1.024 | 1.012 | 8 |
| 721–1536 | synthesis | full | q8 | 1.047 | 1.020 | 8 |
| 721–1536 | synthesis | reduced | q1 | 1.097 | 1.089 | 8 |
| 721–1536 | synthesis | reduced | q8 | 1.052 | 1.051 | 8 |
| 1537–3072 | analysis | full | q1 | 0.974 | 0.976 | 6 |
| 1537–3072 | analysis | full | q8 | 0.848 | 0.859 | 6 |
| 1537–3072 | analysis | reduced | q1 | 1.080 | 1.073 | 6 |
| 1537–3072 | analysis | reduced | q8 | 1.025 | 1.024 | 6 |
| 1537–3072 | synthesis | full | q1 | 0.956 | 0.963 | 6 |
| 1537–3072 | synthesis | full | q8 | 0.870 | 0.859 | 6 |
| 1537–3072 | synthesis | reduced | q1 | 1.071 | 1.143 | 6 |
| 1537–3072 | synthesis | reduced | q8 | 1.044 | 1.028 | 6 |

### Fast and slow measured cases

| case | operation | q | Rust ms | DUCC ms | ratio | relative L2 |
|---|---|---:|---:|---:|---:|---:|
| **Largest ratios** |  |  |  |  |  |  |
| T214 m214 even_smooth | analysis | q1 | 1.64008 | 0.840898 | 1.950 | 1.623e-14 |
| T212 m212 even_smooth | analysis | q1 | 1.5826 | 0.836299 | 1.892 | 1.610e-14 |
| T217 m217 even_smooth | analysis | q1 | 1.72094 | 0.910418 | 1.890 | 1.576e-14 |
| T216 m216 even_smooth | analysis | q1 | 1.72165 | 0.913583 | 1.884 | 1.668e-14 |
| T213 m213 even_smooth | analysis | q1 | 1.54755 | 0.829006 | 1.867 | 1.604e-14 |
| T210 m210 even_smooth | analysis | q1 | 1.51313 | 0.813801 | 1.859 | 1.395e-14 |
| T198 m198 even_smooth | analysis | q1 | 1.26614 | 0.689481 | 1.836 | 1.424e-14 |
| T209 m209 even_smooth | analysis | q1 | 1.54823 | 0.843897 | 1.835 | 1.471e-14 |
| **Smallest ratios** |  |  |  |  |  |  |
| T63 m63 odd_minimum | analysis | q1 | 0.0642645 | 0.344548 | 0.187 | 3.415e-15 |
| T65 m65 odd_minimum | analysis | q1 | 0.0968556 | 0.383519 | 0.253 | 4.147e-15 |
| T63 m63 odd_minimum | synthesis | q1 | 0.0970764 | 0.35113 | 0.276 | 1.465e-14 |
| T65 m65 odd_minimum | synthesis | q1 | 0.118812 | 0.389779 | 0.305 | 7.952e-15 |
| T1 m1 even_smooth | analysis | q1 | 0.000390896 | 0.00120814 | 0.324 | 3.853e-16 |
| T1 m1 odd_minimum | analysis | q1 | 0.000391608 | 0.0012036 | 0.325 | 2.900e-16 |
| T48 m48 odd_minimum | synthesis | q1 | 0.0581163 | 0.170618 | 0.341 | 5.911e-15 |
| T255 m63 odd_minimum | analysis | q1 | 0.543595 | 1.56871 | 0.347 | 1.631e-14 |

### Longitude-shape sensitivity

- analysis q1: odd/even median runtime factor Rust 1.095×, DUCC 1.199× across 70 matched cases.
- analysis q8: odd/even median runtime factor Rust 1.073×, DUCC 1.051× across 70 matched cases.
- synthesis q1: odd/even median runtime factor Rust 1.128×, DUCC 1.166× across 70 matched cases.
- synthesis q8: odd/even median runtime factor Rust 1.114×, DUCC 1.074× across 70 matched cases.

### Crossover and numerical results

- analysis full q1: 43 observed points below parity and 237 above; lowest-ratio degree 1, highest-ratio degree 214; observed parity crossings T10→T11, T62→T63, T67→T68, T70→T71, T71→T72, T74→T75, T75→T76, T78→T79, T79→T80, T82→T83, T83→T84, T86→T87, T87→T88, T90→T91, T91→T92, T94→T95, T95→T96, T98→T99, T99→T100, T102→T103, T103→T104, T106→T107, T107→T108, T110→T111, T111→T112, T114→T115, T115→T116, T118→T119, T119→T120, T122→T123, T123→T124, T126→T127, T127→T128, T130→T131, T131→T132, T134→T135, T135→T136, T138→T139, T139→T140, T142→T143, T143→T144, T162→T163, T163→T164, T166→T167, T167→T168, T1280→T1536 (sampled points only).
- analysis full q8: 164 observed points below parity and 116 above; lowest-ratio degree 16, highest-ratio degree 320; observed parity crossings T3→T4, T131→T132, T132→T133, T146→T147, T149→T150, T150→T151, T159→T160, T166→T167, T173→T174, T174→T175, T182→T183, T183→T184, T184→T185, T185→T186, T190→T191, T192→T193, T194→T195, T195→T196, T720→T721, T721→T768, T1280→T1536 (sampled points only).
- analysis reduced q1: 48 observed points below parity and 231 above; lowest-ratio degree 2, highest-ratio degree 197; observed parity crossings T27→T28, T29→T30, T35→T36, T39→T40, T41→T42, T42→T43, T43→T44, T45→T46, T51→T52, T53→T54, T55→T56, T79→T80, T81→T82, T109→T110, T111→T112, T134→T135, T135→T136 (sampled points only).
- analysis reduced q8: 263 observed points below parity and 16 above; lowest-ratio degree 29, highest-ratio degree 3; observed parity crossings T3→T4, T511→T512, T512→T513, T513→T640, T720→T721, T721→T768, T1536→T1792, T1792→T2047, T2560→T3072 (sampled points only).
- synthesis full q1: 27 observed points below parity and 253 above; lowest-ratio degree 1, highest-ratio degree 512; observed parity crossings T17→T18, T19→T20, T21→T22, T73→T74, T74→T75, T513→T640, T640→T720, T1280→T1536, T2049→T2560, T2560→T3072 (sampled points only).
- synthesis full q8: 104 observed points below parity and 176 above; lowest-ratio degree 10, highest-ratio degree 320; observed parity crossings T85→T86, T88→T89, T90→T91, T96→T97, T97→T98, T101→T102, T104→T105, T107→T108, T111→T112, T118→T119, T119→T120, T136→T137, T137→T138, T1280→T1536 (sampled points only).
- synthesis reduced q1: 62 observed points below parity and 217 above; lowest-ratio degree 2, highest-ratio degree 159; observed parity crossings T35→T36, T39→T40, T55→T56, T60→T61, T61→T62, T69→T70, T70→T71, T80→T81, T82→T83, T127→T128, T128→T129, T129→T130, T131→T132, T132→T133, T136→T137, T137→T138, T138→T139 (sampled points only).
- synthesis reduced q8: 259 observed points below parity and 20 above; lowest-ratio degree 25, highest-ratio degree 1792; observed parity crossings T258→T300, T1792→T2047, T2047→T2048, T2560→T3072 (sampled points only).
Maximum random-output relative L2 error: `2.536e-13`; maximum absolute error: `1.515e-08`.
T0 remains in the raw results, coverage audit, correctness checks and plots; it is excluded from the degree-band and fastest/slowest throughput summaries.

Largest q8 high-degree process-pair spreads (triplets are per-process medians in ms; all observations are retained):
- T2047 m511 even_smooth analysis ducc: 10.40× across process pairs (13.85, 13.84, 144 ms).
- T2047 m511 even_smooth analysis rustsht: 6.70× across process pairs (97.77, 14.6, 16.62 ms).
Validation ran for 1260 case/thread configurations: finite edge outputs 1260/1260, reused-output reset checks 1260/1260; maximum roundtrip relative L2 `5.563e-13`, selected-basis recovery `6.349e-12`, and adjoint residual `5.374e-12`. Sidecar results include zero, subnormal and extreme-finite checks.
Observed CPU frequency median/p10/p90 `4.467/1.746/4.739` GHz; scheduler wait median/p90 `0.01132/0.2692`; involuntary-switch median/max `64/166295`. No batch samples are removed from the saved distributions.
There were 0 recorded failure/warning events. Inspect `data/ducc-full-transform-parity-current-2026-10-22/failures.csv` and the JSON `failures` array; unsupported cases are labeled separately from worker and numerical-validation errors.

## Principal figures

- [Analysis, full mmax](./data/ducc-full-transform-parity-current-2026-10-22/ratio-analysis-full-mmax.svg)
- [Analysis, reduced mmax](./data/ducc-full-transform-parity-current-2026-10-22/ratio-analysis-reduced-mmax.svg)
- [Synthesis, full mmax](./data/ducc-full-transform-parity-current-2026-10-22/ratio-synthesis-full-mmax.svg)
- [Synthesis, reduced mmax](./data/ducc-full-transform-parity-current-2026-10-22/ratio-synthesis-reduced-mmax.svg)
- [Absolute analysis latency](./data/ducc-full-transform-parity-current-2026-10-22/runtime-analysis.svg)
- [Absolute synthesis latency](./data/ducc-full-transform-parity-current-2026-10-22/runtime-synthesis.svg)
- [Precomputed table versus recurrence](./data/ducc-full-transform-parity-current-2026-10-22/legendre-table-vs-recurrence.svg)
- [Paired FFT versus independent rows](./data/ducc-full-transform-parity-current-2026-10-22/paired-fft-benefit.svg)

The four principal figures plot measured warmed RustSHT/DUCC ratios for even-smooth longitude cases only; q1 and q8 are separate curves and measured points are not smoothed. Odd-minimum cases remain in the data and shape summary. The compact-table/recurrence figure comes from the separate forced-route source sweep documented in [its report](scalar-legendre-table-recurrence-sweep-2026-10-22.md); the paired-FFT figure comes from the same-source paired/independent validation screen in [its report](paired-odd-longitude-fft-production-validation-2026-10-22.md). These controls are not mixed into the current-source Rust/DUCC timing series.

## Reproduction and attribution

The machine-readable artifacts are [`results.csv`](./data/ducc-full-transform-parity-current-2026-10-22/results.csv), [`results.json`](./data/ducc-full-transform-parity-current-2026-10-22/results.json), and [`coverage-audit.json`](./data/ducc-full-transform-parity-current-2026-10-22/coverage-audit.json). Rust plan setup/first-call fields, calibrated warm batch distributions, process pairing, per-case numerical errors, host scheduling/frequency diagnostics, and every requested geometry are retained.

```sh
git checkout 08b938a1d9f89455b9c7a0b8bd81776988ea314a
git -C "$DUCC_SRC" checkout --detach 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b
# Build once with the checked-in native parity CMake project and the recorded toolchain/flags.
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --lmax 0 --lmax 1 --lmax 2 --lmax 3 --lmax 4 --lmax 5 --lmax 6 --lmax 7 --lmax 8 --lmax 9 --lmax 10 --lmax 11 --lmax 12 --lmax 13 --lmax 14 --lmax 15 --lmax 16 --lmax 17 --lmax 18 --lmax 19 --lmax 20 --lmax 21 --lmax 22 --lmax 23 --lmax 24 --lmax 25 --lmax 26 --lmax 27 --lmax 28 --lmax 29 --lmax 30 --lmax 31 --retain-local-host-telemetry --no-report --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-00-t000-031
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --retain-local-host-telemetry --no-report --lmax 32 --lmax 33 --lmax 34 --lmax 35 --lmax 36 --lmax 37 --lmax 38 --lmax 39 --lmax 40 --lmax 41 --lmax 42 --lmax 43 --lmax 44 --lmax 45 --lmax 46 --lmax 47 --lmax 48 --lmax 49 --lmax 50 --lmax 51 --lmax 52 --lmax 53 --lmax 54 --lmax 55 --lmax 56 --lmax 57 --lmax 58 --lmax 59 --lmax 60 --lmax 61 --lmax 62 --lmax 63 --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-01-t032-063
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --retain-local-host-telemetry --no-report --lmax 64 --lmax 65 --lmax 66 --lmax 67 --lmax 68 --lmax 69 --lmax 70 --lmax 71 --lmax 72 --lmax 73 --lmax 74 --lmax 75 --lmax 76 --lmax 77 --lmax 78 --lmax 79 --lmax 80 --lmax 81 --lmax 82 --lmax 83 --lmax 84 --lmax 85 --lmax 86 --lmax 87 --lmax 88 --lmax 89 --lmax 90 --lmax 91 --lmax 92 --lmax 93 --lmax 94 --lmax 95 --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-02-t064-095
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --retain-local-host-telemetry --no-report --lmax 96 --lmax 97 --lmax 98 --lmax 99 --lmax 100 --lmax 101 --lmax 102 --lmax 103 --lmax 104 --lmax 105 --lmax 106 --lmax 107 --lmax 108 --lmax 109 --lmax 110 --lmax 111 --lmax 112 --lmax 113 --lmax 114 --lmax 115 --lmax 116 --lmax 117 --lmax 118 --lmax 119 --lmax 120 --lmax 121 --lmax 122 --lmax 123 --lmax 124 --lmax 125 --lmax 126 --lmax 127 --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-03-t096-127
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --retain-local-host-telemetry --no-report --lmax 128 --lmax 129 --lmax 130 --lmax 131 --lmax 132 --lmax 133 --lmax 134 --lmax 135 --lmax 136 --lmax 137 --lmax 138 --lmax 139 --lmax 140 --lmax 141 --lmax 142 --lmax 143 --lmax 144 --lmax 145 --lmax 146 --lmax 147 --lmax 148 --lmax 149 --lmax 150 --lmax 151 --lmax 152 --lmax 153 --lmax 154 --lmax 155 --lmax 156 --lmax 157 --lmax 158 --lmax 159 --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-04-t128-159
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --retain-local-host-telemetry --no-report --lmax 160 --lmax 161 --lmax 162 --lmax 163 --lmax 164 --lmax 165 --lmax 166 --lmax 167 --lmax 168 --lmax 169 --lmax 170 --lmax 171 --lmax 172 --lmax 173 --lmax 174 --lmax 175 --lmax 176 --lmax 177 --lmax 178 --lmax 179 --lmax 180 --lmax 181 --lmax 182 --lmax 183 --lmax 184 --lmax 185 --lmax 186 --lmax 187 --lmax 188 --lmax 189 --lmax 190 --lmax 191 --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-05-t160-191
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --retain-local-host-telemetry --no-report --lmax 192 --lmax 193 --lmax 194 --lmax 195 --lmax 196 --lmax 197 --lmax 198 --lmax 199 --lmax 200 --lmax 201 --lmax 202 --lmax 203 --lmax 204 --lmax 205 --lmax 206 --lmax 207 --lmax 208 --lmax 209 --lmax 210 --lmax 211 --lmax 212 --lmax 213 --lmax 214 --lmax 215 --lmax 216 --lmax 217 --lmax 218 --lmax 219 --lmax 220 --lmax 221 --lmax 222 --lmax 223 --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-06-t192-223
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --retain-local-host-telemetry --no-report --lmax 224 --lmax 225 --lmax 226 --lmax 227 --lmax 228 --lmax 229 --lmax 230 --lmax 231 --lmax 232 --lmax 233 --lmax 234 --lmax 235 --lmax 236 --lmax 237 --lmax 238 --lmax 239 --lmax 240 --lmax 241 --lmax 242 --lmax 243 --lmax 244 --lmax 245 --lmax 246 --lmax 247 --lmax 248 --lmax 249 --lmax 250 --lmax 251 --lmax 252 --lmax 253 --lmax 254 --lmax 255 --lmax 256 --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-07-t224-256
/usr/bin/python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src <temporary-path> --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --production-source-revision 819d1f447043cadbc85b417b5ef3698139bfe875 --sweep-id rust-819d-ducc-42ce-full-2026-10-22 --dense-sweep --retain-local-host-telemetry --no-report --lmax 257 --lmax 258 --lmax 300 --lmax 320 --lmax 384 --lmax 511 --lmax 512 --lmax 513 --lmax 640 --lmax 720 --lmax 721 --lmax 768 --lmax 896 --lmax 1023 --lmax 1024 --lmax 1025 --lmax 1280 --lmax 1536 --lmax 1792 --lmax 2047 --lmax 2048 --lmax 2049 --lmax 2560 --lmax 3072 --results-dir reports/data/ducc-full-transform-parity-current-2026-10-22/shards/chunk-08-t257-3072
```

The October 20 sweep is retained as a historical illustration only. It used a different Rust source/toolchain combination; its points are not merged into this dataset or used to compute the ratios above.
