# FFT-friendly longitude-count selection

## Finding

`GridSpec::fft_friendly_nphi(mmax)` returns the smallest longitude count at or above `2*mmax + 1` whose prime factors are 2, 3, 5 or 7. The function is an explicit suggestion: constructing a `GridSpec` with a supplied `nphi` keeps that count unchanged. The suggestion does not include oversampling for nonlinear products or a fixed external grid.

The helper and API note are in commit `8feac1c`; the optional Python benchmark argument is in `7be20d2`. Both commits are on `main`. The report and raw samples are committed separately.

The selected count reduced measured RustSHT GL transform time for every tested full-order bandlimit except T72, where the current default was already the selected count. One-thread improvements ranged from 6% to 28% for T63–T1023 scalar and spin-1 STANDARD operations. Eight-thread native runs showed larger gains at T127 and above, with some higher-bandlimit cases affected by host contention. Reduced-`mmax` runs also improved when the suggested count was derived from `mmax` rather than `lmax`.

## Source comparison and experiment plan

RustSHT `GridSpec::new` accepts explicit `ntheta` and `nphi`. `RealFftPlan` uses a complex FFT of length `nphi/2` with real packing for even counts, and a full complex FFT of length `nphi` for odd counts. RustFFT 6.4.1 plans small butterflies, mixed-radix and Good–Thomas decompositions, and Rader or Bluestein algorithms for other lengths. A short-factor count can therefore change both the FFT decomposition and the longitude work.

