# Atmospheric-resolution performance and ecTrans study

## Findings

RustSHT completed a rectangular Gauss–Legendre benchmark matrix from T42 through T2047. The matrix contains 29 distinct grids and 232 f64 scalar and spin-1 STANDARD synthesis and analysis timing rows at q1 and q8. At T2047 on the `2048×4096` grid, reused-workspace RustSHT medians were 3.36/0.80 s for scalar synthesis and 1.10/0.22 s for scalar analysis; spin-1 medians were 4.28/1.00 s and 1.82/0.36 s. The first-use workspace costs were about 60 ms, compared with 0.1 ms for plan construction. Peak process RSS was 552 MiB for the scalar run and 585 MiB for spin 1. Matched Python plan-method and DUCC 0.41.0 comparisons agree to relative L2 errors between `3.7e-14` and `3.6e-13` for T720, T1023, T1279, and T2047. DUCC ran faster in every measured case. The largest persistent gap appeared in scalar synthesis at q8, where the Python plan method was 7.4–17.3 times the DUCC time. Those figures describe Python call timings; the native reused-workspace matrix is reported separately. High-resolution analysis profiles identify Legendre recurrence and coefficient accumulation as the largest measured phase. At q8, this phase accounted for 68–90% of analysis time from T720 through T2047. The forward-FFT stage already schedules latitude rows across the worker pool; it accounted for 7–24% at q8. Matrix-matched synthesis profiles show that Legendre recurrence and coefficient accumulation also dominate synthesis, accounting for 84–97% at q8; inverse FFTs accounted for 3–15%. The order-work estimate balanced T1279 and T2047 analysis leaves within 1%, while measured slowest-to-fastest leaf times differed by 1.6–2.5×. The T720 field-reuse baseline reached most of its field-level throughput with eight independent transforms. Thirty-two one-thread workspaces used about 520 MiB before field arrays and increased peak process RSS to 1.5–2.0 GiB, with little additional synthesis throughput over eight fields. No dense-matrix or butterfly candidate was implemented. Dense Legendre storage estimates exceed RustSHT's current table cap by more than an order of magnitude at T720 and reach 16 GiB at T2047; the butterfly crossover remains unmeasured.

## Benchmark method

### Software and hardware

