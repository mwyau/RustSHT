# Scalar T64 recurrence dispatch experiment

## Scope and source versions

Baseline benchmark checkout HEAD: `d68d8e48e982bc1e0ef347e95c744a703cbadf70` (scalar production source last changed at `980e8e4f`); candidate source commit: `0ffcddda3f020d7c7144301de4320e127c5167b5`.
Benchmark case/timing harness: `d68d8e48e982bc1e0ef347e95c744a703cbadf70`; plot-only x-axis adjustment: `949c1adff92c3837df8e763f27ec78d8e13f4e39` (no timing or case-selection code changed there).
Rust: `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (rustc commit `58cf8f9c3`, LLVM 23.1.1) with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; runtime dispatch: Pulp V3 (four f64 lanes, AVX2/FMA).
Pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0); `DUCC sources: -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; benchmark driver: -O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`.
Host: AMD Ryzen 9 5950X 16-Core Processor; 16 visible physical cores; CPU features avx2, fma.

Both sweeps contain 212 configurations at 61 degrees, with 848 matched operation/thread rows. Every deterministic map and coefficient-array SHA-256 matches. Each row has 30 timing batches over three interleaved process pairs per backend. T0 remains in the machine results and numerical coverage, but is excluded from throughput summaries and charts.

Baseline and candidate used separate runs of the same immutable DUCC v3 build; each Rust/DUCC ratio uses its DUCC timing from the corresponding run. Absolute runtime charts show the geometric mean of the two DUCC measurements as a shared reference. No Rust 1.99.0 or older-DUCC timings are mixed into this comparison.

## Dispatch behavior

This comparison first tests a global f64 spin-0 scalar GL rule: recurrence at T64 and above for both analysis and synthesis. That experimental rule is not the final production policy. f32 did not support the compact scalar table and is unchanged. Spin-1 retains its separate table selector and reuse policy.

For full-order, exact-sampling GL, finite moderate inputs, default reuse, the pre-change route matrix is:

| lmax | q1 S | q1 A | q8 S | q8 A |
|---:|---|---|---|---|
| 63 | recurrence | table | table | recurrence |
| 64 | recurrence | table | table | table |
| 127 | recurrence | table | table | table |
| 128 | table | table | table | table |
| 255 | table | recurrence | table | table |
| 256 | recurrence | recurrence | recurrence | recurrence |

The global-cutoff candidate keeps the T63 row and selects recurrence for all four T64 cells and every higher spin-0 bandlimit. Reduced mmax, over-resolved geometries, non-default reuse estimates, and unsafe table inputs can select recurrence below T64 under the baseline exceptions.

## Warm Rust/DUCC latency ratios and Rust-only change

Arithmetic/geometric means below are computed over matched configuration rows in the indicated lmax band, excluding T0. `candidate/base` is the geometric mean of the candidate-to-baseline Rust warm latency; values below 1 indicate candidate improvement. Counts are matched rows, not transform repetitions.

| lmax band | operation | q | n | baseline ratio AM/GM | candidate ratio AM/GM | candidate/base GM |
|---|---|---:|---:|---:|---:|---:|
| T1–T32 | analysis | 1 | 122 | 3.494/3.226 | 3.550/3.262 | 0.805 |
| T1–T32 | analysis | 8 | 122 | 0.322/0.276 | 0.411/0.373 | 0.777 |
| T1–T32 | synthesis | 1 | 122 | 3.428/3.053 | 3.513/3.120 | 0.812 |
| T1–T32 | synthesis | 8 | 122 | 0.368/0.320 | 0.456/0.415 | 0.786 |
| T40–T63 | analysis | 1 | 12 | 1.813/1.657 | 1.781/1.631 | 0.972 |
| T40–T63 | analysis | 8 | 12 | 0.906/0.848 | 0.978/0.922 | 0.938 |
| T40–T63 | synthesis | 1 | 12 | 1.370/1.275 | 1.378/1.285 | 0.975 |
| T40–T63 | synthesis | 8 | 12 | 0.805/0.742 | 0.881/0.832 | 0.909 |
| T64–T127 | analysis | 1 | 24 | 1.769/1.694 | 1.985/1.920 | 0.590 |
| T64–T127 | analysis | 8 | 24 | 0.600/0.533 | 1.634/1.464 | 0.323 |
| T64–T127 | synthesis | 1 | 24 | 1.309/1.279 | 1.275/1.247 | 0.563 |
| T64–T127 | synthesis | 8 | 24 | 0.609/0.496 | 1.315/1.209 | 0.294 |
| T128–T255 | analysis | 1 | 48 | 2.160/1.991 | 2.208/2.047 | 0.555 |
| T128–T255 | analysis | 8 | 48 | 1.059/0.958 | 2.016/1.910 | 0.204 |
| T128–T255 | synthesis | 1 | 48 | 2.087/1.788 | 1.103/1.078 | 0.373 |
| T128–T255 | synthesis | 8 | 48 | 1.135/0.922 | 1.527/1.495 | 0.172 |
| T256 | analysis | 1 | 4 | 1.109/1.096 | 1.172/1.163 | 0.569 |
| T256 | analysis | 8 | 4 | 0.669/0.666 | 1.941/1.918 | 0.259 |
| T256 | synthesis | 1 | 4 | 1.259/1.250 | 1.190/1.184 | 0.598 |
| T256 | synthesis | 8 | 4 | 0.715/0.711 | 2.075/2.046 | 0.279 |

### Longitude and order coverage

These geometric-mean summaries separate the two longitude samplings and full versus quarter-order truncation; the matched CSV contains every configuration, including the additional mmax=1 cases.

| selection | op | q | n | baseline Rust/DUCC GM | candidate Rust/DUCC GM | candidate/base Rust GM |
|---|---|---:|---:|---:|---:|---:|
| odd_minimum | analysis | 1 | 92 | 2.295 | 2.344 | 0.735 |
| odd_minimum | analysis | 8 | 92 | 0.378 | 0.593 | 0.553 |
| odd_minimum | synthesis | 1 | 92 | 2.168 | 2.041 | 0.699 |
| odd_minimum | synthesis | 8 | 92 | 0.430 | 0.638 | 0.543 |
| even_smooth | analysis | 1 | 118 | 2.732 | 2.821 | 0.702 |
| even_smooth | analysis | 8 | 118 | 0.474 | 0.772 | 0.483 |
| even_smooth | synthesis | 1 | 118 | 2.386 | 2.068 | 0.623 |
| even_smooth | synthesis | 8 | 118 | 0.478 | 0.703 | 0.453 |
| full mmax | analysis | 1 | 107 | 2.358 | 2.438 | 0.726 |
| full mmax | analysis | 8 | 107 | 0.476 | 0.750 | 0.496 |
| full mmax | synthesis | 1 | 107 | 2.300 | 1.869 | 0.592 |
| full mmax | synthesis | 8 | 107 | 0.539 | 0.716 | 0.424 |
| quarter mmax | analysis | 1 | 103 | 2.835 | 2.900 | 0.715 |
| quarter mmax | analysis | 8 | 103 | 0.390 | 0.629 | 0.538 |
| quarter mmax | synthesis | 1 | 103 | 2.362 | 2.363 | 0.736 |
| quarter mmax | synthesis | 8 | 103 | 0.387 | 0.629 | 0.577 |

### T63/T64 and T255/T256 warmed and first-call latency

Full mmax, even-smooth GL. First-call values are the runner's first native call after plan/workspace setup; setup remains separate.

| lmax | op | q | baseline/candidate Rust warm ms | baseline/candidate DUCC warm ms | candidate/base Rust | Rust first-call baseline/candidate ms | DUCC first-call baseline/candidate ms | Rust setup baseline/candidate ms | DUCC setup baseline/candidate ms | baseline/candidate Rust/DUCC |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 63 | analysis | 1 | 0.0970628/0.0957992 | 0.0626377/0.0613122 | 0.987 | 0.610317/0.595815 | 0.294763/0.295353 | 0.223124/0.222576 | 0.050277/0.051275 | 1.550/1.562 |
| 63 | analysis | 8 | 0.0610018/0.0574589 | 0.0446249/0.0371295 | 0.942 | 0.088602/0.087644 | 0.561332/0.487291 | 0.554888/0.591306 | 0.0514/0.051146 | 1.367/1.548 |
| 63 | synthesis | 1 | 0.084796/0.0844273 | 0.0637647/0.0630889 | 0.996 | 0.117859/0.113221 | 0.291647/0.279223 | 0.227913/0.226915 | 0.049686/0.051406 | 1.330/1.338 |
| 63 | synthesis | 8 | 0.0597408/0.0498753 | 0.0451087/0.0392805 | 0.835 | 0.595087/0.676666 | 0.625617/0.544829 | 0.529189/0.594231 | 0.059546/0.054933 | 1.324/1.270 |
| 64 | analysis | 1 | 0.205512/0.143806 | 0.152301/0.0734956 | 0.700 | 1.03386/0.163763 | 0.527926/0.294142 | 0.351824/0.234919 | 0.094624/0.053342 | 1.349/1.957 |
| 64 | analysis | 8 | 0.180967/0.0669265 | 0.345759/0.037278 | 0.370 | 0.998751/0.087848 | 1.23491/0.493133 | 1.23517/0.608273 | 0.095556/0.05236 | 0.523/1.795 |
| 64 | synthesis | 1 | 0.188222/0.0982588 | 0.145307/0.0760045 | 0.522 | 0.231289/0.120972 | 0.500633/0.283973 | 0.341965/0.235089 | 0.09307/0.053933 | 1.295/1.293 |
| 64 | synthesis | 8 | 0.153793/0.0513675 | 0.33519/0.0402023 | 0.334 | 1.06408/0.085373 | 1.30516/0.552666 | 1.08552/0.620617 | 0.091147/0.055135 | 0.459/1.278 |
| 255 | analysis | 1 | 2.72585/1.49509 | 2.24031/1.21632 | 0.548 | 3.12889/1.74558 | 3.42603/1.94297 | 1.72298/0.996979 | 1.28068/0.779136 | 1.217/1.229 |
| 255 | analysis | 8 | 5.52169/0.360304 | 2.59173/0.225985 | 0.065 | 50.3916/0.493645 | 4.11239/1.20832 | 2.72372/1.62533 | 1.3353/0.822238 | 2.131/1.594 |
| 255 | synthesis | 1 | 9.01862/1.58106 | 2.18676/1.36852 | 0.175 | 52.8528/1.74664 | 3.37398/1.9676 | 1.66142/0.947435 | 1.42882/0.775109 | 4.124/1.155 |
| 255 | synthesis | 8 | 5.00982/0.413915 | 2.42389/0.233416 | 0.083 | 53.6469/0.576752 | 4.35205/1.27191 | 2.79851/1.53168 | 1.3797/0.887852 | 2.067/1.773 |
| 256 | analysis | 1 | 2.86011/1.5939 | 2.36747/1.27698 | 0.557 | 3.29197/1.77444 | 3.49641/1.96657 | 1.73748/1.02194 | 1.48703/0.81251 | 1.208/1.248 |
| 256 | analysis | 8 | 1.84517/0.376987 | 2.66716/0.234107 | 0.204 | 3.43747/0.512992 | 4.43058/1.26176 | 3.04096/1.73398 | 1.3811/0.858437 | 0.692/1.610 |
| 256 | synthesis | 1 | 2.84861/1.65941 | 2.2268/1.4022 | 0.583 | 3.16727/1.8157 | 3.44141/2.01731 | 1.77547/1.01934 | 1.40279/0.812328 | 1.279/1.183 |
| 256 | synthesis | 8 | 1.68659/0.423144 | 2.53477/0.237929 | 0.251 | 3.67578/0.625064 | 4.33705/1.29007 | 3.03557/1.69436 | 1.48985/0.895076 | 0.665/1.778 |

## Paired-case extrema and numerical agreement

Maximum warm relative L2 errors versus DUCC were 9.027e-14 before and 2.353e-14 after the policy change. Maximum absolute errors were 2.228e-10 and 2.842e-11.
Across 424 baseline and 424 candidate configuration validation suites, finite-output, roundtrip, basis-mode and adjoint checks are retained in the source datasets. The candidate's worst sampled relative numerical row is `T63-m15-even_smooth`.

The largest Rust-only latency reductions and increases are listed for diagnosis; timing overlap is not treated as a meaningful difference.

| operation | q | strongest reduction (case, candidate/base) | largest increase (case, candidate/base) |
|---|---:|---|---|
| analysis | 1 | `T22-m22-even_smooth` 0.477× | `T29-m7-odd_minimum` 1.997× |
| analysis | 8 | `T255-m255-even_smooth` 0.065× | `T28-m28-even_smooth` 2.860× |
| synthesis | 1 | `T254-m254-even_smooth` 0.169× | `T29-m7-odd_minimum` 1.934× |
| synthesis | 8 | `T216-m216-even_smooth` 0.053× | `T28-m28-even_smooth` 2.467× |

## Host diagnostics and interpretation

The baseline run's host summary is:

- Run-queue wait median/p90: 20.4%/79.2%; median CPU frequency: 4.02 GHz.
- Candidate run-queue wait median/p90: 2.3%/35.3%; median CPU frequency: 4.57 GHz.
- Median of the per-row fraction of short batches inside 20–100 ms: baseline 93.3%; candidate 100.0%.

Treat isolated short-degree and q8 movements in view of these host summaries and paired p10–p90 intervals. The table compares complete native operations, including FFT and setup-independent output production; it does not attribute a change to the Legendre recurrence alone.

## Focused boundary reruns and policy decision

The full sweep's baseline and global T64 candidate ran at different host loads. To check the route changes with lower, more similar load, three focused baseline/candidate reruns used the same inputs, GCC 15.2, pinned DUCC v3, q1/q8 and the standard interleaved 30-batch protocol. Each run collected three independent process pairs. Rust and DUCC were interleaved within each source run; baseline and candidate source runs were sequential, not interleaved across revisions.

| rerun | baseline run-queue median/p90 | candidate run-queue median/p90 | baseline/candidate median CPU GHz | baseline/candidate numerical rows |
|---|---:|---:|---:|---:|
| T63/T64 and T255/T256 | 0.7%/16.4% | 1.3%/24.0% | 4.542/4.542 | 72/72 |
| T127/T128 | 0.3%/5.4% | 1.0%/5.5% | 4.567/4.566 | 32/32 |
| T191/T223 | 0.5%/13.3% | 0.8%/16.0% | 4.542/4.542 | 32/32 |

The table below reports full-order GL cases, geometric means over odd-minimum and even-smooth longitude layouts. `candidate/base` compares Rust warm latency; the Rust/DUCC ratios are from each source run. The candidate here is the tested global T64 policy, which forced recurrence for both analysis and synthesis.

| lmax | operation | q | n | candidate/base Rust | baseline Rust/DUCC | candidate Rust/DUCC |
|---:|---|---:|---:|---:|---:|---:|
| 63 | analysis | 1 | 2 | 0.866 | 0.731 | 0.735 |
| 63 | analysis | 8 | 2 | 0.743 | 0.870 | 1.270 |
| 63 | synthesis | 1 | 2 | 0.878 | 0.576 | 0.685 |
| 63 | synthesis | 8 | 2 | 0.789 | 0.735 | 1.175 |
| 64 | analysis | 1 | 2 | 1.304 | 1.241 | 1.607 |
| 64 | analysis | 8 | 2 | 1.004 | 1.849 | 1.843 |
| 64 | synthesis | 1 | 2 | 1.010 | 1.188 | 1.201 |
| 64 | synthesis | 8 | 2 | 0.794 | 1.978 | 1.543 |
| 127 | analysis | 1 | 2 | 1.238 | 2.001 | 2.477 |
| 127 | analysis | 8 | 2 | 1.007 | 3.082 | 3.090 |
| 127 | synthesis | 1 | 2 | 1.014 | 1.265 | 1.281 |
| 127 | synthesis | 8 | 2 | 0.604 | 3.716 | 2.242 |
| 128 | analysis | 1 | 2 | 1.274 | 1.447 | 1.848 |
| 128 | analysis | 8 | 2 | 1.085 | 1.456 | 1.542 |
| 128 | synthesis | 1 | 2 | 0.538 | 1.769 | 0.962 |
| 128 | synthesis | 8 | 2 | 0.280 | 4.272 | 1.184 |
| 191 | analysis | 1 | 2 | 1.153 | 1.803 | 2.068 |
| 191 | analysis | 8 | 2 | 1.057 | 1.887 | 1.774 |
| 191 | synthesis | 1 | 2 | 0.445 | 2.237 | 0.992 |
| 191 | synthesis | 8 | 2 | 0.408 | 4.552 | 1.088 |
| 223 | analysis | 1 | 2 | 1.001 | 2.058 | 2.085 |
| 223 | analysis | 8 | 2 | 0.771 | 2.402 | 1.998 |
| 223 | synthesis | 1 | 2 | 0.397 | 2.367 | 0.940 |
| 223 | synthesis | 8 | 2 | 0.212 | 4.127 | 1.344 |
| 255 | analysis | 1 | 2 | 0.996 | 1.070 | 1.077 |
| 255 | analysis | 8 | 2 | 0.515 | 2.817 | 1.452 |
| 255 | synthesis | 1 | 2 | 0.347 | 3.056 | 1.068 |
| 255 | synthesis | 8 | 2 | 0.239 | 6.576 | 1.579 |
| 256 | analysis | 1 | 2 | 1.011 | 1.289 | 1.301 |
| 256 | analysis | 8 | 2 | 1.146 | 1.650 | 1.670 |
| 256 | synthesis | 1 | 2 | 1.017 | 1.251 | 1.273 |
| 256 | synthesis | 8 | 2 | 1.159 | 1.835 | 1.844 |

For full-order q1 analysis, switching table-selected blocks to recurrence raised Rust latency by 28–33% at T64, 24% at T127, 24–31% at T128, and 14–16% at T191. At T223 it was neutral within these samples. At T128, synthesis recurrence reduced full-order Rust latency by 43–49% at q1 and 69–75% at q8; it also improved table-selected q8 synthesis at T64 and T127. At T255, recurrence improved full-order synthesis at q1/q8 and q8 analysis, while q1 analysis already selected recurrence under the original exception. These outcomes reject one global T64 threshold for both operations.

The retained production policy is operation-specific: f64 scalar synthesis uses recurrence from T64; analysis keeps the prior table/reuse selection and its oversampled/T255 exceptions. f32, spin-1, other grids and public APIs are unchanged. The final policy was checked from a clean worktree at commit `d6382614` over 104 rows at T63/T64/T127/T128/T255/T256, with full/reduced mmax, both longitude samplings, q1/q8, 312 numerical comparisons and maximum relative L2 error 2.353e-14.

| lmax | operation | q | n | final-policy Rust/DUCC GM | Rust warm ms GM | DUCC warm ms GM | max relative L2 |
|---:|---|---:|---:|---:|---:|---:|---:|
| 63 | analysis | 1 | 2 | 0.725 | 0.105818 | 0.146035 | 3.530e-15 |
| 63 | analysis | 8 | 2 | 1.259 | 0.069252 | 0.0550238 | 3.149e-15 |
| 63 | synthesis | 1 | 2 | 0.693 | 0.102432 | 0.147871 | 1.493e-14 |
| 63 | synthesis | 8 | 2 | 1.054 | 0.0657339 | 0.0623947 | 7.071e-15 |
| 64 | analysis | 1 | 2 | 1.230 | 0.117969 | 0.0959365 | 3.608e-15 |
| 64 | analysis | 8 | 2 | 1.664 | 0.0767291 | 0.0461207 | 3.608e-15 |
| 64 | synthesis | 1 | 2 | 1.187 | 0.117665 | 0.0991619 | 4.872e-15 |
| 64 | synthesis | 8 | 2 | 1.530 | 0.068981 | 0.0450926 | 4.872e-15 |
| 127 | analysis | 1 | 2 | 1.998 | 0.555793 | 0.278163 | 8.523e-15 |
| 127 | analysis | 8 | 2 | 3.098 | 0.238591 | 0.0770269 | 8.523e-15 |
| 127 | synthesis | 1 | 2 | 1.263 | 0.387515 | 0.306832 | 6.947e-15 |
| 127 | synthesis | 8 | 2 | 2.151 | 0.173462 | 0.0806585 | 6.947e-15 |
| 128 | analysis | 1 | 2 | 1.443 | 0.662645 | 0.459273 | 8.678e-15 |
| 128 | analysis | 8 | 2 | 1.797 | 0.225923 | 0.125733 | 8.678e-15 |
| 128 | synthesis | 1 | 2 | 1.033 | 0.528585 | 0.51151 | 6.257e-15 |
| 128 | synthesis | 8 | 2 | 1.107 | 0.141387 | 0.127774 | 6.257e-15 |
| 255 | analysis | 1 | 2 | 1.024 | 3.0016 | 2.93196 | 1.536e-14 |
| 255 | analysis | 8 | 2 | 1.426 | 4.45839 | 3.1258 | 1.721e-14 |
| 255 | synthesis | 1 | 2 | 1.138 | 3.26054 | 2.8639 | 1.416e-14 |
| 255 | synthesis | 8 | 2 | 0.596 | 1.85265 | 3.10797 | 1.416e-14 |
| 256 | analysis | 1 | 2 | 1.217 | 2.93285 | 2.40915 | 1.573e-14 |
| 256 | analysis | 8 | 2 | 0.620 | 1.63975 | 2.64339 | 1.573e-14 |
| 256 | synthesis | 1 | 2 | 1.413 | 3.18204 | 2.25172 | 1.280e-14 |
| 256 | synthesis | 8 | 2 | 0.792 | 1.99979 | 2.52505 | 1.280e-14 |

The final-policy run's median/p90 run-queue wait was 3.4%/64.7%; median CPU frequency was 4.53 GHz. Retain q8 ratios as provisional where scheduling pressure was high.

Full focused samples: [matched boundary CSV](./data/scalar-t64-recurrence-transition-2026-10-20/comparison/focused-boundary-comparison.csv), [T63/T64 and T255/T256 baseline rerun](./data/scalar-t64-recurrence-transition-2026-10-20/baseline-boundary-check/results.json), [T127/T128 rerun](./data/scalar-t64-recurrence-transition-2026-10-20/baseline-127-128-check/results.json), [T191/T223 rerun](./data/scalar-t64-recurrence-transition-2026-10-20/baseline-191-223-check/results.json), [final synthesis-only policy run](./data/scalar-t64-recurrence-transition-2026-10-20/final-production-policy/results.json). Candidate counterparts are stored beside each baseline dataset.

## Figures

- [Analysis Rust/DUCC ratio](./data/scalar-t64-recurrence-transition-2026-10-20/comparison/ratio-analysis.svg)
- [Synthesis Rust/DUCC ratio](./data/scalar-t64-recurrence-transition-2026-10-20/comparison/ratio-synthesis.svg)
- [Analysis absolute runtime](./data/scalar-t64-recurrence-transition-2026-10-20/comparison/runtime-analysis.svg)
- [Synthesis absolute runtime](./data/scalar-t64-recurrence-transition-2026-10-20/comparison/runtime-synthesis.svg)
- Each sweep also contains regenerated issue-style ratio, runtime, speedup, full/reduced-mmax and cold/warm figures.

## Reproduction

```sh
git worktree add --detach "${BASELINE_TREE:?set baseline worktree}" d68d8e48e982bc1e0ef347e95c744a703cbadf70
(cd "$BASELINE_TREE" && python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC:?set pinned DUCC checkout}" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --max-lmax 256 --results-dir "$RESULT_ROOT/baseline" --no-report)
git worktree add --detach "${CANDIDATE_TREE:?set candidate worktree}" 0ffcddda3f020d7c7144301de4320e127c5167b5
(cd "$CANDIDATE_TREE" && python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src "$DUCC_SRC" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --max-lmax 256 --results-dir "$RESULT_ROOT/candidate" --no-report)
git worktree add --detach "${FINAL_TREE:?set final-policy worktree}" d6382614d1726e28a8fc0ae1208d3d15516fac39
(cd "$FINAL_TREE" && python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src "$DUCC_SRC" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --lmax 63 --lmax 64 --lmax 127 --lmax 128 --lmax 255 --lmax 256 --results-dir "$RESULT_ROOT/final-production-policy" --no-report)
python3 scripts/compare_scalar_t64_policy.py
```

`matched-comparison.csv` contains all 848 matched case/operation/thread rows and the per-case baseline, candidate, DUCC, improvement, ratio, cold-start and error measurements. The source `results.json` files retain process samples, batch distributions and numerical-suite outputs.