SHTns revision [`4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`](https://github.com/shtns/shtns/tree/4e69ceb3fc5f19475d55b03ff0fdec210dc3075e) uses `fft_int` in `shtns_set_grid_auto` to choose an even size with factors up to seven. With its default `nl_order=1`, the requested longitude size is `2*MMAX+1`. `sht_auto` selects a Gaussian grid; `sht_gauss` allows on-the-fly or matrix algorithms; `sht_gauss_fly` recomputes Legendre values; `sht_quick_init` reduces initialization work. The source also contains initialization-time kernel selection. RustSHT uses the factorization idea without a timing autotuner or an implicit grid change. The RustFFT measurements favored the smallest smooth count; at T72 this is odd `nphi=147`, which is also the current default. SHTns rounds that case to even `nphi=150`; RustSHT's T72 measurements did not show an operation-independent reason to add those samples. SHTns timings were not collected for this study.

The screen compared three counts: the minimum odd count `2*mmax+1`, the existing `m1b_bench` default `2*lmax+3`, and the smallest 2/3/5/7-smooth count at or above the minimum. T720 also compared `nphi=1470`, the larger smooth candidate from the initial screen, with the refined candidate `1458`. The reduced-order screen used `mmax=floor(lmax/2)` and compared the minimum odd count, the smooth suggestion and the current full-`lmax` benchmark default.

## Measurement method

The Rust baseline was commit `a13aa53fd2532010766efcfdd9c41b3698944a20`. Measurements used f64 GL grids with `ntheta=lmax+1`, `phi0=0.37`, scalar and spin-1 STANDARD synthesis and analysis, and `m1b_bench` release builds. Full-order runs used `mmax=lmax`; reduced runs used the values listed below. The benchmark generated the same deterministic coefficients for each matching case, synthesized the map before analysis, reused the plan and workspace, and recorded five warm timings per process.

Each native case ran in three separate processes in randomized order. The table reports the median of each process's five warm samples, then the median across process medians. T720 `nphi=1443` appeared in both the initial screen and refinement, so its table baseline uses six process medians; `nphi=1458` uses three. One-thread processes were pinned to CPU 0; eight-thread processes were pinned to physical cores 0–7. These are separate logical CPUs on the AMD Ryzen 9 5950X. Peak resident memory came from `/usr/bin/time`. The host was shared: the first native screen's load average changed from 3.51 to 4.86, and some eight-thread process ranges were wide. The raw logs preserve all samples and RSS values.

The machine ran Linux `7.0.14-20-pve`, AMD Ryzen 9 5950X (16 cores, 32 hardware threads), AVX2/FMA, without AVX-512. Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0`, LLVM `23.1.1`. Rust used the standard release profile (`opt-level=3`, 16 codegen units), with no `RUSTFLAGS`, LTO override or `target-cpu=native`; Pulp retained runtime dispatch. The native command was:

```sh
cargo build --release --example m1b_bench
taskset -c 0 target/release/examples/m1b_bench 1 511 1 both GL STANDARD 512 1024 511 requested-only
taskset -c 0-7 target/release/examples/m1b_bench 1 511 8 both GL STANDARD 512 1024 511 requested-only
```

`m1b_bench` accepts explicit `ntheta`, `nphi` and `mmax` arguments. The Python comparison used the updated `examples/python_perf_baseline.py`, installed `ducc0==0.41.0`, and the same `nphi` for both plans. Its source reference was DUCC revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. DUCC source was not rebuilt for this study. Python comparisons at one thread used two processes for T127/T511 and one process for T720; each process measured five warm samples. Eight-thread Python timings varied substantially and are excluded from threshold decisions.

## Full-order native results

Each cell gives default-to-suggested time in milliseconds as `1 thread / 8 threads`. Full-order grid dimensions were `(lmax+1) x nphi`. The percent columns are relative reductions from the default.

| Bandlimit | `mmax` | Default → suggested `nphi` | Scalar synthesis | Scalar analysis | Spin-1 synthesis | Spin-1 analysis |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T63 | 63 | 129 → 128 | 0.158/0.107 → 0.114/0.079 (28%/26%) | 0.160/0.102 → 0.129/0.061 (19%/40%) | 0.277/0.216 → 0.191/0.150 (31%/31%) | 0.300/0.179 → 0.232/0.107 (23%/40%) |
| T72 | 72 | 147 → 147 | 0.169/0.078 → 0.169/0.078 (0%/0%) | 0.140/0.085 → 0.140/0.085 (0%/0%) | 0.298/0.208 → 0.298/0.208 (0%/0%) | 0.328/0.137 → 0.328/0.137 (0%/0%) |
| T127 | 127 | 257 → 256 | 0.889/0.329 → 0.771/0.205 (13%/38%) | 0.676/0.296 → 0.590/0.233 (13%/21%) | 2.615/0.959 → 2.356/0.664 (10%/31%) | 1.107/0.551 → 0.927/0.357 (16%/35%) |
| T255 | 255 | 513 → 512 | 5.936/1.586 → 5.444/1.092 (8%/31%) | 4.150/1.644 → 3.825/1.046 (8%/36%) | 12.445/4.476 → 11.687/3.502 (6%/22%) | 7.095/2.657 → 6.400/1.965 (10%/26%) |
| T511 | 511 | 1025 → 1024 | 44.078/12.244 → 41.583/9.622 (6%/21%) | 23.795/7.437 → 21.881/5.318 (8%/29%) | 58.913/20.834 → 53.879/15.561 (9%/25%) | 39.281/12.377 → 35.327/8.162 (10%/34%) |
| T720 | 720 | 1443 → 1458 | 106.348/34.991 → 98.199/26.409 (8%/25%) | 64.114/20.179 → 57.294/12.770 (11%/37%) | 136.422/52.099 → 120.351/36.913 (12%/29%) | 104.921/34.718 → 91.222/20.367 (13%/41%) |
| T1023 | 1023 | 2049 → 2048 | 344.398/112.361 → 310.258/78.555 (10%/30%) | 180.305/61.583 → 150.518/29.935 (17%/51%) | 425.088/154.300 → 359.940/89.620 (15%/42%) | 295.501/110.317 → 232.884/47.315 (21%/57%) |

The q1 samples were stable across process repetitions. Eight-thread gains were larger in many cases, although T255 and several T1023 cases had outlying process ranges. Those rows should be treated as evidence of direction and scale, not as precise speedup factors. T720's selected size is 15 samples larger than the current default; it reduced the q1 measured times by 8–13% and q8 times by 25–41% in this screen.

## Reduced-order results

The suggestion uses `mmax`, so reduced-order transforms can use fewer longitude samples. Here `mmax=floor(lmax/2)`, `ntheta=lmax+1`; the comparison default is still `2*lmax+3`. Entries report the percent reduction for one and eight threads.

| `lmax` / `mmax` | Default → suggested `nphi` (minimum odd) | Scalar synthesis | Scalar analysis | Spin-1 synthesis | Spin-1 analysis |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 72 / 36 | 147 → 75 (73) | 13%/15% | 7%/22% | 13%/20% | 11%/17% |
| 127 / 63 | 257 → 128 (127) | 35%/52% | 26%/58% | 38%/53% | 27%/55% |
| 255 / 127 | 513 → 256 (255) | 13%/39% | 14%/38% | 11%/32% | 17%/49% |
| 511 / 255 | 1025 → 512 (511) | 10%/31% | 14%/39% | 12%/36% | 18%/51% |
| 720 / 360 | 1443 → 729 (721) | 11%/28% | 17%/49% | 16%/37% | 21%/55% |

These timings compare different map shapes. The API returns a suggested dimension; applications that require a particular longitude grid should continue using that dimension.

## DUCC comparison and numerical results

The Python benchmark used `phi0=0.37`, matched inputs and output dimensions, RustSHT plan methods with reusable workspaces, and installed DUCC 0.41.0. At T127 and T511, the table reports the median across two process medians; each process used five warm timings. At T720, one process used five warm timings. Times are milliseconds as synthesis/analysis.

| Bandlimit / spin | `nphi` default → suggested | RustSHT Python plan | DUCC 0.41.0 |
| --- | ---: | ---: | ---: |
| T127 scalar | 257 → 256 | 0.883/0.617 → 0.762/0.519 | 0.724/0.696 → 0.312/0.272 |
| T127 spin 1 | 257 → 256 | 2.545/1.117 → 2.320/0.935 | 1.534/1.353 → 0.735/0.548 |
| T511 scalar | 1025 → 1024 | 45.216/24.148 → 42.630/22.059 | 10.212/6.445 → 9.665/5.995 |
| T511 spin 1 | 1025 → 1024 | 59.373/40.214 → 54.083/35.985 | 22.444/13.136 → 20.929/12.011 |
| T720 scalar | 1443 → 1458 | 109.555/67.047 → 99.758/57.796 | 24.949/15.563 → 23.125/14.740 |
| T720 spin 1 | 1443 → 1458 | 137.708/106.768 → 123.754/91.977 | 52.119/30.636 → 44.781/30.149 |

For the suggested counts, the maximum RustSHT-versus-DUCC errors across the tested T127, T511 and T720 cases were:

| Operation | Maximum absolute error | Maximum RMS error | Maximum relative L2 error |
| --- | ---: | ---: | ---: |
| Scalar synthesis | `7.815e-14` | `1.423e-15` | `8.544e-14` |
| Scalar analysis | `1.541e-14` | `2.331e-16` | `5.987e-14` |
| Spin-1 synthesis | `7.455e-14` | `1.564e-15` | `9.123e-14` |
| Spin-1 analysis | `1.630e-14` | `2.203e-16` | `6.003e-14` |

The outputs at each count were compared with DUCC on the same count. These comparisons establish agreement for the sampled coefficients and maps; they are not independent analytic validation. Existing tests cover analytic fields, odd and even longitude counts, adjoints, GL/CC/F1 and f32. No numerical kernel changed in this milestone.

## Memory, setup and rejected alternatives

The suggested sizes changed native peak RSS by less than 1 MiB at the tested fixed `mmax`, except for variations from thread workspaces and the T255 Legendre table. At T720, `nphi=1458` raises the longitude sample count by about 1.0% over 1443; q1 peak RSS was 51.5 MiB versus 51.4 MiB at the default, and q8 was 77.4 MiB versus 76.6 MiB. The q1 workspace allocation median at the suggested count was 6.502 ms and plan construction was 0.038 ms. At T1023, the q1 workspace allocation median was 13.434 ms, plan construction was 0.056 ms, and peak RSS was 99.9 MiB; q8 peak RSS was 148.9 MiB. The complete per-case setup and memory samples are in the native logs.

The first T720 screen tested 1470 before checking 1458. The refinement showed that 1458 is also smooth and uses fewer samples; 1458 was kept as the helper result. The smallest valid odd count was slower at most measured tiers, especially when it led to a difficult full-length FFT. Selecting the minimum count without considering its factors was rejected. Always forcing an even count, as SHTns does, was also rejected: RustFFT handles smooth odd sizes directly, and RustSHT's T72 current count 147 performed as well as the measured smooth alternatives across most operations. No per-operation timing table or runtime autotuner was added.

The selected count is a useful suggestion for callers that can choose their map grid. It is not a replacement for a user-specified shape and does not set an oversampling factor. The transform plan, workspace, recurrence policy, FFT adapter and numerical conventions are unchanged.

## Reproduction files and checks

Raw process output is committed in [`data/fft-friendly-nphi-2026-10-14/`](data/fft-friendly-nphi-2026-10-14/): native full-order cases, reduced-`mmax` cases, the T720 candidate refinement, and Python/DUCC comparisons. `examples/python_perf_baseline.py` accepts an optional tenth positional argument for `nphi`; omitting it preserves its existing `2*lmax+3` default.

The native matrix used:

```text
taskset -c 0     target/release/examples/m1b_bench <spin> <lmax> 1 both GL STANDARD <lmax+1> <nphi> <mmax> requested-only
taskset -c 0-7   target/release/examples/m1b_bench <spin> <lmax> 8 both GL STANDARD <lmax+1> <nphi> <mmax> requested-only
```

The Python comparison used:

```text
RAYON_NUM_THREADS=<threads> taskset -c <affinity> uv run python examples/python_perf_baseline.py <lmax> <spin> <threads> GL 1 plan both both STANDARD <nphi>
```

The checks passed: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (142 Rust unit tests and 10 f32 tests passed, 8 ignored), `cargo test -p rustsht --no-default-features` (127 Rust unit tests and 10 f32 tests passed, 7 ignored), and `uv run pytest python/tests` (16 passed). The Python baseline's optional explicit `nphi` argument was exercised in the T127, T511 and T720 comparisons. High-band release tests were not rerun because the transform kernels are unchanged.