The native `m1b_bench` binary was built from RustSHT revision `2756ba4a1d9bcf6b7b1a1293a185e98e8e6c74b2` on branch `opt/sht-kernel-performance`. Its SHA256 is `34274c791ae73ef1c909639aebfa74747141173bb31390a27b490c289601861b`. The compiler was rustc `1.99.0 (b940084d7 2026-09-28)` with LLVM `23.1.1`; builds used the default release profile and no global target CPU flag. Pulp runtime dispatch was enabled. The CPU was an x86-64 CPU with host topology omitted, AVX2 and FMA, and no AVX-512. The host was shared and CPU affinity did not isolate cores. The Python extension was rebuilt from revision `cde39cbcbfb4ea3eded307f6bbcc6c259c90681e`; its SHA256 is `52a3f15ea7c5e1e1b70a3f15ec2a705b6d24d1b40da60135314a45241ce8066e`. Python was 3.11.15, NumPy was 2.4.6, and DUCC was `ducc0==0.41.0`, upstream revision [`c1ed01b8fda69d2a378190f0915f1a19762dddbf`](https://github.com/mreineck/ducc/tree/c1ed01b8fda69d2a378190f0915f1a19762dddbf).

### Grids and measurements

Each plan used full `mmax=lmax`, `ntheta=lmax+1`, `phi0=0.37`, and GL geometry. The small and medium truncations used `GridSpec::fft_friendly_nphi(mmax)`. For T511, T639, T720, T799, T1023, T1279, and T2047, the native matrix tested every distinct count among `2*mmax+1`, `2*lmax+3`, `2*mmax+2`, and `GridSpec::fft_friendly_nphi(mmax)`. The 29-grid matrix and its complete timing summary are in [`grid-matrix.csv`](data/atmospheric-resolution-2026-10-15/grid-matrix.csv) and [`native/summary.csv`](data/atmospheric-resolution-2026-10-15/native/summary.csv). Native f64 timings reuse a plan and workspace. Each process measured five samples after one warm-up and recorded plan construction, workspace allocation, first calls where available, process RSS, and elapsed time. Native q1/q8 processes used 8 pinned CPU cores; q16 cases used 16 pinned CPU cores. The f32 harness measured q1 and q8 scalar and spin-1 operations at T511 and T1023 on even longitude grids. DUCC comparisons used the Python `ShtPlan` method and DUCC Python calls in the same process, with the same deterministic arrays, GL grids, `mmax`, `phi0`, and requested thread count. They ran two sets of five timed samples per method. The Python-method timings include that API boundary and are distinct from native reused-workspace timings. Repeated longitude comparisons used two alternating fresh-process pairs at T720, T1023, and T1279, and four pairs at T2047 spin 1. Concurrent process-placement details were retained only in local diagnostics. Two T2047 q1 Python comparisons were also run. The second ran after a competing compiler process exited. Their per-process medians are both retained because DUCC timing varied between runs. One extension build first failed because the shared `/tmp` tmpfs had no free space. The retry used `TMPDIR=target/rustsht-maturin-tmp`; no existing `/tmp` data was removed. DUCC and RustSHT logs, timing metadata, binary hashes, and reproduction scripts are under [`reports/data/atmospheric-resolution-2026-10-15`](data/atmospheric-resolution-2026-10-15/).

## Native f64 atmospheric matrix

Each cell reports synthesis and analysis medians in milliseconds as `q1/q8`. T720 uses the FFT-friendly grid (`nphi=1458`) to match its Python comparison; other high-resolution rows use the minimum even grid. Every alternative high-resolution longitude count was also measured and is present in the CSV.

| Truncation | `nphi` | Scalar synthesis | Scalar analysis | Spin-1 synthesis | Spin-1 analysis |
| --- | ---: | ---: | ---: | ---: | ---: |
| T42 | 90 | 0.050/0.053 | 0.059/0.040 | 0.093/0.089 | 0.107/0.059 |
| T72 | 147 | 0.162/0.110 | 0.155/0.084 | 0.274/0.208 | 0.275/0.142 |
| T85 | 175 | 0.295/0.147 | 0.233/0.115 | 0.440/0.280 | 0.449/0.198 |
| T159 | 320 | 1.493/0.451 | 0.993/0.268 | 4.330/1.843 | 1.951/0.339 |
| T255 | 512 | 5.496/1.578 | 2.881/0.716 | 10.006/2.880 | 5.298/1.307 |
| T340 | 686 | 13.927/4.854 | 6.080/1.585 | 15.490/8.895 | 11.003/2.170 |
| T360 | 729 | 16.926/6.217 | 7.206/1.804 | 19.551/9.825 | 12.020/2.400 |
| T511 | 1024 | 41.974/9.577 | 16.151/3.285 | 45.064/12.706 | 27.390/5.406 |
| T639 | 1280 | 72.892/26.626 | 28.282/5.612 | 72.608/34.696 | 48.528/9.325 |
| T720 | 1458 | 97.230/37.205 | 41.659/7.370 | 98.608/39.975 | 70.531/13.084 |
| T799 | 1600 | 132.783/60.333 | 55.071/9.504 | 134.214/67.212 | 93.839/17.440 |
| T1023 | 2048 | 303.031/77.520 | 111.958/20.105 | 288.554/73.662 | 190.386/34.611 |
| T1279 | 2560 | 643.644/305.730 | 221.261/42.027 | 590.340/286.207 | 370.618/70.015 |
| T2047 | 4096 | 3357.801/800.970 | 1103.279/218.563 | 4282.534/1003.841 | 1816.869/357.338 |

The q8 speedup over q1 ranges from about 2.0× to 5.8× across the displayed high-resolution rows. T1279 q8 synthesis scales less than the other displayed rows; the benchmark log shows that its q8 medians are slower than the T1023 scaling would predict. The shared-host measurements do not identify whether scheduling, recurrence work, or CPU frequency accounts for that change. Sixteen-thread measurements are summarized separately below.

## RustSHT and DUCC comparison

The table gives Python plan-method and DUCC medians in milliseconds as synthesis/analysis. Relative L2 errors compare RustSHT outputs with DUCC outputs. The T2047 q1 medians combine two process-level medians; the other q1 rows use one process with two five-sample sets. q8 uses one process. Every comparison includes both synthesis and analysis for the same arrays and grid.

| Truncation | Spin | RustSHT q1 S/A | DUCC q1 S/A | RustSHT q8 S/A | DUCC q8 S/A | Relative L2 S/A |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T720 | 0 | 99.2/41.8 | 30.2/28.6 | 37.4/11.6 | 3.5/3.7 | `7.97e-14 / 3.66e-14` |
| T720 | 1 | 99.8/70.4 | 57.1/58.5 | 40.9/16.9 | 7.7/7.5 | `8.41e-14 / 3.71e-14` |
| T1023 | 0 | 307.0/114.6 | 74.5/71.5 | 77.5/21.3 | 8.9/9.0 | `1.15e-13 / 4.89e-14` |
| T1023 | 1 | 301.0/189.0 | 164.4/165.1 | 89.1/35.1 | 28.5/24.5 | `1.15e-13 / 4.66e-14` |
| T1279 | 0 | 652.6/230.7 | 136.8/133.5 | 296.6/42.8 | 17.2/17.0 | `1.51e-13 / 7.48e-14` |
| T1279 | 1 | 606.6/372.2 | 382.2/298.6 | 308.5/113.2 | 83.7/55.3 | `1.51e-13 / 7.17e-14` |
| T2047 | 0 | 3449.3/1193.7 | 578.8/555.5 | 845.6/237.2 | 113.6/81.3 | `2.84e-13 / 1.35e-13` |
| T2047 | 1 | 4343.0/1906.9 | 1224.1/1137.6 | 1072.0/393.1 | 221.1/186.9 | `3.55e-13 / 1.33e-13` |

The matched comparisons show DUCC's largest advantage in synthesis. RustSHT scalar q8 analysis was 2.4–3.2 times the DUCC time at these grids. Spin-1 q8 analysis was 1.4–2.2 times the DUCC time. The two T2047 q1 process runs show substantial DUCC spread: scalar synthesis medians were 644 and 514 ms; spin-1 synthesis medians were 1,164 and 1,285 ms. The combined medians in the table therefore do not imply isolated-host precision. The high-bandlimit release tests independently check scalar T511/T1023/T2047 synthesis and analysis against DUCC 0.41.0 and analytic fields on GL, Clenshaw–Curtis, and Fejér-1 grids. Spin-1 exact analysis at T1023 and selected T2047 spin synthesis and analysis checks also pass. These numerical checks complement the Python relative-L2 comparisons; they are not inferred from roundtrips alone.

## Longitude sampling

For paired q1 processes, the table reports the median reduction in elapsed time when using the minimum even count `2*mmax+2` instead of the minimum odd count `2*mmax+1`. T2047 spin-1 excludes the contaminated pair.

| Truncation | Scalar synthesis | Scalar analysis | Spin-1 synthesis | Spin-1 analysis |
| --- | ---: | ---: | ---: | ---: |
| T720 | 5.0% | 10.2% | 6.3% | 9.4% |
| T1023 | 9.1% | 14.0% | 19.5% | 14.0% |
| T1279 | 7.2% | 11.1% | 14.2% | 12.6% |
| T2047 | 2.7% | −0.3% | 7.8% | 0.1% |

The q1 changes at T720–T1279 were consistent across the two fresh-process pairs. T2047 even-longitude analysis was effectively unchanged. q8 analysis results were noisier: T720 scalar analysis had opposite signs across its two pairs, and T2047 spin-1 analysis had a large within-process outlier. The raw values and quality flags are in [`native/paired-longitudes/summary.csv`](data/atmospheric-resolution-2026-10-15/native/paired-longitudes/summary.csv). At T720, FFT-friendly `nphi=1458` was faster than minimum-even `nphi=1442` in both q1 pairs. The q1 reductions were 2.8–2.9% for scalar synthesis, 6.9–7.4% for scalar analysis, 5.5–6.3% for spin-1 synthesis, and 5.7–7.5% for spin-1 analysis. The q8 reductions ranged from 3.9% to 18.2%, with greater spread in scalar analysis. The helper uses half-length 729, which factors as `3^6`; the minimum-even grid uses half-length 721, which factors as `7×103`. This factorization is consistent with the observed timing difference, but the phase measurements do not isolate FFT-factor effects. At T1023 and above, the helper count matched the minimum-even count.

## Phase, scheduling, and memory results

The analysis phase timer includes row-FFT scheduling, latitude preparation, order-spectrum preparation, coefficient initialization and cleanup, order-work setup, and Legendre recurrence with coefficient accumulation. The final phase combines recurrence and accumulation, so this run does not separate those two costs. Percentages below are medians from the q8 profile. The diagnostic grids are `721×1443` (T720), `1024×2049` (T1023), `1280×2561` (T1279), and `2048×4097` (T2047); these differ from the native matrix grids shown above. All q1 samples and raw phase records are in [`profiles/phase-profile.log`](data/atmospheric-resolution-2026-10-15/profiles/phase-profile.log).

| Truncation | Diagnostic grid | Spin | Row-FFT stage | Recurrence + accumulation | Whole analysis |
| --- | ---: | ---: | ---: | ---: | ---: |
| T720 | 721×1443 | 0 | 20% | 70% | 8.62 ms |
| T720 | 721×1443 | 1 | 20% | 78% | 15.1 ms |
| T1023 | 1024×2049 | 0 | 23% | 68% | 23.7 ms |
| T1023 | 1024×2049 | 1 | 24% | 74% | 41.1 ms |
| T1279 | 1280×2561 | 0 | 12% | 77% | 42.9 ms |
| T1279 | 1280×2561 | 1 | 13% | 85% | 74.0 ms |
| T2047 | 2048×4097 | 0 | 7% | 82% | 234 ms |
| T2047 | 2048×4097 | 1 | 9% | 90% | 390 ms |

The row-FFT stage already parallelizes latitude groups across q>1 workspaces. A previous alternating comparison measured q6 whole-analysis reductions of 36.1% for scalar T1023 and 43.9% for spin 1 when row FFTs were parallelized; q1 differences were within measurement spread. See [`row-fft-parallelism-2026-10-15.md`](row-fft-parallelism-2026-10-15.md). Current q8 analysis profiles place the largest share in recurrence and accumulation, especially at T1279 and T2047. The forward-FFT percentage falls as truncation grows. The synthesis profile uses the same FFT-friendly longitude counts as the native matrix. Its test-only timers record latitude and spectrum setup, Legendre work with coefficient conversion and spectral accumulation, and inverse FFT plus output copy. The setup phase remains below 2.1% of q8 wall time. The q8 phase shares below are medians from five samples; q1 timings and each sample are in [`synthesis-phase-profile.log`](data/atmospheric-resolution-2026-10-15/profiles/synthesis-phase-profile.log). Absolute times are specific to the test diagnostic and are not the native benchmark medians.

| Truncation | Grid | Spin | Legendre + accumulation | Inverse FFT + writeback | Whole synthesis |
| --- | ---: | ---: | ---: | ---: | ---: |
| T720 | 721×1458 | 0 | 92.6% | 6.7% | 33.7 ms |
| T720 | 721×1458 | 1 | 83.6% | 14.5% | 40.3 ms |
| T1023 | 1024×2048 | 0 | 91.5% | 7.4% | 67.7 ms |
| T1023 | 1024×2048 | 1 | 85.1% | 12.8% | 73.9 ms |
| T1279 | 1280×2560 | 0 | 96.6% | 2.9% | 275 ms |
| T1279 | 1280×2560 | 1 | 93.4% | 5.6% | 286 ms |
| T2047 | 2048×4096 | 0 | 96.8% | 2.7% | 752 ms |
| T2047 | 2048×4096 | 1 | 94.7% | 4.2% | 989 ms |

The matrix-matched profile does not identify a dominant inverse-FFT bottleneck. Spin-1 inverse FFTs reach 14.5% at T720 and 12.8% at T1023; they account for 5.6% at T1279 and 4.2% at T2047. The scalar inverse FFT phase is at most 7.4%. A separate odd-longitude diagnostic showed higher FFT shares because odd counts use the full-complex fallback; its raw log is preserved in [`synthesis-phase-profile-odd-longitudes.log`](data/atmospheric-resolution-2026-10-15/profiles/synthesis-phase-profile-odd-longitudes.log) and is excluded from the matrix-matched conclusions. The leaf profile recorded worker assignment, start and finish offsets, recurrence block counts, estimated work, and whole-call time. At q8, T1279 and T2047 leaf work estimates differ by at most 1.0%, but elapsed leaf times differ by 1.6–2.5×. Leaf capacity fractions were 0.59–0.72. The estimate omits order-dependent costs; the profile does not establish that creating more leaves would help. A previous overpartition candidate regressed T511 by 3.19% and T720 by 11.55%. The T2047 native process constructed a plan in about 0.1 ms and allocated a q8 workspace in about 60 ms. The workspace size therefore contributes little to a single multi-second transform, but each independently concurrent transform needs its own scratch. The main matrix recorded process RSS in its `.time` files; T2047 spin 1 peaked at 585 MiB. Detailed first-use and allocation values are in the native raw logs.

## Sixteen-thread and f32 screens

The q16 screen used the same even grids as the q8 matrix at T1023 and T1279. Concurrent process-placement details were retained only in local diagnostics. Treat those q16 values as a noisy scaling screen. T1279 q16 also ran on a shared host. The raw q16 logs and binary metadata are in [`native/q16`](data/atmospheric-resolution-2026-10-15/native/q16/). The f32 screen used `nphi=1024` at T511 and `nphi=2048` at T1023. Values are q1/q8 milliseconds; process RSS was about 16 MiB at T511 and 52 MiB at T1023.

| Truncation | Spin | Synthesis q1/q8 | Analysis q1/q8 |
| --- | ---: | ---: | ---: |
| T511 | 0 | 551/105 | 286/54 |
| T511 | 1 | 1,064/148 | 902/164 |
| T1023 | 0 | 4,064/675 | 2,058/367 |
| T1023 | 1 | 8,353/1,155 | 6,872/1,287 |

The first T1023 screen overlapped unrelated Cargo/rustc activity. A repeat on 8 pinned CPU cores was within 4.6% for every operation, spin, and thread count. The f32 path was slower than f64 on these screens. Existing f32 tests pass scalar synthesis and analysis references, spin-1 T511 analysis, spin synthesis references, analytic fields, and adjoint identities. They do not provide a T1023 f32 DUCC reference, so T1023 f32 timings are performance observations without same-grid high-bandlimit accuracy evidence.

## Independent-field reuse

The T720 f64 batch study reused one immutable plan. A single field used one q8 workspace. Field-parallel cases used one-thread workspaces and scheduled independent transforms over an outer q8 pool. Sequential and field-parallel cases used the same field inputs. The table gives throughput in fields/s at 1, 8, and 32 fields. The last column gives process peak RSS for the 32-field parallel case.

| Operation | Fields/s at 1 / 8 / 32 fields | Parallel workspace RSS increase at 8 / 32 | 32-field peak RSS |
| --- | ---: | ---: | ---: |
| Scalar synthesis | 27.2 / 74.7 / 74.4 | 130 / 518 MiB | 1.76 GiB |
| Scalar analysis | 72.8 / 120.9 / 124.7 | 130 / 518 MiB | 1.51 GiB |
| Spin-1 synthesis | 24.8 / 69.3 / 70.2 | 130 / 519 MiB | 2.01 GiB |
| Spin-1 analysis | 53.0 / 78.0 / 85.6 | 130 / 519 MiB | 1.51 GiB |

With eight independent one-thread transforms, aggregate synthesis throughput rose 2.7–2.8× over one q8 transform. Thirty-two fields added little synthesis throughput. Analysis throughput rose 1.5–1.7× at eight fields; moving from eight to 32 fields added up to 10% in this screen. The additional 8-workspace allocation took 48–54 ms; 32-workspace allocation took 190–199 ms. A simple warm-batch calculation indicates that workspace setup is repaid within one eight-field batch for scalar and spin-1 synthesis and spin-1 analysis, and in about 1.4 batches for scalar analysis. These measurements report aggregate batch throughput; they do not measure individual per-field completion latency. This is caller-level concurrency using the current recurrence, not dense matrix-matrix Legendre multiplication. The raw process logs and checksums are in [`batch-T720`](data/atmospheric-resolution-2026-10-15/batch-T720/).

## ecTrans algorithms and licensing

The read-only ecTrans study used upstream revision [`7de2ee295a0df06299a9cc78595d5148643e487b`](https://github.com/ecmwf-ifs/ectrans/tree/7de2ee295a0df06299a9cc78595d5148643e487b). It inspected `butterfly_alg_mod.F90`, `ledir_mod.F90`, `leinv_mod.F90`, `suleg_mod.F90`, and `tpm_flt.F90`. No ecTrans source, test code, or copyrightable expression was copied or translated, and RustSHT gained no ecTrans dependency. The full source and algorithm notes are in [`ectrans-legendre-strategy-study-2026-10-16.md`](ectrans-legendre-strategy-study-2026-10-16.md).

ecTrans stores per-order Legendre matrices with symmetric and antisymmetric latitude components. It packs fields as matrix columns so one basis block serves multiple vertical levels. Its fast path applies butterfly factors selected from an interpolative-decomposition residual criterion; setup still generates dense matrices before compression. The studied source uses a `1e-7` factorization precision. That setting is not evidence that a RustSHT implementation would meet f64 or f32 transform tolerances. The dense north-hemisphere table payload estimate for an uncapped f64 RustSHT-style layout is about 0.70 GiB at T720, 2.00 GiB at T1023, 3.91 GiB at T1279, and 16.0 GiB at T2047. These are formula estimates, not ecTrans measurements. RustSHT's compact-table cap is 34 MiB and its current degree limit is T255. A dense high-bandlimit matrix candidate was not built because its storage exceeds that cap at every primary target; preprocessing time and matrix-method amortization therefore remain unmeasured. Batched dense multiplication falls outside the current memory policy. Butterfly compression could reduce repeated Legendre work when a plan serves many fields or timesteps. Its crossover, accuracy, preprocessing time, and storage for RustSHT's scalar and spin-weighted kernels have not been measured. The published methods include Wedi, Hamrud, and Mozdzynski's [fast SHT for global NWP and climate models (2013)](https://journals.ametsoc.org/doi/10.1175/MWR-D-13-00016.1), [O’Neil, Woolfe, and Rokhlin (2010)](https://www.sciencedirect.com/science/article/pii/S1063520309000888), [Cheng et al. (2005)](https://epubs.siam.org/doi/10.1137/030602678), [Li and Yang (2017)](https://epubs.siam.org/doi/abs/10.1137/16M1074941), and [Bremer, Chen, and Yang (2021)](https://epubs.siam.org/doi/abs/10.1137/20M1333845). The 2021 IDBF paper reports `O(N² log³N)` complexity including precomputation; that is a published algorithmic result, not a RustSHT benchmark.

## Recommendation

Keep the current RustFFT backend and focus further optimization on Legendre recurrence and coefficient accumulation. Both analysis and matrix-matched synthesis profiles show that Legendre work dominates at the primary high-bandlimit targets; the synthesis inverse FFT stage is at most 15% on the tested even grids. The analysis row FFT already scales across latitude groups. The T720 field-reuse baseline favors eight one-thread workspaces when memory allows; 32 fields do not improve synthesis throughput enough to offset their larger memory use. Do not promote dense Legendre matrices under the current memory cap. Do not add a butterfly execution path based only on asymptotic complexity or ecTrans's compression setting. A future private prototype should first state an error target, derive its factors independently from the published mathematics, measure setup and storage, and compare complete transforms on single-field and multi-field workloads. Any recurrence change should use alternating whole-transform process pairs and report the same numerical checks, even-grid lengths, and field reuse patterns measured here.

## Validation

The following checks passed on this branch:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`: 187 Rust tests and 11 f32 integration tests passed; 13 tests were ignored, including the new synthesis phase diagnostic.
- Release high-bandlimit scalar GL/CC/F1 comparison and analytic test through T2047.
- Release spin-1 exact analysis at T1023, T2047 low-order analysis and adjoint, and sparse synthesis at T2047 across GL/CC/F1.
- Release f32 integration suite: 11 passed.
- `uv run --locked pytest python/tests`: 20 passed. Raw validation logs are in [`validation`](data/atmospheric-resolution-2026-10-15/validation/). The release checks use the pinned DUCC 0.41.0 reference data and independent analytic identities already present in the repository.
