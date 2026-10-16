# M19 current-head FFT performance baseline

Date: 2026-10-16. Source revision: `cd5748bb0bf5a93d43143a0bb0b9119e4d672983`.

## Status and method

The M18 commit `3c02794` contains the production source used at the session-start
head; later commits through `cd5748b` changed results and documentation only. The RustSHT-FFT executable used here has SHA-256
`a97674f383732a07ee860887f5407c12659874ee70eb8d9ab0aa058d968dca17`. The M19 RustFFT and native DUCC captures use that executable and the exact
complex and real SHT target lengths and row counts. Each comparison used six
alternating process pairs, seven warmed samples per operation, one transform
thread and one CPU core on an x86-64 CPU. RustSHT-FFT used Rust 1.99.0 stable,
a portable target and runtime AVX2 dispatch. RustFFT was 6.4.1. DUCC was pinned
to `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; the native build used g++ 15.2,
`-O3 -DNDEBUG -march=native`, and DUCC planner vectorization. These measurements are diagnostic. A separate RustSHT SHT benchmark ran on
8 pinned CPU cores during both captures. The FFT processes stayed pinned to one CPU core, and
the collectors recorded load averages, but the shared host does not establish
quiet-host timing. The M18 RustFFT matrix remains the primary RustFFT baseline
because it measured the same source and binary with tighter paired intervals. The M19 RustFFT run is a current-head repeat; the M19 native DUCC results are
provisional. CPU-frequency telemetry was retained only in local diagnostics. The policy was not captured inside each process
record, so this is host context rather than a per-sample frequency measurement. The M19 matrix has 34 direction/length cells per reference. It measures 1D
public FFT operations shaped as serial SHT row batches, not complete SHT calls. The benchmark records plan construction, workspace construction and bytes,
and steady operation time. A separate M19 diagnostic on the exact `cd5748b`
worktree records retained plan payload, route traces, and one first-call
observation per direction and target length. Portable DUCC, FFTW and Admiral
were measured in six alternating process pairs after the RustFFT and native
DUCC captures. No separate SHT benchmark process was active during those
three captures. Load-average telemetry was retained only in local diagnostics. Native DUCC remains
provisional because a separate SHT benchmark overlapped that capture.

## RustFFT comparison at SHT row counts

Each time cell is `RustSHT-FFT / RustFFT` in microseconds per transform. The
ratio is `RustFFT time / RustSHT-FFT time`; values below one mean RustSHT-FFT is
slower. Intervals are 95% bootstrap intervals over six paired process ratios. The values below are from the M18 six-pair matrix, whose source and executable
match the M19 current head.

| Operation | N | Rows | RustSHT-FFT / RustFFT (µs) | RustFFT / RustSHT-FFT |
| --- | ---: | ---: | ---: | ---: |
| C2C forward | 726 | 361 | 6.45 / 1.57 | 0.244 (0.237–0.252) |
| C2C inverse | 726 | 361 | 7.39 / 1.76 | 0.239 (0.232–0.251) |
| C2C forward | 1,442 | 721 | 55.60 / 13.32 | 0.241 (0.237–0.250) |
| C2C inverse | 1,442 | 721 | 61.24 / 13.46 | 0.221 (0.219–0.224) |
| C2C forward | 1,452 | 721 | 13.22 / 3.37 | 0.255 (0.251–0.257) |
| C2C inverse | 1,452 | 721 | 15.03 / 3.73 | 0.248 (0.246–0.249) |
| C2C forward | 2,904 | 721 | 27.30 / 7.28 | 0.268 (0.260–0.272) |
| C2C inverse | 2,904 | 721 | 30.93 / 7.87 | 0.254 (0.251–0.260) |
| R2C | 723 | 361 | 11.18 / 4.72 | 0.421 (0.408–0.427) |
| C2R | 723 | 361 | 11.92 / 4.78 | 0.402 (0.388–0.407) |
| R2C | 2,049 | 1,024 | 32.01 / 37.06 | 1.158 (1.151–1.163) |
| C2R | 2,049 | 1,024 | 35.50 / 37.23 | 1.049 (1.013–1.055) |
| R2C | 4,097 | 2,048 | 103.48 / 61.61 | 0.595 (0.590–0.601) |
| C2R | 4,097 | 2,048 | 118.66 / 61.16 | 0.515 (0.511–0.520) |

The largest complex absolute cost is N=1,442 at 55–61 µs per row. Its
RustSHT-FFT operation is about 4.1–4.5 times slower than RustFFT. N=726 is
about 4.1–4.2 times slower; N=1,452 is 3.9–4.0 times slower; and N=2,904 is
3.7–3.9 times slower. Real N=723 is 2.4–2.5 times slower and N=4,097 is
1.7–1.9 times slower. N=2,049 remains a regression control and is competitive
with RustFFT in both directions.

## Native DUCC diagnostic comparison

Ratios are `DUCC time / RustSHT-FFT time`; values above one favor RustSHT-FFT. All results below are provisional because the host was active during capture.

| Operation | N | Rows | RustSHT-FFT / DUCC (µs) | DUCC / RustSHT-FFT |
| --- | ---: | ---: | ---: | ---: |
| C2C forward | 726 | 361 | 6.47 / 2.57 | 0.395 (0.386–0.397) |
| C2C inverse | 726 | 361 | 7.42 / 2.66 | 0.359 (0.354–0.483) |
| C2C forward | 1,442 | 721 | 55.12 / 17.94 | 0.327 (0.324–0.331) |
| C2C inverse | 1,442 | 721 | 61.15 / 16.34 | 0.267 (0.265–0.275) |
| C2C forward | 1,452 | 721 | 13.30 / 3.47 | 0.263 (0.246–0.277) |
| C2C inverse | 1,452 | 721 | 15.03 / 3.77 | 0.251 (0.242–0.265) |
| C2C forward | 2,904 | 721 | 27.98 / 8.06 | 0.290 (0.269–0.306) |
| C2C inverse | 2,904 | 721 | 30.76 / 8.61 | 0.280 (0.271–0.296) |
| R2C | 723 | 361 | 11.11 / 11.87 | 1.071 (1.047–1.089) |
| C2R | 723 | 361 | 11.91 / 11.84 | 0.997 (0.921–1.203) |
| R2C | 2,049 | 1,024 | 32.20 / 38.79 | 1.207 (1.183–1.215) |
| C2R | 2,049 | 1,024 | 35.72 / 39.48 | 1.103 (1.082–1.309) |
| R2C | 4,097 | 2,048 | 105.62 / 76.76 | 0.726 (0.698–0.767) |
| C2R | 4,097 | 2,048 | 118.45 / 75.34 | 0.635 (0.623–0.639) |

The DUCC comparisons identify the same complex priorities. The R2C N=723 and
both N=2,049 operations are at or above parity with native DUCC in this
diagnostic capture. Native DUCC is faster for N=4,097 in both directions.

## Current-head cross-library matrix

The table gives paired `reference time / RustSHT-FFT time` ratios; values below
one mean RustSHT-FFT is slower. Each interval is a bootstrap 95% interval over
six process pairs. RustFFT ratios use M18, which measured the same source and
executable with tighter intervals. The other ratios use M19 captures.

| Operation | N | RustFFT | DUCC portable | DUCC native* | FFTW | Admiral |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| C2C forward | 726 | 0.244 (0.237–0.252) | 0.551 (0.548–0.560) | 0.395 (0.386–0.397) | 0.374 (0.362–0.378) | 0.342 (0.337–0.347) |
| C2C inverse | 726 | 0.239 (0.232–0.251) | 0.530 (0.528–0.538) | 0.359 (0.354–0.483) | 0.352 (0.350–0.366) | 0.298 (0.294–0.302) |
| C2C forward | 1,442 | 0.241 (0.237–0.250) | 0.455 (0.448–0.463) | 0.327 (0.324–0.331) | 0.451 (0.441–0.466) | 0.375 (0.366–0.391) |
| C2C inverse | 1,442 | 0.221 (0.219–0.224) | 0.413 (0.409–0.415) | 0.267 (0.265–0.275) | 0.412 (0.397–0.422) | 0.333 (0.331–0.342) |
| C2C forward | 1,452 | 0.255 (0.251–0.257) | 0.581 (0.566–0.585) | 0.263 (0.246–0.277) | 0.458 (0.443–0.481) | 0.272 (0.266–0.277) |
| C2C inverse | 1,452 | 0.248 (0.246–0.249) | 0.559 (0.557–0.563) | 0.251 (0.242–0.265) | 0.432 (0.424–0.439) | 0.238 (0.236–0.243) |
| C2C forward | 2,904 | 0.268 (0.260–0.272) | 0.615 (0.610–0.624) | 0.290 (0.269–0.306) | 0.559 (0.541–0.568) | 0.305 (0.299–0.313) |
| C2C inverse | 2,904 | 0.254 (0.251–0.260) | 0.566 (0.560–0.572) | 0.280 (0.271–0.296) | 0.507 (0.400–0.515) | 0.264 (0.261–0.267) |
| R2C | 723 | 0.421 (0.408–0.427) | 1.522 (1.491–1.967) | 1.071 (1.047–1.089) | 0.801 (0.412–0.808) | 0.954 (0.947–0.957) |
| C2R | 723 | 0.402 (0.388–0.407) | 1.435 (1.324–2.068) | 0.997 (0.921–1.203) | 0.761 (0.749–0.770) | 0.901 (0.895–0.907) |
| R2C | 2,049 | 1.158 (1.151–1.163) | 1.569 (1.561–1.620) | 1.207 (1.183–1.215) | 1.347 (1.060–1.361) | 0.894 (0.881–0.900) |
| C2R | 2,049 | 1.049 (1.013–1.055) | 1.456 (1.442–1.493) | 1.103 (1.082–1.309) | 1.204 (0.936–1.222) | 0.824 (0.815–0.852) |
| R2C | 4,097 | 0.595 (0.590–0.601) | 1.019 (1.014–1.033) | 0.726 (0.698–0.767) | 0.621 (0.581–0.631) | 0.613 (0.609–0.627) |
| C2R | 4,097 | 0.515 (0.511–0.520) | 0.894 (0.884–0.904) | 0.635 (0.623–0.639) | 0.515 (0.442–0.525) | 0.557 (0.543–0.569) |

*Native DUCC results are provisional because a separate SHT benchmark overlapped
that capture. Portable DUCC is a separate scalar-planner build. All comparisons
use one transform thread and include public real packing and scaling. The large complex gaps persist across references. N=1,442 has the highest
absolute C2C cost; N=726, 1,452 and 2,904 also remain several times slower
than each library. Real N=723 beats portable DUCC in this capture but trails
RustFFT, FFTW and Admiral. N=4,097 is near portable-DUCC parity for R2C and
slower for C2R, while the other references remain faster. N=2,049 remains
competitive with RustFFT and DUCC but slower than Admiral in this capture.

## Confirmed routes and priorities

Factorization follows the pinned DUCC order in `src/complex.rs`. At the M19
head, the complex target plans use these stages:

| N | Factorization and notable pass | M19 execution |
| ---: | --- | --- |
| 726 | `[2, 3, 11, 11]`; radix 11 at `(l1, ido) = (6, 11)` then `(66, 1)` | First radix-2 prefix uses AVX2. Both radix-11 passes use the scalar path. |
| 1,452 | `[4, 3, 11, 11]`; radix 11 at `(12, 11)` then `(132, 1)` | First radix-4 prefix uses AVX2. Both radix-11 passes use the scalar path. |
| 2,904 | `[8, 3, 11, 11]`; radix 11 at `(24, 11)` then `(264, 1)` | First radix-8 prefix uses AVX2. Both radix-11 passes use the scalar path. |
| 1,442 | `[2, 7, 103]`; terminal radix 103 has `ido = 1` | First radix-2 prefix uses AVX2. The radix-7 pass and cached-phase general-prime radix-103 pass are scalar. |
| 682 control | `[2, 11, 31]`; radix 11 has `(l1, ido) = (2, 31)` | Existing AVX2 radix-11 and radix-31 paths are selected. |

The real N=723 and N=4,097 plans use the bounded radix-241 Rader route with a
240-point complex convolution. Both retain Bluestein fallbacks for inputs
outside the Rader safety bound. N=4,097 has a 484-point fallback convolution. The route is documented in the [M7 N=4,097 report](m7-rader-4097-report.md). The remaining-work ranking is:

1. C2C N=1,442: largest absolute complex cost and largest relative gap; the
   terminal radix-103 direct pass is not vectorized.
2. C2C N=2,904: second-largest complex cost and a 3.7–3.9× RustFFT gap; its
   first radix-11 pass has `ido = 11`.
3. C2C N=1,452 and N=726: smaller absolute cost but gaps near four times;
   both also contain a radix-11 `ido = 11` pass.
4. Real N=723 and N=4,097: preserve their bounded Rader paths and investigate
   the full cost around convolution and reconstruction.
5. Preserve N=2,049 and N=682 as controls; both are comparatively strong.

## Supporting data and remaining measurements

- [M18 RustFFT six-pair raw output](../benches/results/m18-cross-library-sht-targets/m18-rustfft-six-pairs.jsonl)
  and [summary](../benches/results/m18-cross-library-sht-targets/rustfft-summary.csv).
- [M19 RustFFT current-head repeat](../benches/results/m19-current-head-cross-library/m19-rustfft-six-pairs.jsonl)
  and [summary](../benches/results/m19-current-head-cross-library/m19-rustfft-summary.csv).
- [M19 native DUCC six-pair raw output](../benches/results/m19-current-head-cross-library/m19-ducc-native-six-pairs.jsonl)
  and [summary](../benches/results/m19-current-head-cross-library/m19-ducc-native-summary.csv).
- [M19 portable DUCC six-pair raw output](../benches/results/m19-current-head-cross-library/m19-ducc-portable-six-pairs.jsonl)
  and [summary](../benches/results/m19-current-head-cross-library/m19-ducc-portable-summary.csv).
- M19 [FFTW raw output](../benches/results/m19-current-head-cross-library/m19-fftw-six-pairs.jsonl)
  and [summary](../benches/results/m19-current-head-cross-library/m19-fftw-summary.csv),
  and [Admiral raw output](../benches/results/m19-current-head-cross-library/m19-admiral-six-pairs.jsonl)
  and [summary](../benches/results/m19-current-head-cross-library/m19-admiral-summary.csv).
- Per-reference [M19 configurations](../benches/results/m19-current-head-cross-library/)
  record commands, binary hashes, affinity, toolchain and workload. The portable DUCC, FFTW and Admiral configurations and raw process records are
retained alongside the summaries. No source changes or optimization
acceptance are based on this baseline.

## Plan, workspace and first-call diagnostics

The current-head route and memory test reports heap payload retained by each
plan and caller-owned workspace bytes. The route test ran from a detached
worktree at `cd5748b`; one test passed. The measurement counts allocated
elements and excludes allocator metadata.

| Plan | N | Retained plan payload (bytes) | Workspace (bytes) |
| --- | ---: | ---: | ---: |
| C2C | 146 | 8,948 | 4,672 |
| C2C | 150 | 7,440 | 2,560 |
| C2C | 256 | 9,488 | 4,352 |
| C2C | 300 | 12,240 | 4,960 |
| C2C | 512 | 18,256 | 8,448 |
| C2C | 722 | 30,246 | 12,160 |
| C2C | 726 | 32,112 | 11,968 |
| C2C | 1,024 | 43,072 | 16,640 |
| C2C | 1,442 | 65,301 | 26,368 |
| C2C | 1,452 | 55,344 | 23,584 |
| C2C | 2,904 | 101,808 | 46,816 |
| Real | 257 | 60,488 | 46,032 |
| Real | 723 | 69,312 | 53,536 |
| Real | 1,025 | 34,489 | 28,944 |
| Real | 1,443 | 43,290 | 40,976 |
| Real | 2,049 | 190,721 | 159,856 |
| Real | 4,097 | 150,353 | 133,824 |

The [first-call capture](../benches/results/m19-current-head-cross-library/m19-first-call-diagnostics.jsonl)
and [summary](../benches/results/m19-current-head-cross-library/m19-first-call-summary.csv)
contain a first timed public operation and a warm-call median for every
direction and length in the exact SHT target matrix. Each row is one fresh
process and is a provisional observation, not a six-pair estimate. The complete [route and memory trace](../benches/results/m19-current-head-cross-library/m19-route-memory-diagnostics.txt)
records the factorization and actual SIMD dispatch for all diagnostic lengths. It confirms that M19's radix-11 `ido = 11` passes at N=726, 1,452 and 2,904
use the scalar route, while AVX2 is selected for each eligible first pass.
