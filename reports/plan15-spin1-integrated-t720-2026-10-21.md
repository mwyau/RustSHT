# Plan 15 integrated spin-1 GL/CC/F1 T720 screen

## Scope and source

This focused current-source check exercises the accepted Plan 15 changes on
GL, CC and F1 with exact latitude sampling, even-smooth longitude counts,
full and quarter `mmax`, analysis and synthesis, and q1/q8. It is an integration
screen, not a new optimization A/B or a replacement for the 170-geometry
Phase A baseline. The scalar T64–T256 diagnostic remains a separate completed
Plan 14 interruption.

Production code is the tree at `32bb0a2f`; the recorded source HEAD is
`1033d4994eaf3c783d0eb6276773b54e989084d3`, whose changes are documentation
only. The Rust binary SHA-256 is
`7f35c656bd24e0b437a61468a642c7c750bd0816196448e44eb9171dfa9e5e47`. It uses
Rust 1.99.1 / LLVM 23.1.1 and
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. DUCC is pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, built with GCC 15.2.0; SHT
sources use `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with the
validation driver compiled using `-fno-fast-math`. Both binary hashes and
the fixture checksums are in the raw result JSON.

The Ryzen 9 5950X q1 trials were pinned to physical CPU 0; q8 used CPUs 0–7
within one L3 domain. Rust reused its plan/workspace and DUCC reused its plan;
timed calls include complete transforms and output reset, while setup and
first calls are recorded separately. Three interleaved Rust/DUCC process
pairs were used, with ten calibrated batches near 25 ms for short calls and
individual calls for long operations. The dataset contains six geometries,
24 workload rows, 144 backend worker records and 72 numerical/reset
comparisons. No case failed.

## Results

Latencies are median milliseconds with p10–p90 in parentheses. Ratio is the
median of the three paired process ratios; excess is Rust minus DUCC. Error
is maximum relative L2 across the three paired output checks.

| Grid | Order | Operation | Threads | Rust ms (p10–p90) | DUCC ms (p10–p90) | Ratio | Excess ms | Relative L2 |
|---|---|---|---:|---:|---:|---:|---:|---:|
| CC | reduced | analysis | q1 | 115.546 (80.571–121.157) | 57.112 (56.403–57.799) | 2.04× | +58.434 | 6.83e-14 |
| CC | reduced | synthesis | q1 | 85.131 (83.491–85.919) | 24.620 (24.378–25.283) | 3.46× | +60.511 | 3.63e-13 |
| CC | full | analysis | q1 | 294.838 (201.685–299.906) | 190.483 (186.106–193.892) | 1.54× | +104.355 | 6.10e-14 |
| CC | full | synthesis | q1 | 179.828 (111.558–185.411) | 58.619 (58.167–59.488) | 3.09× | +121.209 | 2.29e-13 |
| F1 | reduced | analysis | q1 | 119.551 (117.999–121.689) | 56.126 (51.703–57.949) | 2.13× | +63.425 | 6.76e-14 |
| F1 | reduced | synthesis | q1 | 47.529 (45.618–76.371) | 23.267 (22.961–23.905) | 1.99× | +24.262 | 8.08e-13 |
| F1 | full | analysis | q1 | 203.072 (201.170–296.189) | 189.465 (134.444–192.262) | 1.51× | +13.607 | 6.11e-14 |
| F1 | full | synthesis | q1 | 103.293 (102.612–173.019) | 57.089 (49.449–57.790) | 2.06× | +46.204 | 5.23e-13 |
| GL | reduced | analysis | q1 | 48.717 (48.009–49.385) | 21.251 (21.049–21.759) | 2.30× | +27.467 | 5.21e-14 |
| GL | reduced | synthesis | q1 | 74.700 (73.400–75.436) | 23.637 (23.451–23.812) | 3.17× | +51.063 | 5.22e-13 |
| GL | full | analysis | q1 | 72.956 (72.337–107.999) | 51.475 (49.158–51.882) | 1.47× | +21.481 | 4.76e-14 |
| GL | full | synthesis | q1 | 171.131 (102.384–173.411) | 55.207 (54.737–56.159) | 3.04× | +115.924 | 3.93e-13 |
| CC | reduced | analysis | q8 | 26.670 (23.188–30.085) | 17.475 (16.650–18.127) | 1.59× | +9.195 | 6.83e-14 |
| CC | reduced | synthesis | q8 | 47.173 (43.313–47.579) | 4.348 (3.855–4.956) | 10.77× | +42.825 | 3.63e-13 |
| CC | full | analysis | q8 | 64.994 (54.771–76.006) | 34.292 (33.920–36.329) | 1.85× | +30.702 | 6.10e-14 |
| CC | full | synthesis | q8 | 90.577 (88.369–91.297) | 10.021 (9.494–11.334) | 9.00× | +80.556 | 2.29e-13 |
| F1 | reduced | analysis | q8 | 26.795 (22.459–31.558) | 17.285 (16.569–18.297) | 1.61× | +9.510 | 6.76e-14 |
| F1 | reduced | synthesis | q8 | 30.707 (25.563–31.528) | 3.756 (3.427–4.333) | 7.99× | +26.951 | 8.08e-13 |
| F1 | full | analysis | q8 | 57.231 (53.569–67.496) | 35.463 (34.471–36.455) | 1.64× | +21.769 | 6.11e-14 |
| F1 | full | synthesis | q8 | 49.108 (48.354–49.789) | 9.393 (8.604–10.986) | 5.10× | +39.715 | 5.23e-13 |
| GL | reduced | analysis | q8 | 8.694 (8.214–10.772) | 3.096 (2.895–4.316) | 2.95× | +5.597 | 5.21e-14 |
| GL | reduced | synthesis | q8 | 28.012 (26.416–29.045) | 3.410 (3.314–3.937) | 8.29× | +24.602 | 5.22e-13 |
| GL | full | analysis | q8 | 20.093 (18.047–21.301) | 7.524 (6.969–8.804) | 2.63× | +12.569 | 4.76e-14 |
| GL | full | synthesis | q8 | 36.029 (33.128–37.556) | 9.490 (8.769–10.829) | 3.79× | +26.539 | 3.93e-13 |

## Interpretation and limits

All outputs were finite and repeat/reset-consistent. Across the 72 paired
checks, maximum absolute error was `5.93e-9` and maximum relative L2 error
was `8.09e-13`. The largest pointwise relative error after excluding
reference entries below `1e-12` of the reference peak was `5.80e-7`; those
near-zero values do not affect the reported relative L2 measure.

The current-source check confirms the remaining large q8 synthesis gaps on
all three grids: CC is 9.00–10.77×, F1 is 5.10–7.99×, and GL is 3.79–8.29×
for these T720 exact-grid cases. The q8 analysis gaps are smaller (1.59–2.95×).
q1 synthesis also retains material excess, particularly CC full order
(`+121.2 ms`) and GL full order (`+115.9 ms`). These are current-source
Rust/DUCC differences, not regressions relative to a prior Rust source.

Median q8 scheduler wait was 9.6% and p90 was 19.3%, so small q8 deltas are
not suitable for attributing source-level gains. q1 median wait was 0.6%,
though some q1 p10–p90 ranges show cross-process regime variation. The JSON
retains each process distribution, first-call/setup latency, runqueue wait,
frequency observations, fixture hashes and binary hashes. This screen does
not isolate stages; the prior F1 q8 phase probe measured recurrence plus
spectrum accumulation as the dominant Rust synthesis phase. The accepted
surplus-group queue reduced T720 q8 F1 Rust synthesis by 25.9% reduced and
41.2% full versus its matched baseline, but that mechanism does not explain
or remove the remaining recurrence gap. The next Plan 15 task is to inspect
the executed CC/F1 spin-1 synthesis recurrence/spectrum path against DUCC at
comparable boundaries; do not reopen the deferred odd-FFT work.

## Validation and reproduction

On this source, serialized release validation passed:

- `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo test --workspace --all-features --release -- --test-threads=1` — 248 unit tests and all integration tests passed; 33 tests ignored.
- `cargo test --release --no-default-features -- --test-threads=1` — 201 unit tests and all integration tests passed; 16 ignored. Rust emitted the existing warnings for private experimental cfg names.
- `cargo fmt --check` and strict all-feature release Clippy passed.

Sanitized summary data and the reproducible runner are in
[`reports/data/plan15-integrated-t720-current-2026-10-21`](data/plan15-integrated-t720-current-2026-10-21/README.md).
Raw per-process captures remain local and untracked. Reproduction uses the
pinned DUCC checkout and command recorded in the data README.
