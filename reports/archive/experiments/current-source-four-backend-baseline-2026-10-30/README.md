# Current-source four-backend GL T720 baseline — 2026-10-30

Exact parent source `afd3df285dcb601371bb5b7ba1ec4fdc56ab1645`; pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. This screen covers all four transform families, full M720/reduced M180 and q1/q8. Each cell has three interleaved warm whole-call pairs. It includes scheduling, Legendre work, FFT and workspace reuse.

- CPU: AMD Ryzen 9 5950X; q1 CPU 0, q8 CPUs 0–7.
- Compiler: rustc 1.99.1; flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- The f64 fixtures are componentwise widenings of matching f32 fixtures.
- `paired-runs.csv` retains each pair’s worker milliseconds, ratios, output comparisons and fixture hashes. `metadata.json` gives binary hashes, source tree and build provenance. No output arrays or binaries are included.

The source-side f32 route is native for B1–B3. B0’s guarded f64 child sidecar declined the fixtures, so its measured route was the native f32 fallback. Every f32 cell misses at least one parity ratio; these are three-pair screens, not parity PASS claims.

## Median whole-call results

| Family | Grid | q | Rust f32 ms | Rust f64 ms | DUCC f32 ms | DUCC f64 ms | f32/f64 | f32/DUCC32 | f64/DUCC64 |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| B1 scalar analysis | Reduced M180 | q1 | 41.981 | 8.766 | 6.293 | 6.492 | 4.816 | 6.756 | 1.348 |
| B1 scalar analysis | Reduced M180 | q8 | 6.734 | 1.463 | 1.044 | 1.072 | 4.604 | 6.563 | 1.351 |
| B0 scalar synthesis | Reduced M180 | q1 | 103.952 | 6.582 | 8.711 | 8.661 | 15.792 | 11.917 | 0.764 |
| B0 scalar synthesis | Reduced M180 | q8 | 25.876 | 1.047 | 1.355 | 1.363 | 23.151 | 19.095 | 0.752 |
| B3 spin-1 analysis | Reduced M180 | q1 | 59.079 | 16.010 | 13.105 | 12.961 | 3.726 | 4.552 | 1.223 |
| B3 spin-1 analysis | Reduced M180 | q8 | 8.922 | 2.604 | 2.036 | 2.016 | 3.654 | 4.449 | 1.304 |
| B2 spin-1 synthesis | Reduced M180 | q1 | 29.274 | 17.306 | 18.385 | 18.569 | 1.674 | 1.592 | 0.932 |
| B2 spin-1 synthesis | Reduced M180 | q8 | 4.912 | 2.928 | 2.825 | 2.782 | 1.696 | 1.739 | 1.043 |
| B1 scalar analysis | Full M720 | q1 | 137.256 | 20.899 | 15.243 | 16.034 | 6.568 | 9.028 | 1.322 |
| B1 scalar analysis | Full M720 | q8 | 19.083 | 3.612 | 2.489 | 2.470 | 5.760 | 7.667 | 1.470 |
| B0 scalar synthesis | Full M720 | q1 | 320.753 | 15.757 | 19.809 | 20.205 | 20.192 | 16.220 | 0.780 |
| B0 scalar synthesis | Full M720 | q8 | 53.853 | 2.468 | 2.995 | 2.975 | 21.822 | 18.191 | 0.826 |
| B3 spin-1 analysis | Full M720 | q1 | 138.920 | 40.030 | 31.880 | 32.511 | 3.481 | 4.371 | 1.234 |
| B3 spin-1 analysis | Full M720 | q8 | 20.219 | 6.518 | 4.759 | 4.919 | 2.987 | 4.112 | 1.312 |
| B2 spin-1 synthesis | Full M720 | q1 | 71.887 | 40.324 | 42.611 | 44.102 | 1.781 | 1.692 | 0.917 |
| B2 spin-1 synthesis | Full M720 | q8 | 11.066 | 6.857 | 6.163 | 6.481 | 1.642 | 1.827 | 1.058 |

This baseline is retained as a compact numerical record. Full per-pair captures and the original runner remain in local experiment scratch; exact fixture, binary and source hashes needed to identify the run are preserved here.
