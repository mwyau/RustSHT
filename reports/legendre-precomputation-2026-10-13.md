# Legendre precomputation and recurrence at T72–T720

Updated 2026-10-14. Branch: `main`. The scalar recurrence baseline is commit `7c7e963884add727a02b0a76e297f9fc04681271`. The bounded scalar GL table was added in `d251af6`; its T72 gate expansion is `d7f3dea`. The spin-1 analysis table began in `a151ab1` with recurrence baseline `552fd83`, and was later extended through T255 in the update recorded below and in [the milestone log](../plans/05-validation-and-milestones.md#t255-spin-1-gl-analysis-table). DUCC comparisons use `ducc0==0.41.0` and source revision [`c1ed01b8fda69d2a378190f0915f1a19762dddbf`](https://github.com/mreineck/ducc/tree/c1ed01b8fda69d2a378190f0915f1a19762dddbf).

## Findings

The compact Legendre table benefits repeated scalar GL synthesis through T255, with a mixed result for scalar analysis at T255 and only a clear single-thread analysis gain at T72. It stores triangular harmonic support on the northern hemisphere with contiguous latitude lanes. The scalar table cap is 34 MiB. The initial lazy spin-1 analysis table was limited to T72/T127 (at most 4.1 MiB); a measured extension now reaches T255 with a 32.375 MiB table per plan. At T255 it reduces warm native analysis by 31.2%/26.2% at one/eight threads, but first-use construction takes about 31–32 ms. Plan reuse determines whether first-call table construction repays its cost. Torch-harmonics PR #322 stores complete order-degree-latitude tensors. Its folded CC/F1 projection is assembled in bounded blocks, but the final cached tensor is still full-sized. With identical full GL dimensions, the Rust layout uses one quarter of a dense scalar table's float64 payload. A later direct screening run measured its CPU and RTX 5070 paths; the conditions and limitations are recorded below. Temporary compact-table runs at T360, T511 and T720 reduced synthesis time. Analysis gains were smaller or reversed at eight threads. At T720 the table occupied 716.9 MiB, added about 679 ms of plan construction, improved one-thread synthesis by 25%, and slowed eight-thread analysis by 37%. Production scalar transforms therefore use recurrence above T255. Spin-1 precomputation is enabled for STANDARD exact GL analysis through T255 when its degree-extended table fits the same 34 MiB cap; spin synthesis, f32, CC and F1 continue to use recurrence. The table improves RustSHT relative to its recurrence baseline but does not establish general DUCC parity. At T72, Python-plan scalar synthesis/analysis measured `0.348/0.301 ms` versus DUCC `0.268/0.344 ms` at one thread; at T127 the respective timings were `0.921/0.629 ms` versus `0.728/0.670 ms`. At T255 they were `6.304/4.446 ms` versus `1.915/1.329 ms`. Reused native workspaces are faster than Python-plan methods, which allocate a workspace on each call. T720 scalar and spin-1 operations continue to use recurrence and remain several times slower than DUCC in these Python-plan measurements.

### Spin-1 compact-table analysis experiment (2026-10-14)

This follow-up uses commit `552fd83` as the spin-1 recurrence baseline and records the accepted candidate in commit `a151ab1`. The experiment asks whether a bounded, symmetry-reduced table can improve repeated small-band STANDARD analysis without copying Torch-harmonics' dense `(m,l,k)` layout. The implementation reuses the existing triangular north-hemisphere scalar table and extends its scalar degree range to `lmax+1`, which the spin-1 coefficient conversion requires. It is created lazily on the first eligible STANDARD exact GL analysis and retained by that plan. The table contains contiguous northern latitude lanes and reflects southern contributions by degree parity. It does not materialize a second full map or spectrum. The original implementation was eligible only for f64 GL exact grids at T72/T127 with near-full `mmax`; all other cases, including T255 at that stage, used recurrence. The measured initial payloads were 0.783 MiB at T72 and 4.094 MiB at T127. The later T255 extension and its separate setup/amortization evidence are recorded below. The table path preserves the established per-ring polar cutoffs and falls back to scaled recurrence if a Fourier input is non-finite or exceeds `1e100`. A focused extreme-input test uses finite `1e150` maps and compares both coefficient channels with the general recurrence. A separate cancellation regression exposed that iterative parity reconstruction `(north+south)-2*south` can erase a representable north/south difference. The implementation now forms the even and odd weighted sums directly once per lane. The strictly read-only Sol review identified the cancellation case; its follow-up review found the direct-sum correction sound, including asymmetric cutoffs and the equatorial singleton. The added buffer work remains included in the performance measurements below. Native comparisons used the release `m1b_bench` harness, one warm-up and five timed calls per process, three alternating process pairs, f64 GL, full `mmax`, `phi0=0.37`, reused plans/workspaces, and one/eight-thread Rayon pools. Runs were 8 pinned CPU cores on the x86-64 CPU; rustc 1.99.0 used `opt-level=3`, 16 codegen units, no LTO or native target flag, and Pulp AVX2/FMA runtime dispatch. Values are medians in milliseconds; ranges show the three process medians.

| Bandlimit | Method | Analysis 1T | Analysis 8T | First call 1T | First call 8T |
| --- | --- | ---: | ---: | ---: | ---: |
| T72 | recurrence | `0.438 (0.429–0.444)` | `0.210 (0.205–0.211)` | `0.507 (0.462–0.584)` | `0.257 (0.210–0.286)` |
| T72 | compact table | `0.278 (0.263–0.285)` | `0.145 (0.127–0.147)` | `1.070 (1.050–1.154)` | `1.013 (0.904–1.082)` |
| T127 | recurrence | `2.023 (1.977–2.025)` | `0.940 (0.899–0.952)` | `2.279 (2.235–2.432)` | `1.084 (1.044–1.096)` |
| T127 | compact table | `1.074 (1.068–1.078)` | `0.510 (0.510–0.545)` | `5.222 (5.191–5.570)` | `4.430 (4.429–4.617)` |

Steady-state improvement is 36.5%/31.0% at T72 and 46.9%/45.7% at T127 for one/eight threads. The T72 first-call surcharge is 0.563 ms at one thread and about 0.756 ms in the fresh-plan eight-thread screen; based on steady savings, the total call count to break even is about five and thirteen. At T127, the corresponding estimates are about five and nine total calls. These estimates use the 1T first-call surcharge at 1T and fresh-plan 8T timing at 8T; the 8T screen constructed its fresh plan after a separate cold 1T call in the same process and should be treated as approximate. `/usr/bin/time` peak RSS increased by about 732 KiB at T72 and 3,968 KiB at T127, consistent with the 0.783/4.094 MiB table payloads and allocator/runtime noise. Plan construction itself remains lazy and does not include table setup. After the parity correction, rebuilt Python-plan comparisons against `ducc0==0.41.0` use deterministic matching T72/T127 maps, GL sampling, `phi0=0.37`, two process runs of five calls, and one/eight threads. Errors compare every output coefficient after converting DUCC's coefficient layout. Timings are the median of the two process medians, in milliseconds.

| Bandlimit | Threads | Rust plan | DUCC | Max absolute error | RMS error | Relative-L2 error |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T72 | 1 | 0.278 | 0.288 | `2.434e-15` | `1.854e-16` | `5.070e-15` |
| T72 | 8 | 0.311 | 0.407 | `2.434e-15` | `1.854e-16` | `5.070e-15` |
| T127 | 1 | 1.129 | 1.361 | `6.999e-15` | `2.108e-16` | `1.006e-14` |
| T127 | 8 | 3.929 | 2.705 | `6.999e-15` | `2.108e-16` | `1.006e-14` |

The Python wrapper allocates a workspace for each plan method call. At eight threads the T127 process ranges were 3.853–4.005 ms for RustSHT and 2.413–2.997 ms for DUCC; this interface-level comparison does not match the reusable native workspace result. The Python runs also reported map-input copy times from 0.004 to 0.052 ms and plan construction from 0.018 to 0.029 ms. The spin table is lazy, so ordinary plan construction does not include its setup. At the time of this initial spin-1 compact-table experiment, PyTorch was unavailable, so it did not include Torch runtime measurements. A separate follow-up below measures Torch-harmonics directly. The T72/T127 version added small tables for a measured repeated-analysis workload; the later T255 extension is larger and is accepted specifically for repeated plans within the measured memory/setup budget. It does not add a Torch-like dense or blocked matrix implementation.

## Software and benchmark conditions

The CPU was an x86-64 CPU, host topology omitted, with AVX2 and FMA and no AVX-512. The host ran Linux. Processes were 8 pinned CPU cores. Rust was `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0`, LLVM `23.1.1`; C++ was GCC `15.2.0`. Rust release used `opt-level=3`, the default 16 codegen units, no LTO, no `target-cpu=native`, and Pulp runtime dispatch. All timing values are milliseconds. Native measurements through T720 used `examples/m1b_bench.rs` with f64, full `mmax`, GL dimensions `(lmax+1) × (2*lmax+3)`, `phi0=0.37`, one warm-up and five timed samples per process. The harness reports separate one-thread and eight-thread Rayon executions with reused native plans and workspaces. Three process runs were collected for T127/T255 and the temporary T360/T511/T720 table screen. Those recurrence baselines came from `7c7e963`; production-table data came from `d251af6` before report-only edits. The T72 gate expansion was measured separately in five alternating baseline/candidate process pairs with 25 timed samples per process. Its recurrence baseline was `e2a8bd0`; the candidate changed the minimum table bandlimit and added the T72 regression checks. The paired methodology and conditions are detailed below. Python comparisons used `examples/python_perf_baseline.py`, Python `3.11.15`, NumPy `2.4.6`, the rebuilt release Rust extension and installed `ducc0==0.41.0`. Inputs, `phi0`, precision, grid and thread counts matched. Each run group warmed the operation and timed five calls; two or three run groups were collected. The benchmark's reusable Python plan still allocates a Rust workspace on every method call. Procedural functions also construct a plan on every call. These results are not native reused-workspace timings. Representative commands (repeat each invocation in separate processes to obtain process medians) are:

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo run --release --example m1b_bench -- 0 255 1 synthesis GL STANDARD
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" cargo run --release --example m1b_bench -- 0 255 8 analysis GL STANDARD
RAYON_NUM_THREADS=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 255 0 1 GL 2 plan both both STANDARD
RAYON_NUM_THREADS=8 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 255 1 8 GL 2 plan both both STANDARD
```

For other cases, substitute the bandlimit, spin, thread count, and `GL`, `CC` or `F1`. Native `m1b_bench` takes spin, bandlimit, thread count, operation, grid and mode. The Python harness takes bandlimit, spin, thread count, grid, process-run count, scope, operation, backend and mode. `/usr/bin/time -v` was used for peak-RSS measurements. The local DUCC source build came from the pinned source revision. The source CMake defaults select C++17, `-O3 -ffast-math -march=native`, interprocedural optimization and stripped symbols. The built extension reported version `0.41.1`; the installed wheel reported `0.41.0` and its wheel build flags were not recorded. At T511 and T720, source-build and wheel timings were similar. The source build measured:

| Grid | Spin | Synthesis 1T / 8T | Analysis 1T / 8T |
| --- | ---: | ---: | ---: |
| GL T511 | 0 | `10.127 / 3.091` | `6.599 / 1.153` |
| GL T511 | 1 | `20.608 / 4.966` | `12.822 / 1.942` |
| GL T720 | 0 | `23.931 / 6.584` | `15.076 / 2.299` |
| GL T720 | 1 | `48.967 / 9.133` | `31.293 / 4.465` |

These runs used two process medians of five calls and were 8 pinned CPU cores. T127 source-build eight-thread measurements varied sharply between processes and are not used for a ratio claim. The source build is a compiler-configuration check; the installed DUCC wheel remains the primary Python comparison. The machine's RTX 5070 has 12,227 MiB of memory. Direct Torch allocation and execution measurements are recorded in the follow-up section below.

## Torch-harmonics storage and Rust table layout

The Torch-harmonics source inspected for PR [#322](https://github.com/NVIDIA/torch-harmonics/pull/322) was head `37751d3e90b8f12fe9439c52847f4c5d70d5a627`, based on `97f7d2756d2d2d3b974d8ee31ca04e032fdf1d05`. The [`legendre.py` source](https://github.com/NVIDIA/torch-harmonics/blob/37751d3e90b8f12fe9439c52847f4c5d70d5a627/torch_harmonics/legendre.py) computes Legendre values by streaming degree recurrence state, then returns a rectangular `(m,l,k)` tensor. Scalar support outside `m <= l` is masked to zero; the allocated tensor is not packed triangularly. `RealSHT` uses that layout for analysis, while `InverseRealSHT` permutes it to `(m,k,l)` and makes the degree axis contiguous for synthesis contractions ([`sht.py`](https://github.com/NVIDIA/torch-harmonics/blob/37751d3e90b8f12fe9439c52847f4c5d70d5a627/torch_harmonics/sht.py)). The single-process modules register the full tensor as a buffer. The regular Legendre cache inherits the cache helper's default 20-entry limit; `_precompute_folded_projection` sets a two-entry limit. Both return copies of cached values. These entry limits do not impose a byte budget. Distributed transforms can request local order and latitude ranges and store a shard, reducing each rank's table size; the serial transform stores the full range ([`distributed_sht.py`](https://github.com/NVIDIA/torch-harmonics/blob/37751d3e90b8f12fe9439c52847f4c5d70d5a627/torch_harmonics/distributed/distributed_sht.py)). The folded CC/F1 projection combines native quadrature, complementary-grid quadrature, meridional interpolation and its adjoint into analysis weights. It is not the synthesis basis table. In the serial path, it allocates the complete `(m,l,k)` output, then computes it in order and degree blocks. The block rule limits the real Legendre basis intermediate to at most `2^20` float64 values (8 MiB); the final projection is still full-sized. The vector projection has two rows. The block limit bounds construction scratch, not the cached projection or registered module buffer ([`sht.py`, `_precompute_folded_projection`](https://github.com/NVIDIA/torch-harmonics/blob/37751d3e90b8f12fe9439c52847f4c5d70d5a627/torch_harmonics/sht.py#L2405-L2563)). At full GL dimensions `(721,721,721)`, the dense float64 Torch scalar table is 2.79 GiB and the two-row vector table is 5.59 GiB. On the reported 12,227 MiB RTX 5070, those payloads alone occupy about 23% and 47% of device memory. These figures exclude inputs, FFT workspaces, converted buffers, model activations, and any other live tensors. A cached host copy and a device module buffer may also coexist; peak residency depends on construction and transfer order. The direct follow-up confirms the table sizes and records measured peak allocations; it did not reproduce an out-of-memory failure because the GPU was otherwise available. For full triangular support and a GL grid with `ntheta=lmax+1`, Rust's table payload is

```text
8 * ceil(ntheta / 2) * sum(m=0..lmax, lmax - m + 1) bytes
```

The dense scalar Torch payload is `8*(lmax+1)^3` bytes; the vector payload is twice that. Storing only the scalar triangular support across all latitudes would halve the dense payload. Applying north/south symmetry as well gives the Rust table size, about one quarter of Torch's scalar table. If both vector rows admit valid spin-reflection reconstruction, the two-row packed payload would be about twice the Rust scalar payload; this is a shape estimate, not a spin-table implementation or benchmark. The table sizes are:

| Bandlimit | Rust compact table | Torch scalar table | Torch two-row vector table |
| --- | ---: | ---: | ---: |
| T72 | 0.762 MiB (eligible) | 2.97 MiB | 5.93 MiB |
| T127 | 4.03 MiB | 16 MiB | 32 MiB |
| T255 | 32.13 MiB | 128 MiB | 256 MiB |
| T360 | 90.23 MiB (temporary) | 358.9 MiB | 717.9 MiB |
| T511 | 256.5 MiB (temporary) | 1 GiB | 2 GiB |
| T720 | 716.87 MiB (temporary) | 2.79 GiB | 5.59 GiB |

Rust stores only `m <= l <= lmax` and northern latitude rows. Within each `(m,l)` row, latitude is contiguous. The reflected southern basis is `(-1)^(l+m)` times the northern value. Scalar synthesis and analysis use the table for GL; `S`, `A`, `S*` and `A*` retain their separate FFT and quadrature conventions. The table uses existing order-major workspace scratch and allocates no second full map or spectrum per transform. The current plan gate requires f64 scalar GL, exact GL sampling, `72 <= lmax <= 255`, `mmax >= ceil(3*lmax/4)`, and a table no larger than 34 MiB. The compact Rust table was first compared with Rust's recurrence kernels. The direct Torch follow-up below adds same-machine CPU comparisons and separate GPU screening timings. The Torch implementation is organized around cached tensor contractions, while Rust's recurrence keeps degree-dependent basis values in bounded latitude scratch. At T127 and T255, a compact Rust GL table measurably reduces scalar synthesis time when the plan is reused; scalar analysis benefits most at T127 and is mixed at T255. At T720, a 716.9 MiB temporary table improved synthesis but slowed eight-thread analysis and added 679 ms of setup; it was not enabled in production. The table requires about four times less payload than the full Torch scalar table at the same GL shape. A memory-bounded blocked Rust transform could generate a basis tile, contract it into the output, and discard the tile. That would cap resident basis storage, but it would either regenerate basis values for each transform or require a bounded tile cache. No timing here shows that this would outperform the recurrence kernel or Torch-harmonics' full contraction on its target GPU. The existing compact table already tests the persistent-table option at small GL bands. No second dense or blocked precomputation path was added. The CC/F1 folded projection is a different analysis operator from a GL basis table; Rust continues to use its exact resampling and recurrence path. These results do not justify another table implementation. The current Rust table already tests triangular storage, north/south symmetry, contiguous latitude rows, bounded plan memory and setup amortization against the recurrence implementation. It improves repeated scalar GL synthesis through T255 under a 34 MiB cap, while its analysis benefit is band- and thread-dependent. Larger tables consume increasing memory and lose analysis throughput at eight threads. Torch-harmonics' blocked construction limits temporary basis storage but still materializes its full final projection. A streamed CPU tile could lower resident memory further, but it would recompute basis values or need a cache; neither option has measured an end-to-end advantage over recurrence or the existing compact table. The direct Torch results below favor retaining the measured compact small-band table and recurrence at larger bands; they do not support adding a second dense or blocked Rust method.

## Native recurrence and table results

Each cell gives one-thread / eight-thread medians. Baseline plan construction did not build a table. Candidate plan construction includes the table. Candidate timing ranges are shown in the text rather than hiding setup behind execution throughput.

| Bandlimit | Plan setup, baseline → table | Scalar synthesis, recurrence → table | Scalar analysis, recurrence → table | Table payload |
| --- | ---: | ---: | ---: | ---: |
| T72 | `0.037 → 0.653 ms` | `0.401/0.522 → 0.353/0.352` | `0.462/0.450 → 0.266/0.266` | 0.762 MiB |
| T127 | `0.018 → 2.402 ms` | `1.884/0.542 → 0.962/0.343` | `1.114/0.579 → 0.702/0.347` | 4.03 MiB |
| T255 | `0.017 → 33.238 ms` | `13.579/4.018 → 6.574/1.681` | `6.553/2.244 → 5.011/1.565` | 32.13 MiB |

At T127, three candidate process runs ranged `0.961–1.007 ms` / `0.343–0.358 ms` for synthesis and `0.701–0.710` / `0.336–0.389 ms` for analysis. T255 ranges were `6.248–6.781` / `1.653–1.760 ms` for synthesis and `4.854–5.494` / `1.549–1.647 ms` for analysis. Table setup medians were `2.402 ms` and `33.238 ms`; workspace allocation was about `0.04 ms` and `0.97 ms`. `/usr/bin/time -v` measured 41,496 KiB peak process RSS for the T255 table transform, which contains a 32.13 MiB table payload. The T72 table reduced synthesis by 12%/33% and analysis by 42%/41% at one/eight threads. Its 0.616 ms setup increment amortizes after about 13/4 synthesis calls or 4/4 analysis calls at one/eight threads. A repeated pair of one synthesis and one analysis repays setup after about three one-thread iterations or two eight-thread iterations. The measured setup break-even for one operation is approximately:

| Bandlimit | Synthesis 1T / 8T calls | Analysis 1T / 8T calls |
| --- | ---: | ---: |
| T72 | 13 / 4 | 4 / 4 |
| T127 | 3 / 12 | 6 / 11 |
| T255 | 5 / 15 | 22 / 49 |

These counts divide measured table setup by the difference between the recurrence and table medians. They omit allocation differences and variation between processes. A paired synthesis-plus-analysis workload amortizes setup sooner because both operations use the table. One-shot calls do not: the Python procedural T127 scalar synthesis/analysis medians were `3.261/2.908 ms`, versus `0.925/0.622 ms` through the Python plan methods; at T255 they were `38.154/35.990 ms`, versus `6.196/4.473 ms`. The T255 procedural cost includes rebuilding a roughly 32 ms table.

### Temporary higher-bandlimit table screen

The table gates were temporarily raised to include T360, T511 and T720. Plan construction confirmed table activation at all three sizes. The gates were then restored to T255 and the large-table changes were not committed. Three native process runs gave:

| Bandlimit | Table setup | Scalar synthesis recurrence → table | Scalar analysis recurrence → table | Table payload |
| --- | ---: | ---: | ---: | ---: |
| T360 | `0.034 → 89.6 ms` | `26.145/6.918 → 16.184/5.132` | `15.558/5.712 → 11.997/5.368` | 90.23 MiB |
| T511 | `0.035 → 251.2 ms` | `60.979/15.962 → 43.239/12.873` | `36.171/10.142 → 32.731/14.435` | 256.5 MiB |
| T720 | `0.091 → 678.9 ms` | `153.981/43.642 → 115.706/34.477` | `97.507/27.644 → 89.412/37.918` | 716.87 MiB |

At T360, the eight-thread analysis medians overlapped their process variation. At T511, table analysis was about 9% slower at one thread and 42% slower at eight. At T720, it was about 8% faster at one thread and 37% slower at eight. The T720 table process reached 786,656 KiB peak RSS. This memory and eight-thread analysis behavior do not justify raising the production cap for an atmospheric workload that uses both analysis and synthesis. A synthesis-only table policy could be measured separately, but it would require a separate selection rule and is not part of this implementation. The temporary high-band Python comparison against the installed DUCC wheel measured these relative-L2 errors for table-backed scalar `S/A`: T360 `2.652e-14/3.561e-14`, T511 `4.690e-14/4.707e-14`, and T720 `5.349e-14/7.719e-14`. The Python method also allocates a workspace per call; native reusable-workspace timings are the performance acceptance data.

## GL Python-plan comparison against DUCC

The table entries below report `Rust / DUCC` milliseconds. Each row separates synthesis and analysis, one and eight threads. At T72/T127/T255 the Rust GL scalar rows use the compact table. Spin 1 and the other bandlimits use recurrence. The newly measured T72 Python run is a correctness cross-check; process-level Rust timings at eight threads had substantial scheduling variation and are not used to accept the native kernel.

| Bandlimit | Spin | S 1T R/D | A 1T R/D | S 8T R/D | A 8T R/D | Max relative-L2 S/A | Max 8T process range |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T72 | 0 | `0.348/0.268` | `0.301/0.344` | `0.392/0.520` | `0.297/0.596` | `5.409e-15/7.052e-15` | 0.479 ms |
| T72 | 1 | `0.300/0.276` | `0.483/0.290` | `0.631/0.469` | `0.258/0.190` | `9.137e-15/7.670e-15` | 2.734 ms |
| T127 | 0 | `0.921/0.728` | `0.629/0.670` | `4.998/0.174` | `0.332/0.147` | `1.070e-14/1.114e-14` | 6.622 ms |
| T127 | 1 | `3.182/1.564` | `3.005/1.357` | `3.315/0.251` | `4.866/0.292` | `1.487e-14/1.364e-14` | 1.675 ms |
| T255 | 0 | `6.304/1.915` | `4.446/1.329` | `1.746/0.697` | `6.491/0.248` | `2.001e-14/2.277e-14` | 10.973 ms |
| T255 | 1 | `14.189/4.073` | `17.841/3.016` | `4.730/1.317` | `5.978/0.506` | `3.017e-14/2.267e-14` | 7.276 ms |
| T360 | 0 | `24.524/9.768` | `15.468/7.965` | `7.621/2.085` | `6.325/1.379` | `4.216e-14/3.436e-14` | 6.999 ms |
| T360 | 1 | `25.456/22.552` | `38.066/17.284` | `10.170/4.704` | `12.673/2.896` | `4.530e-14/3.187e-14` | 2.263 ms |
| T511 | 0 | `59.290/10.197` | `35.308/6.434` | `17.126/3.648` | `11.276/1.126` | `6.632e-14/3.484e-14` | 2.184 ms |
| T511 | 1 | `60.566/20.679` | `91.003/13.081` | `24.695/4.691` | `29.241/2.303` | `6.697e-14/5.128e-14` | 5.599 ms |
| T720 | 0 | `149.967/24.223` | `93.233/14.870` | `47.198/6.306` | `36.372/2.365` | `8.545e-14/6.020e-14` | 3.869 ms |
| T720 | 1 | `145.424/48.525` | `222.770/31.584` | `60.576/9.280` | `67.339/5.145` | `9.123e-14/6.920e-14` | 1.809 ms |

Input copies were small relative to these operations: at T720 the coefficient copy took about `0.1–0.3 ms`, the map copy about `0.8–1.4 ms`, and output allocation about `0.001 ms`. The Python API's per-call workspace allocation explains some of the difference between these measurements and native reuse, but input copying does not explain the high-band transform gap.

## CC and F1 Python-plan comparisons

The scalar table is GL-only. CC and F1 continue to use their existing resampling and recurrence kernels. These rows are current Python-plan medians in milliseconds, `Rust / DUCC`, with errors from the same inputs. They use two process medians of five calls; several T127 eight-thread runs had scheduling outliers.

| Grid / bandlimit | Spin | Synthesis 1T R/D | Analysis 1T R/D | Synthesis 8T R/D | Analysis 8T R/D | Max relative-L2 S/A |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| CC T127 | 0 | `1.647/0.738` | `1.626/1.356` | `1.455/0.884` | `2.992/0.486` | `1.268e-14/6.466e-15` |
| CC T127 | 1 | `3.848/1.886` | `4.578/2.824` | `2.798/1.974` | `6.485/0.958` | `1.785e-14/1.104e-14` |
| CC T511 | 0 | `58.562/10.578` | `43.765/19.085` | `23.636/3.220` | `17.968/2.887` | `5.367e-14/2.979e-14` |
| CC T511 | 1 | `64.844/21.597` | `120.495/40.655` | `30.254/4.924` | `45.611/7.103` | `5.235e-14/4.181e-14` |
| CC T720 | 0 | `150.013/20.753` | `122.882/60.545` | `49.290/2.904` | `56.984/10.982` | `9.625e-14/5.620e-14` |
| CC T720 | 1 | `151.987/50.062` | `337.554/130.188` | `58.504/9.923` | `123.864/22.321` | `8.997e-14/6.519e-14` |
| F1 T127 | 0 | `2.155/0.751` | `1.624/1.374` | `2.809/0.884` | `1.390/0.506` | `1.781e-14/6.429e-15` |
| F1 T127 | 1 | `3.461/1.815` | `4.610/2.843` | `2.558/1.505` | `6.573/1.007` | `1.511e-14/1.105e-14` |
| F1 T511 | 0 | `62.859/10.347` | `47.367/19.648` | `16.386/3.498` | `18.348/2.820` | `6.742e-14/2.989e-14` |
| F1 T511 | 1 | `56.860/20.770` | `118.849/40.339` | `23.877/4.826` | `45.071/6.223` | `8.722e-14/4.172e-14` |
| F1 T720 | 0 | `148.343/20.582` | `121.955/58.587` | `46.427/2.849` | `57.703/10.846` | `9.180e-14/5.635e-14` |
| F1 T720 | 1 | `146.182/48.895` | `314.895/122.412` | `57.954/9.426` | `122.814/23.142` | `9.751e-14/6.505e-14` |

The all-grid comparisons confirm that the GL table selection does not affect CC/F1. Spin-1 timing gaps remain larger in analysis than synthesis at the high bands. No FFT code or CC/F1 numerical path changed in this work.

## Numerical checks and limitations

The table-versus-recurrence test covers scalar synthesis `S`, exact analysis `A`, synthesis adjoint `S*` and analysis adjoint `A*` at rectangular and full truncations, odd and even longitude counts, nonzero `phi0`, and an equatorial ring. Relative L2 differences are below `2e-12`; the DUCC comparisons above are substantially tighter. A focused asymmetric-cutoff case sets one member of a reflected ring pair inactive and the other active, then checks phase and parity in the order spectrum. The table builder rejects any active exact-zero basis value except an exact equatorial parity zero. Coefficients or Fourier inputs with magnitude above `1e100`, non-finite inputs, and the tested `1e300` case use the scaled recurrence; the extreme-input output matched the recurrence exactly. The table is f64-only. Its high-bandlimit screen passed DUCC comparisons for scalar synthesis and analysis with relative-L2 errors below `7.8e-14`, but its timing does not justify a production gate above T255. The initial spin-1 table experiment covered T72/T127; the later measured extension through T255 is recorded below. High-band release checks covered scalar T511/T1023/T2047 on GL/CC/F1, spin 1/2 T511 on GL/CC/F1, and spin 1/2 T1023 exact analysis on all three grids. Existing spin-3/4 regression tests passed in the routine all-feature suite. Validation on source commit `d251af6` passed:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features` (125 Rust tests and 10 f32 tests passed; 8 high-band tests were ignored)
- `cargo test -p rustsht --no-default-features` (115 Rust tests and 10 f32 tests passed; 7 high-band tests were ignored)
- `uv run pytest python/tests` (8 passed)
- Release scalar GL/CC/F1 T511/T1023/T2047 DUCC and analytic checks
- Release spin-1/2 GL/CC/F1 T511 checks and spin-1/2 T1023 exact-analysis checks

The ignored high-spin T1023/T2047 tests were not run in this comparison. No numerical tolerances were changed. No new dependencies were added. The source table carries the DUCC `sht_inner_loop.h` license notice and the pinned source revision.

### Available phase attribution

Hardware counters remained unavailable. This experiment measured complete reusable-workspace table versus recurrence operations and did not add invasive phase timers to the current source. The most recent earlier-revision T511 timers in `plans/05-validation-and-milestones.md` split the Legendre/accumulation work from longitude FFT and row movement. They are historical attribution, not timings for `d251af6`:

| Operation / threads | Legendre, recurrence and accumulation Rust / DUCC | FFT and row movement Rust / DUCC | Whole timed phases Rust / DUCC | Absolute excess in Legendre phase |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis 1T | `75.27 / 6.66 ms` | `3.48 / 1.79 ms` | `78.75 / 8.45 ms` | `68.61 ms` |
| Scalar synthesis 8T | `25.78 / 1.04 ms` | `3.52 / 0.38 ms` | `29.30 / 1.42 ms` | `24.74 ms` |
| Scalar analysis 1T | `154.90 / 4.49 ms` | `3.40 / 1.85 ms` | `158.30 / 6.34 ms` | `150.41 ms` |
| Scalar analysis 8T | `27.10 / 0.71 ms` | `3.40 / 0.30 ms` | `30.50 / 1.01 ms` | `26.39 ms` |
| Spin-1 synthesis 1T | `469.40 / 14.80 ms` | `7.80 / 3.74 ms` | `477.20 / 18.54 ms` | `454.60 ms` |
| Spin-1 synthesis 8T | `75.90 / 2.00 ms` | `7.90 / 0.58 ms` | `83.80 / 2.58 ms` | `73.90 ms` |
| Spin-1 analysis 1T | `684.30 / 8.88 ms` | `6.70 / 3.83 ms` | `691.00 / 12.71 ms` | `675.42 ms` |
| Spin-1 analysis 8T | `173.90 / 1.40 ms` | `7.10 / 0.62 ms` | `181.00 / 2.02 ms` | `172.50 ms` |

The phase sums approximately reconcile with the scoped operations. In that earlier revision, recurrence and accumulation were about 88–99% of RustSHT's phase total or 11 CPU cores% of DUCC's, depending on operation and thread count. Their absolute excess dominates the FFT/row phase excess in every row. This supports prioritizing Legendre and accumulation work, not an FFT change. The current table comparison shows that basis precomputation reduces recurrence cost at selected small bands; the temporary large tables do not give a robust all-operation, eight-thread benefit.

### T511 spin-2 analysis instruction profile


The paired scalar recurrence and accumulation visitor accounted for about 88.2% of all instruction references across the two profile parts. `analyze_spin_order_group` accounted for about 97.3%; RustFFT forward transforms accounted for 2.4%. General spin recurrence, spin coefficient conversion and factor preparation accounted for about 4.6%, 1.4% and 1.1%, respectively. The first profile part recorded 90.7 million calls to `multiply_complex_scaled_product` from the paired analysis reducer. The profile did not separate normal-range SIMD work from scaled fallback work, so this call count does not establish how much of the total time each regime uses. This spin-2 result is an instruction-level profile, not an elapsed-time comparison with DUCC or Torch-harmonics. It places the next optimization effort in the paired recurrence and analysis accumulation. The compact table comparison remains the only measured precomputation result: it helps scalar GL transforms through T255, while the T720 table adds 716.9 MiB and slows eight-thread analysis by 37%. No larger table or Torch-compatible dense tensor was added.

### T511 scalar analysis instruction profile


The paired scalar recurrence visitor accounted for 86.6% of instruction references, and its Pulp recurrence vectorizer accounted for 46.3% as exclusive cost. The paired analysis accumulator vectorizer accounted for 11.6%; its binary-scaled complex multiplication helpers accounted for about 9.0% combined. These instruction counts indicate that recurrence updates and accumulation still dominate scalar analysis. They do not show that the table method should extend to T511 or higher: the compact T720 table increased process peak RSS to 786,656 KiB, added 679 ms of setup, and slowed eight-thread analysis by 37%. The profile also does not compare Rust and DUCC instructions under equivalent builds.

### Rejected two-accumulator analysis experiment

The scalar paired-analysis reducer was changed temporarily to alternate vector blocks between two SIMD accumulators per channel, with the aim of shortening each accumulator's dependency chain. The baseline binary came from `fbc8e76`; the candidate changed only this reducer. Five alternating process pairs ran the native `m1b_bench` release example with reused plans and workspaces, f64 GL STANDARD analysis, one warm-up and five timed samples per process. Runs were 8 pinned CPU cores on the x86-64 CPU under Linux, using Rust 1.99.0, release `opt-level=3`, 16 codegen units, LTO off, runtime AVX2/FMA dispatch, and no `target-cpu=native`. Values are medians of process medians, with ranges across the five process medians, in milliseconds:

| Operation | Baseline 1T | Candidate 1T | Baseline 8T | Candidate 8T |
| --- | ---: | ---: | ---: | ---: |
| Scalar analysis T511 | `26.169 [25.563–27.202]` | `27.467 [27.106–27.729]` | `8.697 [8.404–8.866]` | `8.659 [8.386–8.882]` |
| Scalar analysis T1023 | `190.877 [189.835–193.858]` | `200.935 [199.923–201.892]` | `63.973 [63.277–64.550]` | `64.252 [63.825–65.099]` |
| Spin-1 analysis T511 | `75.860 [75.388–77.121]` | `82.040 [80.157–82.400]` | `21.675 [21.095–22.471]` | `22.189 [21.857–22.378]` |

The candidate was 5.0%, 5.3% and 8.1% slower in the one-thread cases; eight-thread results overlapped or changed by less than 2.4%. The AVX2 `AnalysisPairAccumulation` body grew from 1,006 to 1,919 bytes. Disassembly showed no YMM stack spill loads or stores in the vector body, so register spills do not explain the regression. The candidate passed focused scalar-reference, extreme-finite-input, spin-1 conversion and DUCC tests, but its whole-transform timings did not improve. The source change was reverted. This interleaving variant is not a basis for revisiting prior fused-accumulation prototypes without new evidence.

## Review and further experiments

The read-only Sol review examined parity, cutoffs, equatorial handling, all four scalar operators, exceptional-value fallback, slice ownership and thread safety. It found no correctness blocker. The review asked for an underflow guarantee narrower than the initial code comment and a regression for asymmetric mirrored cutoffs; both are covered in commit `d251af6`. The reviewer also noted that the table's 2.4/33 ms plan setup requires repeated plan use. That setup cost is included above. A separate review of the T72 gate change found no numerical or bounds issue. It flagged the native sample-count statement, which had generalized the earlier three-process runs to the T72 measurements; the methods section now separates T72's five paired process runs with 25 timed samples from the earlier comparisons. The next performance work should continue with scalar and spin-1 recurrence and accumulation at T255–T1023. The precomputed table is not a substitute for reducing the high-band recurrence cost. A separate experiment could test whether a synthesis-only table at T360/T720 is worth its memory when synthesis dominates, but it should keep analysis on recurrence and include setup amortization. FFT optimization remains deferred.

### Reusable Python workspace screen

A temporary Python-plan change cached one `ShtWorkspace` behind a plan-local mutex and reused it across calls. It preserved separate workspaces across plan instances but serialized concurrent calls on the same plan. The candidate passed formatting, Clippy, the all-feature Rust suite and all nine Python tests, including a same-plan concurrent-call check. Load-average telemetry was retained only in local diagnostics. For T511 GL spin-1, three Python-plan run-group medians were `132.955 → 128.867 ms` for one-thread synthesis, `290.418 → 204.866 ms` for one-thread analysis, `164.283 → 170.243 ms` for eight-thread synthesis and `194.699 → 196.993 ms` for eight-thread analysis. The one-thread analysis ranges overlapped near 200 ms (`202.807–325.046` baseline; `196.628–208.499` candidate), and neither eight-thread operation improved. A separate native workspace-allocation sample was 15.369 ms, but it was not a matched before/after whole-transform comparison. No repeatable end-to-end gain was established, so the candidate and its same-plan serialization behavior were reverted. Python plan calls continue to allocate per-call workspaces until a quieter matched benchmark demonstrates a net benefit.

## Follow-up: SIMD recurrence range checks

The paired recurrence checks both adjacent degree states across each latitude lane before deciding to leave the ordinary floating-point range. This protects the established binary-scaled fallback from subnormal/normal boundary changes. A Pulp SIMD scan now evaluates the range predicate for full vectors and retains the scalar reference predicate for the tail. Its ordered-mask selection preserves `f64::max` behavior when one input is NaN; two NaNs still force the scaled path. The test exercises both `Arch::Scalar` and the runtime-dispatched architecture on 17 lanes, covering the vector body and tail. The matched native experiment compared base `b20f7c2` with the SIMD candidate using `examples/m1b_bench.rs`, f64 GL STANDARD analysis at T511/T1023, five timed calls per process, and five alternating process pairs. It used reused plans and workspaces, deterministic benchmark inputs, Rust `1.99.0`, `opt-level=3`, 16 codegen units, LTO off, portable Pulp AVX2/FMA runtime dispatch and no `target-cpu=native`. Runs were 8 pinned CPU cores on the x86-64 CPU under Linux. The table reports medians and min–max across the five process medians in milliseconds:

| Operation | Baseline 1T | SIMD 1T | Change | Baseline 8T | SIMD 8T | Change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar analysis T511 | `31.574 [31.408–32.479]` | `26.070 [25.797–26.414]` | `-17.4%` | `9.412 [9.275–9.869]` | `8.681 [8.472–8.967]` | `-7.8%` |
| Scalar analysis T1023 | `236.031 [232.663–239.510]` | `191.275 [189.024–191.831]` | `-19.0%` | `70.278 [69.872–71.803]` | `64.420 [63.784–64.719]` | `-8.3%` |
| Spin-1 analysis T511 | `93.459 [90.956–94.067]` | `75.354 [74.984–76.292]` | `-19.4%` | `24.061 [23.718–24.854]` | `21.830 [21.748–22.145]` | `-9.3%` |

Synthesis controls were within process variation: scalar T511 changed `-1.6%/-2.5%` and spin-1 T511 changed `0.0%/+1.2%` at one/eight threads. Release DUCC regressions passed for scalar T511/T1023/T2047 on GL/CC/F1 and spin-1/2 T511 on GL/CC/F1. The numerical fixtures do not print aggregate errors; the change does not alter recurrence values, summation order, or tolerances. The read-only Sol review found no correctness blocker, confirmed exceptional-value and boundary behavior, and suggested a mixed-lane test. That test now places a failing lane among otherwise in-range lanes in both the SIMD vector body and the scalar tail. Full Rust, no-default-feature, Clippy, formatting and Python checks passed as recorded in the milestone notes. This is a recurrence-path optimization and does not add or alter a precomputed table.

### Mixed-exponent SIMD analysis accumulation


#### Native timing method and results

`examples/m1b_bench.rs` measured f64 GL STANDARD analysis with full `mmax`, `phi0=0.37`, `nphi=2*lmax+3`, reused plans and workspaces, one warm-up and five timed calls per process. The baseline binary was built from commit `0cdfbcf`; the candidate was built from the final worktree. Three alternating baseline/candidate process pairs ran on 8 pinned CPU cores of an x86-64 CPU under Linux. Rust was `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch (AVX2/FMA), and no `target-cpu=native`. Values are medians of process medians with min–max ranges in milliseconds:

| Operation | Bandlimit | Baseline 1T | Candidate 1T | Change | Baseline 8T | Candidate 8T | Change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar analysis | T255 | `4.771 [4.710–4.906]` | `4.238 [4.230–4.397]` | `-11.2%` | `1.436 [1.420–1.437]` | `1.472 [1.322–1.510]` | `+2.5%` |
| Scalar analysis | T511 | `26.228 [25.818–26.537]` | `23.747 [23.667–23.897]` | `-9.5%` | `8.473 [8.385–8.690]` | `7.505 [7.167–7.562]` | `-11.4%` |
| Scalar analysis | T1023 | `189.855 [189.804–196.505]` | `179.946 [178.149–180.466]` | `-5.2%` | `64.650 [63.751–64.863]` | `60.932 [60.908–62.077]` | `-5.8%` |
| Spin-1 STANDARD analysis | T255 | `14.824 [14.807–15.066]` | `11.687 [11.514–11.751]` | `-21.2%` | `5.447 [5.274–5.448]` | `3.648 [3.616–3.714]` | `-33.0%` |
| Spin-1 STANDARD analysis | T511 | `77.061 [76.703–77.666]` | `66.074 [66.066–66.125]` | `-14.3%` | `22.081 [21.903–22.458]` | `17.132 [17.131–17.370]` | `-22.4%` |
| Spin-1 STANDARD analysis | T1023 | `531.046 [529.801–537.344]` | `493.537 [492.631–500.033]` | `-7.1%` | `152.413 [152.265–153.234]` | `138.055 [137.907–138.901]` | `-9.4%` |

Scalar T255 eight-thread ranges overlap, so that row does not establish a change. Other candidate ranges are separated. The timings reuse native plans and workspaces, excluding plan and workspace construction.

#### DUCC and numerical comparison

The final release Python extension was compared with installed `ducc0==0.41.0`, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, using `examples/python_perf_baseline.py`. The workload was f64 T511 GL (`512x1025`), STANDARD analysis, full `mmax`, `phi0=0.37`; both libraries used explicit one- or eight-thread execution. The Python plan allocates an `ShtWorkspace` per call. Three run groups each took five timed calls. Rust and DUCC medians in milliseconds were:

| Operation | Threads | RustSHT Python plan | DUCC 0.41.0 | Max abs. error | RMS error | Relative L2 error |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar analysis | 1 | `24.125` | `6.496` | `1.238e-14` | `1.779e-16` | `3.484e-14` |
| Scalar analysis | 8 | `7.783` | `1.003` | `1.238e-14` | `1.779e-16` | `3.484e-14` |
| Spin-1 analysis | 1 | `65.218` | `12.604` | `1.788e-14` | `2.619e-16` | `5.128e-14` |
| Spin-1 analysis | 8 | `17.744` | `1.924` | `1.788e-14` | `2.619e-16` | `5.128e-14` |

The scalar eight-thread Python measurements had a `2.930 ms` run-group spread. The native reused-workspace comparison is the performance acceptance measure; RustSHT still takes about 2.5–9.2 times longer in these Python-plan cases. Focused regressions compare mixed scaled and normal lanes with scalar references at 64- and 61-lane widths, verify subnormal basis fallback bitwise, and confirm that extreme finite products that overflow before longitude scaling use the scaled path. `cargo test --workspace --all-features` passed after the final SIMD mask change (128 Rust tests and 10 f32 integration tests; 8 ignored). No-default-feature tests passed (118 Rust and 10 f32 tests; 7 ignored); Clippy, formatting and nine Python tests passed. Release DUCC tests passed (30 passed, 7 ignored), as did the ignored scalar T2047 GL/CC/F1 DUCC and analytic check and the T1023 spin exact-analysis DUCC check. The read-only Sol review found no correctness blocker and requested the all-feature suite after the final mask edit; that suite passed. Full acceptance evidence is recorded in [the milestone notes](../plans/05-validation-and-milestones.md#mixed-exponent-simd-analysis-accumulation).

## T511 spin-1 analysis profile and worker-granularity screen


Temporary timing around the recursive leaves of T511 spin-1 analysis measured eight leaves with estimated degree-lane work between 7.35 and 7.64 million per leaf. Elapsed leaf times ranged from 7.38 to 10.46 ms. All leaves started within 0.35 ms and finished between 7.66 and 10.64 ms, indicating moderate cost spread without a delayed branch. The hooks were removed after profiling. A temporary scheduling candidate allowed recursive analysis to use all 16 recurrence workspaces already present in the T511 workspace, rather than limiting the split to the eight Rayon workers. This doubled the number of order tasks without adding workspaces. Three alternating baseline/candidate process pairs ran release `m1b_bench`, with one warm-up and five timed calls, on the x86-64 CPU, 32 pinned CPU cores. The table reports medians of process medians and min–max ranges, in milliseconds:

| Analysis | Threads | Current schedule | 2x task candidate | Change |
| --- | ---: | ---: | ---: | ---: |
| Scalar T511 | 1 | `24.283 [24.000–24.562]` | `23.684 [23.564–23.726]` | `-2.5%` |
| Scalar T511 | 8 | `7.562 [7.443–7.597]` | `7.712 [7.674–7.961]` | `+2.0%` |
| Spin-1 T511 | 1 | `66.695 [65.734–67.497]` | `65.591 [65.401–66.465]` | `-1.7%` |
| Spin-1 T511 | 8 | `17.744 [17.722–20.961]` | `17.753 [17.151–18.652]` | `+0.1%` |

The one-thread spin difference is not meaningful because that path is serial. Scalar analysis was slower at eight threads, and spin-1 was unchanged within the process spread. Peak RSS samples overlapped. Focused scalar/spin scheduling tests passed, but the scheduling change was reverted and is not accepted. A current-source scaling screen measured T511 scalar analysis medians of 23.736/14.624/10.357/7.715/7.638/7.077 ms and spin-1 medians of 66.764/37.857/26.124/17.617/16.368/13.769 ms at 1/2/4/8/16/32 threads. These values are scaling observations, not paired candidate measurements. The subsequent fast-versus-general comparison confirms that the spin-1 scalar-conversion route is much cheaper through the tested low and high bands. Active-work leaf timing, path-status counters and the twofold-task experiment are now recorded; together they do not support another scheduler change.

## Spin-1 analysis fast-path comparison and coefficient-factor experiment

A temporary binary disabled only spin-1 scalar-conversion fast analysis; spin synthesis and the spin-2 fast path were unchanged. The comparison used `examples/m1b_bench.rs` at f64 GL T63/T127/T255/T511, with the same generated inputs, plans and workspaces, one warm-up and five timed operations per process, and explicit one-/eight-thread pools. Three alternating process pairs were 8 pinned CPU cores on the x86-64 CPU. Both binaries used Rust `1.99.0`, release `opt-level=3`, the default 16 codegen units, LTO off and portable Pulp runtime dispatch (AVX2/FMA); no native-only target flags were used. The production source was not changed by the disabled-path build. Results are process-median medians and min–max ranges, in milliseconds:

| Bandlimit | Fast path 1T | General path 1T | Fast path 8T | General path 8T |
| --- | ---: | ---: | ---: | ---: |
| T63 | `0.376 [0.351–0.464]` | `2.353 [2.331–2.452]` | `0.208 [0.188–0.218]` | `0.445 [0.438–0.499]` |
| T127 | `2.177 [2.015–2.289]` | `17.911 [17.898–17.995]` | `0.958 [0.933–0.969]` | `3.012 [2.979–3.261]` |
| T255 | `11.250 [11.141–11.310]` | `140.362 [140.107–140.937]` | `3.714 [3.638–3.716]` | `24.610 [24.303–24.636]` |
| T511 | `64.722 [64.678–65.032]` | `1083.668 [1082.033–1085.000]` | `17.272 [17.141–17.349]` | `201.072 [200.925–202.373]` |

The scalar-conversion path was 2.1–16.7 times faster at one thread and 2.1–11.6 times faster at eight threads. It materially benefits T63 and T127 as well as high bands; the existing low-band eligibility should remain. The general recurrence is still required for polar rings and unsupported domains. `spin1_standard_analysis_scalar_conversion_matches_general_and_adjoint_on_all_grids` passed for GL, CC and F1 through T255, and the release T511 DUCC/roundtrip case passed on all three grids. The latter checks spin 1 and spin 2; its existing test tolerances and references were not modified. A second candidate reused the scalar recurrence's order factors in the spin-1 scalar-to-spin coefficient conversion. Algebraically, its two coefficient multipliers are `-(l+1) epsilon_(l,m)` and `l epsilon_(l+1,m)`, so the candidate removed repeated square roots from `scalar_to_spin1`. The helper's disassembly shrank from 225 to 209 instructions and its scalar `sqrtsd`/`divsd` counts fell from 3/3 to 1/1. The whole-transform experiment used the same CPU/compiler/flags/workloads and three alternating process pairs:

| Bandlimit | Baseline 1T | Candidate 1T | Baseline 8T | Candidate 8T |
| --- | ---: | ---: | ---: | ---: |
| T63 | `0.351 [0.344–0.364]` | `0.346 [0.329–0.384]` | `0.234 [0.219–0.234]` | `0.233 [0.208–0.255]` |
| T127 | `2.035 [2.008–2.084]` | `1.991 [1.976–2.194]` | `0.957 [0.929–0.973]` | `0.988 [0.964–1.011]` |
| T255 | `11.295 [11.230–11.376]` | `11.231 [11.026–11.472]` | `3.656 [3.603–3.690]` | `3.708 [3.615–3.841]` |
| T511 | `65.860 [65.425–66.209]` | `64.752 [64.562–65.808]` | `16.856 [16.796–17.337]` | `17.640 [16.315–20.247]` |

All baseline and candidate process ranges overlap. The candidate's apparent T511 one-thread reduction is `1.7%`, smaller than the spread; eight-thread times are noisy, with no clear gain. The candidate passed the all-grid scalar/spin recurrence comparison, exact-analysis and adjoint checks, the small DUCC fixture, and the ignored release T511 GL/CC/F1 DUCC test. It used no extra memory but was reverted because fewer helper instructions did not produce a repeatable whole-transform speedup. This result reinforces that arithmetic instruction reductions in the coefficient-conversion tail are not a measured bottleneck at present.

## Direct Torch-harmonics runtime and memory screen

This follow-up directly ran the Torch-harmonics PR #322 source at commit [`37751d3e90b8f12fe9439c52847f4c5d70d5a627`](https://github.com/NVIDIA/torch-harmonics/tree/37751d3e90b8f12fe9439c52847f4c5d70d5a627), based on `97f7d2756d2d2d3b974d8ee31ca04e032fdf1d05`. The temporary environment used Python `3.11.15`, PyTorch `2.14.1+cu130`, CUDA runtime `13.0`, driver `595.104.02`, an NVIDIA GeForce RTX 5070 with `12,227 MiB`, and the x86-64 CPU described above. Optional attention/disco package imports were stubbed only in the temporary environment; the measured SHT calls used Torch-harmonics' PyTorch modules. Neither that checkout nor its dependencies were changed in this repository. Torch modules were `RealSHT`, `InverseRealSHT`, `RealVectorSHT` and `InverseRealVectorSHT`, using f64, `norm="ortho"`, `csphase=True`, full truncation `lmax=mmax=T+1`, and `nphi=2*T+3`. GL and folded CC/F1 analysis were measured. Torch CPU used `torch.set_num_threads(1 or 8)`; CPU affinity was not pinned. GPU input tensors were created on the device, so timed calls exclude host input transfers. Each case had three warmups, five repeated event-timed groups and five synchronized single-call timings in one process. CUDA rows report the median event time; CPU table rows use the median synchronized single-call time because repeated event groups were noisier. These are screening measurements from one process per case, not process-level confidence intervals. The Rust comparator was the release `m1b_bench` built from current commit `476bf1c26fa2018df52a03af61af18f1d56376e5`, with one temporary environment gate to force scalar recurrence instead of an eligible compact table. No numerical implementation source differed from the committed tree. Rust used the same Ryzen CPU, 8 pinned CPU cores, one warmup, five timed samples and reused native plans/workspaces. GL scalar table/recurrence timings were collected in three process runs; the newly collected CC/F1 timings below use one process per configuration and are screening values. Torch CPU timings use five host-synchronized samples from one process. All values are milliseconds and listed as one-thread/eight-thread. The committed Torch screening harness is [`torch_harmonics_bench.py`](torch_harmonics_bench.py). From a checkout with the pinned PR revision installed into the active Python environment, run one configuration per process, for example:

```sh
python reports/torch_harmonics_bench.py 720 0 synthesis GL cuda 1
python reports/torch_harmonics_bench.py 720 1 analysis CC cuda 8
python reports/torch_harmonics_bench.py 255 0 analysis GL one CPU core
```

Arguments are bandlimit, spin, operation, grid, device and Torch CPU thread count. The script reports plan construction, transfer, table bytes, device peaks, process peak RSS, event samples and synchronized samples as JSON. If source checkout lacks optional attention/disco extensions, the script stubs only their availability checks as unavailable; the SHT modules do not use those optional kernels. For Rust recurrence, build the release `m1b_bench` from the stated commit and apply the one-line temporary gate recorded here: in `ScalarTable::new`, add `|| std::env::var_os("RUSTSHT_BENCH_DISABLE_LEGENDRE_TABLE").is_some()` to the existing early-return eligibility condition. Then run, for example:

```sh
RUSTSHT_BENCH_DISABLE_LEGENDRE_TABLE=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 127 1 synthesis GL STANDARD
RUSTSHT_BENCH_DISABLE_LEGENDRE_TABLE=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 720 8 analysis F1 STANDARD
```

The temporary gate was used only in `/tmp`; it was not part of the committed implementation. The Rust harness reports five-sample medians and separately times plan/workspace construction.

### Same-CPU GL execution

| Spin | Bandlimit | Operation | Rust recurrence | Rust compact table | Torch CPU dense table |
| ---: | ---: | --- | ---: | ---: | ---: |
| 0 | T127 | Synthesis | `1.757 / 0.609` | `0.918 / 0.364` | `1.60 / 0.486` |
| 0 | T127 | Analysis | `0.769 / 0.450` | `0.608 / 0.342` | `1.316 / 0.424` |
| 0 | T255 | Synthesis | `12.111 / 3.363` | `6.015 / 1.535` | `15.4 / 7.5` |
| 0 | T255 | Analysis | `4.110 / 1.565` | `4.260 / 1.430` | `15.03 / 6.47` |
| 1 | T127 | Synthesis | `2.934 / 0.926` | n/a | `9.15 / 1.58` |
| 1 | T127 | Analysis | `2.014 / 0.940` | n/a | `8.47 / 1.27` |
| 1 | T255 | Synthesis | `12.223 / 4.379` | n/a | `62.6 / 30.5` |
| 1 | T255 | Analysis | `11.142 / 3.687` | n/a | `58.6 / 28.9` |

Rust compact tables are triangular in `(m,l)` and store the northern latitude half contiguously. The scalar tables were 4.03 MiB at T127 and 32.13 MiB at T255. In this Torch CPU screen, Rust's spin-1 rows used recurrence; the later Rust-only T255 table extension is measured separately below. At T255 Torch's CPU vector table was about 5.1–7.8 times slower than Rust recurrence depending on operation and thread count. Torch's dense scalar table was slower than Rust's T255 compact table for both operations and thread counts; one-thread scalar analysis was also slightly slower than Rust recurrence. At T127 Torch scalar synthesis was close to Rust recurrence, but its 16 MiB table and roughly 36–48 ms setup were not competitive with Rust's compact table for repeated calls. Torch CPU vector timings were substantially slower than Rust recurrence at both bands. These CPU timings support keeping the existing compact-table gates and recurrence implementation; they do not motivate another CPU matrix path. The latest scalar table-versus-recurrence rerun also shows the small-band boundary. At T72, table/recurrent scalar timings were synthesis `0.219/0.209 ms` (1T) and `0.092/0.117 ms` (8T), analysis `0.122/0.174 ms` and `0.083/0.099 ms`. The one-thread synthesis difference favors recurrence; the eight-thread synthesis and analysis ranges overlap. Table plan setup was `0.462 ms` versus `0.027 ms` for recurrence, for a `0.762 MiB` table. At T127, table/recurrent synthesis was `0.918/1.757 ms` and `0.364/0.609 ms`; analysis was `0.608/0.769 ms` and `0.342/0.450 ms`. At T255, synthesis was `6.015/12.111 ms` and `1.535/3.363 ms`; analysis was `4.260/4.110 ms` and `1.430/1.565 ms`. The T255 table improved synthesis by about 50–54%, while its analysis change was `-3.6%` at 1T and `+8.6%` at 8T. T127/T255 figures are medians across three process runs; ranges are shown here:

| Band | Operation | Table 1T | Recurrence 1T | Table 8T | Recurrence 8T |
| --- | --- | ---: | ---: | ---: | ---: |
| T72 | Synthesis | `0.219 [0.186–0.303]` | `0.209 [0.177–0.218]` | `0.092 [0.092–0.109]` | `0.117 [0.108–0.120]` |
| T72 | Analysis | `0.122 [0.122–0.134]` | `0.174 [0.163–0.179]` | `0.083 [0.080–0.090]` | `0.099 [0.084–0.103]` |
| T127 | Synthesis | `0.918 [0.901–0.961]` | `1.757 [1.689–1.765]` | `0.364 [0.335–0.364]` | `0.609 [0.482–0.627]` |
| T127 | Analysis | `0.608 [0.603–0.617]` | `0.769 [0.755–0.781]` | `0.342 [0.337–0.351]` | `0.450 [0.436–0.461]` |
| T255 | Synthesis | `6.015 [5.946–6.039]` | `12.111 [12.038–12.117]` | `1.535 [1.519–1.550]` | `3.363 [3.356–3.376]` |
| T255 | Analysis | `4.260 [4.226–4.503]` | `4.110 [4.080–4.209]` | `1.430 [1.336–1.449]` | `1.565 [1.519–1.597]` |

Setup was `2.336 ms` at T127 and `30.876 ms` at T255, against recurrence setup of `0.018/0.017 ms`. The table remains a repeated-workload specialization, not a universal replacement for recurrence. For spin-1 exact GL analysis, the compact table at T72 measured `0.298/0.124 ms` versus recurrence `0.436/0.195 ms`; at T127 it measured `1.085/0.506 ms` versus recurrence `2.037/0.940 ms` (1T/8T). It used `0.783/4.094 MiB`; first-call medians including lazy creation were `1.228/1.036 ms` at T72 and `5.481/4.731 ms` at T127. Estimated amortization is about 6/13 total calls at T72 and 4/9 at T127 for 1T/8T. The table path remains narrowly scoped to exact f64 GL analysis.

### Torch GPU execution and dense-table cost

The following Torch GL CUDA medians include the PyTorch transform call with input already on the GPU. They are not a same-hardware comparison with the Rust CPU timings above. They show the GPU execution and storage tradeoff of Torch's tensor-contraction approach.

| Bandlimit | Torch scalar table | Torch two-row vector table | Scalar synthesis / analysis | Vector synthesis / analysis |
| --- | ---: | ---: | ---: | ---: |
| T72 | 2.97 MiB | 5.93 MiB | `0.309 / 0.243 ms` | `0.662 / 0.594 ms` |
| T127 | 16 MiB | 32 MiB | `0.333 / 0.247 ms` | `0.672 / 0.609 ms` |
| T255 | 128 MiB | 256 MiB | `0.647 / 0.477 ms` | `1.964 / 1.779 ms` |
| T511 | 1 GiB | 2 GiB | `3.634 / 3.516 ms` | `13.859 / 13.766 ms` |
| T720 | 2.79 GiB | 5.59 GiB | `9.894 / 9.840 ms` | `38.54 / 38.43 ms` |

At T720 GL, the scalar table plan took `2.1–2.19 s` to build and `0.38–0.44 s` to transfer; the vector plans took `9.69–12.70 s` to build and `0.777–1.145 s` to transfer. Measured CUDA peak allocation during a transform was about `3.09 GB` scalar and `6.15–6.23 GB` vector. The corresponding host process peak RSS was about `9.7 GiB` scalar and `30.2 GB` (`28.8 GiB`) for the GL vector construction run. With the GPU otherwise idle, these runs completed; the vector table consumes nearly half the GPU's available memory before another model or application allocates its own tensors. A single-GPU run cannot reproduce the user's OOM condition when other allocations are absent. At T720, plan construction plus transfer is equivalent to about 252–267 scalar GL GPU calls at the measured per-call latency, or 89 CPU cores vector calls. For folded CC/F1 analysis, the corresponding ratios are about 1,880–1,960 scalar calls or 11 CPU cores vector calls. These ratios show the table setup burden relative to Torch's resident-GPU transform time; they are not CPU-versus-GPU break-even points. On the same CPU at T127, Torch scalar synthesis saves only about `0.12–0.16 ms` per call against Rust recurrence while taking roughly `36–48 ms` to build its 16 MiB table, so that comparison needs hundreds of repeated calls to amortize setup. Rust's compact scalar table is already faster than Torch CPU in that case. At T255, the Torch CPU scalar and vector tables are slower per call than Rust recurrence or the compact scalar table, so setup cannot produce a CPU crossover. The next table places the Torch T511/T720 GPU event times beside current Rust native CPU times. It is a cross-device context comparison only. Rust figures are three process medians; Torch CUDA figures are single-process medians.

| Band | Spin | Rust CPU synthesis 1T / 8T | Rust CPU analysis 1T / 8T | Torch GPU synthesis / analysis |
| --- | ---: | ---: | ---: | ---: |
| T511 | 0 | `56.573 / 15.329 ms` | `23.931 / 7.529 ms` | `3.634 / 3.516 ms` |
| T511 | 1 | `59.296 / 21.268 ms` | `65.377 / 16.903 ms` | `13.859 / 13.766 ms` |
| T720 | 0 | `140.244 / 40.942 ms` | `64.521 / 20.338 ms` | `9.894 / 9.840 ms` |
| T720 | 1 | `134.985 / 52.561 ms` | `176.547 / 45.471 ms` | `38.54 / 38.43 ms` |

### Folded CC/F1 projection

PR #322's `_precompute_folded_projection` was measured for exact CC/F1 analysis. It constructs the final projection in bounded order/degree blocks; the temporary real Legendre basis is capped at `2^20` float64 values (8 MiB). The cached result is still a full dense table. The following CC/F1 rows report Torch GPU timing alongside Rust native CPU timing only to show scale; these devices are not comparable for a language-speed claim.

| Grid / band | Spin | Dense projection | Torch build | Host-to-device | Torch GPU analysis | Rust CPU analysis 1T / 8T |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| CC T127 | 0 | 16.125 MiB | 128.6 ms | not recorded | 0.240 ms | `1.168 / 0.775 ms` |
| CC T127 | 1 | 32.26 MiB | 235 ms | not recorded | 0.598 ms | `3.728 / 1.811 ms` |
| F1 T127 | 0 | 16 MiB | 130.5 ms | not recorded | 0.246 ms | `1.166 / 0.780 ms` |
| F1 T127 | 1 | 32 MiB | 209 ms | not recorded | 0.592 ms | `3.729 / 1.930 ms` |
| CC T720 | 0 | 2.80 GiB | 18.796 s | 0.399 s | 9.877 ms | `87.540 / 45.598 ms` |
| CC T720 | 1 | 5.59 GiB | 35.70 s | 0.802 s | 38.72 ms | `251.959 / 99.791 ms` |
| F1 T720 | 0 | 2.79 GiB | 18.141 s | 0.423 s | 9.80 ms | `88.894 / 46.155 ms` |
| F1 T720 | 1 | 5.59 GiB | 35.598 s | 0.769 s | 38.43 ms | `249.488 / 102.786 ms` |

The T720 CC payloads are the exact byte counts `3,002,601,616` and `6,005,203,232` (about 2.80 and 5.59 GiB); the table above uses MiB for readability in the smaller rows and GiB-equivalent units for the larger ones. F1 payloads are `2,998,442,888` and `5,996,885,776` bytes. The CC/F1 construction runs reached peak process RSS of about 6.7 GiB scalar and 12.0 GiB vector on this machine. The folded basis block cap therefore bounds construction scratch, but does not bound the final projection or make the plan's resident memory small. CC/F1 also show that table setup can take tens of seconds at T720 before the first transform.

### Result and limits

The direct CPU screen did not find a Torch dense-table crossover over the current Rust kernels at T127/T255: Torch CPU scalar and vector tables were generally slower than Rust's compact scalar table or recurrence, and their setup and memory costs were higher. Torch's GPU path has much lower transform latency on this RTX 5070 at high bands, but that result combines a different processor, data residency and execution backend; it is not evidence that a dense CPU precompute beats Rust recurrence. The 5.59 GiB vector table and multi-second construction make a Torch-like table unsuitable as an assumed default for a GPU shared with a 12 GB model. No second dense or blocked Rust implementation was added. The current triangular, hemispheric, contiguous Rust table is the relevant CPU table experiment and already uses about one quarter of Torch's scalar GL payload. A bounded CPU basis tile would reduce persistent memory only by recalculating it on each call or by retaining a cache; no measurement here shows that either design improves whole-transform time over Rust's recurrence or its current compact table. The folded CC/F1 projection is a distinct operator; its bounded construction blocks do not establish an advantage for adding that representation to the Rust core. Torch outputs were timed but not numerically compared against Rust or DUCC in this screening run. No production numerical code changed. Existing Rust compact-table versus recurrence and DUCC comparisons, adjoint tests, and high-bandlimit regression results remain the correctness evidence for Rust. CUDA samples were one process per configuration; CPU Torch samples were also single-process screens. GPU/CPU comparisons above must not be interpreted as controlled compiler or language comparisons. No OOM was reproduced, and Torch's temporary setup, device transfer, and resident memory can vary with its cache state and other live allocations.
