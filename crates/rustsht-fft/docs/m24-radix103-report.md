# M24: N=1442 radix-103 pass

The AVX2 radix-103 pass reduced the measured N=1442 complex transform time by about 45% forward and 47% inverse relative to the M21 build. The exact 95% paired bootstrap intervals exclude no speedup. Four non-target cells were 1–2% slower; their absolute per-transform differences were 14–257 ns.

## Implementation

Commit `9a85e38` adds an f64 AVX2 general-prime pass for the N=1442 factorization (`radix=103`, `l1=14`, `ido=1`). The kernel pairs adjacent positive output frequencies in AVX2 lanes and computes their paired negative frequencies. Dispatch selects it only for this plan and only when AVX2 is available. f32 and unsupported CPUs use the existing general-prime pass. Inverse transforms use the new kernel only when deferred normalization passes the existing range-safety check.

The kernel keeps two 51-element temporary arrays in `MaybeUninit` storage. It writes every element before reading either array. This avoids a zero-fill and copy for each of the 14 outer groups. The code diff hash is `1f9d4a3655867915c740bd41baf4a09760c5f62f70cc7eb1fae7b0480bcf1531`.

## Measurement method

The candidate was built from M23 head `cc866b9` plus the M24 source patch. The reference executable was built from M21 commit `c48e653978079da8d126757a9362c27b4b64e2c7`; the `src/` tree at M23 matched M21 before the M24 patch. Candidate and reference binary SHA-256 values are recorded in the capture configuration.

The release benchmark ran on an AMD Ryzen 9 5950X, pinned to logical CPU 30, with Rust 1.99.0 and portable x86_64 target settings. Runtime AVX2 detection selected the optimized path. The CPU frequency driver was `amd-pstate-epp`; the governor was `performance`, with a 1.746–5.086 GHz policy range when checked after capture. The runner did not sample instantaneous CPU frequency during each process.

The matrix contains 34 operation/length cells: forward and inverse complex transforms at 11 lengths, and real-to-complex and complex-to-real transforms at six lengths. Each cell uses the scalar or spin-1 SHT row count recorded in `benches/rustfft.rs`. Six process pairs alternated candidate-first and reference-first order. Each process warmed and timed seven samples per operation. The summary reports the batch operation time divided by row count, in nanoseconds per transform; it also records plan time, workspace time, and workspace bytes. The paired speedup is the median of six reference/candidate ratios. Its interval is the exact 2.5th–97.5th percentile interval over all 46,656 bootstrap resamples of those six ratios.

All 12 benchmark processes exited successfully. Their recorded one-minute load averages ranged from 1.60 to 1.71; five-minute averages ranged from 2.68 to 3.00. Periodic process checks found no separate SHT or FFT workload during the capture. The runner records process IDs and load averages before, during, and after each run, and fsyncs each JSONL record.

## N=1442 results

| Direction | Candidate median | M21 median | M21 / candidate | Exact 95% paired interval |
|---|---:|---:|---:|---:|
| Forward | 30.03 µs | 55.22 µs | 1.834× | [1.791, 1.852] |
| Inverse | 32.00 µs | 60.43 µs | 1.888× | [1.876, 1.902] |

Both operations used 721 SHT rows. The candidate used 26,368 workspace bytes; M21 used the same amount. Candidate plan time was 26.6–27.2 µs and workspace creation time was about 0.32 µs. The corresponding M21 medians were 26.6–26.8 µs and 0.31 µs.

## Regression controls

The full 34-cell summary is in `benches/results/m24-radix103/m24-radix103-no-scratch-init-clean-summary.csv`. Selected controls are below. A ratio below 1 means the candidate was slower.

| Workload | M21 / candidate speedup | Exact 95% paired interval |
|---|---:|---:|
| C2C N=1024 forward | 0.988× | [0.980, 1.013] |
| C2C N=1024 inverse | 0.999× | [0.997, 1.014] |
| C2C N=1452 forward | 1.033× | [1.000, 1.063] |
| C2C N=1452 inverse | 1.010× | [0.989, 1.034] |
| C2C N=2904 forward | 1.032× | [1.004, 1.060] |
| C2C N=2904 inverse | 1.010× | [0.986, 1.035] |
| R2C N=2049 | 0.992× | [0.976, 1.010] |
| C2R N=2049 | 1.018× | [1.010, 1.043] |

Four cells had paired intervals below 1: C2C forward N=150 (0.986×), C2C forward N=300 (0.983×), C2R N=257 (0.980×), and C2R N=1443 (0.989×). Their candidate medians were 14–257 ns above M21 per transform. The added N=1442 eligibility condition in runtime dispatch could contribute to small changes outside the target length, but the capture does not isolate that branch from other code-generation effects. The largest non-target slowdown by paired median ratio was 2.04%.

The earlier M24 captures remain in the results directory. `m24-radix103-no-scratch-init-six-pairs.jsonl` is diagnostic only because a separate SHT process overlapped it. The uncontended confirmation is `m24-radix103-no-scratch-init-clean-six-pairs.jsonl`; its config, deterministic summarizer, and runner are stored alongside it.

## Numerical and code review

The focused AVX2 test compares all radix-103 outputs against direct DFT sums for `l1 = 1, 14, 15`, in both directions, and checks guard elements. The route test verifies N=1442 f64 dispatch, the high-amplitude inverse fallback, and the f32 general-prime fallback. The existing range-safety fallback test returns finite high-amplitude inverse results.

A read-only review of the M24 source found no concrete issue in the radix-103 equations, scaling, bounds, initialization, dispatch, or fallbacks. The reviewer requested an uncontended performance capture; the matrix above supplies that evidence.

The benchmark host supports AVX2. The f32 fallback and high-amplitude inverse fallback were tested on this host; no non-AVX2 hardware run was available.

The following checks passed on the M24 source:

- `cargo test --workspace --all-features` — 105 tests passed.
- `cargo test --workspace --all-features --release` — 105 tests passed.
- `cargo test -p rustsht-fft --all-features radix103` — focused radix-103 tests passed.
- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

This comparison measures M24 against M21. It does not repeat the cross-library DUCC, RustFFT, FFTW, or Admiral measurements. `m17-cross-library-report.md` and `m19-current-head-performance-report.md` contain earlier cross-library comparisons, not M24 head-to-head results.
